use crate::app::wake_word_command_handoff::WakeCommandHandoffAudio;
use crate::asr::lifecycle::LocalAsrResource;
use crate::asr::moonshine::{MoonshineModelInstaller, MoonshineSmallEngine, MoonshineTinyEngine};
use crate::asr::runtime_metrics::RuntimeMetrics;
use crate::asr::transcript_state::{StreamingTranscriptUpdate, TranscriptStateMachine};
use crate::asr::types::LocalAsrArchitecture;
use crate::asr::types::LocalAsrRuntimeDiagnostics;
use crate::asr::whisper::WhisperModelInstaller;
use crate::asr::{AsrError, AsrErrorKind, AsrEvent};
use crate::audio::capture::AudioCapture;
use crate::audio::resample::AudioResampler;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::{mpsc, oneshot, Notify};
use tracing::debug;

#[path = "pipeline/worker.rs"]
mod worker;
use worker::{run_worker, StartupWorkerGuard};

/// Hard bound for microphone chunks waiting on local Moonshine inference.
///
/// `AudioCapture` emits 100 ms chunks at the requested target rate, so eight
/// queued chunks cap waiting microphone audio at roughly 800 ms. The producer
/// never blocks the CPAL callback: when this queue is full, the newest chunk is
/// dropped by `AudioCapture`, which owns the authoritative overload counter.
pub const LOCAL_ASR_QUEUE_CAPACITY_CHUNKS: usize = 8;

/// Whisper runs whole-utterance CPU inference for each partial. Keep up to 5.6
/// seconds queued during that synchronous work so normal capture is not lost.
pub const WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS: usize = 56;

pub const fn local_asr_queue_capacity(architecture: LocalAsrArchitecture) -> usize {
    match architecture {
        LocalAsrArchitecture::WhisperSmall => WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        LocalAsrArchitecture::MoonshineTinyStreaming
        | LocalAsrArchitecture::MoonshineSmallStreaming => LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
    }
}

pub(crate) const LOCAL_ASR_INPUT_SAMPLE_RATE_HZ: u32 = 16_000;

const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(25);

const PRODUCTION_WORKER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const WORKER_STARTUP_TIMEOUT: Duration = Duration::from_millis(250);

/// Provider-neutral event emitted by the dedicated local-ASR inference worker.
pub type LocalAsrPipelineEvent = AsrEvent;

/// Bounded-worker diagnostics exposed to the ASR diagnostics command and tests.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalAsrPipelineDiagnostics {
    pub architecture: LocalAsrArchitecture,
    pub input_sample_rate_hz: u32,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub running: bool,
    pub last_error: Option<AsrError>,
    pub first_partial_latency_ms: Option<u64>,
    pub first_final_latency_ms: Option<u64>,
    pub last_transcription_latency_ms: Option<u32>,
    pub processed_audio_ms: u64,
    pub inference_wall_time_ms: u64,
    pub real_time_factor: Option<f32>,
    pub process_cpu_time_ms: Option<u64>,
    pub average_cpu_utilization_percent: Option<f32>,
    pub baseline_resident_memory_bytes: Option<u64>,
    pub resident_memory_bytes: Option<u64>,
    pub peak_resident_memory_bytes: Option<u64>,
}

pub type LocalAsrPipelineEventCallback =
    Arc<dyn Fn(LocalAsrPipelineEvent, LocalAsrEventDeliveryAck) + Send + Sync>;

#[derive(Default)]
pub(crate) struct LocalAsrEventDeliveryTracker {
    pending: AtomicUsize,
    changed: Notify,
}

pub struct LocalAsrEventDeliveryAck {
    tracker: Arc<LocalAsrEventDeliveryTracker>,
}

impl LocalAsrEventDeliveryTracker {
    pub(crate) fn begin(self: &Arc<Self>) -> LocalAsrEventDeliveryAck {
        self.pending.fetch_add(1, Ordering::SeqCst);
        LocalAsrEventDeliveryAck {
            tracker: self.clone(),
        }
    }

    pub(crate) async fn wait_until_idle(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.pending.load(Ordering::SeqCst) == 0 {
                return;
            }
            changed.await;
        }
    }
}

impl Drop for LocalAsrEventDeliveryAck {
    fn drop(&mut self) {
        if self.tracker.pending.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.tracker.changed.notify_waiters();
        }
    }
}

