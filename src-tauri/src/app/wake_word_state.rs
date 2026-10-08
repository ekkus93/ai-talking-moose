use super::state::{AppSettings, AppState};
use super::wake_word::engine::{NativeKwsSession, NativeKwsSessionPaths};
use super::wake_word_command_lifecycle::{
    complete_command_interaction, suspend_for_command_interaction,
    CommandInteractionTerminalOutcome,
};
use super::wake_word_composition::WakeWordApplicationRuntime;
use super::wake_word_local_listener_thread::{
    spawn_wake_local_listener_thread, WakeLocalListenerEvent, WakeLocalListenerHandle,
};
use crate::asr::AsrMode;
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::sync::mpsc;

const NATIVE_WAKE_LISTENER_STOP_WAIT: Duration = Duration::from_millis(100);

use super::wake_word_listener_controller::NativeWakeListenerStopPlan;
pub(crate) use super::wake_word_listener_controller::{
    NativeWakeListenerController, NativeWakeListenerLifecyclePhase,
};

pub(crate) fn wake_word_asr_mode_supported(mode: AsrMode) -> bool {
    matches!(
        mode,
        AsrMode::MoonshineTinyStreaming | AsrMode::MoonshineSmallStreaming | AsrMode::WhisperSmall
    )
}

pub(crate) const WAKE_WORD_UNSUPPORTED_ASR_MESSAGE: &str =
    "Asr settings do not support local command ASR wake word activation. Choose a local command ASR mode (Moonshine or Whisper) in Settings > Speech Recognition.";

