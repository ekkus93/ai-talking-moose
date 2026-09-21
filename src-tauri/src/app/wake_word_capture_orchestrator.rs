use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_capture_consumer::{
    start_authoritative_wake_capture, WakeCapturePcmConsumer, WakeCapturePcmError,
};
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_pcm_router::WakePcmRouteOutcome;
use crate::audio::capture::{AudioCapture, AudioCaptureError};
use std::time::Instant;
use tokio::sync::mpsc;

/// Owns the receive side of the application's single authoritative Wake capture stream.
///
/// This coordinator deliberately does not own `AudioCapture`. The application retains that one
/// physical microphone owner and lends `&mut AudioCapture` only while replacing/starting its
/// current stream. Keeping the receiver and canonical consumer together prevents a second queue
/// or resampler from being introduced between capture, ring retention, and KWS.
pub(crate) struct WakeCaptureOrchestrator<E: SherpaKwsEngine> {
    receiver: mpsc::Receiver<Vec<u8>>,
    consumer: WakeCapturePcmConsumer<E>,
}

#[derive(Debug)]
pub(crate) enum WakeCaptureOrchestratorError {
    Capture(AudioCaptureError),
    CaptureClosed,
    Pcm(WakeCapturePcmError),
}

impl<E: SherpaKwsEngine> WakeCaptureOrchestrator<E> {
    pub(crate) fn start(
        capture: &mut AudioCapture,
        device_name: Option<String>,
        consumer: WakeCapturePcmConsumer<E>,
    ) -> Result<Self, WakeCaptureOrchestratorError> {
        let receiver = start_authoritative_wake_capture(capture, device_name)
            .map_err(WakeCaptureOrchestratorError::Capture)?;
        Ok(Self { receiver, consumer })
    }

