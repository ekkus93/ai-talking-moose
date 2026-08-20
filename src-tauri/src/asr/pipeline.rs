use crate::asr::lifecycle::LocalAsrResource;
use crate::asr::moonshine::{
    MoonshineModelArchitecture, MoonshineModelInstaller, MoonshineSmallEngine, MoonshineTinyEngine,
    MoonshineTinyTranscriptUpdate, MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
};
use crate::asr::transcript_state::{StreamingTranscriptUpdate, TranscriptStateMachine};
use crate::asr::{AsrError, AsrErrorKind, AsrEvent};
use crate::audio::capture::AudioCapture;
use crate::audio::resample::AudioResampler;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};

/// Hard bound for microphone chunks waiting on local Moonshine inference.
///
/// `AudioCapture` emits 100 ms chunks at the requested target rate, so eight
/// queued chunks cap waiting microphone audio at roughly 800 ms. The producer
/// never blocks the CPAL callback: when this queue is full, the newest chunk is
/// dropped by `AudioCapture`, which owns the authoritative overload counter.
pub const LOCAL_ASR_QUEUE_CAPACITY_CHUNKS: usize = 8;

const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Provider-neutral event emitted by the dedicated local-ASR inference worker.
pub type LocalAsrPipelineEvent = AsrEvent;

/// Bounded-worker diagnostics used by later ASR diagnostics/UI work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAsrPipelineDiagnostics {
    pub architecture: MoonshineModelArchitecture,
    pub input_sample_rate_hz: u32,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub running: bool,
    pub last_error: Option<AsrError>,
}

pub type LocalAsrPipelineEventCallback = Arc<dyn Fn(AsrEvent) + Send + Sync>;

trait PipelineEngine: Send {
    fn input_sample_rate_hz(&self) -> u32;
    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError>;
    fn stop(&mut self) -> Result<(), AsrError>;
}

