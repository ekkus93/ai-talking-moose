use super::state::{AppSettings, AppState};
use super::wake_word_listener_status::classify_native_listener_status;
use super::wake_word_state::{
    apply_configured_native_wake_listener_settings_change,
    apply_configured_native_wake_listener_settings_change_with_control,
    control_native_wake_listener, native_wake_listener_is_active, NativeWakeListenerControl,
};
use crate::asr::wake_word_diagnostics::WakeWordListenerStatus;
use crate::asr::wake_word_runtime::WakeWordRuntimePhase;
use crate::asr::AsrMode;

fn begin_listening(state: &AppState) {
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();
}

fn stop_runtime_for_fake_listener(state: &AppState) -> Result<bool, String> {
    state
        .wake_word_runtime
        .apply_enabled_setting(false)
        .map_err(|error| error.to_string())?;
    Ok(false)
}

#[test]
fn enabled_asr_mode_change_to_unsupported_fails_closed_without_listener() {
    let state = AppState::new_for_tests().unwrap();
    control_native_wake_listener(&state, NativeWakeListenerControl::Stop).unwrap();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();

    let previous = AppSettings {
        wake_word_enabled: true,
        asr_mode: AsrMode::MoonshineTinyStreaming,
        ..Default::default()
    };
    let next = AppSettings {
        wake_word_enabled: true,
        asr_mode: AsrMode::GeminiLiveAudio,
        ..previous.clone()
    };

    let error = apply_configured_native_wake_listener_settings_change(&state, &previous, &next)
        .unwrap_err();

    assert_eq!(error, "Wake Word V1 requires local Moonshine command ASR");
    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
}

#[test]
fn enabled_input_device_change_without_startup_config_enters_loading_without_listener_claim() {
    let state = AppState::new_for_tests().unwrap();
    control_native_wake_listener(&state, NativeWakeListenerControl::Stop).unwrap();

    let previous = AppSettings {
        wake_word_enabled: true,
        input_device: Some("Old Wake Input".to_string()),
        ..Default::default()
    };
    let next = AppSettings {
        wake_word_enabled: true,
        input_device: Some("New Wake Input".to_string()),
        ..previous.clone()
    };

    apply_configured_native_wake_listener_settings_change(&state, &previous, &next).unwrap();

    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
}

#[test]
fn enable_from_disabled_restarts_listener_with_pending_settings() {
    let state = AppState::new_for_tests().unwrap();
    let previous = AppSettings::default();
    let next = AppSettings {
        wake_word_enabled: true,
        ..previous.clone()
    };
    let mut actions = Vec::new();

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        false,
        |state, action| match action {
            NativeWakeListenerControl::RestartForSettings { settings } => {
                actions.push("restart");
                assert!(settings.wake_word_enabled);
                assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);
                state
                    .wake_word_runtime
                    .apply_enabled_setting(true)
                    .map_err(|error| error.to_string())?;
                state
                    .wake_word_runtime
                    .mark_loaded()
                    .map_err(|error| error.to_string())?;
                Ok(true)
            }
            other => panic!("unexpected listener action: {other:?}"),
        },
    )
    .unwrap();

    assert_eq!(actions, vec!["restart"]);
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );
}

#[test]
fn disable_from_listening_stops_before_reporting_disabled() {
    let state = AppState::new_for_tests().unwrap();
    begin_listening(&state);
    let previous = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };
    let next = AppSettings::default();
    let mut actions = Vec::new();

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => {
                actions.push("stop");
                stop_runtime_for_fake_listener(state)
            }
            other => panic!("unexpected listener action: {other:?}"),
        },
    )
    .unwrap();

    assert_eq!(actions, vec!["stop"]);
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
}