fn ensure_wake_word_asr_mode_supported(mode: AsrMode) -> Result<(), String> {
    if wake_word_asr_mode_supported(mode) {
        Ok(())
    } else {
        Err(WAKE_WORD_UNSUPPORTED_ASR_MESSAGE.to_string())
    }
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

fn start_native_wake_listener_thread_with_config(
    state: &AppState,
    settings: &AppSettings,
    app_data_dir: &Path,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
) -> Result<bool, String> {
    ensure_wake_word_asr_mode_supported(settings.asr_mode)?;
    let Some(generation) = state.wake_listener_controller.reserve_start() else {
        return Ok(false);
    };
    start_native_wake_listener_thread_reserved(state, settings, app_data_dir, event_tx, generation)
}

fn start_native_wake_listener_thread_reserved(
    state: &AppState,
    settings: &AppSettings,
    app_data_dir: &Path,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
    generation: u64,
) -> Result<bool, String> {
    ensure_wake_word_asr_mode_supported(settings.asr_mode)?;

    if let Err(error) = state.wake_word_runtime.apply_enabled_setting(true) {
        state.wake_listener_controller.cancel_start(generation);
        return Err(error.to_string());
    }

    let paths = native_kws_paths_from_app_data_dir(app_data_dir);
    let runtime = state.wake_word_runtime.clone();
    let capture = state.audio_capture.clone();
    let device_name = settings.input_device.clone();
    let handle = match spawn_wake_local_listener_thread::<NativeKwsSession, _>(
        capture,
        runtime,
        device_name,
        move |wake_runtime| {
            wake_runtime
                .native_capture_consumer(paths)
                .map_err(|error| error.message)
        },
        event_tx,
    ) {
        Ok(handle) => handle,
        Err(error) => {
            state.wake_listener_controller.cancel_start(generation);
            return Err(error.to_string());
        }
    };

    match state
        .wake_listener_controller
        .publish_start(generation, handle)
    {
        Ok(()) => Ok(true),
        Err((stale_handle, stopping_generation)) => {
            let _ = supervise_native_wake_listener_shutdown(
                state.wake_listener_controller.clone(),
                stopping_generation,
                stale_handle,
            )?;
            Ok(false)
        }
    }
}

fn restart_native_wake_listener_thread_with_settings(
    state: &AppState,
    settings: &AppSettings,
) -> Result<bool, String> {
    stop_native_wake_listener_thread(state)?;
    if !settings.wake_word_enabled {
        state.wake_listener_controller.cancel_all_restart_requests();
        return Ok(false);
    }

    let Some(config) = state.wake_listener_controller.config() else {
        state.wake_listener_controller.cancel_all_restart_requests();
        return Ok(false);
    };
    if config.event_tx.is_closed() {
        state.wake_listener_controller.cancel_all_restart_requests();
        return Ok(false);
    }

    // Preserve restart intent across a bounded Stop. Each request receives a unique generation;
    // a later restart overwrites the pending token, so at most the newest request can claim the
    // lifecycle once the previous native thread actually reaches Stopped.
    let restart_generation = state.wake_listener_controller.schedule_restart();
    if let Some(generation) = state
        .wake_listener_controller
        .claim_scheduled_restart(restart_generation)
    {
        return start_native_wake_listener_thread_reserved(
            state,
            settings,
            &config.app_data_dir,
            config.event_tx,
            generation,
        );
    }

    let deferred_state = state.clone();
    let deferred_settings = settings.clone();
    tauri::async_runtime::spawn(async move {
        if !wait_for_native_wake_listener_stopped(&deferred_state, Duration::from_secs(5)).await {
            deferred_state
                .wake_listener_controller
                .cancel_restart_request(restart_generation);
            deferred_state.wake_word_runtime.record_runtime_error();
            return;
        }
        if !deferred_state
            .wake_listener_controller
            .restart_request_is_current(restart_generation)
        {
            return;
        }
        let Some(generation) = deferred_state
            .wake_listener_controller
            .claim_scheduled_restart(restart_generation)
        else {
            return;
        };
        let Some(config) = deferred_state.wake_listener_controller.config() else {
            deferred_state
                .wake_listener_controller
                .cancel_start(generation);
            deferred_state.wake_word_runtime.record_runtime_error();
            return;
        };
        if config.event_tx.is_closed() {
            deferred_state
                .wake_listener_controller
                .cancel_start(generation);
            deferred_state.wake_word_runtime.record_runtime_error();
            return;
        }
        if start_native_wake_listener_thread_reserved(
            &deferred_state,
            &deferred_settings,
            &config.app_data_dir,
            config.event_tx,
            generation,
        )
        .is_err()
        {
            deferred_state.wake_word_runtime.record_runtime_error();
        }
    });

    Ok(true)
}

pub(crate) fn native_wake_listener_is_active(state: &AppState) -> bool {
    state.wake_listener_controller.is_running()
}

pub(crate) fn native_wake_listener_lifecycle_phase(
    state: &AppState,
) -> NativeWakeListenerLifecyclePhase {
    state.wake_listener_controller.phase()
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
    apply_configured_native_wake_listener_settings_change_with_control(
        state,
        previous,
        next,
        state.conversation_mgr.is_active(),
        control_native_wake_listener,
    )
}

pub(crate) fn apply_configured_native_wake_listener_settings_change_with_control<F>(
    state: &AppState,
    previous: &AppSettings,
    next: &AppSettings,
    conversation_active: bool,
    mut control: F,
) -> Result<(), String>
where
    F: FnMut(&AppState, NativeWakeListenerControl) -> Result<bool, String>,
{
    let wake_enabled_changed = previous.wake_word_enabled != next.wake_word_enabled;
    let input_device_changed = previous.input_device != next.input_device;
    let asr_mode_changed = previous.asr_mode != next.asr_mode;
    let wake_listener_must_change = wake_enabled_changed
        || (next.wake_word_enabled && (input_device_changed || asr_mode_changed));
    if !wake_listener_must_change {
        return Ok(());
    }

    if !next.wake_word_enabled {
        control(state, NativeWakeListenerControl::Stop)?;
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    if !wake_word_asr_mode_supported(next.asr_mode) {
        control(state, NativeWakeListenerControl::Stop)?;
        state.wake_word_runtime.record_runtime_error();
        return Err(WAKE_WORD_UNSUPPORTED_ASR_MESSAGE.to_string());
    }

    if input_device_changed || asr_mode_changed {
        control(state, NativeWakeListenerControl::Stop)?;
    }

    if conversation_active {
        prepare_runtime_for_pending_listener_restart(&state.wake_word_runtime)?;
        return Ok(());
    }

    match control(
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
            state
                .wake_listener_controller
                .configure(&app_data_dir, &event_tx);
            let settings = state.settings.read().clone();
            if !settings.wake_word_enabled {
                state
                    .wake_word_runtime
                    .apply_enabled_setting(false)
                    .map_err(|error| error.to_string())?;
                state.wake_listener_controller.cancel_all_restart_requests();
                stop_native_wake_listener_thread(state)?;
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
            state.wake_listener_controller.cancel_all_restart_requests();
            stop_native_wake_listener_thread(state)?;
            Ok(false)
        }
        NativeWakeListenerControl::TransferToCommand => {
            state.wake_listener_controller.cancel_all_restart_requests();
            let was_active = native_wake_listener_is_active(state);
            stop_native_wake_listener_thread(state)?;
            if native_wake_listener_lifecycle_phase(state)
                == NativeWakeListenerLifecyclePhase::Stopping
            {
                return Err("Wake listener shutdown is still in progress".to_string());
            }
            if was_active {
                state
                    .wake_word_runtime
                    .apply_enabled_setting(true)
                    .map_err(|error| error.to_string())?;
                if state.wake_word_runtime.phase()
                    == crate::asr::wake_word_runtime::WakeWordRuntimePhase::Loading
                {
                    state
                        .wake_word_runtime
                        .mark_loaded()
                        .map_err(|error| error.to_string())?;
                }
            }
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
        if let Err(error) =
            control_native_wake_listener(state, NativeWakeListenerControl::RestartConfigured)
        {
            state.wake_word_runtime.record_runtime_error();
            return Err(error);
        }
        Ok(true)
    } else {
        control_native_wake_listener(state, NativeWakeListenerControl::Stop)?;
        Ok(false)
    }
}

fn supervise_native_wake_listener_shutdown(
    controller: NativeWakeListenerController,
    finish_generation: Option<u64>,
    handle: WakeLocalListenerHandle,
) -> Result<std_mpsc::Receiver<()>, String> {
    handle.request_shutdown();
    let (done_tx, done_rx) = std_mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("wake-word-listener-join".to_string())
        .spawn(move || {
            let _ = handle.join();
            if let Some(generation) = finish_generation {
                controller.finish_stop(generation);
            }
            let _ = done_tx.send(());
        })
        .map_err(|_| "Wake listener shutdown supervisor could not start".to_string())?;
    Ok(done_rx)
}

fn stop_native_wake_listener_thread(state: &AppState) -> Result<(), String> {
    match state.wake_listener_controller.begin_stop() {
        NativeWakeListenerStopPlan::AlreadyStopped
        | NativeWakeListenerStopPlan::AlreadyStopping
        | NativeWakeListenerStopPlan::AwaitingStart => Ok(()),
        NativeWakeListenerStopPlan::Supervise { generation, handle } => {
            let done = supervise_native_wake_listener_shutdown(
                state.wake_listener_controller.clone(),
                Some(generation),
                handle,
            )?;
            match done.recv_timeout(NATIVE_WAKE_LISTENER_STOP_WAIT) {
                Ok(()) | Err(std_mpsc::RecvTimeoutError::Timeout) => Ok(()),
                Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                    Err("Wake listener shutdown supervisor ended unexpectedly".to_string())
                }
            }
        }
    }
}

pub(crate) async fn wait_for_native_wake_listener_stopped(
    state: &AppState,
    timeout: Duration,
) -> bool {
    tokio::time::timeout(timeout, async {
        loop {
            if native_wake_listener_lifecycle_phase(state)
                == NativeWakeListenerLifecyclePhase::Stopped
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok()
}

#[cfg(test)]
#[path = "wake_word_state_tests.rs"]
mod tests;
