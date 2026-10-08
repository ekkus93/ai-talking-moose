use super::*;
use crate::test_support::{assert_log_capture_live, capture_logs};
use base64::Engine as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex as StdMutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct PushGate {
    state: StdMutex<(bool, bool)>,
    changed: Condvar,
}

impl PushGate {
    fn block_until_released(&self) {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.changed.notify_all();
        while !state.1 {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn wait_until_entered(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.0 {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.1 = true;
        self.changed.notify_all();
    }
}

#[derive(Default)]
struct FakeState {
    pushes: AtomicUsize,
    stops: AtomicUsize,
    received_pcm: StdMutex<Vec<Vec<f32>>>,
    fail_push: StdMutex<Option<AsrError>>,
    updates: StdMutex<Vec<StreamingTranscriptUpdate>>,
    stop_updates: StdMutex<Vec<StreamingTranscriptUpdate>>,
    push_gate: StdMutex<Option<Arc<PushGate>>>,
    block_push: AtomicBool,
    worker_thread: StdMutex<Option<thread::ThreadId>>,
}

struct FakeEngine {
    state: Arc<FakeState>,
    sample_rate: u32,
}

impl PipelineEngine for FakeEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        self.sample_rate
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        *self.state.worker_thread.lock().unwrap() = Some(thread::current().id());
        self.state.pushes.fetch_add(1, Ordering::SeqCst);
        self.state.received_pcm.lock().unwrap().push(pcm.to_vec());
        // Clone the optional gate while holding the mutex, then release the
        // mutex before blocking. Keeping the temporary lock guard alive across
        // block_until_released() deadlocks tests that clear the gate before
        // releasing the worker.
        let push_gate = self.state.push_gate.lock().unwrap().clone();
        if let Some(gate) = push_gate {
            gate.block_until_released();
        }
        while self.state.block_push.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
        }
        if let Some(error) = self.state.fail_push.lock().unwrap().clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut *self.state.updates.lock().unwrap()))
    }

    fn stop(&mut self) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        self.state.stops.fetch_add(1, Ordering::SeqCst);
        Ok(std::mem::take(
            &mut *self.state.stop_updates.lock().unwrap(),
        ))
    }
}

fn callback_events() -> (
    LocalAsrPipelineEventCallback,
    Arc<StdMutex<Vec<LocalAsrPipelineEvent>>>,
) {
    let events = Arc::new(StdMutex::new(Vec::new()));
    let callback_events = events.clone();
    let callback: LocalAsrPipelineEventCallback = Arc::new(move |event| {
        callback_events.lock().unwrap().push(event);
    });
    (callback, events)
}

async fn fake_pipeline(state: Arc<FakeState>) -> LocalAsrPipeline {
    fake_pipeline_for_architecture(LocalAsrArchitecture::MoonshineTinyStreaming, state).await
}

async fn fake_pipeline_for_architecture(
    architecture: LocalAsrArchitecture,
    state: Arc<FakeState>,
) -> LocalAsrPipeline {
    let (callback, _) = callback_events();
    LocalAsrPipeline::start_architecture(
        architecture,
        local_asr_queue_capacity(architecture),
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
        WORKER_STARTUP_TIMEOUT,
        None,
    )
    .await
    .unwrap()
}

fn wait_until(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for worker");
        thread::sleep(Duration::from_millis(1));
    }
}

async fn wait_until_async(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for worker");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[tokio::test]
async fn cancel_during_readiness_cooperatively_stops_and_reaps_worker() {
    let state = Arc::new(FakeState::default());
    let factory_entered = Arc::new(AtomicBool::new(false));
    let release_factory = Arc::new(AtomicBool::new(false));
    let reaped = Arc::new(AtomicBool::new(false));
    let (callback, _) = callback_events();

    let worker_state = state.clone();
    let entered = factory_entered.clone();
    let release = release_factory.clone();
    let reaped_for_start = reaped.clone();
    let start_task = tokio::spawn(async move {
        LocalAsrPipeline::start_with_factory_and_reaper_probe(
            move || {
                entered.store(true, Ordering::SeqCst);
                while !release.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(1));
                }
                Ok(Box::new(FakeEngine {
                    state: worker_state,
                    sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
                }) as Box<dyn PipelineEngine>)
            },
            callback,
            reaped_for_start,
        )
        .await
    });

    wait_until_async(|| factory_entered.load(Ordering::SeqCst)).await;
    start_task.abort();
    let join_error = match start_task.await {
        Ok(_) => panic!("aborted startup task unexpectedly completed"),
        Err(error) => error,
    };
    assert!(join_error.is_cancelled());

    release_factory.store(true, Ordering::SeqCst);
    wait_until_async(|| state.stops.load(Ordering::SeqCst) == 1).await;
    wait_until_async(|| reaped.load(Ordering::SeqCst)).await;
}

