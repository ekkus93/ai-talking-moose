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
use std::time::{Duration, Instant};

const WAKE_CAPTURE_HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(250);
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
        let mut wake = self.wake.lock().await;
        let result = {
            let mut capture = self.capture.lock();
            WakeCaptureOrchestrator::start(&mut capture, device_name, consumer)
        };
        match result {
            Ok(orchestrator) => {
                *wake = Some(orchestrator);
                Ok(())
            }
            Err(error) => {
                self.capture.lock().stop();
                *wake = None;
                Err(error)
            }
        }
    }

    pub(crate) async fn route_next(
        &self,
        now: Instant,
    ) -> Result<WakePcmRouteOutcome, WakeCaptureOrchestratorError> {
        loop {
            if !self.capture.lock().is_active() {
                let mut wake = self.wake.lock().await;
                let orchestrator = wake
                    .as_mut()
                    .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
                orchestrator.clear_handoff();
                orchestrator
                    .consumer()
                    .router()
                    .runtime()
                    .record_runtime_error();
                return Err(WakeCaptureOrchestratorError::CaptureClosed);
            }

            let mut wake = self.wake.lock().await;
            let orchestrator = wake
                .as_mut()
                .ok_or(WakeCaptureOrchestratorError::CaptureClosed)?;
            match tokio::time::timeout(
                WAKE_CAPTURE_HEALTH_POLL_INTERVAL,
                orchestrator.route_next(now),
            )
            .await
            {
                Ok(result) => return result,
                Err(_) => {
                    // A CPAL runtime failure marks the authoritative capture inactive but does
                    // not guarantee that its PCM sender is dropped immediately. Release the Wake
                    // lock and poll the capture-health bit so a device disconnect cannot strand
                    // the listener forever waiting on an otherwise-open queue.
                    drop(wake);
                }
            }
        }
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

    /// Resolve a cancelled command interaction against the latest Wake-enabled setting.
    ///
    /// When Wake remains enabled, ownership returns to Listening through the same shared capture
    /// owner. If the user disabled Wake during the interaction, cancellation tears down Wake state
    /// and leaves that shared capture owner idle for manual listen instead of accidentally
    /// reopening the microphone.
    pub(crate) async fn cancel_command_interaction(
        &self,
        wake_word_enabled: bool,
        device_name: Option<String>,
    ) -> Result<(), WakeCaptureOrchestratorError> {
        if wake_word_enabled {
            self.return_to_wake_listening(device_name).await
        } else {
            self.disable().await;
            Ok(())
        }
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

    async fn enter_command_interaction(owner: &AuthoritativeWakeCaptureOwner<TestEngine>) {
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
    async fn failed_start_clears_wake_state_and_leaves_capture_stopped() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        let result = owner
            .start_wake(
                Some("definitely-not-a-real-wake-test-microphone".to_string()),
                consumer(),
            )
            .await;

        assert!(result.is_err());
        assert!(owner.shares_capture_with(&app_capture));
        assert!(!app_capture.lock().is_active());
        assert!(owner.wake.lock().await.is_none());
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

        enter_command_interaction(&owner).await;
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
    async fn failed_command_return_leaves_capture_stopped_and_wake_recoverable() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        enter_command_interaction(&owner).await;
        assert!(!app_capture.lock().is_active());

        // Switch the shared owner to a real-device mode so an impossible device name deterministically
        // exercises ASR-return startup failure without constructing a second capture owner.
        *app_capture.lock() = AudioCapture::new();
        let result = owner
            .return_to_wake_listening(Some(
                "definitely-not-a-real-wake-test-microphone".to_string(),
            ))
            .await;

        assert!(result.is_err());
        assert!(owner.shares_capture_with(&app_capture));
        assert!(!app_capture.lock().is_active());
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
        assert_eq!(phase, WakeWordRuntimePhase::Error);

        // A later reconnect remains possible through the same authoritative owner.
        *app_capture.lock() = AudioCapture::new_mock();
        owner.restart_wake(None).await.unwrap();
        assert!(app_capture.lock().is_active());
        let recovered_phase = {
            let wake = owner.wake.lock().await;
            wake.as_ref()
                .unwrap()
                .consumer()
                .router()
                .runtime()
                .snapshot(Instant::now())
                .phase
        };
        assert_eq!(recovered_phase, WakeWordRuntimePhase::Listening);
    }

    #[tokio::test]
    async fn cancellation_with_wake_enabled_returns_to_same_shared_capture_owner() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        enter_command_interaction(&owner).await;
        assert!(!app_capture.lock().is_active());

        owner.cancel_command_interaction(true, None).await.unwrap();

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
    async fn cancellation_after_wake_disable_releases_capture_for_manual_listen() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        enter_command_interaction(&owner).await;
        assert!(!app_capture.lock().is_active());

        owner.cancel_command_interaction(false, None).await.unwrap();

        assert!(owner.shares_capture_with(&app_capture));
        assert!(!app_capture.lock().is_active());
        assert!(owner.wake.lock().await.is_none());

        let (manual_pcm_tx, _manual_pcm_rx) = tokio::sync::mpsc::channel(1);
        app_capture
            .lock()
            .start(None, V1_KWS_SAMPLE_RATE_HZ, manual_pcm_tx, None)
            .unwrap();
        assert!(app_capture.lock().is_active());
        app_capture.lock().stop();
    }

    #[tokio::test]
    async fn inactive_authoritative_capture_fails_wake_closed_without_queue_close() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();
        assert!(app_capture.lock().is_active());

        // Runtime device failures set AudioCapture inactive. The queue may still have a live
        // sender, so Wake must observe the authoritative capture-health bit rather than waiting
        // forever for receiver closure.
        app_capture.lock().stop();

        assert!(matches!(
            owner.route_next(Instant::now()).await,
            Err(WakeCaptureOrchestratorError::CaptureClosed)
        ));

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
        assert_eq!(phase, WakeWordRuntimePhase::Error);
    }

    #[tokio::test]
    async fn repeated_wake_command_cycles_reuse_exact_shared_capture_owner() {
        let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());
        owner.start_wake(None, consumer()).await.unwrap();

        for _ in 0..100 {
            assert!(owner.shares_capture_with(&app_capture));
            assert!(app_capture.lock().is_active());

            enter_command_interaction(&owner).await;
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

        owner.disable().await;
        assert!(!app_capture.lock().is_active());
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
