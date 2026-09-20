use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_pcm_router::{CanonicalWakePcmRouter, WakePcmRouteError, WakePcmRouteOutcome};
use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
use std::time::Instant;

/// Byte-level consumer for the one authoritative `AudioCapture` microphone stream.
///
/// `AudioCapture` owns the physical device and emits canonical little-endian PCM16 chunks at the
/// requested target rate. This adapter deliberately owns no capture device and performs no
/// resampling: it only decodes the bytes from that existing stream into i16 samples once, then
/// feeds the `CanonicalWakePcmRouter` serially so the ring buffer and KWS see the same
/// chronological sample timeline.
pub(crate) struct WakeCapturePcmConsumer<E: SherpaKwsEngine> {
    router: CanonicalWakePcmRouter<E>,
}

#[derive(Debug)]
pub(crate) enum WakeCapturePcmError {
    EmptyChunk,
    OddByteLength,
    Route(WakePcmRouteError),
    Handoff(String),
}

impl<E: SherpaKwsEngine> WakeCapturePcmConsumer<E> {
    pub(crate) fn new(router: CanonicalWakePcmRouter<E>) -> Self {
        Self { router }
    }

    /// Decode one canonical PCM16-LE microphone chunk and route it through Wake Word.
    ///
    /// The conversion is intentionally lossless and local to this boundary. Invalid byte chunks
    /// fail before the router can mutate ring, KWS, or handoff state.
    pub(crate) fn route_capture_chunk(
        &mut self,
        pcm16_le: &[u8],
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakeCapturePcmError> {
        let samples = decode_pcm16_le_chunk(pcm16_le)?;
        self.router
            .route(V1_KWS_SAMPLE_RATE_HZ, &samples, now)
            .map_err(WakeCapturePcmError::Route)
    }

    /// Transfer accumulated wake pre-roll plus post-trigger live PCM to the command-ASR payload.
    pub(crate) fn transfer_handoff_audio_to_asr(
        &mut self,
    ) -> Result<Option<WakeCommandHandoffAudio>, WakeCapturePcmError> {
        self.router
            .transfer_handoff_audio_to_asr()
            .map_err(WakeCapturePcmError::Handoff)
    }

    pub(crate) fn clear_handoff(&mut self) {
        self.router.clear_handoff();
    }

    pub(crate) fn return_to_wake_listening(&mut self) -> Result<(), WakeCapturePcmError> {
        self.router
            .return_to_wake_listening()
            .map_err(WakeCapturePcmError::Route)
    }

    pub(crate) fn handoff_live_samples(&self) -> usize {
        self.router.handoff_live_samples()
    }

    pub(crate) fn router(&self) -> &CanonicalWakePcmRouter<E> {
        &self.router
    }

    pub(crate) fn router_mut(&mut self) -> &mut CanonicalWakePcmRouter<E> {
        &mut self.router
    }
}

fn decode_pcm16_le_chunk(pcm16_le: &[u8]) -> Result<Vec<i16>, WakeCapturePcmError> {
    if pcm16_le.is_empty() {
        return Err(WakeCapturePcmError::EmptyChunk);
    }
    if pcm16_le.len() % 2 != 0 {
        return Err(WakeCapturePcmError::OddByteLength);
    }

    Ok(pcm16_le
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{
        validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
    };
    use crate::app::wake_word::runtime::{WakeWordRuntimeManager, WakeWordRuntimePhase};

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

    fn consumer_with_engine(engine: RecordingEngine) -> WakeCapturePcmConsumer<RecordingEngine> {
        WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(listening_runtime(), engine))
    }

    fn bytes(samples: &[i16]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }

    #[test]
    fn capture_bytes_are_decoded_and_routed_in_order() {
        let mut consumer = consumer_with_engine(RecordingEngine::default());
        let now = Instant::now();

        consumer.route_capture_chunk(&bytes(&[1, -2]), now).unwrap();
        consumer.route_capture_chunk(&bytes(&[3, -4]), now).unwrap();

        assert_eq!(
            consumer.router_mut().engine_mut().frames,
            vec![vec![1, -2], vec![3, -4]]
        );
        assert_eq!(
            consumer
                .router()
                .runtime()
                .snapshot(now)
                .ring_buffer_samples,
            4
        );
    }

    #[test]
    fn invalid_capture_bytes_do_not_mutate_router_state() {
        let mut consumer = consumer_with_engine(RecordingEngine::default());
        let now = Instant::now();
        consumer.route_capture_chunk(&bytes(&[7, 8]), now).unwrap();
        let before = consumer.router().runtime().snapshot(now);

        assert!(matches!(
            consumer.route_capture_chunk(&[], now),
            Err(WakeCapturePcmError::EmptyChunk)
        ));
        assert!(matches!(
            consumer.route_capture_chunk(&[1, 2, 3], now),
            Err(WakeCapturePcmError::OddByteLength)
        ));

        assert_eq!(consumer.router_mut().engine_mut().frames, vec![vec![7, 8]]);
        assert_eq!(
            consumer
                .router()
                .runtime()
                .snapshot(now)
                .ring_buffer_samples,
            before.ring_buffer_samples
        );
    }

    #[test]
    fn trigger_then_live_capture_transfers_one_command_payload() {
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut consumer = consumer_with_engine(engine);
        let now = Instant::now();

        let trigger = consumer
            .route_capture_chunk(&bytes(&[11, 12]), now)
            .unwrap();
        assert!(trigger.trigger_accepted);
        assert_eq!(
            consumer.router().runtime().snapshot(now).phase,
            WakeWordRuntimePhase::Triggered
        );

        let live = consumer
            .route_capture_chunk(&bytes(&[13, 14]), now)
            .unwrap();
        assert!(live.live_handoff_retained);
        assert_eq!(consumer.handoff_live_samples(), 2);

        let handoff = consumer
            .transfer_handoff_audio_to_asr()
            .unwrap()
            .expect("accepted trigger should create command handoff audio");
        assert_eq!(handoff.sample_rate_hz(), V1_KWS_SAMPLE_RATE_HZ);
        assert_eq!(handoff.samples_i16(), &[11, 12, 13, 14]);
        assert_eq!(handoff.to_pcm16_le_bytes(), bytes(&[11, 12, 13, 14]));
        assert!(consumer.transfer_handoff_audio_to_asr().unwrap().is_none());
    }

    #[test]
    fn returning_to_wake_resets_kws_and_allows_later_capture() {
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut consumer = consumer_with_engine(engine);
        let now = Instant::now();

        consumer
            .route_capture_chunk(&bytes(&[21, 22]), now)
            .unwrap();
        consumer
            .route_capture_chunk(&bytes(&[23, 24]), now)
            .unwrap();
        assert!(consumer.transfer_handoff_audio_to_asr().unwrap().is_some());

        consumer.return_to_wake_listening().unwrap();
        assert_eq!(consumer.router_mut().engine_mut().reset_count, 1);
        assert_eq!(
            consumer.router().runtime().snapshot(now).phase,
            WakeWordRuntimePhase::Listening
        );

        consumer
            .route_capture_chunk(&bytes(&[25, 26]), now)
            .unwrap();
        assert_eq!(
            consumer.router_mut().engine_mut().frames,
            vec![vec![21, 22], vec![25, 26]]
        );
    }
}
