use super::state::AppState;
use super::wake_word::engine::{
    validate_pcm_frame, SherpaKwsConfig, SherpaKwsEngine, WakeWordDetection, WakeWordError,
};
use super::wake_word::runtime::WakeWordRuntimePhase;
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use crate::audio::capture::AudioCapture;
use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
use std::time::Instant;

#[derive(Default)]
struct TestWakeEngine {
    config: SherpaKwsConfig,
    frames: Vec<Vec<i16>>,
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
        self.frames.push(samples.to_vec());
        Ok(None)
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        Ok(())
    }
}

fn bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

#[tokio::test]
async fn app_state_composes_shared_capture_with_authoritative_wake_runtime() {
    let state = AppState::new_for_tests().unwrap();
    *state.audio_capture.lock() = AudioCapture::new_mock();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();

    let owner = AuthoritativeWakeCaptureOwner::<TestWakeEngine>::from_shared_capture(
        state.audio_capture.clone(),
    );
    let mut consumer = state
        .wake_word_runtime
        .capture_consumer(TestWakeEngine::default());

    consumer
        .route_capture_chunk(&bytes(&[1, 2, 3]), Instant::now())
        .unwrap();
    assert_eq!(
        state
            .wake_word_runtime
            .snapshot(Instant::now())
            .ring_buffer_samples,
        3,
        "Wake consumer must share AppState's authoritative Wake runtime manager"
    );

    owner.start_wake(None, consumer).await.unwrap();
    assert!(state.audio_capture.lock().is_active());
    assert_eq!(
        state.audio_capture.lock().diagnostics().sample_rate_hz,
        Some(V1_KWS_SAMPLE_RATE_HZ)
    );

    owner.disable().await;
    assert!(!state.audio_capture.lock().is_active());
    assert_eq!(
        state.wake_word_runtime.snapshot(Instant::now()).phase,
        WakeWordRuntimePhase::Disabled
    );
}