impl PipelineEngine for MoonshineTinyEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        MoonshineTinyEngine::input_sample_rate_hz(self)
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError> {
        MoonshineTinyEngine::push_pcm(self, pcm)
    }

    fn stop(&mut self) -> Result<(), AsrError> {
        MoonshineTinyEngine::stop(self)
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
    architecture: MoonshineModelArchitecture,
    input_sample_rate_hz: u32,
    pcm_sender: Option<mpsc::Sender<Vec<u8>>>,
    running: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<AsrError>>>,
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
            MoonshineModelArchitecture::TinyStreaming,
            move || {
                MoonshineTinyEngine::open(&installer)
                    .map(|engine| Box::new(engine) as Box<dyn PipelineEngine>)
            },
            event_callback,
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
            MoonshineModelArchitecture::SmallStreaming,
            move || {
                MoonshineSmallEngine::open_small(&installer)
                    .map(|engine| Box::new(engine) as Box<dyn PipelineEngine>)
            },
            event_callback,
        )
        .await
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

    pub fn diagnostics(&self) -> LocalAsrPipelineDiagnostics {
        LocalAsrPipelineDiagnostics {
            architecture: self.architecture,
            input_sample_rate_hz: self.input_sample_rate_hz,
            queue_depth: self.pcm_sender.as_ref().map_or(0, |sender| {
                LOCAL_ASR_QUEUE_CAPACITY_CHUNKS.saturating_sub(sender.capacity())
            }),
            queue_capacity: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
            running: self.is_running(),
            last_error: self.last_error.lock().clone(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Request worker termination and join it without blocking the async runtime.
    /// Safe to call repeatedly.
    pub async fn stop_and_join(&mut self) -> Result<(), AsrError> {
        self.request_stop();
        let Some(worker) = self.worker.take() else {
            self.running.store(false, Ordering::SeqCst);
            return Ok(());
        };

        let result = tokio::task::spawn_blocking(move || worker.join())
            .await
            .map_err(|_| worker_join_error())?;
        self.running.store(false, Ordering::SeqCst);
        match result {
            Ok(worker_result) => worker_result,
            Err(_) => Err(worker_join_error()),
        }
    }

    fn request_stop(&mut self) {
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
            MoonshineModelArchitecture::TinyStreaming,
            factory,
            event_callback,
        )
        .await
    }

    async fn start_architecture<F>(
        architecture: MoonshineModelArchitecture,
        factory: F,
        event_callback: LocalAsrPipelineEventCallback,
    ) -> Result<Self, AsrError>
    where
        F: FnOnce() -> Result<Box<dyn PipelineEngine>, AsrError> + Send + 'static,
    {
        let (pcm_tx, mut pcm_rx) = mpsc::channel::<Vec<u8>>(LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
        let running = Arc::new(AtomicBool::new(false));
        let stop_requested = Arc::new(AtomicBool::new(false));
        let last_error = Arc::new(Mutex::new(None));
        let (ready_tx, ready_rx) = oneshot::channel::<Result<(), AsrError>>();

        let worker_running = running.clone();
        let worker_stop = stop_requested.clone();
        let worker_last_error = last_error.clone();
        let worker_callback = event_callback.clone();
        let worker = thread::Builder::new()
            .name(match architecture {
                MoonshineModelArchitecture::TinyStreaming => "moonshine-tiny-asr".to_string(),
                MoonshineModelArchitecture::SmallStreaming => "moonshine-small-asr".to_string(),
            })
            .spawn(move || {
                let mut engine = match factory() {
                    Ok(engine) => engine,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.clone()));
                        return Err(error);
                    }
                };

                if engine.input_sample_rate_hz() != MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ {
                    let error = AsrError {
                        kind: AsrErrorKind::Internal,
                        message:
                            "Moonshine streaming engine reported an unexpected input sample rate."
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
                    return engine.stop();
                }

                let result = run_worker(
                    engine.as_mut(),
                    &mut pcm_rx,
                    &worker_stop,
                    &worker_last_error,
                    &worker_callback,
                );
                worker_running.store(false, Ordering::SeqCst);
                result
            })
            .map_err(|_| AsrError {
                kind: AsrErrorKind::Internal,
                message: "Failed to start the Moonshine inference worker.".to_string(),
                retryable: true,
            })?;

        match ready_rx.await {
            Ok(Ok(())) => Ok(Self {
                architecture,
                input_sample_rate_hz: MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
                pcm_sender: Some(pcm_tx),
                running,
                stop_requested,
                last_error,
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                join_finished_startup_worker(worker).await?;
                Err(error)
            }
            Err(_) => {
                join_finished_startup_worker(worker).await?;
                Err(worker_join_error())
            }
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
}

impl Drop for LocalAsrPipeline {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

fn run_worker(
    engine: &mut dyn PipelineEngine,
    pcm_rx: &mut mpsc::Receiver<Vec<u8>>,
    stop_requested: &AtomicBool,
    last_error: &Mutex<Option<AsrError>>,
    event_callback: &LocalAsrPipelineEventCallback,
) -> Result<(), AsrError> {
    let mut terminal_error = None;
    let mut transcript_state = TranscriptStateMachine::default();
    while !stop_requested.load(Ordering::SeqCst) {
        let bytes = match pcm_rx.try_recv() {
            Ok(bytes) => bytes,
            Err(TryRecvError::Empty) => {
                thread::sleep(WORKER_POLL_INTERVAL);
                continue;
            }
            Err(TryRecvError::Disconnected) => break,
        };
        if stop_requested.load(Ordering::SeqCst) {
            break;
        }
        if bytes.is_empty() {
            continue;
        }

        let pcm = match decode_mono_i16_le(&bytes) {
            Ok(pcm) => pcm,
            Err(error) => {
                record_terminal_error(last_error, event_callback, &error);
                terminal_error = Some(error);
                break;
            }
        };
        match engine.push_pcm(&pcm) {
            Ok(updates) => {
                for update in updates {
                    for event in transcript_state.apply(map_transcript_update(update)) {
                        event_callback(event);
                    }
                }
            }
            Err(error) => {
                record_terminal_error(last_error, event_callback, &error);
                terminal_error = Some(error);
                break;
            }
        }
    }

    if let Err(stop_error) = engine.stop() {
        if terminal_error.is_none() {
            record_terminal_error(last_error, event_callback, &stop_error);
            terminal_error = Some(stop_error);
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
    last_error: &Mutex<Option<AsrError>>,
    event_callback: &LocalAsrPipelineEventCallback,
    error: &AsrError,
) {
    *last_error.lock() = Some(error.clone());
    event_callback(AsrEvent::Error {
        error: error.clone(),
    });
}

fn map_transcript_update(update: MoonshineTinyTranscriptUpdate) -> StreamingTranscriptUpdate {
    match update {
        MoonshineTinyTranscriptUpdate::Partial { line_id, text, .. } => {
            StreamingTranscriptUpdate::Partial {
                segment_id: line_id,
                text,
            }
        }
        MoonshineTinyTranscriptUpdate::Final { line_id, text, .. } => {
            StreamingTranscriptUpdate::Final {
                segment_id: line_id,
                text,
            }
        }
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
        message: "The Moonshine inference worker terminated unexpectedly.".to_string(),
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