#[test]
fn input_device_change_stops_then_restarts_with_new_device() {
    let state = AppState::new_for_tests().unwrap();
    begin_listening(&state);
    let previous = AppSettings {
        wake_word_enabled: true,
        input_device: Some("Old Wake Input".to_string()),
        ..Default::default()
    };
    let next = AppSettings {
        input_device: Some("New Wake Input".to_string()),
        ..previous.clone()
    };
    let mut actions = Vec::new();

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => {
                actions.push("stop".to_string());
                stop_runtime_for_fake_listener(state)
            }
            NativeWakeListenerControl::RestartForSettings { settings } => {
                actions.push(format!(
                    "restart:{}",
                    settings.input_device.as_deref().unwrap_or("default")
                ));
                state
                    .wake_word_runtime
                    .apply_enabled_setting(true)
                    .map_err(|error| error.to_string())?;
                state
                    .wake_word_runtime
                    .mark_loaded()
                    .map_err(|error| error.to_string())?;
                Ok(true)
            }
            other => panic!("unexpected listener action: {other:?}"),
        },
    )
    .unwrap();

    assert_eq!(actions, vec!["stop", "restart:New Wake Input"]);
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );
}

#[test]
fn supported_asr_mode_change_stops_then_restarts_with_new_mode() {
    let state = AppState::new_for_tests().unwrap();
    begin_listening(&state);
    let previous = AppSettings {
        wake_word_enabled: true,
        asr_mode: AsrMode::MoonshineTinyStreaming,
        ..Default::default()
    };
    let next = AppSettings {
        asr_mode: AsrMode::MoonshineSmallStreaming,
        ..previous.clone()
    };
    let mut saw_restart = false;

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => stop_runtime_for_fake_listener(state),
            NativeWakeListenerControl::RestartForSettings { settings } => {
                saw_restart = true;
                assert_eq!(settings.asr_mode, AsrMode::MoonshineSmallStreaming);
                state
                    .wake_word_runtime
                    .apply_enabled_setting(true)
                    .map_err(|error| error.to_string())?;
                state
                    .wake_word_runtime
                    .mark_loaded()
                    .map_err(|error| error.to_string())?;
                Ok(true)
            }
            other => panic!("unexpected listener action: {other:?}"),
        },
    )
    .unwrap();

    assert!(saw_restart);
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );
}

#[test]
fn active_conversation_defers_device_restart_and_reports_pending() {
    let state = AppState::new_for_tests().unwrap();
    begin_listening(&state);
    let previous = AppSettings {
        wake_word_enabled: true,
        input_device: Some("Old Wake Input".to_string()),
        ..Default::default()
    };
    let next = AppSettings {
        input_device: Some("New Wake Input".to_string()),
        ..previous.clone()
    };
    let mut actions = Vec::new();

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        true,
        |state, action| match action {
            NativeWakeListenerControl::Stop => {
                actions.push("stop");
                stop_runtime_for_fake_listener(state)
            }
            other => panic!("restart must be deferred while conversation is active: {other:?}"),
        },
    )
    .unwrap();

    assert_eq!(actions, vec!["stop"]);
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
    assert_eq!(
        classify_native_listener_status(&next, state.wake_word_runtime.phase(), false, true),
        WakeWordListenerStatus::PendingUntilIdle
    );
}

#[test]
fn listener_restart_failure_fails_closed_without_false_listening_claim() {
    let state = AppState::new_for_tests().unwrap();
    begin_listening(&state);
    let previous = AppSettings {
        wake_word_enabled: true,
        input_device: Some("Old Wake Input".to_string()),
        ..Default::default()
    };
    let next = AppSettings {
        input_device: Some("New Wake Input".to_string()),
        ..previous.clone()
    };

    let error = apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => stop_runtime_for_fake_listener(state),
            NativeWakeListenerControl::RestartForSettings { .. } => {
                Err("Wake listener could not start".to_string())
            }
            other => panic!("unexpected listener action: {other:?}"),
        },
    )
    .unwrap_err();

    assert_eq!(error, "Wake listener could not start");
    assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
    assert_eq!(
        classify_native_listener_status(&next, state.wake_word_runtime.phase(), false, false),
        WakeWordListenerStatus::FailedClosed
    );
}