#[tokio::test]
async fn readiness_timeout_returns_without_detaching_worker() {
    let state = Arc::new(FakeState::default());
    let reaped = Arc::new(AtomicBool::new(false));
    let (callback, _) = callback_events();
    let worker_state = state.clone();
    let reaped_for_start = reaped.clone();

    let started = Instant::now();
    let result = LocalAsrPipeline::start_with_factory_and_reaper_probe(
        move || {
            thread::sleep(WORKER_STARTUP_TIMEOUT + Duration::from_millis(75));
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }) as Box<dyn PipelineEngine>)
        },
        callback,
        reaped_for_start,
    )
    .await;

    let error = result
        .err()
        .expect("startup should have a bounded readiness timeout");
    assert_eq!(error.kind, AsrErrorKind::RuntimeUnavailable);
    assert!(started.elapsed() < WORKER_STARTUP_TIMEOUT + Duration::from_millis(200));
    wait_until_async(|| state.stops.load(Ordering::SeqCst) == 1).await;
    wait_until_async(|| reaped.load(Ordering::SeqCst)).await;
}

#[tokio::test]
async fn worker_runs_off_the_async_caller_thread() {
    let caller_thread = thread::current().id();
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    assert_ne!(
        state.worker_thread.lock().unwrap().as_ref(),
        Some(&caller_thread)
    );
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn ingress_is_hard_bounded_and_drops_newest() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline = fake_pipeline(state.clone()).await;
    let sender = pipeline.test_sender();

    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }
    assert_eq!(
        pipeline.diagnostics().queue_depth,
        LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );
    assert!(matches!(
        sender.try_send(vec![0, 0]),
        Err(mpsc::error::TrySendError::Full(_))
    ));

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(pipeline.diagnostics().queue_depth, 0);
}

#[tokio::test]
async fn wake_handoff_primes_existing_ingress_in_exact_sample_order() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    let samples = vec![i16::MIN, -1234, 0, 2345, i16::MAX];
    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, samples.clone()).unwrap();

    pipeline.prime_wake_handoff(handoff).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);

    let expected = AudioResampler::i16_to_f32(&samples);
    assert_eq!(state.received_pcm.lock().unwrap().as_slice(), &[expected]);
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn wake_handoff_queue_full_fails_without_dropping_or_reordering_payload() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline = fake_pipeline(state.clone()).await;
    let sender = pipeline.test_sender();
    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }

    let handoff =
        WakeCommandHandoffAudio::new(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ, vec![7, 8, 9]).unwrap();
    let error = pipeline.prime_wake_handoff(handoff).unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::AudioInput);

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn converts_capture_i16_le_to_engine_f32() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    let samples = [i16::MIN, 0, i16::MAX];
    let bytes = AudioResampler::i16_to_bytes(&samples);
    pipeline.test_sender().try_send(bytes).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);

    {
        let received = state.received_pcm.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert!((received[0][0] + 1.0).abs() < 0.0001);
        assert!(received[0][1].abs() < 0.0001);
        assert!((received[0][2] - (32767.0 / 32768.0)).abs() < 0.0001);
    }
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn malformed_pcm_is_typed_terminal_audio_error() {
    let state = Arc::new(FakeState::default());
    let (callback, events) = callback_events();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![1]).unwrap();
    wait_until(|| !pipeline.is_running());
    assert_eq!(
        pipeline.diagnostics().last_error.unwrap().kind,
        AsrErrorKind::AudioInput
    );
    assert!(matches!(
        events.lock().unwrap().as_slice(),
        [AsrEvent::Error {
            error: AsrError {
                kind: AsrErrorKind::AudioInput,
                ..
            }
        }]
    ));
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::AudioInput
    );
}

