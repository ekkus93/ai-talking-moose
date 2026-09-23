use super::state::AppState;
use super::wake_word::engine::{
    NativeKwsSession, NativeKwsSessionPaths, SherpaKwsEngine, WakeWordError,
};
use super::wake_word::runtime::WakeWordRuntimeError;
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_capture_orchestrator::WakeCaptureOrchestratorError;
use super::wake_word_composition::WakeWordApplicationRuntime;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;

static PRODUCTION_WAKE_OWNER: OnceLock<
    AsyncMutex<Option<Arc<AuthoritativeWakeCaptureOwner<NativeKwsSession>>>>,
> = OnceLock::new();

/// Access the one Wake Word runtime owned directly by authoritative application state.
///
/// Physical microphone capture remains exclusively owned by `AppState::audio_capture`;
/// the Wake runtime owns lifecycle/KWS state only.
pub(crate) fn runtime_from_app_state(state: &AppState) -> &WakeWordApplicationRuntime {
    &state.wake_word_runtime
}

/// Resolve the deterministic production model/runtime roots under the app data directory.
///
/// The native runtime layout mirrors `wake-word-artifacts.json`'s `runtime.*.install_root` so
/// artifacts prepared by repository tooling are consumed from the same fail-closed identity path.
pub(crate) fn native_kws_paths_from_app_data_dir(app_data_dir: &Path) -> NativeKwsSessionPaths {
    NativeKwsSessionPaths {
        model_dir: app_data_dir
            .join("models")
            .join("wake-word")
            .join("sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"),
        runtime_dir: app_data_dir
            .join("runtime")
            .join("sherpa-onnx")
            .join("v1.13.8")
            .join(native_runtime_platform_dir()),
    }
}

fn native_runtime_platform_dir() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-arm64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        "unsupported"
    }
}

fn production_owner_slot(
) -> &'static AsyncMutex<Option<Arc<AuthoritativeWakeCaptureOwner<NativeKwsSession>>>> {
    PRODUCTION_WAKE_OWNER.get_or_init(|| AsyncMutex::new(None))
}

async fn retained_native_owner() -> Option<Arc<AuthoritativeWakeCaptureOwner<NativeKwsSession>>> {
    production_owner_slot().lock().await.clone()
}

