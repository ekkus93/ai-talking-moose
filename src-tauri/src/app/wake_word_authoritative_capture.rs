use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_capture_consumer::WakeCapturePcmConsumer;
use super::wake_word_capture_orchestrator::{
    WakeCaptureOrchestrator, WakeCaptureOrchestratorError,
};
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_pcm_router::WakePcmRouteOutcome;
use crate::audio::capture::AudioCapture;
use parking_lot::Mutex as CaptureMutex;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Serializes Wake receive state around the application's existing authoritative microphone owner.
///
/// `AppState::audio_capture`, manual conversation, diagnostics, and Wake Word all share the exact
/// same `Arc<Mutex<AudioCapture>>`. This type never constructs or hides another `AudioCapture`.
/// The receive-side orchestrator uses an async-aware mutex because routing awaits queue input; the
/// physical capture lock is held only around synchronous start/stop replacement operations.
pub(crate) struct AuthoritativeWakeCaptureOwner<E: SherpaKwsEngine> {
    capture: Arc<CaptureMutex<AudioCapture>>,
    wake: Mutex<Option<WakeCaptureOrchestrator<E>>>,
}

impl<E: SherpaKwsEngine> AuthoritativeWakeCaptureOwner<E> {
    pub(crate) fn from_shared_capture(capture: Arc<CaptureMutex<AudioCapture>>) -> Self {
        Self {
            capture,
            wake: Mutex::new(None),
        }
    }

    pub(crate) async fn start_wake(
        &self,
        device_name: Option<String>,
        consumer: WakeCapturePcmConsumer<E>,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        let orchestrator = {
            let mut capture = self.capture.lock();
            WakeCaptureOrchestrator::start(&mut capture, device_name, consumer)?
        };
        *self.wake.lock().await = Some(orchestrator);
        Ok(())
    }

    pub(crate) async fn route_next(
        &self,
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakeCaptureOrchestratorError> {
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        orchestrator.route_next(now).await
    }

    /// Stop Wake capture and transfer its chronological handoff payload to command ASR.
    ///
    /// Stopping the shared AppState capture before returning the payload guarantees the normal
    /// command path can replace that same owner without a simultaneous competing microphone open.
    pub(crate) async fn transfer_to_command_asr(
        &self,
    ) -> Result<Option<WakeCommandHandoffAudio>, WakeCaptureOrchestratorError> {
        self.capture.lock().stop();
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        orchestrator.transfer_handoff_audio_to_asr()
    }

    pub(crate) async fn return_to_wake_listening(
        &self,
        device_name: Option<String>,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        let mut capture = self.capture.lock();
        orchestrator.resume_capture_after_command(&mut capture, device_name)
    }

    pub(crate) async fn restart_wake(
        &self,
        device_name: Option<String>,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        let mut wake = self.wake.lock().await;
        let orchestrator = wake
            .as_mut()
            .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
        let mut capture = self.capture.lock();
        orchestrator.restart_after_capture_error(&mut capture, device_name)
    }

    pub(crate) async fn disable(&self) {
        let mut wake = self.wake.lock().await;
        let mut capture = self.capture.lock();
        if let Some(orchestrator) = wake.as_mut() {
            orchestrator.disable(&mut capture);
        } else {
            capture.stop();
        }
        *wake = None;
    }

    #[cfg(test)]
    fn shares_capture_with(&self, capture: &Arc<CaptureMutex<AudioCapture>>) -> bool {
        Arc::ptr_eq(&self.capture, capture)
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
    async fn wake_start_and_disable_use_exact_shared_app_capture_owner() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        assert!(owner.shares_capture_with(&app_capture));

        owner.start_wake(None, consumer()).await.unwrap();
        assert!(app_capture.lock().is_active());

        owner.disable().await;
        assert!(!app_capture.lock().is_active());
    }

    #[tokio::test]
    async fn replacing_wake_epoch_reuses_exact_shared_app_capture_owner() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        owner.start_wake(None, consumer()).await.unwrap();

        assert!(owner.shares_capture_with(&app_capture));
        assert!(app_capture.lock().is_active());
        owner.disable().await;
        assert!(!app_capture.lock().is_active());
    }

    #[tokio::test]
    async fn command_return_reuses_exact_shared_app_capture_owner() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        assert!(app_capture.lock().is_active());

        {
            let wake = owner.wake.lock().await;
            wake.as_ref()
                .unwrap()
                .consumer()
                .router()
                .runtime()
                .suspend_for_talking()
                .unwrap();
        }

        assert!(owner.transfer_to_command_asr().await.unwrap().is_none());
        assert!(!app_capture.lock().is_active());

        owner.return_to_wake_listening(None).await.unwrap();

        assert!(owner.shares_capture_with(&app_capture));
        assert!(app_capture.lock().is_active());
        let phase = {
            let wake = owner.wake.lock().await;
            wake.as_ref()
                .unwrap()
                .consumer()
                .router()
                .runtime()
                .snapshot(Instant::now())
                .phase
        };
        assert_eq!(phase, WakeWordRuntimePhase::Listening);
    }

    #[tokio::test]
    async fn wake_disable_releases_shared_capture_for_manual_listen() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        assert!(app_capture.lock().is_active());

        owner.disable().await;
        assert!(!app_capture.lock().is_active());

        let (manual_pcm_tx, _manual_pcm_rx) = tokio::sync::mpsc::channel(1);
        app_capture
            .lock()
            .start(None, V1_KWS_SAMPLE_RATE_HZ, manual_pcm_tx, None)
            .unwrap();

        assert!(owner.shares_capture_with(&app_capture));
        assert!(app_capture.lock().is_active());
        assert_eq!(
            app_capture.lock().diagnostics().sample_rate_hz,
            Some(V1_KWS_SAMPLE_RATE_HZ)
        );
        app_capture.lock().stop();
    }
}