    /// Route exactly one next canonical capture chunk, preserving receiver order.
    ///
    /// A closed capture queue is a terminal event for this listening epoch. Fail the Wake runtime
    /// closed and discard any pending handoff so a device disconnect cannot leave stale pre-roll
    /// or a false Listening state. Reconnect is deliberately serialized by the application through
    /// a later `start` on the same `AudioCapture` owner; this path never opens a replacement stream.
    pub(crate) async fn route_next(
        &mut self,
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakeCaptureOrchestratorError> {
        let Some(chunk) = self.receiver.recv().await else {
            self.consumer.clear_handoff();
            self.consumer.router().runtime().record_runtime_error();
            return Err(WakeCaptureOrchestratorError::CaptureClosed);
        };
        self.consumer
            .route_capture_chunk(&chunk, now)
            .map_err(WakeCaptureOrchestratorError::Pcm)
    }

    pub(crate) fn transfer_handoff_audio_to_asr(
        &mut self,
    ) -> Result<Option<WakeCommandHandoffAudio>, WakeCaptureOrchestratorError> {
        self.consumer
            .transfer_handoff_audio_to_asr()
            .map_err(WakeCaptureOrchestratorError::Pcm)
    }

    pub(crate) fn clear_handoff(&mut self) {
        self.consumer.clear_handoff();
    }

    pub(crate) fn return_to_wake_listening(&mut self) -> Result<(), WakeCaptureOrchestratorError> {
        self.consumer
            .return_to_wake_listening()
            .map_err(WakeCaptureOrchestratorError::Pcm)
    }

    /// Disable Wake Word without creating or retaining another microphone stream.
    ///
    /// The sole application capture owner is stopped, all pending handoff audio is discarded, and
    /// the shared Wake runtime is transitioned to Disabled. Manual listen may subsequently start
    /// this same `AudioCapture` object through the existing command path.
    pub(crate) fn disable(&mut self, capture: &mut AudioCapture) {
        capture.stop();
        self.consumer.clear_handoff();
        self.consumer.router().runtime().disable();
    }

    pub(crate) fn consumer(&self) -> &WakeCapturePcmConsumer<E> {
        &self.consumer
    }

    pub(crate) fn consumer_mut(&mut self) -> &mut WakeCapturePcmConsumer<E> {
        &mut self.consumer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{
        validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
    };
    use crate::app::wake_word::runtime::{WakeWordRuntimeManager, WakeWordRuntimePhase};
    use crate::app::wake_word_pcm_router::CanonicalWakePcmRouter;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

    #[derive(Default)]
    struct RecordingEngine {
        config: SherpaKwsConfig,
        frames: Vec<Vec<i16>>,
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
            Ok(None)
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }
    }

    fn listening_consumer() -> WakeCapturePcmConsumer<RecordingEngine> {
        let runtime = WakeWordRuntimeManager::new();
        runtime.begin_enable().unwrap();
        runtime.mark_loaded().unwrap();
        WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
            runtime,
            RecordingEngine::default(),
        ))
    }

    #[test]
    fn start_uses_the_existing_capture_owner_at_the_canonical_rate() {
        let mut capture = AudioCapture::new_mock();
        let orchestrator =
            WakeCaptureOrchestrator::start(&mut capture, None, listening_consumer()).unwrap();

        assert!(capture.is_active());
        assert_eq!(
            capture.diagnostics().sample_rate_hz,
            Some(V1_KWS_SAMPLE_RATE_HZ)
        );
        assert_eq!(
            orchestrator
                .consumer()
                .router()
                .runtime()
                .snapshot(Instant::now())
                .ring_buffer_samples,
            0
        );
    }

    #[test]
    fn replacing_orchestrator_reuses_same_capture_owner_instead_of_multiplying_owners() {
        let mut capture = AudioCapture::new_mock();
        let first =
            WakeCaptureOrchestrator::start(&mut capture, None, listening_consumer()).unwrap();
        assert!(capture.is_active());

        let second =
            WakeCaptureOrchestrator::start(&mut capture, None, listening_consumer()).unwrap();
        assert!(capture.is_active());
        assert_eq!(
            capture.diagnostics().sample_rate_hz,
            Some(V1_KWS_SAMPLE_RATE_HZ)
        );

        drop(first);
        drop(second);
        capture.stop();
        assert!(!capture.is_active());
    }

    #[test]
    fn disable_stops_single_capture_owner_and_clears_wake_state() {
        let mut capture = AudioCapture::new_mock();
        let mut orchestrator =
            WakeCaptureOrchestrator::start(&mut capture, None, listening_consumer()).unwrap();
        assert!(capture.is_active());
        assert!(orchestrator
            .consumer()
            .router()
            .runtime()
            .append_listening_pcm(&[1, 2, 3]));

        orchestrator.disable(&mut capture);

        assert!(!capture.is_active());
        let snapshot = orchestrator
            .consumer()
            .router()
            .runtime()
            .snapshot(Instant::now());
        assert_eq!(snapshot.phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(snapshot.ring_buffer_samples, 0);
        assert_eq!(snapshot.handoff_pre_roll_samples, 0);
    }

    #[tokio::test]
    async fn closed_capture_queue_fails_wake_runtime_closed_without_reopening_capture() {
        let (sender, receiver) = mpsc::channel(1);
        drop(sender);
        let mut orchestrator = WakeCaptureOrchestrator {
            receiver,
            consumer: listening_consumer(),
        };
        assert_eq!(
            orchestrator
                .consumer()
                .router()
                .runtime()
                .snapshot(Instant::now())
                .phase,
            WakeWordRuntimePhase::Listening
        );

        assert!(matches!(
            orchestrator.route_next(Instant::now()).await,
            Err(WakeCaptureOrchestratorError::CaptureClosed)
        ));

        let snapshot = orchestrator
            .consumer()
            .router()
            .runtime()
            .snapshot(Instant::now());
        assert_eq!(snapshot.phase, WakeWordRuntimePhase::Error);
        assert_eq!(snapshot.ring_buffer_samples, 0);
        assert_eq!(snapshot.handoff_pre_roll_samples, 0);
    }
}