pub trait PipelineEngine: Send {
    fn input_sample_rate_hz(&self) -> u32;
    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError>;
    fn stop(&mut self) -> Result<Vec<StreamingTranscriptUpdate>, AsrError>;

    /// Called after the pipeline has applied updates and synchronously emitted
    /// their events. Engines may release per-utterance state after final delivery.
    fn updates_delivered(&mut self, _updates: &[StreamingTranscriptUpdate]) {}
}

impl PipelineEngine for MoonshineTinyEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        MoonshineTinyEngine::input_sample_rate_hz(self)
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        MoonshineTinyEngine::push_pcm(self, pcm)
    }

    fn stop(&mut self) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        MoonshineTinyEngine::stop(self)?;
        Ok(Vec::new())
    }
}

/// Bounded microphone-to-Moonshine Tiny/Small pipeline.
///
/// The pipeline does not create another microphone. `start_capture` starts the
/// caller-owned authoritative `AudioCapture` and gives it this pipeline's
/// bounded ingress sender. AudioCapture performs device-format conversion,
/// downmixing, and resampling before emitting 16-bit mono PCM at 16 kHz.
/// The dedicated OS worker converts those bounded chunks to `f32` and performs
/// all native Moonshine inference off Tokio and off the CPAL callback thread.
pub struct LocalAsrPipeline {
    architecture: LocalAsrArchitecture,
    input_sample_rate_hz: u32,
    queue_capacity_chunks: usize,
    pcm_sender: Option<mpsc::Sender<Vec<u8>>>,
    running: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
    abort_requested: Arc<AtomicBool>,
    metrics: Arc<Mutex<RuntimeMetrics>>,
    event_delivery: Arc<LocalAsrEventDeliveryTracker>,
    worker: Option<JoinHandle<Result<(), AsrError>>>,
}

impl LocalAsrPipeline {
    /// Start the production Moonshine Tiny worker. Model verification and
    /// native engine construction occur on the dedicated worker thread.
    pub async fn start_tiny(
        installer: Arc<MoonshineModelInstaller>,
        event_callback: LocalAsrPipelineEventCallback,
    ) -> Result<Self, AsrError> {
        Self::start_architecture(
            LocalAsrArchitecture::MoonshineTinyStreaming,
            LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
            move || {
                MoonshineTinyEngine::open(&installer)
                    .map(|engine| Box::new(engine) as Box<dyn PipelineEngine>)
            },
            event_callback,
            PRODUCTION_WORKER_STARTUP_TIMEOUT,
            None,
        )
        .await
    }

    /// Start the production Moonshine Small worker with the same bounded queue,
    /// transcript state machine, lifecycle, and local-only microphone contract.
    pub async fn start_small(
        installer: Arc<MoonshineModelInstaller>,
        event_callback: LocalAsrPipelineEventCallback,
    ) -> Result<Self, AsrError> {
        Self::start_architecture(
            LocalAsrArchitecture::MoonshineSmallStreaming,
            LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
            move || {
                MoonshineSmallEngine::open_small(&installer)
                    .map(|engine| Box::new(engine) as Box<dyn PipelineEngine>)
            },
            event_callback,
            PRODUCTION_WORKER_STARTUP_TIMEOUT,
            None,
        )
        .await
    }

    /// Start the production Whisper.cpp `whisper-small` worker with the same
    /// bounded queue, transcript state machine, lifecycle, and local-only
    /// microphone contract.
    pub async fn start_whisper(
        installer: Arc<WhisperModelInstaller>,
        event_callback: LocalAsrPipelineEventCallback,
    ) -> Result<Self, AsrError> {
        Self::start_architecture(
            LocalAsrArchitecture::WhisperSmall,
            WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
            move || {
                crate::asr::whisper::engine::open(installer)
                    .map(|engine| Box::new(engine) as Box<dyn PipelineEngine>)
            },
            event_callback,
            PRODUCTION_WORKER_STARTUP_TIMEOUT,
            None,
        )
        .await
    }