/// Compose Wake routing around the one microphone owner stored in authoritative application state.
///
/// This function is the production boundary for WWR-300: Wake receives PCM through the same
/// `AppState::audio_capture` object used by manual command listening instead of constructing a
/// competing capture owner. The returned owner still requires a caller-supplied capture consumer so
/// production can use a verified native KWS session while tests can keep deterministic fake engines.
/// The application-startup caller is intentionally staged separately from this ownership seam.
#[allow(dead_code)]
pub(crate) fn capture_owner_from_app_state<E: SherpaKwsEngine>(
    state: &AppState,
) -> AuthoritativeWakeCaptureOwner<E> {
    AuthoritativeWakeCaptureOwner::from_shared_capture(state.audio_capture.clone())
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum WakeWordStartupError {
    Runtime(WakeWordRuntimeError),
    Native(WakeWordError),
    Capture(WakeCaptureOrchestratorError),
}

/// Start the production Wake listener from authoritative application state.
///
/// This is the lifecycle/start boundary for WWR-300. It honors the persisted setting, constructs
/// the real verified native KWS session, marks the shared Wake runtime loaded only after native
/// construction succeeds, and starts capture through `AppState::audio_capture` rather than through
/// a Wake-owned microphone. Any native or capture startup failure leaves Wake in a recoverable Error
/// state and does not leave the shared capture owner active.
#[allow(dead_code)]
pub(crate) async fn start_native_wake_from_app_state(
    state: &AppState,
    paths: NativeKwsSessionPaths,
) -> Result<Option<AuthoritativeWakeCaptureOwner<NativeKwsSession>>, WakeWordStartupError> {
    let settings = state.settings.read().clone();
    if !settings.wake_word_enabled {
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(WakeWordStartupError::Runtime)?;
        return Ok(None);
    }

    state
        .wake_word_runtime
        .apply_enabled_setting(true)
        .map_err(WakeWordStartupError::Runtime)?;

    let consumer = match state.wake_word_runtime.native_capture_consumer(paths) {
        Ok(consumer) => consumer,
        Err(error) => {
            state.wake_word_runtime.record_runtime_error();
            return Err(WakeWordStartupError::Native(error));
        }
    };

    state
        .wake_word_runtime
        .mark_loaded()
        .map_err(WakeWordStartupError::Runtime)?;

    let owner = capture_owner_from_app_state::<NativeKwsSession>(state);
    if let Err(error) = owner
        .start_wake(settings.input_device.clone(), consumer)
        .await
    {
        state.wake_word_runtime.record_capture_error();
        return Err(WakeWordStartupError::Capture(error));
    }

    Ok(Some(owner))
}

/// Start Wake from AppState and retain the receive-side owner for the full listening epoch.
///
/// Without this retention boundary a started native listener could be dropped by its caller before
/// the capture task transfers a trigger to command ASR. The stored owner is still only a receive
/// coordinator; the physical microphone remains `AppState::audio_capture`.
pub(crate) async fn start_and_retain_native_wake_from_app_state(
    state: &AppState,
    paths: NativeKwsSessionPaths,
) -> Result<bool, WakeWordStartupError> {
    let started = start_native_wake_from_app_state(state, paths).await?;
    let mut slot = production_owner_slot().lock().await;
    if let Some(previous) = slot.take() {
        previous.disable().await;
    }
    if let Some(owner) = started {
        *slot = Some(Arc::new(owner));
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Disable the retained production Wake owner, if any, without touching manual conversation state.
pub(crate) async fn disable_native_wake_from_app_state(
    state: &AppState,
) -> Result<(), WakeWordStartupError> {
    let mut slot = production_owner_slot().lock().await;
    if let Some(owner) = slot.take() {
        owner.disable().await;
    }
    state
        .wake_word_runtime
        .apply_enabled_setting(false)
        .map_err(WakeWordStartupError::Runtime)
}

/// Placeholder for the production receive loop until the native KWS owner is moved to a dedicated
/// local-thread runner.
///
/// `NativeKwsSession` deliberately is not `Send`, so driving it from Tauri's multithreaded async
/// runtime would be unsound. This function is intentionally inert instead of weakening that safety
/// boundary; the retained owner/start helpers remain available for the upcoming local-runner slice.
#[allow(dead_code)]
pub(crate) fn spawn_retained_native_wake_listener<R>(_state: AppState, _app: tauri::AppHandle<R>)
where
    R: tauri::Runtime,
{
    warn!("Wake Word native receive loop requires a dedicated local-thread runner before startup");
}

/// Resolve command completion against the retained Wake owner.
///
/// When Wake remains enabled it reopens the same shared `AudioCapture` owner through the retained
/// orchestrator. If Wake was disabled during the interaction it tears down Wake state instead of
/// accidentally reopening the microphone.
pub(crate) async fn resume_retained_native_wake_after_command_from_app_state<R>(
    state: &AppState,
    _app: &tauri::AppHandle<R>,
    wake_word_enabled: bool,
) -> Result<(), WakeWordStartupError>
where
    R: tauri::Runtime,
{
    if !wake_word_enabled {
        disable_native_wake_from_app_state(state).await?;
        return Ok(());
    }

    let input_device = state.settings.read().input_device.clone();
    if let Some(owner) = retained_native_owner().await {
        owner
            .return_to_wake_listening(input_device)
            .await
            .map_err(WakeWordStartupError::Capture)
    } else {
        state
            .wake_word_runtime
            .resume_after_interaction(true)
            .map_err(WakeWordStartupError::Runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{
        validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
    };
    use crate::asr::wake_word_runtime::WakeWordRuntimePhase;
    use crate::audio::capture::AudioCapture;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

    #[derive(Default)]
    struct TestWakeEngine {
        config: SherpaKwsConfig,
    }

    impl SherpaKwsEngine for TestWakeEngine {
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

    #[test]
    fn app_state_directly_owns_wake_runtime_without_owning_capture() {
        let state = AppState::new_for_tests().unwrap();
        assert!(!state.settings.read().wake_word_enabled);
        assert_eq!(
            runtime_from_app_state(&state).phase(),
            WakeWordRuntimePhase::Disabled
        );
        assert!(!state.audio_capture.lock().diagnostics().active);
    }

    #[test]
    fn native_paths_live_under_application_data_directory() {
        let temp = tempfile::tempdir().unwrap();
        let paths = native_kws_paths_from_app_data_dir(temp.path());

        assert!(paths.model_dir.starts_with(temp.path()));
        assert!(paths.runtime_dir.starts_with(temp.path()));
        assert!(paths
            .model_dir
            .ends_with("models/wake-word/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"));
        assert!(paths.runtime_dir.ends_with(native_runtime_platform_dir()));
    }

    #[tokio::test]
    async fn capture_owner_from_app_state_uses_exact_app_capture() {
        let state = AppState::new_for_tests().unwrap();
        *state.audio_capture.lock() = AudioCapture::new_mock();
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        state.wake_word_runtime.mark_loaded().unwrap();

        let owner = capture_owner_from_app_state::<TestWakeEngine>(&state);
        let consumer = state
            .wake_word_runtime
            .capture_consumer(TestWakeEngine::default());

        owner.start_wake(None, consumer).await.unwrap();
        assert!(state.audio_capture.lock().is_active());
        assert_eq!(
            state.audio_capture.lock().diagnostics().sample_rate_hz,
            Some(V1_KWS_SAMPLE_RATE_HZ)
        );

        owner.disable().await;
        assert!(!state.audio_capture.lock().is_active());
        assert_eq!(
            state
                .wake_word_runtime
                .snapshot(std::time::Instant::now())
                .phase,
            WakeWordRuntimePhase::Disabled
        );
    }

    #[tokio::test]
    async fn disabled_startup_does_not_load_native_or_open_capture() {
        let state = AppState::new_for_tests().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let paths = NativeKwsSessionPaths {
            model_dir: temp.path().join("missing-model"),
            runtime_dir: temp.path().join("missing-runtime"),
        };

        let owner = start_native_wake_from_app_state(&state, paths)
            .await
            .unwrap();

        assert!(owner.is_none());
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Disabled
        );
        assert!(!state.audio_capture.lock().is_active());
    }

    #[tokio::test]
    async fn enabled_startup_fails_closed_before_capture_when_native_artifacts_missing() {
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().wake_word_enabled = true;
        let temp = tempfile::tempdir().unwrap();
        let paths = NativeKwsSessionPaths {
            model_dir: temp.path().join("missing-model"),
            runtime_dir: temp.path().join("missing-runtime"),
        };

        let result = start_native_wake_from_app_state(&state, paths).await;
        assert!(matches!(result, Err(WakeWordStartupError::Native(_))));
        assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
        assert!(!state.audio_capture.lock().is_active());
    }
}
