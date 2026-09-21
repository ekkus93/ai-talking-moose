use super::wake_word::engine::{
    validate_pcm_frame, SherpaKwsConfig, SherpaKwsEngine, WakeWordDetection, WakeWordError,
};
use super::wake_word::runtime::WakeWordRuntimeManager;
use super::wake_word_pcm_router::CanonicalWakePcmRouter;
use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
use std::time::Instant;

#[derive(Default)]
struct TriggerEngine {
    config: SherpaKwsConfig,
    detect_next: bool,
    reset_count: usize,
}

impl SherpaKwsEngine for TriggerEngine {
    fn config(&self) -> &SherpaKwsConfig {
        &self.config
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        validate_pcm_frame(sample_rate_hz, samples)?;
        if std::mem::take(&mut self.detect_next) {
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
        Ok(())
    }
}

fn listening_runtime() -> WakeWordRuntimeManager {
    let runtime = WakeWordRuntimeManager::new();
    runtime.begin_enable().unwrap();
    runtime.mark_loaded().unwrap();
    runtime
}

#[test]
fn second_phrase_after_resume_yields_second_trigger_without_cooldown() {
    let runtime = listening_runtime();
    let engine = TriggerEngine {
        detect_next: true,
        ..Default::default()
    };
    let mut router = CanonicalWakePcmRouter::new(runtime, engine);
    let now = Instant::now();

    let first = router
        .route(V1_KWS_SAMPLE_RATE_HZ, &[10, 11], now)
        .unwrap();
    assert!(first.trigger_accepted);
    assert_eq!(router.runtime().snapshot(now).trigger_count, 1);
    assert!(router.transfer_handoff_to_asr().is_some());

    router.return_to_wake_listening().unwrap();
    assert_eq!(router.engine_mut().reset_count, 1);

    router.engine_mut().detect_next = true;
    let second = router
        .route(V1_KWS_SAMPLE_RATE_HZ, &[20, 21], now)
        .unwrap();
    assert!(second.trigger_accepted);
    assert_eq!(router.runtime().snapshot(now).trigger_count, 2);
    assert!(router.transfer_handoff_to_asr().is_some());
}
