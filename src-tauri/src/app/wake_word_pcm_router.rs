use super::wake_word::engine::{
    validate_pcm_frame, SherpaKwsEngine, WakeWordDetection, WakeWordError,
};
use super::wake_word::runtime::{WakeWordRuntimeError, WakeWordRuntimeManager};
use crate::asr::wake_word_handoff::WakeAsrHandoff;
use std::time::Instant;

/// Result of routing one canonical microphone chunk through the Wake Word listening path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WakePcmRouteOutcome {
    pub retained: bool,
    pub detection: Option<WakeWordDetection>,
    pub trigger_accepted: bool,
    pub live_handoff_retained: bool,
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
    handoff: Option<WakeAsrHandoff>,
}

impl<E: SherpaKwsEngine> CanonicalWakePcmRouter<E> {
    pub(crate) fn new(runtime: WakeWordRuntimeManager, engine: E) -> Self {
        Self {
            runtime,
            engine,
            handoff: None,
        }
    }

    pub(crate) fn route(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakePcmRouteError> {
        // Reject non-canonical input before either consumer can mutate state.
        validate_pcm_frame(sample_rate_hz, samples).map_err(WakePcmRouteError::Engine)?;

        if let Some(handoff) = self.handoff.as_mut() {
            handoff
                .append_live(sample_rate_hz, samples)
                .map_err(WakePcmRouteError::Engine)?;
            return Ok(WakePcmRouteOutcome {
                retained: false,
                detection: None,
                trigger_accepted: false,
                live_handoff_retained: true,
            });
        }

        let retained = self
            .runtime
            .append_listening_pcm_frame(sample_rate_hz, samples)
            .map_err(WakePcmRouteError::Runtime)?;
        if !retained {
            return Ok(WakePcmRouteOutcome {
                retained: false,
                detection: None,
                trigger_accepted: false,
                live_handoff_retained: false,
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
        if trigger_accepted {
            if let Some(pre_roll) = self
                .runtime
                .take_triggered_pre_roll()
                .map_err(WakePcmRouteError::Runtime)?
            {
                self.handoff = Some(WakeAsrHandoff::new(pre_roll));
            }
        }

        Ok(WakePcmRouteOutcome {
            retained,
            detection,
            trigger_accepted,
            live_handoff_retained: false,
        })
    }

    /// Transfer the accumulated wake pre-roll plus post-trigger live PCM to command ASR once.
    ///
    /// Removing the handoff as part of transfer makes ownership explicit: after this call the
    /// router no longer buffers live startup audio and the command-ASR side owns the returned
    /// chronological sample vector.
    pub(crate) fn transfer_handoff_to_asr(&mut self) -> Option<Vec<i16>> {
        let mut handoff = self.handoff.take()?;
        handoff.take_for_asr()
    }

    pub(crate) fn take_handoff_for_asr(&mut self) -> Option<Vec<i16>> {
        self.transfer_handoff_to_asr()
    }

    pub(crate) fn clear_handoff(&mut self) {
        if let Some(handoff) = self.handoff.as_mut() {
            handoff.clear();
        }
        self.handoff = None;
    }

    /// Return ownership from command ASR to wake listening after interaction completion/failure.
    ///
    /// The KWS stream is reset before the runtime can listen again so stale inference state cannot
    /// trigger on samples from the previous command interaction.
    pub(crate) fn return_to_wake_listening(&mut self) -> Result<(), WakePcmRouteError> {
        self.clear_handoff();
        self.engine
            .reset_stream()
            .map_err(WakePcmRouteError::Engine)?;
        self.runtime
            .resume_after_interaction()
            .map_err(WakePcmRouteError::Runtime)
    }

    pub(crate) fn handoff_live_samples(&self) -> usize {
        self.handoff
            .as_ref()
            .map_or(0, WakeAsrHandoff::live_samples)
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
        reset_count: usize,
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
    fn detection_atomically_moves_runtime_out_of_listening_and_starts_handoff() {
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
        assert_eq!(router.handoff_live_samples(), 0);
        assert_eq!(router.engine_mut().frames, vec![vec![11, 12]]);
    }

    #[test]
    fn post_trigger_chunks_are_preserved_for_command_asr_without_refeeding_kws() {
        let runtime = listening_runtime();
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut router = CanonicalWakePcmRouter::new(runtime, engine);
        let now = Instant::now();

        router.route(V1_KWS_SAMPLE_RATE_HZ, &[11, 12], now).unwrap();
        let after = router.route(V1_KWS_SAMPLE_RATE_HZ, &[13, 14], now).unwrap();
        assert!(!after.retained);
        assert!(after.detection.is_none());
        assert!(!after.trigger_accepted);
        assert!(after.live_handoff_retained);
        assert_eq!(router.engine_mut().frames, vec![vec![11, 12]]);
        assert_eq!(router.handoff_live_samples(), 2);
        assert_eq!(
            router.transfer_handoff_to_asr().unwrap(),
            vec![11, 12, 13, 14]
        );
    }

    #[test]
    fn handoff_transfer_is_single_use_and_return_resumes_listening() {
        let runtime = listening_runtime();
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut router = CanonicalWakePcmRouter::new(runtime, engine);
        let now = Instant::now();

        router.route(V1_KWS_SAMPLE_RATE_HZ, &[11, 12], now).unwrap();
        router.route(V1_KWS_SAMPLE_RATE_HZ, &[13, 14], now).unwrap();
        assert_eq!(
            router.transfer_handoff_to_asr().unwrap(),
            vec![11, 12, 13, 14]
        );
        assert_eq!(router.transfer_handoff_to_asr(), None);

        let during_transfer = router.route(V1_KWS_SAMPLE_RATE_HZ, &[15, 16], now).unwrap();
        assert!(!during_transfer.retained);
        assert!(!during_transfer.live_handoff_retained);
        assert_eq!(router.engine_mut().frames, vec![vec![11, 12]]);

        router.return_to_wake_listening().unwrap();
        assert_eq!(
            router.runtime().snapshot(now).phase,
            WakeWordRuntimePhase::Listening
        );
        assert_eq!(router.engine_mut().reset_count, 1);

        let resumed = router.route(V1_KWS_SAMPLE_RATE_HZ, &[17, 18], now).unwrap();
        assert!(resumed.retained);
        assert_eq!(
            router.engine_mut().frames,
            vec![vec![11, 12], vec![17, 18]]
        );
    }

    #[test]
    fn invalid_post_trigger_frame_does_not_mutate_live_handoff() {
        let runtime = listening_runtime();
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut router = CanonicalWakePcmRouter::new(runtime, engine);
        let now = Instant::now();

        router.route(V1_KWS_SAMPLE_RATE_HZ, &[21, 22], now).unwrap();
        router.route(V1_KWS_SAMPLE_RATE_HZ, &[23, 24], now).unwrap();
        let before_live = router.handoff_live_samples();

        assert!(router.route(48_000, &[25, 26], now).is_err());
        assert!(router.route(V1_KWS_SAMPLE_RATE_HZ, &[], now).is_err());
        assert_eq!(router.handoff_live_samples(), before_live);
        assert_eq!(router.take_handoff_for_asr().unwrap(), vec![21, 22, 23, 24]);
    }

    #[test]
    fn clearing_handoff_drops_stale_audio_after_failed_startup() {
        let runtime = listening_runtime();
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut router = CanonicalWakePcmRouter::new(runtime, engine);
        let now = Instant::now();

        router.route(V1_KWS_SAMPLE_RATE_HZ, &[31, 32], now).unwrap();
        router.route(V1_KWS_SAMPLE_RATE_HZ, &[33, 34], now).unwrap();
        assert_eq!(router.handoff_live_samples(), 2);

        router.clear_handoff();

        assert_eq!(router.handoff_live_samples(), 0);
        assert_eq!(router.take_handoff_for_asr(), None);
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
        assert!(!outcome.live_handoff_retained);
        assert!(router.engine_mut().frames.is_empty());
    }
}
