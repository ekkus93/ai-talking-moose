use super::state::{AppSettings, AppState};
use super::wake_word::engine::{
    NativeKwsSession, NativeKwsSessionPaths, SherpaKwsEngine, WakeWordError,
};
use super::wake_word::runtime::WakeWordRuntimeError;
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_capture_orchestrator::WakeCaptureOrchestratorError;
use super::wake_word_composition::WakeWordApplicationRuntime;
use super::wake_word_local_listener_thread::{
    spawn_wake_local_listener_thread, WakeLocalListenerEvent, WakeLocalListenerHandle,
};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::sync::mpsc;

static NATIVE_WAKE_LISTENER: OnceLock<Mutex<Option<WakeLocalListenerHandle>>> = OnceLock::new();
static NATIVE_WAKE_LISTENER_CONFIG: OnceLock<NativeWakeListenerConfig> = OnceLock::new();

#[derive(Clone)]
struct NativeWakeListenerConfig {
    app_data_dir: PathBuf,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
}

fn native_wake_listener_slot() -> &'static Mutex<Option<WakeLocalListenerHandle>> {
    NATIVE_WAKE_LISTENER.get_or_init(|| Mutex::new(None))
}

fn remember_native_wake_listener_config(
    app_data_dir: &Path,
    event_tx: &mpsc::UnboundedSender<WakeLocalListenerEvent>,
) {
    let _ = NATIVE_WAKE_LISTENER_CONFIG.set(NativeWakeListenerConfig {
        app_data_dir: app_data_dir.to_path_buf(),
        event_tx: event_tx.clone(),
    });
}

pub(crate) fn runtime_from_app_state(state: &AppState) -> &WakeWordApplicationRuntime {
    &state.wake_word_runtime
}

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

pub(crate) fn start_native_wake_listener_thread_from_app_state(
    state: &AppState,
    app_data_dir: &Path,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
) -> Result<bool, String> {
    remember_native_wake_listener_config(app_data_dir, &event_tx);
    let settings = state.settings.read().clone();
    if !settings.wake_word_enabled {
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(|error| error.to_string())?;
        stop_native_wake_listener_thread();
        return Ok(false);
    }

    start_native_wake_listener_thread_with_config(state, app_data_dir, event_tx)
}

fn start_native_wake_listener_thread_with_config(
    state: &AppState,
    app_data_dir: &Path,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
) -> Result<bool, String> {
    let settings = state.settings.read().clone();
    let slot = native_wake_listener_slot();
    if slot.lock().is_some() {
        return Ok(false);
    }

    state
        .wake_word_runtime
        .apply_enabled_setting(true)
        .map_err(|error| error.to_string())?;

    let paths = native_kws_paths_from_app_data_dir(app_data_dir);
    let runtime = state.wake_word_runtime.clone();
    let capture = state.audio_capture.clone();
    let device_name = settings.input_device.clone();
    let handle = spawn_wake_local_listener_thread::<NativeKwsSession, _>(
        capture,
        runtime,
        device_name,
        move |wake_runtime| {
            wake_runtime
                .native_capture_consumer(paths)
                .map_err(|error| error.message)
        },
        event_tx,
    )
    .map_err(|error| error.to_string())?;

    *slot.lock() = Some(handle);
    Ok(true)
}

pub(crate) fn restart_native_wake_listener_thread_from_configured_app_state(
    state: &AppState,
) -> Result<bool, String> {
    if !state.settings.read().wake_word_enabled {
        stop_native_wake_listener_thread();
        return Ok(false);
    }

    let Some(config) = NATIVE_WAKE_LISTENER_CONFIG.get() else {
        return Ok(false);
    };
    if config.event_tx.is_closed() {
        return Ok(false);
    }
    start_native_wake_listener_thread_with_config(
        state,
        &config.app_data_dir,
        config.event_tx.clone(),
    )
}

pub(crate) fn native_wake_listener_is_active() -> bool {
    native_wake_listener_slot().lock().is_some()
}

pub(crate) fn apply_configured_native_wake_listener_settings_change(
    state: &AppState,
    previous: &AppSettings,
    next: &AppSettings,
) -> Result<(), String> {
    let wake_enabled_changed = previous.wake_word_enabled != next.wake_word_enabled;
    let input_device_changed = previous.input_device != next.input_device;
    let wake_listener_must_change = wake_enabled_changed || (next.wake_word_enabled && input_device_changed);
    if !wake_listener_must_change {
        return Ok(());
    }

    if !next.wake_word_enabled {
        stop_native_wake_listener_thread();
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    if input_device_changed {
        stop_native_wake_listener_thread();
    }

    if state.conversation_mgr.is_active() {
        state
            .wake_word_runtime
            .apply_enabled_setting(true)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    match restart_native_wake_listener_thread_from_configured_app_state(state) {
        Ok(true) => Ok(()),
        Ok(false) => state
            .wake_word_runtime
            .apply_enabled_setting(true)
            .map_err(|error| error.to_string()),
        Err(error) => {
            state.wake_word_runtime.record_runtime_error();
            Err(error)
        }
    }
}

pub(crate) fn clear_native_wake_listener_thread() {
    stop_native_wake_listener_thread();
}

pub(crate) fn stop_native_wake_listener_thread() {
    let Some(handle) = native_wake_listener_slot().lock().take() else {
        return;
    };
    let _ = handle.shutdown();
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

    #[test]
    fn disabled_local_thread_start_retains_restart_configuration_without_listener() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        let started =
            start_native_wake_listener_thread_from_app_state(&state, temp.path(), event_tx.clone())
                .unwrap();

        assert!(!started);
        assert!(native_wake_listener_slot().lock().is_none());
        assert!(NATIVE_WAKE_LISTENER_CONFIG.get().is_some());
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Disabled
        );
    }

    #[test]
    fn settings_disable_stops_listener_and_runtime_state() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        let previous = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        let next = AppSettings::default();

        apply_configured_native_wake_listener_settings_change(&state, &previous, &next).unwrap();

        assert!(!native_wake_listener_is_active());
        assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn settings_enable_without_startup_config_enters_loading_without_false_listener_claim() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        let previous = AppSettings::default();
        let next = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };

        apply_configured_native_wake_listener_settings_change(&state, &previous, &next).unwrap();

        assert!(!native_wake_listener_is_active());
        assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Loading);
    }
}
