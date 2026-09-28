use super::state::AppState;
use super::wake_word_command_lifecycle::{
    complete_command_interaction, suspend_for_command_interaction,
    CommandInteractionTerminalOutcome,
};
use super::wake_word_state::{
    control_native_wake_listener, native_wake_listener_is_active, NativeWakeListenerControl,
};
use crate::asr::wake_word_runtime::WakeWordRuntimePhase;

fn enabled_listening_state() -> AppState {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().wake_word_enabled = true;
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );
    state
}

#[test]
fn manual_start_failure_after_active_wake_guard_does_not_leave_wake_suspended() {
    let state = enabled_listening_state();

    let listener_was_active =
        control_native_wake_listener(&state, NativeWakeListenerControl::TransferToCommand)
            .unwrap();

    assert!(!listener_was_active);
    assert!(!native_wake_listener_is_active());
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::SuspendedTalking
    );

    complete_command_interaction(
        &state.wake_word_runtime,
        true,
        CommandInteractionTerminalOutcome::RecoverableFailure,
    )
    .unwrap();

    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
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
fn cancel_and_recoverable_failure_paths_release_command_suspension() {
    for outcome in [
        CommandInteractionTerminalOutcome::Cancelled,
        CommandInteractionTerminalOutcome::RecoverableFailure,
    ] {
        let state = enabled_listening_state();

        assert!(suspend_for_command_interaction(&state.wake_word_runtime).unwrap());
        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::SuspendedTalking
        );

        complete_command_interaction(&state.wake_word_runtime, true, outcome).unwrap();

        assert_eq!(
            state.wake_word_runtime.phase(),
            WakeWordRuntimePhase::Listening,
            "terminal outcome {outcome:?} must not leave Wake suspended"
        );
    }
}

#[test]
fn disable_during_manual_transfer_wins_over_restart_after_cancel() {
    let state = enabled_listening_state();

    assert!(suspend_for_command_interaction(&state.wake_word_runtime).unwrap());
    state.settings.write().wake_word_enabled = false;

    complete_command_interaction(
        &state.wake_word_runtime,
        state.settings.read().wake_word_enabled,
        CommandInteractionTerminalOutcome::Cancelled,
    )
    .unwrap();

    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
}