#[tokio::test]
async fn inference_error_is_preserved_and_emitted() {
    let state = Arc::new(FakeState::default());
    *state.fail_push.lock().unwrap() = Some(AsrError {
        kind: AsrErrorKind::Inference,
        message: "fake inference failure".to_string(),
        retryable: true,
    });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| !pipeline.is_running());
    assert_eq!(
        pipeline.diagnostics().last_error.unwrap().kind,
        AsrErrorKind::Inference
    );
    assert!(matches!(
        events.lock().unwrap().as_slice(),
        [AsrEvent::Error {
            error: AsrError {
                kind: AsrErrorKind::Inference,
                ..
            }
        }]
    ));
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::Inference
    );
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn transcript_updates_cross_worker_boundary() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 7,
            text: "hello".to_string(),
            latency_ms: 1,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| !events.lock().unwrap().is_empty());
    {
        let events = events.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            [
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::PartialTranscript {
                    text: "hello".to_string(),
                },
            ]
        );
    }
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn diagnostics_measure_audio_latency_rtf_cpu_and_memory_without_fabrication() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 11,
            text: "measured".to_string(),
            latency_ms: 17,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();

    let pcm = vec![0_i16; 1_600];
    pipeline
        .test_sender()
        .try_send(AudioResampler::i16_to_bytes(&pcm))
        .unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    thread::sleep(Duration::from_millis(5));
    state.block_push.store(false, Ordering::SeqCst);
    wait_until(|| events.lock().unwrap().len() >= 2);

    let diagnostics = pipeline.diagnostics();
    assert_eq!(diagnostics.processed_audio_ms, 100);
    assert!(diagnostics.inference_wall_time_ms >= 5);
    assert!(diagnostics.real_time_factor.is_some_and(|rtf| rtf > 0.0));
    assert!(diagnostics.first_partial_latency_ms.is_some());
    assert_eq!(diagnostics.last_transcription_latency_ms, Some(17));
    assert!(diagnostics.process_cpu_time_ms.is_some());
    assert!(diagnostics.average_cpu_utilization_percent.is_some());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        assert!(diagnostics.baseline_resident_memory_bytes.is_some());
        assert!(diagnostics.resident_memory_bytes.is_some());
        assert!(diagnostics.peak_resident_memory_bytes.is_some());
    }

    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn blank_native_update_does_not_count_as_first_useful_transcript() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 12,
            text: "   ".to_string(),
            latency_ms: 13,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();

    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    wait_until(|| pipeline.diagnostics().last_transcription_latency_ms == Some(13));

    let diagnostics = pipeline.diagnostics();
    assert!(events.lock().unwrap().is_empty());
    assert_eq!(diagnostics.first_partial_latency_ms, None);
    assert_eq!(diagnostics.first_final_latency_ms, None);
    assert_eq!(diagnostics.last_transcription_latency_ms, Some(13));

    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn final_transcript_crosses_worker_as_provider_neutral_lifecycle() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 8,
            text: "complete".to_string(),
            latency_ms: 1,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| events.lock().unwrap().len() == 3);
    {
        let events = events.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            [
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::FinalTranscript {
                    text: "complete".to_string(),
                },
                AsrEvent::SpeechEnded { monotonic_ms: None },
            ]
        );
    }
    pipeline.stop_and_join().await.unwrap();
}

#[test]
fn sensitive_asr_payloads_are_processed_without_entering_tracing() {
    const TRANSCRIPT: &str = "PRIVATE_ASR_TRANSCRIPT_7f0f9c";
    const RAW_PCM: &[u8] = b"RAW_AUDIO_PCM_181!";
    let pcm_bytes = RAW_PCM.to_vec();
    let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 181,
            text: TRANSCRIPT.to_string(),
            latency_ms: 1,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();

    let (_, logs) = capture_logs(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut pipeline = LocalAsrPipeline::start_with_factory(
                move || {
                    Ok(Box::new(FakeEngine {
                        state: worker_state,
                        sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
                    }))
                },
                callback,
            )
            .await
            .unwrap();
            pipeline.test_sender().try_send(pcm_bytes).unwrap();
            wait_until(|| events.lock().unwrap().len() == 3);

            assert!(matches!(
                events.lock().unwrap().as_slice(),
                [
                    AsrEvent::SpeechStarted { .. },
                    AsrEvent::FinalTranscript { text },
                    AsrEvent::SpeechEnded { .. },
                ] if text.as_str() == TRANSCRIPT
            ));
            {
                let received = state.received_pcm.lock().unwrap();
                assert_eq!(
                    received.len(),
                    1,
                    "raw PCM must cross the production worker boundary"
                );
                assert!(!received[0].is_empty());
            }
            pipeline.stop_and_join().await.unwrap();
        });
    });

    assert_log_capture_live(&logs);
    assert!(!logs.contains(TRANSCRIPT));
    assert!(!logs.contains(std::str::from_utf8(RAW_PCM).unwrap()));
    assert!(!logs.contains(&pcm_base64));
}

