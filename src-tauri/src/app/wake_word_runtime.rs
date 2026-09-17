use super::wake_word_engine::{
    SherpaKwsEngine, WakeWordDetection, WakeWordError, WakeWordErrorKind,
};
use crate::audio::PcmRingBuffer;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeWordRuntimeState {
    Disabled,
    Loading,
    Listening,
    Suspended,
    Triggered,
    Error,
    Stopping,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WakeWordDiagnostics {
    pub enabled: bool,
    pub state: WakeWordRuntimeState,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub inference_threads: u16,
    pub threshold: f32,
    pub score: f32,
    pub ring_buffer_capacity_samples: usize,
    pub ring_buffer_duration_ms: u64,
    pub trigger_count: u64,
    pub last_trigger_age: Option<Duration>,
    pub talking_suspended: bool,
    pub last_error: Option<String>,
}

pub struct WakeWordRuntimeManager<E: SherpaKwsEngine> {
    state: WakeWordRuntimeState,
    engine: Option<E>,
    ring_buffer: PcmRingBuffer,
    trigger_in_flight: bool,
    trigger_count: u64,
    last_trigger_at: Option<Instant>,
    last_error: Option<String>,
}

impl<E: SherpaKwsEngine> WakeWordRuntimeManager<E> {
    pub fn disabled() -> Self {
        Self {
            state: WakeWordRuntimeState::Disabled,
            engine: None,
            ring_buffer: PcmRingBuffer::wake_word_v1(),
            trigger_in_flight: false,
            trigger_count: 0,
            last_trigger_at: None,
            last_error: None,
        }
    }

    pub fn state(&self) -> WakeWordRuntimeState {
        self.state
    }

    pub fn trigger_count(&self) -> u64 {
        self.trigger_count
    }

    pub fn last_trigger_age(&self) -> Option<Duration> {
        self.last_trigger_at.map(|instant| instant.elapsed())
    }

    pub fn diagnostics(&self) -> WakeWordDiagnostics {
        let config = self.engine.as_ref().map(|engine| engine.config());
        let sample_rate_hz = config.map_or(16_000, |config| config.sample_rate_hz);
        WakeWordDiagnostics {
            enabled: self.state != WakeWordRuntimeState::Disabled,
            state: self.state,
            sample_rate_hz,
            channels: config.map_or(1, |config| config.channels),
            inference_threads: config.map_or(1, |config| config.threads),
            threshold: config.map_or(0.25, |config| config.threshold),
            score: config.map_or(1.0, |config| config.score),
            ring_buffer_capacity_samples: 2 * sample_rate_hz as usize,
            ring_buffer_duration_ms: 2_000,
            trigger_count: self.trigger_count,
            last_trigger_age: self.last_trigger_age(),
            talking_suspended: self.state == WakeWordRuntimeState::Suspended,
            last_error: self.last_error.clone(),
        }
    }

    pub fn begin_loading(&mut self) -> Result<(), WakeWordError> {
        if self.state != WakeWordRuntimeState::Disabled {
            return Err(invalid_transition("wake runtime is already active"));
        }
        self.last_error = None;
        self.state = WakeWordRuntimeState::Loading;
        Ok(())
    }

    pub fn finish_loading(&mut self, engine: E) -> Result<(), WakeWordError> {
        if self.state != WakeWordRuntimeState::Loading {
            return Err(invalid_transition("wake runtime is not loading"));
        }
        if let Err(error) = engine.config().validate() {
            self.record_error(&error);
            return Err(error);
        }
        self.engine = Some(engine);
        self.ring_buffer.clear();
        self.trigger_in_flight = false;
        self.last_error = None;
        self.state = WakeWordRuntimeState::Listening;
        Ok(())
    }

    pub fn feed_pcm(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        if self.state != WakeWordRuntimeState::Listening || self.trigger_in_flight {
            return Ok(None);
        }
        self.ring_buffer.append(samples);
        let Some(engine) = self.engine.as_mut() else {
            let error = WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "wake runtime has no loaded KWS engine",
                false,
            );
            self.record_error(&error);
            return Err(error);
        };
        let detection = match engine.accept_pcm16_mono(sample_rate_hz, samples) {
            Ok(detection) => detection,
            Err(error) => {
                self.record_error(&error);
                return Err(error);
            }
        };
        if detection.is_some() {
            self.trigger_in_flight = true;
            self.trigger_count = self.trigger_count.saturating_add(1);
            self.last_trigger_at = Some(Instant::now());
            self.state = WakeWordRuntimeState::Triggered;
        }
        Ok(detection)
    }

    pub fn pre_roll_snapshot(&self) -> Vec<i16> {
        self.ring_buffer.snapshot()
    }

    pub fn suspend(&mut self) -> Result<(), WakeWordError> {
        match self.state {
            WakeWordRuntimeState::Listening | WakeWordRuntimeState::Triggered => {
                self.state = WakeWordRuntimeState::Suspended;
                Ok(())
            }
            WakeWordRuntimeState::Suspended => Ok(()),
            _ => Err(invalid_transition(
                "wake runtime cannot suspend from current state",
            )),
        }
    }

    pub fn resume(&mut self) -> Result<(), WakeWordError> {
        if self.state != WakeWordRuntimeState::Suspended {
            return Err(invalid_transition("wake runtime is not suspended"));
        }
        let Some(engine) = self.engine.as_mut() else {
            let error = WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "wake runtime has no loaded KWS engine",
                false,
            );
            self.record_error(&error);
            return Err(error);
        };
        if let Err(error) = engine.reset_stream() {
            self.record_error(&error);
            return Err(error);
        }
        self.ring_buffer.clear();
        self.trigger_in_flight = false;
        self.last_error = None;
        self.state = WakeWordRuntimeState::Listening;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), WakeWordError> {
        if self.state == WakeWordRuntimeState::Disabled {
            return Ok(());
        }
        self.state = WakeWordRuntimeState::Stopping;
        let shutdown_result = if let Some(engine) = self.engine.as_mut() {
            engine.shutdown()
        } else {
            Ok(())
        };
        if let Err(error) = &shutdown_result {
            self.last_error = Some(error.message.clone());
        }
        self.engine = None;
        self.ring_buffer.clear();
        self.trigger_in_flight = false;
        self.state = WakeWordRuntimeState::Disabled;
        shutdown_result
    }

    fn record_error(&mut self, error: &WakeWordError) {
        self.last_error = Some(error.message.clone());
        self.state = WakeWordRuntimeState::Error;
    }
}

