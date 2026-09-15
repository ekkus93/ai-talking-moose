use super::{
    SherpaKwsEngine, WakeWordEngine, WakeWordError, WakeWordErrorKind, DEFAULT_KEYWORDS_SCORE,
    DEFAULT_KEYWORDS_THRESHOLD, WAKE_CHANNELS, WAKE_INFERENCE_THREADS, WAKE_SAMPLE_RATE_HZ,
};
use crate::audio::resample::AudioResampler;
use crate::audio::PcmRingBuffer;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

const ENGINE_QUEUE_CAPACITY: usize = 8;
const COMMAND_QUEUE_CHUNK_SAMPLES: usize = 1_600;
const RING_SECONDS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeWordRuntimeState {
    Disabled,
    Loading,
    Listening,
    Suspended,
    Triggered,
    CommandHandoff,
    Error,
    Stopping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WakeWordDiagnostics {
    pub enabled: bool,
    pub state: WakeWordRuntimeState,
    pub engine: String,
    pub runtime_version: String,
    pub platform: String,
    pub architecture: String,
    pub inference_threads: i32,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub ring_buffer_seconds: u32,
    pub ring_buffer_capacity_samples: usize,
    pub keywords_score: f32,
    pub keywords_threshold: f32,
    pub trigger_count: u64,
    pub last_trigger_unix_ms: Option<u64>,
    pub initialization_duration_ms: Option<u64>,
    pub suspended_for_talking: bool,
    pub dropped_engine_chunks: u64,
    pub dropped_command_chunks: u64,
    pub last_error: Option<String>,
}

pub type WakeTriggerCallback = Arc<dyn Fn() + Send + Sync>;

type EngineFactory = Box<dyn FnOnce() -> Result<Box<dyn WakeWordEngine>, WakeWordError> + Send>;

struct RuntimeInner {
    enabled: bool,
    state: WakeWordRuntimeState,
    ring: PcmRingBuffer,
    command_sink: Option<mpsc::Sender<Vec<u8>>>,
    level_sink: Option<mpsc::Sender<f32>>,
    trigger_count: u64,
    last_trigger_unix_ms: Option<u64>,
    initialization_duration_ms: Option<u64>,
    suspended_for_talking: bool,
    dropped_engine_chunks: u64,
    dropped_command_chunks: u64,
    last_error: Option<String>,
}

impl RuntimeInner {
    fn new() -> Self {
        Self {
            enabled: false,
            state: WakeWordRuntimeState::Disabled,
            ring: PcmRingBuffer::for_duration(WAKE_SAMPLE_RATE_HZ, WAKE_CHANNELS, RING_SECONDS),
            command_sink: None,
            level_sink: None,
            trigger_count: 0,
            last_trigger_unix_ms: None,
            initialization_duration_ms: None,
            suspended_for_talking: false,
            dropped_engine_chunks: 0,
            dropped_command_chunks: 0,
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct WakeWordRuntimeManager {
    inner: Arc<Mutex<RuntimeInner>>,
    engine_sender: Arc<Mutex<Option<mpsc::Sender<Vec<i16>>>>>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
    stop_requested: Arc<AtomicBool>,
}

impl WakeWordRuntimeManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner::new())),
            engine_sender: Arc::new(Mutex::new(None)),
            worker: Arc::new(Mutex::new(None)),
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(
        &self,
        model_root: PathBuf,
        trigger_callback: WakeTriggerCallback,
    ) -> Result<(), WakeWordError> {
        self.start_with_factory(
            Box::new(move || {
                SherpaKwsEngine::open(&model_root)
                    .map(|engine| Box::new(engine) as Box<dyn WakeWordEngine>)
            }),
            trigger_callback,
        )
        .await
    }

    async fn start_with_factory(
        &self,
        factory: EngineFactory,
        trigger_callback: WakeTriggerCallback,
    ) -> Result<(), WakeWordError> {
        {
            let mut inner = self.inner.lock();
            if matches!(
                inner.state,
                WakeWordRuntimeState::Loading
                    | WakeWordRuntimeState::Listening
                    | WakeWordRuntimeState::Triggered
                    | WakeWordRuntimeState::CommandHandoff
                    | WakeWordRuntimeState::Suspended
            ) {
                return Ok(());
            }
            inner.enabled = true;
            inner.state = WakeWordRuntimeState::Loading;
            inner.last_error = None;
            inner.ring.clear();
            inner.command_sink = None;
            inner.level_sink = None;
        }
        self.stop_requested.store(false, Ordering::SeqCst);
        let (engine_tx, mut engine_rx) = mpsc::channel::<Vec<i16>>(ENGINE_QUEUE_CAPACITY);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<u64, WakeWordError>>();
        let inner = self.inner.clone();
        let stop = self.stop_requested.clone();
        let callback = trigger_callback.clone();
        let worker = thread::Builder::new()
            .name("wake-word-kws".to_string())
            .spawn(move || {
                let started = Instant::now();
                let mut engine = match factory() {
                    Ok(engine) => engine,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(started.elapsed().as_millis() as u64));
                while !stop.load(Ordering::SeqCst) {
                    let Some(samples) = engine_rx.blocking_recv() else {
                        break;
                    };
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    match engine.process_pcm_i16(&samples) {
                        Ok(true) => {
                            let accepted = {
                                let mut state = inner.lock();
                                if state.enabled && state.state == WakeWordRuntimeState::Listening {
                                    state.state = WakeWordRuntimeState::Triggered;
                                    state.trigger_count = state.trigger_count.saturating_add(1);
                                    state.last_trigger_unix_ms = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .ok()
                                        .map(|duration| duration.as_millis() as u64);
                                    true
                                } else {
                                    false
                                }
                            };
                            if accepted {
                                callback();
                            }
                        }
                        Ok(false) => {}
                        Err(error) => {
                            let mut state = inner.lock();
                            state.state = WakeWordRuntimeState::Error;
                            state.last_error = Some(error.message);
                            break;
                        }
                    }
                }
                let _ = engine.shutdown();
            })
            .map_err(|_| {
                WakeWordError::runtime("The local wake-word worker could not be started.")
            })?;

        *self.engine_sender.lock() = Some(engine_tx);
        *self.worker.lock() = Some(worker);
        match tokio::time::timeout(Duration::from_secs(30), ready_rx).await {
            Ok(Ok(Ok(duration_ms))) => {
                let mut inner = self.inner.lock();
                inner.initialization_duration_ms = Some(duration_ms);
                inner.state = WakeWordRuntimeState::Listening;
                Ok(())
            }
            Ok(Ok(Err(error))) => {
                self.record_start_error(&error);
                Err(error)
            }
            Ok(Err(_)) | Err(_) => {
                let error = WakeWordError::runtime(
                    "The local wake-word worker did not become ready before the startup timeout.",
                );
                self.record_start_error(&error);
                Err(error)
            }
        }
    }

    fn record_start_error(&self, error: &WakeWordError) {
        self.stop_requested.store(true, Ordering::SeqCst);
        self.engine_sender.lock().take();
        let mut inner = self.inner.lock();
        inner.state = WakeWordRuntimeState::Error;
        inner.last_error = Some(error.message.clone());
    }

    pub fn ingest_pcm_bytes(&self, bytes: Vec<u8>) -> Result<(), WakeWordError> {
        if !bytes.len().is_multiple_of(2) {
            return Err(WakeWordError {
                kind: WakeWordErrorKind::AudioInput,
                message: "Wake-word capture produced malformed 16-bit PCM.".to_string(),
                retryable: true,
            });
        }
        let samples = AudioResampler::bytes_to_i16(&bytes);
        let (state, command_sink) = {
            let mut inner = self.inner.lock();
            match inner.state {
                WakeWordRuntimeState::Listening | WakeWordRuntimeState::Triggered => {
                    inner.ring.write(&samples);
                }
                WakeWordRuntimeState::CommandHandoff => {}
                _ => return Ok(()),
            }
            (inner.state, inner.command_sink.clone())
        };

        match state {
            WakeWordRuntimeState::Listening => {
                if let Some(sender) = self.engine_sender.lock().clone() {
                    if sender.try_send(samples).is_err() {
                        let mut inner = self.inner.lock();
                        inner.dropped_engine_chunks = inner.dropped_engine_chunks.saturating_add(1);
                    }
                }
            }
            WakeWordRuntimeState::CommandHandoff => {
                if let Some(sender) = command_sink {
                    if sender.try_send(bytes).is_err() {
                        let mut inner = self.inner.lock();
                        inner.dropped_command_chunks =
                            inner.dropped_command_chunks.saturating_add(1);
                    }
                }
            }
            WakeWordRuntimeState::Triggered => {}
            _ => {}
        }
        Ok(())
    }

    pub fn route_input_level(&self, level: f32) {
        let sink = self.inner.lock().level_sink.clone();
        if let Some(sink) = sink {
            let _ = sink.try_send(level);
        }
    }

    pub fn begin_command_handoff(
        &self,
        command_sink: mpsc::Sender<Vec<u8>>,
        level_sink: Option<mpsc::Sender<f32>>,
    ) -> Result<usize, WakeWordError> {
        let snapshot = {
            let mut inner = self.inner.lock();
            if !inner.enabled || inner.state != WakeWordRuntimeState::Triggered {
                return Err(WakeWordError {
                    kind: WakeWordErrorKind::InvalidState,
                    message: "Wake-word audio is not available for command handoff.".to_string(),
                    retryable: true,
                });
            }
            inner.state = WakeWordRuntimeState::CommandHandoff;
            inner.command_sink = Some(command_sink.clone());
            inner.level_sink = level_sink;
            inner.ring.snapshot()
        };

        let mut replayed = 0usize;
        for chunk in snapshot.chunks(COMMAND_QUEUE_CHUNK_SAMPLES) {
            let bytes = AudioResampler::i16_to_bytes(chunk);
            command_sink.try_send(bytes).map_err(|_| WakeWordError {
                kind: WakeWordErrorKind::InvalidState,
                message: "Command ASR was not ready to accept wake-word pre-roll.".to_string(),
                retryable: true,
            })?;
            replayed += chunk.len();
        }
        Ok(replayed)
    }

    pub fn finish_command_handoff(&self) {
        let mut inner = self.inner.lock();
        if !inner.enabled {
            return;
        }
        inner.state = WakeWordRuntimeState::Suspended;
        inner.command_sink = None;
        inner.level_sink = None;
        inner.ring.clear();
        inner.suspended_for_talking = false;
    }

    pub fn suspend_for_talking(&self) {
        let mut inner = self.inner.lock();
        if !inner.enabled {
            return;
        }
        inner.state = WakeWordRuntimeState::Suspended;
        inner.suspended_for_talking = true;
        inner.command_sink = None;
        inner.level_sink = None;
        inner.ring.clear();
    }

    pub fn resume_listening(&self) -> Result<(), WakeWordError> {
        {
            let inner = self.inner.lock();
            if !inner.enabled {
                return Ok(());
            }
        }
        let mut inner = self.inner.lock();
        inner.command_sink = None;
        inner.level_sink = None;
        inner.ring.clear();
        inner.suspended_for_talking = false;
        inner.state = WakeWordRuntimeState::Listening;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), WakeWordError> {
        {
            let mut inner = self.inner.lock();
            inner.state = WakeWordRuntimeState::Stopping;
            inner.enabled = false;
            inner.command_sink = None;
            inner.level_sink = None;
            inner.ring.clear();
        }
        self.stop_requested.store(true, Ordering::SeqCst);
        self.engine_sender.lock().take();
        let worker = self.worker.lock().take();
        if let Some(worker) = worker {
            tokio::task::spawn_blocking(move || worker.join())
                .await
                .map_err(|_| WakeWordError::runtime("The wake-word worker could not be joined."))?
                .map_err(|_| {
                    WakeWordError::runtime("The wake-word worker terminated unexpectedly.")
                })?;
        }
        let mut inner = self.inner.lock();
        inner.state = WakeWordRuntimeState::Disabled;
        inner.last_error = None;
        Ok(())
    }

    pub fn diagnostics(&self) -> WakeWordDiagnostics {
        let inner = self.inner.lock();
        WakeWordDiagnostics {
            enabled: inner.enabled,
            state: inner.state,
            engine: "sherpa-onnx-kws".to_string(),
            runtime_version: "1.13.8".to_string(),
            platform: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            inference_threads: WAKE_INFERENCE_THREADS,
            sample_rate_hz: WAKE_SAMPLE_RATE_HZ,
            channels: WAKE_CHANNELS,
            ring_buffer_seconds: RING_SECONDS,
            ring_buffer_capacity_samples: inner.ring.capacity(),
            keywords_score: DEFAULT_KEYWORDS_SCORE,
            keywords_threshold: DEFAULT_KEYWORDS_THRESHOLD,
            trigger_count: inner.trigger_count,
            last_trigger_unix_ms: inner.last_trigger_unix_ms,
            initialization_duration_ms: inner.initialization_duration_ms,
            suspended_for_talking: inner.suspended_for_talking,
            dropped_engine_chunks: inner.dropped_engine_chunks,
            dropped_command_chunks: inner.dropped_command_chunks,
            last_error: inner.last_error.clone(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.lock().enabled
    }

    pub fn state(&self) -> WakeWordRuntimeState {
        self.inner.lock().state
    }

    pub fn model_root_from_resource_dir(resource_dir: &Path) -> PathBuf {
        resource_dir.join("wake_word/model")
    }
}

impl Default for WakeWordRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct FakeEngine {
        detects_on_nonzero: bool,
        stopped: Arc<AtomicBool>,
    }

    impl WakeWordEngine for FakeEngine {
        fn process_pcm_i16(&mut self, samples: &[i16]) -> Result<bool, WakeWordError> {
            Ok(self.detects_on_nonzero && samples.iter().any(|sample| *sample != 0))
        }

        fn reset(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            self.stopped.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn inference_threads(&self) -> i32 {
            1
        }
    }

    async fn started_manager(
        detects: bool,
    ) -> (WakeWordRuntimeManager, Arc<AtomicU64>, Arc<AtomicBool>) {
        let manager = WakeWordRuntimeManager::new();
        let triggers = Arc::new(AtomicU64::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let triggers_for_callback = triggers.clone();
        let stopped_for_engine = stopped.clone();
        manager
            .start_with_factory(
                Box::new(move || {
                    Ok(Box::new(FakeEngine {
                        detects_on_nonzero: detects,
                        stopped: stopped_for_engine,
                    }))
                }),
                Arc::new(move || {
                    triggers_for_callback.fetch_add(1, Ordering::SeqCst);
                }),
            )
            .await
            .unwrap();
        (manager, triggers, stopped)
    }

    #[tokio::test]
    async fn repeated_positive_frames_create_one_trigger_until_resume() {
        let (manager, triggers, _) = started_manager(true).await;
        manager
            .ingest_pcm_bytes(vec![1_u8, 0_u8].repeat(1600))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        manager
            .ingest_pcm_bytes(vec![1_u8, 0_u8].repeat(1600))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(triggers.load(Ordering::SeqCst), 1);
        assert_eq!(manager.diagnostics().trigger_count, 1);
        manager.resume_listening().unwrap();
        manager
            .ingest_pcm_bytes(vec![1_u8, 0_u8].repeat(1600))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(triggers.load(Ordering::SeqCst), 2);
        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn handoff_replays_chronological_pre_roll_then_forwards_live_pcm() {
        let (manager, _, _) = started_manager(false).await;
        manager
            .ingest_pcm_bytes(AudioResampler::i16_to_bytes(&[1, 2, 3, 4]))
            .unwrap();
        manager.inner.lock().state = WakeWordRuntimeState::Triggered;
        let (tx, mut rx) = mpsc::channel(8);
        let replayed = manager.begin_command_handoff(tx, None).unwrap();
        assert_eq!(replayed, 4);
        let replay = rx.recv().await.unwrap();
        assert_eq!(AudioResampler::bytes_to_i16(&replay), vec![1, 2, 3, 4]);
        manager
            .ingest_pcm_bytes(AudioResampler::i16_to_bytes(&[5, 6]))
            .unwrap();
        let live = rx.recv().await.unwrap();
        assert_eq!(AudioResampler::bytes_to_i16(&live), vec![5, 6]);
        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn talking_suspension_clears_pre_roll_and_resume_is_recoverable() {
        let (manager, _, _) = started_manager(false).await;
        manager
            .ingest_pcm_bytes(AudioResampler::i16_to_bytes(&[7, 8, 9]))
            .unwrap();
        manager.suspend_for_talking();
        assert_eq!(manager.state(), WakeWordRuntimeState::Suspended);
        assert!(manager.diagnostics().suspended_for_talking);
        manager.resume_listening().unwrap();
        manager.inner.lock().state = WakeWordRuntimeState::Triggered;
        let (tx, mut rx) = mpsc::channel(8);
        assert_eq!(manager.begin_command_handoff(tx, None).unwrap(), 0);
        assert!(rx.try_recv().is_err());
        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn stop_is_idempotent_and_reaps_engine() {
        let (manager, _, stopped) = started_manager(false).await;
        manager.stop().await.unwrap();
        manager.stop().await.unwrap();
        assert!(stopped.load(Ordering::SeqCst));
        assert_eq!(manager.state(), WakeWordRuntimeState::Disabled);
    }
}