#[tokio::test]
async fn stop_is_idempotent_and_joins_worker() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.stop_and_join().await.unwrap();
    pipeline.stop_and_join().await.unwrap();
    assert!(!pipeline.is_running());
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stop_drains_accepted_queued_audio_before_finalization() {
    let state = Arc::new(FakeState::default());
    let gate = Arc::new(PushGate::default());
    *state.push_gate.lock().unwrap() = Some(gate.clone());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 42,
            text: "drained utterance".to_string(),
            latency_ms: 3,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    let sender = pipeline.test_sender();
    // AudioCapture emits 100 ms mono PCM at 16 kHz: 1,600 signed 16-bit
    // samples, or 3,200 bytes per accepted chunk. Distinct values prove FIFO.
    let chunks: Vec<Vec<u8>> = [1_i16, 2, 3]
        .into_iter()
        .map(|sample| sample.to_le_bytes().repeat(1_600))
        .collect();

    // The worker deterministically blocks inside the first accepted push. The
    // next two chunks therefore remain queued when normal stop is requested.
    sender.try_send(chunks[0].clone()).unwrap();
    gate.wait_until_entered();
    sender.try_send(chunks[1].clone()).unwrap();
    sender.try_send(chunks[2].clone()).unwrap();

    pipeline.request_stop();
    *state.push_gate.lock().unwrap() = None;
    gate.release();
    pipeline.stop_and_join().await.unwrap();

    assert_eq!(state.pushes.load(Ordering::SeqCst), 3);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert!(
        !pipeline.is_running(),
        "worker must be retired after finalization"
    );
    let received = state.received_pcm.lock().unwrap();
    assert_eq!(received.len(), 3);
    for (actual, chunk) in received.iter().zip(chunks.iter()) {
        assert_eq!(actual.len(), 1_600);
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32_768.0;
        assert!(actual
            .iter()
            .all(|value| (*value - sample).abs() < f32::EPSILON));
    }
    drop(received);
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::FinalTranscript {
                text: "drained utterance".to_string(),
            },
            AsrEvent::SpeechEnded { monotonic_ms: None },
        ]
    );
}

#[tokio::test]
async fn sub_threshold_stop_final_crosses_normal_path_exactly_once() {
    let state = Arc::new(FakeState::default());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 77,
            text: "tail words".to_string(),
            latency_ms: 4,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();

    // One two-byte PCM sample is far below Whisper's partial cadence. It is
    // accepted before stop and the final produced by engine.stop still crosses
    // the ordinary transcript state machine exactly once.
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    pipeline.stop_and_join().await.unwrap();
    pipeline.stop_and_join().await.unwrap();

    assert_eq!(state.pushes.load(Ordering::SeqCst), 1);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::FinalTranscript {
                text: "tail words".to_string(),
            },
            AsrEvent::SpeechEnded { monotonic_ms: None },
        ]
    );
}

#[tokio::test]
async fn whitespace_stop_final_closes_speech_without_user_transcript() {
    let state = Arc::new(FakeState::default());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 78,
            text: "   ".to_string(),
            latency_ms: 2,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();

    pipeline.stop_and_join().await.unwrap();

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::SpeechEnded { monotonic_ms: None },
        ]
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, AsrEvent::FinalTranscript { .. })));
}