    /// Prime the existing Moonshine command-ASR ingress with the exact wake
    /// pre-roll plus post-trigger live PCM before microphone ownership moves to
    /// `start_capture`. This preserves chronological ordering without acoustic
    /// wake-phrase trimming or a second inference path.
    // WWR-310 lands the ingress seam before the production orchestrator calls it.
    #[allow(dead_code)]
    pub(crate) fn prime_wake_handoff(
        &self,
        handoff: WakeCommandHandoffAudio,
    ) -> Result<(), AsrError> {
        if !self.is_running() {
            return Err(invalid_state_error(
                "Local ASR inference is not running; wake handoff was not accepted.",
            ));
        }
        if handoff.sample_rate_hz() != self.input_sample_rate_hz {
            return Err(invalid_state_error(
                "Wake handoff sample rate does not match local ASR input.",
            ));
        }
        let sender = self.pcm_sender.as_ref().ok_or_else(|| {
            invalid_state_error("Local ASR input is closed; wake handoff was not accepted.")
        })?;
        sender
            .try_send(handoff.to_pcm16_le_bytes())
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => AsrError {
                    kind: AsrErrorKind::AudioInput,
                    message: "Local ASR input queue is full; wake handoff was not accepted."
                        .to_string(),
                    retryable: true,
                },
                mpsc::error::TrySendError::Closed(_) => {
                    invalid_state_error("Local ASR input is closed; wake handoff was not accepted.")
                }
            })
    }

    /// Start the existing authoritative microphone on this pipeline's bounded
    /// ingress queue. No second capture object or cloud audio path is created.
    pub fn start_capture(
        &self,
        capture: &mut AudioCapture,
        device_name: Option<String>,
        level_sender: Option<mpsc::Sender<f32>>,
    ) -> Result<(), AsrError> {
        if !self.is_running() {
            return Err(invalid_state_error(
                "Local ASR inference is not running; microphone capture was not started.",
            ));
        }
        let sender = self.pcm_sender.as_ref().cloned().ok_or_else(|| {
            invalid_state_error("Local ASR input is closed; microphone capture was not started.")
        })?;
        capture
            .start(device_name, self.input_sample_rate_hz, sender, level_sender)
            .map_err(|error| AsrError {
                kind: AsrErrorKind::AudioInput,
                message: format!("Failed to start local-ASR microphone capture: {error}"),
                retryable: true,
            })
    }

    /// Acceptance-only producer seam for feeding deterministic PCM through the
    /// exact production worker/queue without opening a physical microphone.
    ///
    /// Returns false when the bounded queue is full, matching capture's
    /// drop-newest overload policy. This remains crate-private.
    #[allow(dead_code)]
    pub(crate) fn try_send_pcm_for_acceptance(&self, pcm_bytes: Vec<u8>) -> Result<bool, AsrError> {
        let sender = self.pcm_sender.as_ref().ok_or_else(|| {
            invalid_state_error("Local ASR input is closed; acceptance PCM was not accepted.")
        })?;
        match sender.try_send(pcm_bytes) {
            Ok(()) => Ok(true),
            Err(mpsc::error::TrySendError::Full(_)) => Ok(false),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(invalid_state_error(
                "Local ASR input is closed; acceptance PCM was not accepted.",
            )),
        }
    }

    pub fn diagnostics(&self) -> LocalAsrPipelineDiagnostics {
        let runtime = self.runtime_diagnostics();
        LocalAsrPipelineDiagnostics {
            architecture: self.architecture,
            input_sample_rate_hz: runtime.input_sample_rate_hz,
            queue_depth: runtime.queue_depth,
            queue_capacity: runtime.queue_capacity,
            running: runtime.streaming,
            last_error: runtime.last_error,
            first_partial_latency_ms: runtime.first_partial_latency_ms,
            first_final_latency_ms: runtime.first_final_latency_ms,
            last_transcription_latency_ms: runtime.last_transcription_latency_ms,
            processed_audio_ms: runtime.processed_audio_ms,
            inference_wall_time_ms: runtime.inference_wall_time_ms,
            real_time_factor: runtime.real_time_factor,
            process_cpu_time_ms: runtime.process_cpu_time_ms,
            average_cpu_utilization_percent: runtime.average_cpu_utilization_percent,
            baseline_resident_memory_bytes: runtime.baseline_resident_memory_bytes,
            resident_memory_bytes: runtime.resident_memory_bytes,
            peak_resident_memory_bytes: runtime.peak_resident_memory_bytes,
        }
    }

    fn runtime_diagnostics(&self) -> LocalAsrRuntimeDiagnostics {
        let queue_depth = self.pcm_sender.as_ref().map_or(0, |sender| {
            self.queue_capacity_chunks.saturating_sub(sender.capacity())
        });
        self.metrics.lock().runtime_diagnostics(
            self.input_sample_rate_hz,
            queue_depth,
            self.queue_capacity_chunks,
            self.is_running(),
        )
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Request worker termination and join it without blocking the async runtime.
    /// Safe to call repeatedly.
    pub async fn stop_and_join(&mut self) -> Result<(), AsrError> {
        self.request_stop();
        let result = if let Some(worker) = self.worker.take() {
            tokio::task::spawn_blocking(move || worker.join())
                .await
                .map_err(|_| worker_join_error())?
        } else {
            Ok(Ok(()))
        };
        self.running.store(false, Ordering::SeqCst);
        self.event_delivery.wait_until_idle().await;
        match result {
            Ok(worker_result) => worker_result,
            Err(_) => Err(worker_join_error()),
        }
    }

    fn request_stop(&mut self) {
        // Graceful stop closes the owned producer and lets the worker drain
        // every chunk already accepted before finalizing the engine.
        self.stop_requested.store(true, Ordering::SeqCst);
        self.pcm_sender.take();
    }

    fn request_abort(&mut self) {
        // Drop is a safety-net abort path. Unlike normal stop_and_join, it may
        // discard accepted queued audio because no async caller can await drain.
        self.abort_requested.store(true, Ordering::SeqCst);
        self.stop_requested.store(true, Ordering::SeqCst);
        self.pcm_sender.take();
    }

    #[cfg(test)]
    async fn start_with_factory<F>(
        factory: F,
        event_callback: LocalAsrPipelineEventCallback,
    ) -> Result<Self, AsrError>
    where
        F: FnOnce() -> Result<Box<dyn PipelineEngine>, AsrError> + Send + 'static,
    {
        Self::start_architecture(
            LocalAsrArchitecture::MoonshineTinyStreaming,
            LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
            factory,
            event_callback,
            WORKER_STARTUP_TIMEOUT,
            None,
        )
        .await
    }

    #[cfg(test)]
    async fn start_with_factory_and_reaper_probe<F>(
        factory: F,
        event_callback: LocalAsrPipelineEventCallback,
        reaped_probe: Arc<AtomicBool>,
    ) -> Result<Self, AsrError>
    where
        F: FnOnce() -> Result<Box<dyn PipelineEngine>, AsrError> + Send + 'static,
    {
        Self::start_architecture(
            LocalAsrArchitecture::MoonshineTinyStreaming,
            LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
            factory,
            event_callback,
            WORKER_STARTUP_TIMEOUT,
            Some(reaped_probe),
        )
        .await
    }

    async fn start_architecture<F>(
        architecture: LocalAsrArchitecture,
        queue_capacity_chunks: usize,
        factory: F,
        event_callback: LocalAsrPipelineEventCallback,
        startup_timeout: Duration,
        startup_reaped_probe: Option<Arc<AtomicBool>>,
    ) -> Result<Self, AsrError>
    where
        F: FnOnce() -> Result<Box<dyn PipelineEngine>, AsrError> + Send + 'static,
    {
        let (pcm_tx, mut pcm_rx) = mpsc::channel::<Vec<u8>>(queue_capacity_chunks);
        let running = Arc::new(AtomicBool::new(false));
        let stop_requested = Arc::new(AtomicBool::new(false));
        let abort_requested = Arc::new(AtomicBool::new(false));
        let metrics = Arc::new(Mutex::new(RuntimeMetrics::new()));
        let event_delivery = Arc::new(LocalAsrEventDeliveryTracker::default());
        let (ready_tx, ready_rx) = oneshot::channel::<Result<(), AsrError>>();

        let worker_running = running.clone();
        let worker_stop = stop_requested.clone();
        let worker_abort = abort_requested.clone();
        let worker_metrics = metrics.clone();
        let worker_event_delivery = event_delivery.clone();
        let worker_callback = event_callback.clone();
        let worker = thread::Builder::new()
            .name(match architecture {
                LocalAsrArchitecture::MoonshineTinyStreaming => "moonshine-tiny-asr".to_string(),
                LocalAsrArchitecture::MoonshineSmallStreaming => "moonshine-small-asr".to_string(),
                LocalAsrArchitecture::WhisperSmall => "whisper-small-asr".to_string(),
            })
            .spawn(move || {
                let mut engine = match factory() {
                    Ok(engine) => engine,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.clone()));
                        return Err(error);
                    }
                };
                if worker_stop.load(Ordering::SeqCst) {
                    return engine.stop().map(|_| ());
                }
                worker_metrics.lock().mark_engine_ready();

                if engine.input_sample_rate_hz() != LOCAL_ASR_INPUT_SAMPLE_RATE_HZ {
                    let error = AsrError {
                        kind: AsrErrorKind::Internal,
                        message:
                            "Local ASR streaming engine reported an unexpected input sample rate."
                                .to_string(),
                        retryable: false,
                    };
                    let _ = engine.stop();
                    let _ = ready_tx.send(Err(error.clone()));
                    return Err(error);
                }

                worker_running.store(true, Ordering::SeqCst);
                if ready_tx.send(Ok(())).is_err() {
                    worker_running.store(false, Ordering::SeqCst);
                    return engine.stop().map(|_| ());
                }

                let result = run_worker(
                    engine.as_mut(),
                    &mut pcm_rx,
                    &worker_stop,
                    &worker_abort,
                    &worker_metrics,
                    &worker_callback,
                    &worker_event_delivery,
                );
                worker_running.store(false, Ordering::SeqCst);
                result
            })
            .map_err(|_| AsrError {
                kind: AsrErrorKind::Internal,
                message: "Failed to start the local ASR inference worker.".to_string(),
                retryable: true,
            })?;

        let mut startup_worker =
            StartupWorkerGuard::new(worker, stop_requested.clone(), startup_reaped_probe);
        match tokio::time::timeout(startup_timeout, ready_rx).await {
            Ok(Ok(Ok(()))) => {
                let pipeline = Self {
                    architecture,
                    input_sample_rate_hz: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
                    queue_capacity_chunks,
                    pcm_sender: Some(pcm_tx),
                    running,
                    stop_requested,
                    abort_requested,
                    metrics,
                    event_delivery,
                    worker: Some(startup_worker.take_worker()),
                };
                let diagnostics = pipeline.diagnostics();
                debug!(
                    architecture = ?diagnostics.architecture,
                    input_sample_rate_hz = diagnostics.input_sample_rate_hz,
                    queue_depth = diagnostics.queue_depth,
                    queue_capacity = diagnostics.queue_capacity,
                    running = diagnostics.running,
                    has_last_error = diagnostics.last_error.is_some(),
                    "Local ASR inference worker started"
                );
                Ok(pipeline)
            }
            Ok(Ok(Err(error))) => {
                join_finished_startup_worker(startup_worker.take_worker()).await?;
                Err(error)
            }
            Ok(Err(_)) => {
                join_finished_startup_worker(startup_worker.take_worker()).await?;
                Err(worker_join_error())
            }
            Err(_) => Err(worker_startup_timeout_error()),
        }
    }

    #[cfg(test)]
    fn test_sender(&self) -> mpsc::Sender<Vec<u8>> {
        self.pcm_sender
            .as_ref()
            .expect("pipeline ingress should be open")
            .clone()
    }
}

