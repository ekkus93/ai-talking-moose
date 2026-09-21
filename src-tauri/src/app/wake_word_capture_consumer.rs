use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_pcm_router::{CanonicalWakePcmRouter, WakePcmRouteError, WakePcmRouteOutcome};
use crate::audio::capture::{AudioCapture, AudioCaptureError};
use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
use std::time::Instant;
use tokio::sync::mpsc;

const WAKE_CAPTURE_QUEUE_CHUNKS: usize = 16;

/// Start Wake Word capture through the application's one authoritative `AudioCapture` owner.
///
/// `AudioCapture::start` replaces any stream already owned by that same object before opening the
/// requested stream, so this boundary cannot create a second independently-owned microphone
/// stream. Capture performs the one source->mono->16 kHz canonicalization and this module only
/// decodes the resulting PCM16-LE bytes.
pub(crate) fn start_authoritative_wake_capture(
    capture: &mut AudioCapture,
    device_name: Option<String>,
) -> Result<mpsc::Receiver<Vec<u8>>, AudioCaptureError> {
    let (pcm_sender, pcm_receiver) = mpsc::channel(WAKE_CAPTURE_QUEUE_CHUNKS);
    capture.start(device_name, V1_KWS_SAMPLE_RATE_HZ, pcm_sender, None)?;
    Ok(pcm_receiver)
}

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
    if !pcm16_le.len().is_multiple_of(2) {
        return Err(WakeCapturePcmError::OddByteLength);
    }

    let (sample_bytes, remainder) = pcm16_le.as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    Ok(sample_bytes
        .iter()
        .map(|bytes| i16::from_le_bytes(*bytes))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{
        validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
    };
    use crate::app::wake_word::runtime::{WakeWordRuntimeManager, WakeWordRuntimePhase};
    use crate::audio::capture::AudioCaptureMode;

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
    fn authoritative_wake_capture_uses_existing_owner_and_canonical_rate() {
        let mut capture = AudioCapture::new_mock();

        let _receiver = start_authoritative_wake_capture(&mut capture, None).unwrap();

        assert_eq!(capture.mode(), AudioCaptureMode::Mock);
        assert!(capture.is_active());
        let diagnostics = capture.diagnostics();
        assert_eq!(diagnostics.sample_rate_hz, Some(V1_KWS_SAMPLE_RATE_HZ));
        assert_eq!(diagnostics.channels, Some(1));
        assert_eq!(diagnostics.sample_format.as_deref(), Some("I16"));
    }

    #[test]
    fn restarting_wake_capture_replaces_stream_on_same_owner() {
        let mut capture = AudioCapture::new_mock();
        let first_receiver = start_authoritative_wake_capture(&mut capture, None).unwrap();
        assert!(capture.is_active());

        let second_receiver = start_authoritative_wake_capture(&mut capture, None).unwrap();

        assert!(capture.is_active());
        assert_eq!(
            capture.diagnostics().sample_rate_hz,
            Some(V1_KWS_SAMPLE_RATE_HZ)
        );
        drop(first_receiver);
        drop(second_receiver);
        capture.stop();
        assert!(!capture.is_active());
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
    fn wake_phrase_and_immediate_first_command_word_survive_handoff_untrimmed() {
        let engine = RecordingEngine {
            detect_next: true,
            ..Default::default()
        };
        let mut consumer = consumer_with_engine(engine);
        let now = Instant::now();

        // Synthetic contiguous markers stand in for the tail of "Hey, Moose" followed immediately
        // by the first command word. V1 must preserve both sides of this trigger/live boundary.
        let wake_phrase_tail = [101, 102, 103, 104];
        let first_command_word = [105, 106, 107, 108];
        assert!(consumer
            .route_capture_chunk(&bytes(&wake_phrase_tail), now)
            .unwrap()
            .trigger_accepted);
        assert!(consumer
            .route_capture_chunk(&bytes(&first_command_word), now)
            .unwrap()
            .live_handoff_retained);

        let handoff = consumer
            .transfer_handoff_audio_to_asr()
            .unwrap()
            .expect("wake phrase plus command word should reach command ASR");
        assert_eq!(
            handoff.samples_i16(),
            &[101, 102, 103, 104, 105, 106, 107, 108]
        );
        assert_eq!(
            handoff.to_pcm16_le_bytes(),
            bytes(&[101, 102, 103, 104, 105, 106, 107, 108])
        );
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