#[tokio::test]
async fn startup_failure_never_exposes_running_pipeline() {
    let (callback, _) = callback_events();
    let result = LocalAsrPipeline::start_with_factory(
        || {
            Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: "missing".to_string(),
                retryable: true,
            })
        },
        callback,
    )
    .await;
    let error = match result {
        Ok(_) => panic!("startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, AsrErrorKind::ModelNotInstalled);
}

#[tokio::test]
async fn unexpected_engine_sample_rate_fails_before_capture() {
    let state = Arc::new(FakeState::default());
    let (callback, _) = callback_events();
    let result = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: 48_000,
            }))
        },
        callback,
    )
    .await;
    let error = match result {
        Ok(_) => panic!("startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, AsrErrorKind::Internal);
}

#[tokio::test]
async fn mock_capture_uses_same_authoritative_capture_at_16khz() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let mut capture = AudioCapture::new_mock();
    pipeline.start_capture(&mut capture, None, None).unwrap();
    let diagnostics = capture.diagnostics();
    assert!(diagnostics.active);
    assert_eq!(
        diagnostics.sample_rate_hz,
        Some(LOCAL_ASR_INPUT_SAMPLE_RATE_HZ)
    );
    assert_eq!(diagnostics.channels, Some(1));
    capture.stop();
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn processed_chunk_releases_queue_depth() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    wait_until(|| pipeline.diagnostics().queue_depth == 0);
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn stopped_pipeline_refuses_to_start_microphone() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    pipeline.stop_and_join().await.unwrap();
    let mut capture = AudioCapture::new_mock();
    let error = pipeline
        .start_capture(&mut capture, None, None)
        .unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::InvalidState);
    assert!(!capture.is_active());
}

#[tokio::test]
async fn drop_stops_and_joins_worker_as_safety_net() {
    let state = Arc::new(FakeState::default());
    {
        let pipeline = fake_pipeline(state.clone()).await;
        assert!(pipeline.is_running());
    }
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn small_pipeline_uses_same_bounded_worker_and_reports_small_architecture() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline_for_architecture(
        LocalAsrArchitecture::MoonshineSmallStreaming,
        state.clone(),
    )
    .await;

    let diagnostics = pipeline.diagnostics();
    assert_eq!(
        diagnostics.architecture,
        LocalAsrArchitecture::MoonshineSmallStreaming
    );
    assert_eq!(diagnostics.input_sample_rate_hz, 16_000);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);

    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn whisper_pipeline_uses_its_own_inference_absorption_queue() {
    let state = Arc::new(FakeState::default());
    let mut pipeline =
        fake_pipeline_for_architecture(LocalAsrArchitecture::WhisperSmall, state).await;

    let diagnostics = pipeline.diagnostics();
    assert_eq!(diagnostics.architecture, LocalAsrArchitecture::WhisperSmall);
    assert_eq!(
        diagnostics.queue_capacity,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );
    assert_eq!(diagnostics.queue_depth, 0);

    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn pipeline_diagnostics_report_bound_and_running_state() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let diagnostics = pipeline.diagnostics();
    assert_eq!(
        diagnostics.architecture,
        LocalAsrArchitecture::MoonshineTinyStreaming
    );
    assert_eq!(diagnostics.input_sample_rate_hz, 16_000);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
    assert_eq!(diagnostics.queue_depth, 0);
    assert!(diagnostics.running);
    assert!(diagnostics.last_error.is_none());
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn wake_word_stability_continuous_asr_idle_cpu_measurement_reports_diagnostics() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let observation = Duration::from_secs(2);

    tokio::time::sleep(observation).await;

    let diagnostics = pipeline.diagnostics();
    let continuous_asr_idle_cpu_percent = diagnostics
        .average_cpu_utilization_percent
        .expect("continuous ASR idle diagnostics should include CPU utilization");
    assert!(continuous_asr_idle_cpu_percent.is_finite());
    assert!(diagnostics.running);
    assert_eq!(diagnostics.queue_depth, 0);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
    println!(
        "WWR630_CONTINUOUS_ASR_IDLE_CPU continuous_asr_idle_cpu_percent={:.3} observation_ms={} queue_depth={} queue_capacity={} architecture={:?}",
        continuous_asr_idle_cpu_percent,
        observation.as_millis(),
        diagnostics.queue_depth,
        diagnostics.queue_capacity,
        diagnostics.architecture
    );

    pipeline.stop_and_join().await.unwrap();
}
