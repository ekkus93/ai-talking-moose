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
    pub(crate) async fn route_next(
        &mut self,
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakeCaptureOrchestratorError> {
        let chunk = self
            .receiver
            .recv()
            .await
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
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

    pub(crate) fn return_to_wake_listening(
        &mut self,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        self.consumer
            .return_to_wake_listening()
            .map_err(WakeCaptureOrchestratorError::Pcm)
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
    use crate::app::wake_word::runtime::WakeWordRuntimeManager;
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
        let first = WakeCaptureOrchestrator::start(&mut capture, None, listening_consumer()).unwrap();
        assert!(capture.is_active());

        let second = WakeCaptureOrchestrator::start(&mut capture, None, listening_consumer()).unwrap();
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
}