fn invalid_transition(message: &'static str) -> WakeWordError {
    WakeWordError::sanitized(WakeWordErrorKind::InvalidConfiguration, message, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word_engine::{SherpaKwsConfig, V1_KWS_SAMPLE_RATE_HZ};

    struct FakeEngine {
        config: SherpaKwsConfig,
        detect_next: bool,
        fail_next: bool,
        reset_count: usize,
        shutdown_count: usize,
    }

    impl FakeEngine {
        fn new() -> Self {
            Self {
                config: SherpaKwsConfig::default(),
                detect_next: false,
                fail_next: false,
                reset_count: 0,
                shutdown_count: 0,
            }
        }
    }

    impl SherpaKwsEngine for FakeEngine {
        fn config(&self) -> &SherpaKwsConfig {
            &self.config
        }

        fn accept_pcm16_mono(
            &mut self,
            _sample_rate_hz: u32,
            _samples: &[i16],
        ) -> Result<Option<WakeWordDetection>, WakeWordError> {
            if self.fail_next {
                self.fail_next = false;
                return Err(WakeWordError::sanitized(
                    WakeWordErrorKind::Inference,
                    "failed /private/model.onnx token abcdefghijklmnopqrstuvwxyz123456",
                    true,
                ));
            }
            if self.detect_next {
                self.detect_next = false;
                Ok(Some(WakeWordDetection::v1_detected(1.0)))
            } else {
                Ok(None)
            }
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
            self.reset_count += 1;
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            self.shutdown_count += 1;
            Ok(())
        }
    }

    #[test]
    fn lifecycle_is_single_owner_and_duplicate_start_fails_closed() {
        let mut manager = WakeWordRuntimeManager::<FakeEngine>::disabled();
        assert_eq!(manager.state(), WakeWordRuntimeState::Disabled);
        manager.begin_loading().unwrap();
        assert_eq!(manager.state(), WakeWordRuntimeState::Loading);
        assert!(manager.begin_loading().is_err());
        manager.finish_loading(FakeEngine::new()).unwrap();
        assert_eq!(manager.state(), WakeWordRuntimeState::Listening);
    }

    #[test]
    fn one_detection_creates_one_trigger_until_resume() {
        let mut engine = FakeEngine::new();
        engine.detect_next = true;
        let mut manager = WakeWordRuntimeManager::disabled();
        manager.begin_loading().unwrap();
        manager.finish_loading(engine).unwrap();

        let detection = manager.feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[1, 2, 3]).unwrap();
        assert!(detection.is_some());
        assert_eq!(manager.state(), WakeWordRuntimeState::Triggered);
        assert_eq!(manager.trigger_count(), 1);
        assert!(manager.last_trigger_age().is_some());
        assert!(manager
            .feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[4, 5])
            .unwrap()
            .is_none());
        assert_eq!(manager.trigger_count(), 1);

        manager.suspend().unwrap();
        manager.resume().unwrap();
        assert_eq!(manager.state(), WakeWordRuntimeState::Listening);
        assert!(manager.pre_roll_snapshot().is_empty());
    }

    #[test]
    fn later_detection_after_resume_counts_as_new_trigger() {
        let mut engine = FakeEngine::new();
        engine.detect_next = true;
        let mut manager = WakeWordRuntimeManager::disabled();
        manager.begin_loading().unwrap();
        manager.finish_loading(engine).unwrap();
        assert!(manager
            .feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[1])
            .unwrap()
            .is_some());
        manager.suspend().unwrap();
        manager.resume().unwrap();
        manager.engine.as_mut().unwrap().detect_next = true;
        assert!(manager
            .feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[2])
            .unwrap()
            .is_some());
        assert_eq!(manager.trigger_count(), 2);
    }

    #[test]
    fn pre_roll_tracks_same_pcm_fed_to_kws() {
        let mut manager = WakeWordRuntimeManager::disabled();
        manager.begin_loading().unwrap();
        manager.finish_loading(FakeEngine::new()).unwrap();
        manager.feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[10, 20]).unwrap();
        manager.feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[30, 40]).unwrap();
        assert_eq!(manager.pre_roll_snapshot(), vec![10, 20, 30, 40]);
    }

    #[test]
    fn stop_is_idempotent_and_clears_audio() {
        let mut manager = WakeWordRuntimeManager::disabled();
        manager.begin_loading().unwrap();
        manager.finish_loading(FakeEngine::new()).unwrap();
        manager.feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[7, 8, 9]).unwrap();
        manager.stop().unwrap();
        assert_eq!(manager.state(), WakeWordRuntimeState::Disabled);
        assert!(manager.pre_roll_snapshot().is_empty());
        manager.stop().unwrap();
        assert_eq!(manager.state(), WakeWordRuntimeState::Disabled);
    }

    #[test]
    fn invalid_resume_fails_without_hidden_fallback() {
        let mut manager = WakeWordRuntimeManager::<FakeEngine>::disabled();
        let error = manager.resume().unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::InvalidConfiguration);
        assert_eq!(manager.state(), WakeWordRuntimeState::Disabled);
    }

    #[test]
    fn diagnostics_are_bounded_and_contain_no_pcm() {
        let mut manager = WakeWordRuntimeManager::disabled();
        manager.begin_loading().unwrap();
        manager.finish_loading(FakeEngine::new()).unwrap();
        manager
            .feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[1234, -2345])
            .unwrap();
        let diagnostics = manager.diagnostics();
        assert!(diagnostics.enabled);
        assert_eq!(diagnostics.state, WakeWordRuntimeState::Listening);
        assert_eq!(diagnostics.sample_rate_hz, 16_000);
        assert_eq!(diagnostics.channels, 1);
        assert_eq!(diagnostics.inference_threads, 1);
        assert_eq!(diagnostics.ring_buffer_capacity_samples, 32_000);
        assert_eq!(diagnostics.ring_buffer_duration_ms, 2_000);
        assert_eq!(diagnostics.trigger_count, 0);
        assert!(diagnostics.last_error.is_none());
    }

    #[test]
    fn diagnostics_retain_only_sanitized_error_text() {
        let mut engine = FakeEngine::new();
        engine.fail_next = true;
        let mut manager = WakeWordRuntimeManager::disabled();
        manager.begin_loading().unwrap();
        manager.finish_loading(engine).unwrap();
        assert!(manager.feed_pcm(V1_KWS_SAMPLE_RATE_HZ, &[1]).is_err());
        let diagnostics = manager.diagnostics();
        assert_eq!(diagnostics.state, WakeWordRuntimeState::Error);
        assert_eq!(
            diagnostics.last_error.as_deref(),
            Some("failed <path> token <redacted>")
        );
    }
}
