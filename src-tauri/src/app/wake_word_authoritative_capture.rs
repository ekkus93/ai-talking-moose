use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_capture_consumer::WakeCapturePcmConsumer;
use super::wake_word_capture_orchestrator::{
    WakeCaptureOrchestrator, WakeCaptureOrchestratorError,
};
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_pcm_router::WakePcmRouteOutcome;
use crate::audio::capture::AudioCapture;
use parking_lot::Mutex;
use std::time::Instant;
use tokio::sync::Mutex as AsyncMutex;

/// Serializes the application's one authoritative microphone owner with its Wake receive path.
///
/// Manual conversation and Wake Word both borrow the same `AudioCapture`; this owner never
/// constructs a second capture object. Keeping the orchestrator behind an async-aware mutex makes
/// the capture stream and its receiver one composition unit and provides an explicit
/// ownership-transfer boundary for command ASR.
pub(crate) struct AuthoritativeWakeCaptureOwner<E: SherpaKwsEngine> {
    capture: Mutex<AudioCapture>,
    wake: AsyncMutex<Option<WakeCaptureOrchestrator<E>>>,
}

impl<E: SherpaKwsEngine> AuthoritativeWakeCaptureOwner<E> {
    pub(crate) fn new(capture: AudioCapture) -> Self {
        Self {
            capture: Mutex::new(capture),
            wake: AsyncMutex::new(None),
        }
    }

    pub(crate) async fn start_wake(
        &self,
        device_name: Option<String>,
        consumer: WakeCapturePcmConsumer<E>,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        let mut capture = self.capture.lock();
        let orchestrator = WakeCaptureOrchestrator::start(&mut capture, device_name, consumer)?;
        *self.wake.lock().await = Some(orchestrator);
        Ok(())
    }

    pub(crate) async fn route_next(
        &self,
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakeCaptureOrchestratorError> {
        // The receiver is independent of the CPAL stream handle after start, so routing does not
        // hold the physical-owner lock while awaiting a chunk. The async Wake lock serializes
        // receiver consumption without blocking a Tokio worker thread.
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        orchestrator.route_next(now).await
    }

    /// Stop Wake capture and transfer its chronological handoff payload to command ASR.
    ///
    /// Stopping the sole capture owner before returning the payload guarantees the normal command
    /// path can replace that same owner without a simultaneous competing microphone open.
    pub(crate) async fn transfer_to_command_asr(
        &self,
    ) -> Result<Option<WakeCommandHandoffAudio>, WakeCaptureOrchestratorError> {
        let mut capture = self.capture.lock();
        capture.stop();
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        orchestrator.transfer_handoff_audio_to_asr()
    }

    pub(crate) async fn restart_wake(
        &self,
        device_name: Option<String>,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        let mut capture = self.capture.lock();
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        orchestrator.restart_after_capture_error(&mut capture, device_name)
    }

    pub(crate) async fn disable(&self) {
        let mut capture = self.capture.lock();
        let mut wake = self.wake.lock().await;
        if let Some(orchestrator) = wake.as_mut() {
            orchestrator.disable(&mut capture);
        } else {
            capture.stop();
        }
        *wake = None;
    }

    #[cfg(test)]
    fn capture_is_active(&self) -> bool {
        self.capture.lock().is_active()
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

    #[derive(Default)]
    struct TestEngine {
        config: SherpaKwsConfig,
    }

    impl SherpaKwsEngine for TestEngine {
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

    fn consumer() -> WakeCapturePcmConsumer<TestEngine> {
        let runtime = WakeWordRuntimeManager::new();
        runtime.begin_enable().unwrap();
        runtime.mark_loaded().unwrap();
        WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(runtime, TestEngine::default()))
    }

    #[tokio::test]
    async fn wake_start_and_disable_share_one_physical_capture_owner() {
        let owner = AuthoritativeWakeCaptureOwner::new(AudioCapture::new_mock());
        owner.start_wake(None, consumer()).await.unwrap();
        assert!(owner.capture_is_active());

        owner.disable().await;
        assert!(!owner.capture_is_active());
    }

    #[tokio::test]
    async fn replacing_wake_epoch_reuses_same_authoritative_capture_owner() {
        let owner = AuthoritativeWakeCaptureOwner::new(AudioCapture::new_mock());
        owner.start_wake(None, consumer()).await.unwrap();
        owner.start_wake(None, consumer()).await.unwrap();

        assert!(owner.capture_is_active());
        owner.disable().await;
        assert!(!owner.capture_is_active());
    }
}
