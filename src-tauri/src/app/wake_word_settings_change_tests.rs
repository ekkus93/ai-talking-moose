use super::state::{AppSettings, AppState};
use super::wake_word_state::{
    apply_configured_native_wake_listener_settings_change, control_native_wake_listener,
    native_wake_listener_is_active, NativeWakeListenerControl,
};
use crate::asr::wake_word_runtime::WakeWordRuntimePhase;
use crate::asr::AsrMode;

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
    assert!(!native_wake_listener_is_active());
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

    assert!(!native_wake_listener_is_active());
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
}
