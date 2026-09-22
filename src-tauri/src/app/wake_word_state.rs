use super::state::AppState;
use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_composition::WakeWordApplicationRuntime;

/// Access the one Wake Word runtime owned directly by authoritative application state.
///
/// Physical microphone capture remains exclusively owned by `AppState::audio_capture`;
/// the Wake runtime owns lifecycle/KWS state only.
pub(crate) fn runtime_from_app_state(state: &AppState) -> &WakeWordApplicationRuntime {
    &state.wake_word_runtime
}

/// Compose Wake routing around the one microphone owner stored in authoritative application state.
///
/// This function is the production boundary for WWR-300: Wake receives PCM through the same
/// `AppState::audio_capture` object used by manual command listening instead of constructing a
/// competing capture owner. The returned owner still requires a caller-supplied capture consumer so
/// production can use a verified native KWS session while tests can keep deterministic fake engines.
pub(crate) fn capture_owner_from_app_state<E: SherpaKwsEngine>(
    state: &AppState,
) -> AuthoritativeWakeCaptureOwner<E> {
    AuthoritativeWakeCaptureOwner::from_shared_capture(state.audio_capture.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{
        validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
    };
    use crate::asr::wake_word_runtime::WakeWordRuntimePhase;
    use crate::audio::capture::AudioCapture;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

    #[derive(Default)]
    struct TestWakeEngine {
        config: SherpaKwsConfig,
    }

    impl SherpaKwsEngine for TestWakeEngine {
        fn config(&self) -> &SherpaKwsConfig {
            &self.config
        }

        fn accept_pcm16_mono(
            &mut self,
            sample_rate_hz: u32,
            samples: &[i16],
        ) -> Result<Option<WakeWordDetection>, WakeWordError> {
            validate_pcm_frame(sample_rate_hz, samples)?;
            Ok(None)
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }
    }

    #[test]
    fn app_state_directly_owns_wake_runtime_without_owning_capture() {
        let state = AppState::new_for_tests().unwrap();
        assert!(!state.settings.read().wake_word_enabled);
        assert_eq!(
            runtime_from_app_state(&state).phase(),
            WakeWordRuntimePhase::Disabled
        );
        assert!(!state.audio_capture.lock().diagnostics().active);
    }

    #[tokio::test]
    async fn capture_owner_from_app_state_uses_exact_app_capture() {
        let state = AppState::new_for_tests().unwrap();
        *state.audio_capture.lock() = AudioCapture::new_mock();
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        state.wake_word_runtime.mark_loaded().unwrap();

        let owner = capture_owner_from_app_state::<TestWakeEngine>(&state);
        let consumer = state
            .wake_word_runtime
            .capture_consumer(TestWakeEngine::default());

        owner.start_wake(None, consumer).await.unwrap();
        assert!(state.audio_capture.lock().is_active());
        assert_eq!(
            state.audio_capture.lock().diagnostics().sample_rate_hz,
            Some(V1_KWS_SAMPLE_RATE_HZ)
        );

        owner.disable().await;
        assert!(!state.audio_capture.lock().is_active());
        assert_eq!(
            state.wake_word_runtime.snapshot(std::time::Instant::now()).phase,
            WakeWordRuntimePhase::Disabled
        );
    }
}
