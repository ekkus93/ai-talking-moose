use super::wake_word::runtime::WakeWordRuntimePhase;
use super::wake_word_composition::WakeWordApplicationRuntime;

/// Terminal outcomes that all release command-interaction ownership back to Wake Word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandInteractionTerminalOutcome {
    Success,
    Cancelled,
    RecoverableFailure,
}

/// Guard the V1 Wake Word runtime while the normal command interaction owns the microphone.
///
/// V1 deliberately has no barge-in. Once command ASR starts, Wake Word must remain suspended
/// through ASR, Thinking, and Talking until the interaction reaches a terminal outcome.
pub(crate) fn suspend_for_command_interaction(
    runtime: &WakeWordApplicationRuntime,
) -> Result<bool, String> {
    match runtime.phase() {
        WakeWordRuntimePhase::Listening | WakeWordRuntimePhase::Triggered => {
            runtime
                .suspend_for_talking()
                .map_err(|error| error.to_string())?;
            Ok(true)
        }
        WakeWordRuntimePhase::SuspendedTalking => Ok(true),
        WakeWordRuntimePhase::Disabled
        | WakeWordRuntimePhase::Loading
        | WakeWordRuntimePhase::Error
        | WakeWordRuntimePhase::ShuttingDown => Ok(false),
    }
}

/// Resolve the terminal command-interaction boundary against the latest persisted enable state.
/// Disabling Wake Word during an interaction therefore wins over an otherwise normal resume.
pub(crate) fn resume_after_command_interaction(
    runtime: &WakeWordApplicationRuntime,
    wake_word_enabled: bool,
) -> Result<(), String> {
    runtime
        .resume_after_interaction(wake_word_enabled)
        .map_err(|error| error.to_string())
}

/// Release command ownership for every recoverable terminal path through one policy boundary.
///
/// V1 intentionally treats success, cancellation, and recoverable failure identically for Wake
/// ownership: stale pre-roll/KWS state is cleared by `resume_after_interaction`, and the latest
/// enabled setting decides whether the runtime returns to Listening or remains Disabled.
pub(crate) fn complete_command_interaction(
    runtime: &WakeWordApplicationRuntime,
    wake_word_enabled: bool,
    _outcome: CommandInteractionTerminalOutcome,
) -> Result<(), String> {
    resume_after_command_interaction(runtime, wake_word_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;

    fn listening_runtime() -> WakeWordApplicationRuntime {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        runtime.mark_loaded().unwrap();
        runtime
    }

    #[test]
    fn command_interaction_suspends_wake_until_terminal_resume() {
        let runtime = listening_runtime();
        assert!(suspend_for_command_interaction(&runtime).unwrap());
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);

        resume_after_command_interaction(&runtime, true).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);
    }

    #[test]
    fn disabling_during_command_interaction_wins_over_terminal_resume() {
        let runtime = listening_runtime();
        assert!(suspend_for_command_interaction(&runtime).unwrap());
        runtime.apply_enabled_setting(false).unwrap();

        resume_after_command_interaction(&runtime, false).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn disabled_manual_interaction_never_enables_wake() {
        let runtime = WakeWordApplicationRuntime::from_settings(&AppSettings::default()).unwrap();
        assert!(!suspend_for_command_interaction(&runtime).unwrap());

        resume_after_command_interaction(&runtime, false).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn loading_wake_does_not_claim_command_ownership() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Loading);

        assert!(!suspend_for_command_interaction(&runtime).unwrap());
        complete_command_interaction(&runtime, true, CommandInteractionTerminalOutcome::Success)
            .unwrap();

        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Loading);
    }

    #[test]
    fn wake_error_does_not_block_manual_command_guard() {
        let runtime = listening_runtime();
        runtime.record_runtime_error();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Error);

        assert!(!suspend_for_command_interaction(&runtime).unwrap());

        complete_command_interaction(
            &runtime,
            true,
            CommandInteractionTerminalOutcome::RecoverableFailure,
        )
        .unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Loading);
    }

    #[test]
    fn suspended_runtime_remains_guarded_across_asr_and_thinking() {
        let runtime = listening_runtime();
        assert!(suspend_for_command_interaction(&runtime).unwrap());
        assert!(suspend_for_command_interaction(&runtime).unwrap());
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    }

    #[test]
    fn every_recoverable_terminal_outcome_returns_to_listening_when_enabled() {
        for outcome in [
            CommandInteractionTerminalOutcome::Success,
            CommandInteractionTerminalOutcome::Cancelled,
            CommandInteractionTerminalOutcome::RecoverableFailure,
        ] {
            let runtime = listening_runtime();
            assert!(suspend_for_command_interaction(&runtime).unwrap());
            complete_command_interaction(&runtime, true, outcome).unwrap();
            assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);
        }
    }

    #[test]
    fn every_terminal_outcome_honors_disable_during_interaction() {
        for outcome in [
            CommandInteractionTerminalOutcome::Success,
            CommandInteractionTerminalOutcome::Cancelled,
            CommandInteractionTerminalOutcome::RecoverableFailure,
        ] {
            let runtime = listening_runtime();
            assert!(suspend_for_command_interaction(&runtime).unwrap());
            runtime.apply_enabled_setting(false).unwrap();
            complete_command_interaction(&runtime, false, outcome).unwrap();
            assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
        }
    }
}
