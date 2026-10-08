use super::*;

pub(super) struct StartupWorkerGuard {
    worker: Option<JoinHandle<Result<(), AsrError>>>,
    stop_requested: Arc<AtomicBool>,
    reaped_probe: Option<Arc<AtomicBool>>,
}

impl StartupWorkerGuard {
    pub(super) fn new(
        worker: JoinHandle<Result<(), AsrError>>,
        stop_requested: Arc<AtomicBool>,
        reaped_probe: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            worker: Some(worker),
            stop_requested,
            reaped_probe,
        }
    }

    pub(super) fn take_worker(&mut self) -> JoinHandle<Result<(), AsrError>> {
        self.worker
            .take()
            .expect("startup worker ownership must be available exactly once")
    }
}

impl Drop for StartupWorkerGuard {
    fn drop(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };
        self.stop_requested.store(true, Ordering::SeqCst);
        reap_cancelled_startup_worker(worker, self.reaped_probe.clone());
    }
}

fn reap_cancelled_startup_worker(
    worker: JoinHandle<Result<(), AsrError>>,
    reaped_probe: Option<Arc<AtomicBool>>,
) {
    // Keep the JoinHandle owned after an async startup future is cancelled. The
    // dedicated reaper may block until a native factory call returns, without
    // blocking a Tokio worker thread or detaching the Moonshine inference thread.
    let worker_slot = Arc::new(Mutex::new(Some(worker)));
    let reaper_slot = worker_slot.clone();
    let reaper_probe = reaped_probe.clone();
    let spawn_result = thread::Builder::new()
        .name("moonshine-startup-reaper".to_string())
        .spawn(move || {
            if let Some(worker) = reaper_slot.lock().take() {
                let _ = worker.join();
            }
            if let Some(probe) = reaper_probe {
                probe.store(true, Ordering::SeqCst);
            }
        });

    if spawn_result.is_err() {
        // Thread creation failure is exceptional, but even here do not detach the
        // inference worker: synchronously join as the fail-closed fallback.
        if let Some(worker) = worker_slot.lock().take() {
            let _ = worker.join();
        }
        if let Some(probe) = reaped_probe {
            probe.store(true, Ordering::SeqCst);
        }
    }
}

fn apply_transcript_updates(
    updates: &[StreamingTranscriptUpdate],
    transcript_state: &mut TranscriptStateMachine,
    metrics: &Mutex<RuntimeMetrics>,
    event_callback: &LocalAsrPipelineEventCallback,
    event_delivery: &Arc<LocalAsrEventDeliveryTracker>,
) {
    for update in updates {
        let latency_ms = match &update {
            StreamingTranscriptUpdate::Partial { latency_ms, .. }
            | StreamingTranscriptUpdate::Final { latency_ms, .. } => *latency_ms,
        };
        let emitted_events = transcript_state.apply(update.clone());
        metrics
            .lock()
            .record_transcript_events(update, &emitted_events, latency_ms);
        for event in emitted_events {
            event_callback(event, event_delivery.begin());
        }
    }
}

pub(super) fn run_worker(
    engine: &mut dyn PipelineEngine,
    pcm_rx: &mut mpsc::Receiver<Vec<u8>>,
    stop_requested: &AtomicBool,
    abort_requested: &AtomicBool,
    metrics: &Mutex<RuntimeMetrics>,
    event_callback: &LocalAsrPipelineEventCallback,
    event_delivery: &Arc<LocalAsrEventDeliveryTracker>,
) -> Result<(), AsrError> {
    let mut terminal_error = None;
    let mut transcript_state = TranscriptStateMachine::default();

    loop {
        if abort_requested.load(Ordering::SeqCst) {
            break;
        }

        let bytes = match pcm_rx.try_recv() {
            Ok(bytes) => bytes,
            Err(TryRecvError::Empty) => {
                if stop_requested.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(WORKER_POLL_INTERVAL);
                continue;
            }
            Err(TryRecvError::Disconnected) => break,
        };

        if bytes.is_empty() {
            continue;
        }

        metrics.lock().record_audio_start_if_needed();
        let pcm = match decode_mono_i16_le(&bytes) {
            Ok(pcm) => pcm,
            Err(error) => {
                record_terminal_error(metrics, event_callback, event_delivery, &error);
                terminal_error = Some(error);
                break;
            }
        };

        let inference_started = Instant::now();
        let inference_result = engine.push_pcm(&pcm);
        metrics
            .lock()
            .record_inference(pcm.len(), inference_started.elapsed());

        match inference_result {
            Ok(updates) => {
                apply_transcript_updates(
                    &updates,
                    &mut transcript_state,
                    metrics,
                    event_callback,
                    event_delivery,
                );
                engine.updates_delivered(&updates);
            }
            Err(error) => {
                record_terminal_error(metrics, event_callback, event_delivery, &error);
                terminal_error = Some(error);
                break;
            }
        }
    }

    let stop_started = Instant::now();
    match engine.stop() {
        Ok(updates) => {
            metrics.lock().record_inference(0, stop_started.elapsed());
            if terminal_error.is_none() && !abort_requested.load(Ordering::SeqCst) {
                apply_transcript_updates(
                    &updates,
                    &mut transcript_state,
                    metrics,
                    event_callback,
                    event_delivery,
                );
                engine.updates_delivered(&updates);
            }
        }
        Err(stop_error) => {
            if terminal_error.is_none() {
                record_terminal_error(metrics, event_callback, event_delivery, &stop_error);
                terminal_error = Some(stop_error);
            }
        }
    }

    if let Some(error) = terminal_error {
        Err(error)
    } else {
        Ok(())
    }
}

fn decode_mono_i16_le(bytes: &[u8]) -> Result<Vec<f32>, AsrError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(AsrError {
            kind: AsrErrorKind::AudioInput,
            message: "Local ASR received a malformed 16-bit PCM chunk.".to_string(),
            retryable: true,
        });
    }
    let samples = AudioResampler::bytes_to_i16(bytes);
    Ok(AudioResampler::i16_to_f32(&samples))
}

fn record_terminal_error(
    metrics: &Mutex<RuntimeMetrics>,
    event_callback: &LocalAsrPipelineEventCallback,
    event_delivery: &Arc<LocalAsrEventDeliveryTracker>,
    error: &AsrError,
) {
    metrics.lock().record_error(error);
    event_callback(
        AsrEvent::Error {
            error: error.clone(),
        },
        event_delivery.begin(),
    );
}