#[async_trait]
impl LocalAsrResource for LocalAsrPipeline {
    async fn stop(&mut self) -> Result<(), AsrError> {
        self.stop_and_join().await
    }

    fn diagnostics(&self) -> Option<LocalAsrRuntimeDiagnostics> {
        Some(self.runtime_diagnostics())
    }
}

impl Drop for LocalAsrPipeline {
    fn drop(&mut self) {
        self.request_abort();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

fn invalid_state_error(message: &str) -> AsrError {
    AsrError {
        kind: AsrErrorKind::InvalidState,
        message: message.to_string(),
        retryable: true,
    }
}

fn worker_join_error() -> AsrError {
    AsrError {
        kind: AsrErrorKind::Internal,
        message: "The local ASR inference worker terminated unexpectedly.".to_string(),
        retryable: true,
    }
}

fn worker_startup_timeout_error() -> AsrError {
    AsrError {
        kind: AsrErrorKind::RuntimeUnavailable,
        message: "Local ASR inference worker did not become ready before the startup timeout."
            .to_string(),
        retryable: true,
    }
}

async fn join_finished_startup_worker(
    worker: JoinHandle<Result<(), AsrError>>,
) -> Result<(), AsrError> {
    let joined = tokio::task::spawn_blocking(move || worker.join())
        .await
        .map_err(|_| worker_join_error())?;
    match joined {
        Ok(_) => Ok(()),
        Err(_) => Err(worker_join_error()),
    }
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "pipeline_benchmarks.rs"]
mod benchmarks;
