use crate::app::wake_word_settings::{V1_WAKE_SCORE, V1_WAKE_THRESHOLD};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const SHERPA_KWS_ENGINE_ID: &str = "sherpa-onnx-kws";
pub const V1_KWS_THREADS: usize = 1;
pub const V1_CANONICAL_SAMPLE_RATE_HZ: u32 = 16_000;
pub const V1_CANONICAL_CHANNELS: u16 = 1;
pub const V1_CANONICAL_KEYWORD: &str = "HEY MOOSE";

#[derive(Debug, Clone, PartialEq)]
pub struct SherpaKwsConfig {
    pub model_path: PathBuf,
    pub tokens_path: PathBuf,
    pub keywords_path: PathBuf,
    pub threads: usize,
    pub trigger_threshold: f32,
    pub trigger_score: f32,
}

impl SherpaKwsConfig {
    pub fn v1(model_path: PathBuf, tokens_path: PathBuf, keywords_path: PathBuf) -> Self {
        Self {
            model_path,
            tokens_path,
            keywords_path,
            threads: V1_KWS_THREADS,
            trigger_threshold: V1_WAKE_THRESHOLD,
            trigger_score: V1_WAKE_SCORE,
        }
    }

    pub fn validate(&self) -> Result<(), SherpaKwsError> {
        if self.threads != V1_KWS_THREADS {
            return Err(SherpaKwsError::configuration(
                "Wake Word V1 requires exactly one KWS inference thread.",
            ));
        }
        if !self.trigger_threshold.is_finite()
            || self.trigger_threshold <= 0.0
            || !self.trigger_score.is_finite()
            || self.trigger_score <= 0.0
        {
            return Err(SherpaKwsError::configuration(
                "Wake Word V1 KWS tuning is invalid.",
            ));
        }
        for path in [&self.model_path, &self.tokens_path, &self.keywords_path] {
            if path.as_os_str().is_empty() {
                return Err(SherpaKwsError::configuration(
                    "Wake Word V1 KWS artifacts are not configured.",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SherpaKwsErrorKind {
    Configuration,
    Artifact,
    Runtime,
    ShuttingDown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SherpaKwsError {
    pub kind: SherpaKwsErrorKind,
    pub message: &'static str,
}

impl SherpaKwsError {
    fn configuration(message: &'static str) -> Self {
        Self {
            kind: SherpaKwsErrorKind::Configuration,
            message,
        }
    }

    pub fn artifact() -> Self {
        Self {
            kind: SherpaKwsErrorKind::Artifact,
            message: "Wake Word V1 KWS artifacts are missing or failed verification.",
        }
    }

    pub fn runtime() -> Self {
        Self {
            kind: SherpaKwsErrorKind::Runtime,
            message: "The local Wake Word KWS runtime encountered an internal error.",
        }
    }

    fn shutting_down() -> Self {
        Self {
            kind: SherpaKwsErrorKind::ShuttingDown,
            message: "The local Wake Word KWS runtime is shutting down.",
        }
    }
}

impl std::fmt::Display for SherpaKwsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for SherpaKwsError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KwsFrameOutcome {
    NoTrigger,
    WakeDetected,
}

/// Engine-private native streaming session contract. Implementations may wrap sherpa objects,
/// but callers only provide canonical 16 kHz mono PCM and receive a bounded detection boolean.
pub trait NativeKwsSession: Send {
    fn accept_pcm(&mut self, samples: &[f32]) -> Result<bool, SherpaKwsError>;
    fn reset_stream(&mut self) -> Result<(), SherpaKwsError>;
    fn shutdown(&mut self);
}

/// Narrow ownership boundary for the sherpa-onnx keyword spotter.
///
/// Native sherpa session construction is intentionally kept behind this type so microphone
/// routing and the wake lifecycle never depend on sherpa-specific objects. This boundary has
/// no networking API and accepts only canonical 16 kHz mono PCM. The native session will be
/// attached once the pinned runtime/model manifest contains qualified production artifacts.
pub struct SherpaKwsEngine {
    config: SherpaKwsConfig,
    shutting_down: AtomicBool,
    session: Option<Box<dyn NativeKwsSession>>,
}

impl SherpaKwsEngine {
    pub fn new(config: SherpaKwsConfig) -> Result<Self, SherpaKwsError> {
        config.validate()?;
        Ok(Self {
            config,
            shutting_down: AtomicBool::new(false),
            session: None,
        })
    }

    pub fn attach_session(
        &mut self,
        session: Box<dyn NativeKwsSession>,
    ) -> Result<(), SherpaKwsError> {
        self.ensure_running()?;
        self.session = Some(session);
        Ok(())
    }

    pub fn feed_canonical_pcm(
        &mut self,
        samples: &[f32],
    ) -> Result<KwsFrameOutcome, SherpaKwsError> {
        self.ensure_running()?;
        if samples.is_empty() {
            return Ok(KwsFrameOutcome::NoTrigger);
        }
        let session = self.session.as_mut().ok_or_else(SherpaKwsError::artifact)?;
        if session.accept_pcm(samples)? {
            session.reset_stream()?;
            Ok(KwsFrameOutcome::WakeDetected)
        } else {
            Ok(KwsFrameOutcome::NoTrigger)
        }
    }

    pub fn engine_id(&self) -> &'static str {
        SHERPA_KWS_ENGINE_ID
    }

    pub fn thread_count(&self) -> usize {
        self.config.threads
    }

    pub fn keyword(&self) -> &'static str {
        V1_CANONICAL_KEYWORD
    }

    pub fn sample_rate_hz(&self) -> u32 {
        V1_CANONICAL_SAMPLE_RATE_HZ
    }

    pub fn channels(&self) -> u16 {
        V1_CANONICAL_CHANNELS
    }

    pub fn trigger_threshold(&self) -> f32 {
        self.config.trigger_threshold
    }

    pub fn trigger_score(&self) -> f32 {
        self.config.trigger_score
    }

    pub fn ensure_running(&self) -> Result<(), SherpaKwsError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(SherpaKwsError::shutting_down());
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if !self.shutting_down.swap(true, Ordering::SeqCst) {
            if let Some(session) = self.session.as_mut() {
                session.shutdown();
            }
        }
        self.session = None;
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::SeqCst)
    }

    pub fn artifact_paths(&self) -> (&Path, &Path, &Path) {
        (
            &self.config.model_path,
            &self.config.tokens_path,
            &self.config.keywords_path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn config() -> SherpaKwsConfig {
        SherpaKwsConfig::v1(
            PathBuf::from("model.onnx"),
            PathBuf::from("tokens.txt"),
            PathBuf::from("keywords.txt"),
        )
    }

    #[derive(Default)]
    struct SessionState {
        frames: Vec<Vec<f32>>,
        resets: usize,
        shutdowns: usize,
        trigger_next: bool,
    }

    struct FakeSession(Arc<Mutex<SessionState>>);
    impl NativeKwsSession for FakeSession {
        fn accept_pcm(&mut self, samples: &[f32]) -> Result<bool, SherpaKwsError> {
            let mut state = self.0.lock().unwrap();
            state.frames.push(samples.to_vec());
            Ok(std::mem::take(&mut state.trigger_next))
        }

        fn reset_stream(&mut self) -> Result<(), SherpaKwsError> {
            self.0.lock().unwrap().resets += 1;
            Ok(())
        }

        fn shutdown(&mut self) {
            self.0.lock().unwrap().shutdowns += 1;
        }
    }

    #[test]
    fn v1_configuration_is_deterministic_and_one_threaded() {
        let engine = SherpaKwsEngine::new(config()).unwrap();
        assert_eq!(engine.engine_id(), "sherpa-onnx-kws");
        assert_eq!(engine.thread_count(), 1);
        assert_eq!(engine.keyword(), "HEY MOOSE");
        assert_eq!(engine.sample_rate_hz(), 16_000);
        assert_eq!(engine.channels(), 1);
        assert_eq!(engine.trigger_threshold(), V1_WAKE_THRESHOLD);
        assert_eq!(engine.trigger_score(), V1_WAKE_SCORE);
    }

    #[test]
    fn streaming_pcm_emits_bounded_detection_and_resets_after_trigger() {
        let state = Arc::new(Mutex::new(SessionState {
            trigger_next: true,
            ..Default::default()
        }));
        let mut engine = SherpaKwsEngine::new(config()).unwrap();
        engine
            .attach_session(Box::new(FakeSession(state.clone())))
            .unwrap();
        assert_eq!(
            engine.feed_canonical_pcm(&[0.1, -0.2]).unwrap(),
            KwsFrameOutcome::WakeDetected
        );
        let state = state.lock().unwrap();
        assert_eq!(state.frames, vec![vec![0.1, -0.2]]);
        assert_eq!(state.resets, 1);
    }

    #[test]
    fn non_triggering_frames_do_not_reset_stream() {
        let state = Arc::new(Mutex::new(SessionState::default()));
        let mut engine = SherpaKwsEngine::new(config()).unwrap();
        engine
            .attach_session(Box::new(FakeSession(state.clone())))
            .unwrap();
        assert_eq!(
            engine.feed_canonical_pcm(&[0.0]).unwrap(),
            KwsFrameOutcome::NoTrigger
        );
        assert_eq!(state.lock().unwrap().resets, 0);
    }

    #[test]
    fn missing_native_session_fails_with_sanitized_artifact_error() {
        let mut engine = SherpaKwsEngine::new(config()).unwrap();
        let error = engine.feed_canonical_pcm(&[0.0]).unwrap_err();
        assert_eq!(error.kind, SherpaKwsErrorKind::Artifact);
        assert!(!error.message.contains('/'));
    }

    #[test]
    fn non_v1_thread_policy_fails_closed() {
        let mut config = config();
        config.threads = 2;
        let error = SherpaKwsEngine::new(config).err().unwrap();
        assert_eq!(error.kind, SherpaKwsErrorKind::Configuration);
        assert!(!error.message.contains("model.onnx"));
    }

    #[test]
    fn missing_artifact_configuration_is_sanitized() {
        let mut config = config();
        config.model_path = PathBuf::new();
        let error = SherpaKwsEngine::new(config).err().unwrap();
        assert_eq!(error.kind, SherpaKwsErrorKind::Configuration);
        assert!(!error.message.contains('/'));
        assert!(!error.message.contains(".onnx"));
    }

    #[test]
    fn shutdown_is_idempotent_and_releases_native_session_once() {
        let state = Arc::new(Mutex::new(SessionState::default()));
        let mut engine = SherpaKwsEngine::new(config()).unwrap();
        engine
            .attach_session(Box::new(FakeSession(state.clone())))
            .unwrap();
        engine.shutdown();
        engine.shutdown();
        assert!(engine.is_shutting_down());
        assert_eq!(state.lock().unwrap().shutdowns, 1);
        let error = engine.ensure_running().unwrap_err();
        assert_eq!(error.kind, SherpaKwsErrorKind::ShuttingDown);
    }

    #[test]
    fn public_runtime_errors_do_not_expose_paths_or_audio() {
        for error in [SherpaKwsError::artifact(), SherpaKwsError::runtime()] {
            assert!(!error.message.contains('/'));
            assert!(!error.message.contains(".onnx"));
            assert!(!error.message.contains("pcm"));
        }
    }
}
