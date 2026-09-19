use super::wake_word::engine::{
    validate_pcm_frame, SherpaKwsEngine, WakeWordDetection, WakeWordError,
};
use super::wake_word::runtime::{WakeWordRuntimeError, WakeWordRuntimeManager};
use std::time::Instant;

/// Result of routing one canonical microphone chunk through the Wake Word listening path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WakePcmRouteOutcome {
    pub retained: bool,
    pub detection: Option<WakeWordDetection>,
    pub trigger_accepted: bool,
}

#[derive(Debug)]
pub(crate) enum WakePcmRouteError {
    Engine(WakeWordError),
    Runtime(WakeWordRuntimeError),
}

/// Routes the single canonical 16-kHz mono microphone timeline to both Wake Word consumers.
///
/// This type deliberately owns no capture device and performs no resampling. `AudioCapture`
/// remains the sole physical microphone owner and canonicalizes the stream once. Each chunk is
/// validated once at this boundary, then the exact same sample slice is retained in chronological
/// pre-roll and fed to KWS. A caller must keep one router on the capture-consumer task and call
/// `route` serially; that makes chunk ordering explicit rather than introducing a second queue.
pub(crate) struct CanonicalWakePcmRouter<E: SherpaKwsEngine> {
    runtime: WakeWordRuntimeManager,
    engine: E,
}

impl<E: SherpaKwsEngine> CanonicalWakePcmRouter<E> {
    pub(crate) fn new(runtime: WakeWordRuntimeManager, engine: E) -> Self {
        Self { runtime, engine }
    }

    pub(crate) fn route(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakePcmRouteError> {
        // Reject non-canonical input before either consumer can mutate state.
        validate_pcm_frame(sample_rate_hz, samples).map_err(WakePcmRouteError::Engine)?;

        let retained = self
            .runtime
            .append_listening_pcm_frame(sample_rate_hz, samples)
            .map_err(WakePcmRouteError::Runtime)?;
        if !retained {
            return Ok(WakePcmRouteOutcome {
                retained: false,
                detection: None,
                trigger_accepted: false,
            });
        }

        // Feed exactly the same already-canonicalized slice that was retained above.
        let detection = self
            .engine
            .accept_pcm16_mono(sample_rate_hz, samples)
            .map_err(WakePcmRouteError::Engine)?;
        let trigger_accepted = if detection.is_some() {
            self.runtime
                .accept_trigger(now)
                .map_err(WakePcmRouteError::Runtime)?
        } else {
            false
        };

        Ok(WakePcmRouteOutcome {
            retained,
            detection,
            trigger_accepted,
        })
    }

    pub(crate) fn runtime(&self) -> &WakeWordRuntimeManager {
        &self.runtime
    }

    pub(crate) fn engine_mut(&mut self) -> &mut E {
        &mut self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{SherpaKwsConfig, WakeWordDetection};
    use crate::app::wake_word::runtime::WakeWordRuntimePhase;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

    #[derive(Default)]
    struct RecordingEngine {
        config: SherpaKwsConfig,
        frames: Vec<Vec<i16>>,
        detect_next: bool,
    }

    impl SherpaKwsEngine for RecordingEngine {
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
            if std::mem::take(&mut self.detect_next) {
                Ok(Some(WakeWordDetection::v1_detected(1.0)))
            } else {
                Ok(None)
            }
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
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
    fn same_canonical_chunks_reach_ring_and_kws_in_order() {
        let runtime = listening_runtime();
        let mut router = CanonicalWakePcmRouter::new(runtime, RecordingEngine::default());
        let now = Instant::now();

        router.route(V1_KWS_SAMPLE_RATE_HZ, &[1, 2], now).unwrap();
        router.route(V1_KWS_SAMPLE_RATE_HZ, &[3, 4], now).unwrap();

        assert_eq!(router.engine_mut().frames, vec![vec![1, 2], vec![3, 4]]);
        assert!(router.runtime().accept_trigger(now).unwrap());
        assert_eq!(
            router.runtime().take_triggered_pre_roll().unwrap().unwrap(),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn invalid_frame_mutates_neither_ring_nor_kws() {
        let runtime = listening_runtime();
        let mut router = CanonicalWakePcmRouter::new(runtime, RecordingEngine::default());
        router
            .route(V1_KWS_SAMPLE_RATE_HZ, &[7, 8], Instant::now())
            .unwrap();
        let before = router.runtime().snapshot(Instant::now());

        assert!(router.route(48_000, &[9, 10], Instant::now()).is_err());
        assert!(router
            .route(V1_KWS_SAMPLE_RATE_HZ, &[], Instant::now())
            .is_err());

        assert_eq!(router.engine_mut().frames, vec![vec![7, 8]]);
        assert_eq!(
            router
                .runtime()
                .snapshot(Instant::now())
                .ring_buffer_samples,
            before.ring_buffer_samples
        );
    }

    #[test]
    fn detection_atomically_moves_runtime_out_of_listening_for_later_chunks() {
        let runtime = listening_runtime();
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut router = CanonicalWakePcmRouter::new(runtime, engine);
        let now = Instant::now();

        let detected = router.route(V1_KWS_SAMPLE_RATE_HZ, &[11, 12], now).unwrap();
        assert!(detected.retained);
        assert!(detected.detection.is_some());
        assert!(detected.trigger_accepted);
        assert_eq!(
            router.runtime().snapshot(now).phase,
            WakeWordRuntimePhase::Triggered
        );

        let after = router.route(V1_KWS_SAMPLE_RATE_HZ, &[13, 14], now).unwrap();
        assert!(!after.retained);
        assert!(after.detection.is_none());
        assert!(!after.trigger_accepted);
        assert_eq!(router.engine_mut().frames, vec![vec![11, 12]]);
        assert_eq!(
            router.runtime().take_triggered_pre_roll().unwrap().unwrap(),
            vec![11, 12]
        );
    }

    #[test]
    fn disabled_runtime_never_feeds_kws() {
        let runtime = WakeWordRuntimeManager::new();
        let mut router = CanonicalWakePcmRouter::new(runtime, RecordingEngine::default());
        let outcome = router
            .route(V1_KWS_SAMPLE_RATE_HZ, &[1], Instant::now())
            .unwrap();
        assert!(!outcome.retained);
        assert!(outcome.detection.is_none());
        assert!(router.engine_mut().frames.is_empty());
    }
}
