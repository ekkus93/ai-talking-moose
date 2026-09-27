use super::state::{AppSettings, AppState};
#[cfg(test)]
use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word::engine::{NativeKwsSession, NativeKwsSessionPaths};
#[cfg(test)]
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_command_lifecycle::{
    complete_command_interaction, suspend_for_command_interaction, CommandInteractionTerminalOutcome,
};
use super::wake_word_composition::WakeWordApplicationRuntime;
use super::wake_word_local_listener_thread::{
    spawn_wake_local_listener_thread, WakeLocalListenerEvent, WakeLocalListenerHandle,
};
use crate::asr::AsrMode;
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

pub(crate) fn wake_word_asr_mode_supported(mode: AsrMode) -> bool {
    matches!(
        mode,
        AsrMode::MoonshineTinyStreaming | AsrMode::MoonshineSmallStreaming
    )
}

fn ensure_wake_word_asr_mode_supported(mode: AsrMode) -> Result<(), String> {
    if wake_word_asr_mode_supported(mode) {
        Ok(())
    } else {
        Err("Wake Word V1 requires local Moonshine command ASR".to_string())
    }
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

#[cfg(test)]
fn capture_owner_from_app_state<E: SherpaKwsEngine>(
    state: &AppState,
) -> AuthoritativeWakeCaptureOwner<E> {
    AuthoritativeWakeCaptureOwner::from_shared_capture(state.audio_capture.clone())
}

#[cfg(test)]
#[derive(Debug)]
enum WakeWordStartupError {
    Runtime,
    Native,
    Capture,
}

#[cfg(test)]
async fn start_native_wake_from_app_state(
    state: &AppState,
    paths: NativeKwsSessionPaths,
) -> Result<Option<AuthoritativeWakeCaptureOwner<NativeKwsSession>>, WakeWordStartupError> {
    let settings = state.settings.read().clone();
    if !settings.wake_word_enabled {
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(|_| WakeWordStartupError::Runtime)?;
        return Ok(None);
    }

    state
        .wake_word_runtime
        .apply_enabled_setting(true)
        .map_err(|_| WakeWordStartupError::Runtime)?;

    let consumer = match state.wake_word_runtime.native_capture_consumer(paths) {
        Ok(consumer) => consumer,
        Err(_) => {
            state.wake_word_runtime.record_runtime_error();
            return Err(WakeWordStartupError::Native);
        }
    };

    state
        .wake_word_runtime
        .mark_loaded()
        .map_err(|_| WakeWordStartupError::Runtime)?;

    let owner = capture_owner_from_app_state::<NativeKwsSession>(state);
    if owner
        .start_wake(settings.input_device.clone(), consumer)
        .await
        .is_err()
    {
        state.wake_word_runtime.record_capture_error();
        return Err(WakeWordStartupError::Capture);
    }

    Ok(Some(owner))
}

fn start_native_wake_listener_thread_with_config(
    state: &AppState,
    settings: &AppSettings,
    app_data_dir: &Path,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
) -> Result<bool, String> {
    ensure_wake_word_asr_mode_supported(settings.asr_mode)?;
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

fn restart_native_wake_listener_thread_with_settings(
    state: &AppState,
    settings: &AppSettings,
) -> Result<bool, String> {
    stop_native_wake_listener_thread();
    if !settings.wake_word_enabled {
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
        settings,
        &config.app_data_dir,
        config.event_tx.clone(),
    )
}

#[allow(dead_code)]
pub(crate) fn native_wake_listener_is_active() -> bool {
    native_wake_listener_slot().lock().is_some()
}

fn prepare_runtime_for_pending_listener_restart(
    runtime: &WakeWordApplicationRuntime,
) -> Result<(), String> {
    match runtime.phase() {
        crate::asr::wake_word_runtime::WakeWordRuntimePhase::Disabled
        | crate::asr::wake_word_runtime::WakeWordRuntimePhase::Error => runtime
            .apply_enabled_setting(true)
            .map_err(|error| error.to_string()),
        crate::asr::wake_word_runtime::WakeWordRuntimePhase::Loading
        | crate::asr::wake_word_runtime::WakeWordRuntimePhase::Listening
        | crate::asr::wake_word_runtime::WakeWordRuntimePhase::Triggered
        | crate::asr::wake_word_runtime::WakeWordRuntimePhase::SuspendedTalking => Ok(()),
        crate::asr::wake_word_runtime::WakeWordRuntimePhase::ShuttingDown => runtime
            .apply_enabled_setting(true)
            .map_err(|error| error.to_string()),
    }
}

pub(crate) fn apply_configured_native_wake_listener_settings_change(
    state: &AppState,
    previous: &AppSettings,
    next: &AppSettings,
) -> Result<(), String> {
    let wake_enabled_changed = previous.wake_word_enabled != next.wake_word_enabled;
    let input_device_changed = previous.input_device != next.input_device;
    let asr_mode_changed = previous.asr_mode != next.asr_mode;
    let wake_listener_must_change = wake_enabled_changed
        || (next.wake_word_enabled && (input_device_changed || asr_mode_changed));
    if !wake_listener_must_change {
        return Ok(());
    }

    if !next.wake_word_enabled {
        control_native_wake_listener(state, NativeWakeListenerControl::Stop)?;
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    if next.wake_word_enabled && !wake_word_asr_mode_supported(next.asr_mode) {
        control_native_wake_listener(state, NativeWakeListenerControl::Stop)?;
        state.wake_word_runtime.record_runtime_error();
        return Err("Wake Word V1 requires local Moonshine command ASR".to_string());
    }

    if input_device_changed || asr_mode_changed {
        control_native_wake_listener(state, NativeWakeListenerControl::Stop)?;
    }

    if state.conversation_mgr.is_active() {
        prepare_runtime_for_pending_listener_restart(&state.wake_word_runtime)?;
        return Ok(());
    }

    match control_native_wake_listener(
        state,
        NativeWakeListenerControl::RestartForSettings {
            settings: Box::new(next.clone()),
        },
    ) {
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

#[derive(Clone, Debug)]
pub(crate) enum NativeWakeListenerControl {
    ConfigureAndStart {
        app_data_dir: PathBuf,
        event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
    },
    RestartConfigured,
    RestartForSettings {
        settings: Box<AppSettings>,
    },
    Stop,
    TransferToCommand,
}

pub(crate) fn control_native_wake_listener(
    state: &AppState,
    action: NativeWakeListenerControl,
) -> Result<bool, String> {
    match action {
        NativeWakeListenerControl::ConfigureAndStart {
            app_data_dir,
            event_tx,
        } => {
            remember_native_wake_listener_config(&app_data_dir, &event_tx);
            let settings = state.settings.read().clone();
            if !settings.wake_word_enabled {
                state
                    .wake_word_runtime
                    .apply_enabled_setting(false)
                    .map_err(|error| error.to_string())?;
                stop_native_wake_listener_thread();
                return Ok(false);
            }
            ensure_wake_word_asr_mode_supported(settings.asr_mode)?;
            start_native_wake_listener_thread_with_config(state, &settings, &app_data_dir, event_tx)
        }
        NativeWakeListenerControl::RestartConfigured => {
            let settings = state.settings.read().clone();
            restart_native_wake_listener_thread_with_settings(state, &settings)
        }
        NativeWakeListenerControl::RestartForSettings { settings } => {
            restart_native_wake_listener_thread_with_settings(state, settings.as_ref())
        }
        NativeWakeListenerControl::Stop => {
            stop_native_wake_listener_thread();
            Ok(false)
        }
        NativeWakeListenerControl::TransferToCommand => {
            let was_active = native_wake_listener_is_active();
            stop_native_wake_listener_thread();
            suspend_for_command_interaction(&state.wake_word_runtime)?;
            Ok(was_active)
        }
    }
}

pub(crate) fn complete_native_wake_command_interaction(
    state: &AppState,
    outcome: CommandInteractionTerminalOutcome,
) -> Result<bool, String> {
    let wake_word_enabled = state.settings.read().wake_word_enabled;
    complete_command_interaction(&state.wake_word_runtime, wake_word_enabled, outcome)?;
    if wake_word_enabled {
        if let Err(error) = control_native_wake_listener(state, NativeWakeListenerControl::RestartConfigured) {
            state.wake_word_runtime.record_runtime_error();
            return Err(error);
        }
        Ok(true)
    } else {
        control_native_wake_listener(state, NativeWakeListenerControl::Stop)?;
        Ok(false)
    }
}

fn stop_native_wake_listener_thread() {
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
    fn wake_v1_supports_only_local_moonshine_command_asr() {
        assert!(wake_word_asr_mode_supported(
            AsrMode::MoonshineTinyStreaming
        ));
        assert!(wake_word_asr_mode_supported(
            AsrMode::MoonshineSmallStreaming
        ));
        assert!(!wake_word_asr_mode_supported(AsrMode::GeminiLiveAudio));
    }

    #[test]
    fn settings_reject_unsupported_asr_before_listener_start() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        let previous = AppSettings::default();
        let next = AppSettings {
            wake_word_enabled: true,
            asr_mode: AsrMode::GeminiLiveAudio,
            ..Default::default()
        };
        let error = apply_configured_native_wake_listener_settings_change(&state, &previous, &next)
            .unwrap_err();
        assert_eq!(error, "Wake Word V1 requires local Moonshine command ASR");
        assert!(!native_wake_listener_is_active());
        assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
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
        assert!(matches!(result, Err(WakeWordStartupError::Native)));
        assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
        assert!(!state.audio_capture.lock().is_active());
    }

    #[test]
    fn disabled_local_thread_start_retains_restart_configuration_without_listener() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        let started = control_native_wake_listener(
            &state,
            NativeWakeListenerControl::ConfigureAndStart {
                app_data_dir: temp.path().to_path_buf(),
                event_tx: event_tx.clone(),
            },
        )
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
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Disabled
        );
    }

    #[test]
    fn listener_start_validates_pending_settings_instead_of_persisted_settings() {
        let state = AppState::new_for_tests().unwrap();
        assert_eq!(
            state.settings.read().asr_mode,
            AsrMode::MoonshineTinyStreaming
        );
        let pending = AppSettings {
            wake_word_enabled: true,
            asr_mode: AsrMode::GeminiLiveAudio,
            ..state.settings.read().clone()
        };
        let temp = tempfile::tempdir().unwrap();
        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        let error =
            start_native_wake_listener_thread_with_config(&state, &pending, temp.path(), event_tx)
                .unwrap_err();

        assert_eq!(error, "Wake Word V1 requires local Moonshine command ASR");
        assert!(!native_wake_listener_is_active());
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Disabled
        );
    }

    #[test]
    fn pending_restart_preserves_suspended_command_ownership() {
        let state = AppState::new_for_tests().unwrap();
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        state.wake_word_runtime.mark_loaded().unwrap();
        state.wake_word_runtime.suspend_for_talking().unwrap();

        prepare_runtime_for_pending_listener_restart(&state.wake_word_runtime).unwrap();

        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::SuspendedTalking
        );
    }

    #[test]
    fn pending_restart_moves_disabled_runtime_to_loading() {
        let state = AppState::new_for_tests().unwrap();

        prepare_runtime_for_pending_listener_restart(&state.wake_word_runtime).unwrap();

        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Loading
        );
    }

    #[test]
    fn command_transfer_suspends_listening_runtime_without_recording_error() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        state.wake_word_runtime.mark_loaded().unwrap();

        let was_active =
            control_native_wake_listener(&state, NativeWakeListenerControl::TransferToCommand)
                .unwrap();

        assert!(!was_active);
        assert!(!native_wake_listener_is_active());
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::SuspendedTalking
        );
        assert_eq!(
            state
                .wake_word_runtime
                .snapshot(std::time::Instant::now())
                .last_error,
            None
        );
    }

    #[test]
    fn command_transfer_preserves_manual_availability_from_non_listening_wake_states() {
        for phase in ["disabled", "loading", "error"] {
            stop_native_wake_listener_thread();
            let state = AppState::new_for_tests().unwrap();
            match phase {
                "disabled" => {}
                "loading" => {
                    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
                }
                "error" => {
                    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
                    state.wake_word_runtime.record_runtime_error();
                }
                _ => unreachable!(),
            }

            control_native_wake_listener(&state, NativeWakeListenerControl::TransferToCommand)
                .unwrap();

            let expected = match phase {
                "disabled" => WakeWordRuntimePhase::Disabled,
                "loading" => WakeWordRuntimePhase::Loading,
                "error" => WakeWordRuntimePhase::Error,
                _ => unreachable!(),
            };
            assert_eq!(state.wake_word_runtime.phase(), expected, "{phase}");
        }
    }

    #[test]
    fn terminal_command_completion_uses_latest_enable_setting() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().wake_word_enabled = true;

        let should_restart = complete_native_wake_command_interaction(
            &state,
            CommandInteractionTerminalOutcome::Success,
        )
        .unwrap();

        assert!(should_restart);
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Loading
        );
    }

    #[test]
    fn terminal_command_completion_honors_latest_disable_setting() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().wake_word_enabled = true;
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        state.wake_word_runtime.mark_loaded().unwrap();
        state.wake_word_runtime.suspend_for_talking().unwrap();
        state.settings.write().wake_word_enabled = false;

        let should_restart = complete_native_wake_command_interaction(
            &state,
            CommandInteractionTerminalOutcome::Cancelled,
        )
        .unwrap();

        assert!(!should_restart);
        assert!(!native_wake_listener_is_active());
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Disabled
        );
    }

    #[test]
    fn terminal_command_completion_recovers_wake_error_when_still_enabled() {
        stop_native_wake_listener_thread();
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().wake_word_enabled = true;
        state.wake_word_runtime.apply_enabled_setting(true).unwrap();
        state.wake_word_runtime.record_runtime_error();

        complete_native_wake_command_interaction(
            &state,
            CommandInteractionTerminalOutcome::RecoverableFailure,
        )
        .unwrap();

        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Loading
        );
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
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Loading
        );
    }
}
