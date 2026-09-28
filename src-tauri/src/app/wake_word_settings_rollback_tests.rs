use super::state::{AppSettings, AppState};
use super::wake_word_listener_status::classify_native_listener_status;
use super::wake_word_state::{
    apply_configured_native_wake_listener_settings_change,
    apply_configured_native_wake_listener_settings_change_with_control, NativeWakeListenerControl,
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

fn start_fake_listener(state: &AppState) -> Result<bool, String> {
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

#[test]
fn listener_rollback_restores_previous_runtime_and_listener_settings() {
    let state = AppState::new_for_tests().unwrap();
    begin_listening(&state);
    let previous = AppSettings {
        wake_word_enabled: true,
        input_device: Some("Previous Wake Mic".to_string()),
        ..Default::default()
    };
    let next = AppSettings {
        input_device: Some("Broken Wake Mic".to_string()),
        ..previous.clone()
    };
    let mut actions = Vec::new();

    let forward_error = apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &previous,
        &next,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => {
                actions.push("forward-stop".to_string());
                stop_runtime_for_fake_listener(state)
            }
            NativeWakeListenerControl::RestartForSettings { settings } => {
                actions.push(format!(
                    "forward-restart:{}",
                    settings.input_device.as_deref().unwrap_or("default")
                ));
                Err("Wake listener could not start".to_string())
            }
            other => panic!("unexpected forward action: {other:?}"),
        },
    )
    .unwrap_err();

    assert_eq!(forward_error, "Wake listener could not start");
    assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &next,
        &previous,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => {
                actions.push("rollback-stop".to_string());
                stop_runtime_for_fake_listener(state)
            }
            NativeWakeListenerControl::RestartForSettings { settings } => {
                actions.push(format!(
                    "rollback-restart:{}",
                    settings.input_device.as_deref().unwrap_or("default")
                ));
                start_fake_listener(state)
            }
            other => panic!("unexpected rollback action: {other:?}"),
        },
    )
    .unwrap();

    assert_eq!(
        actions,
        vec![
            "forward-stop",
            "forward-restart:Broken Wake Mic",
            "rollback-stop",
            "rollback-restart:Previous Wake Mic",
        ]
    );
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );
    assert_eq!(
        classify_native_listener_status(&previous, state.wake_word_runtime.phase(), true, false),
        WakeWordListenerStatus::Active
    );
}

#[test]
fn unsupported_asr_settings_failure_is_sanitized_and_actionable() {
    let state = AppState::new_for_tests().unwrap();
    let previous = AppSettings::default();
    let next = AppSettings {
        wake_word_enabled: true,
        asr_mode: AsrMode::GeminiLiveAudio,
        ..previous.clone()
    };

    let error = apply_configured_native_wake_listener_settings_change(&state, &previous, &next)
        .unwrap_err();

    assert_eq!(error, "Wake Word V1 requires local Moonshine command ASR");
    assert!(!error.contains('/'));
    assert!(!error.to_lowercase().contains("secret"));
    assert!(!error.to_lowercase().contains("audio"));
    assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
}

#[test]
fn settings_enable_disable_refreshes_backend_status_after_completion() {
    let state = AppState::new_for_tests().unwrap();
    let disabled = AppSettings::default();
    let enabled = AppSettings {
        wake_word_enabled: true,
        ..disabled.clone()
    };

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &disabled,
        &enabled,
        false,
        |state, action| match action {
            NativeWakeListenerControl::RestartForSettings { .. } => start_fake_listener(state),
            other => panic!("unexpected enable action: {other:?}"),
        },
    )
    .unwrap();
    assert_eq!(
        classify_native_listener_status(&enabled, state.wake_word_runtime.phase(), true, false),
        WakeWordListenerStatus::Active
    );

    apply_configured_native_wake_listener_settings_change_with_control(
        &state,
        &enabled,
        &disabled,
        false,
        |state, action| match action {
            NativeWakeListenerControl::Stop => stop_runtime_for_fake_listener(state),
            other => panic!("unexpected disable action: {other:?}"),
        },
    )
    .unwrap();
    assert_eq!(
        classify_native_listener_status(&disabled, state.wake_word_runtime.phase(), false, false),
        WakeWordListenerStatus::Stopped
    );
}
