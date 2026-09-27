use super::state::AppSettings;
use super::wake_word_state::wake_word_asr_mode_supported;
use crate::asr::wake_word_diagnostics::WakeWordListenerStatus;
use crate::asr::wake_word_runtime::WakeWordRuntimePhase;

/// Classify native Wake listener ownership from the control-plane inputs that
/// matter for product diagnostics. This keeps listener state interpretation out
/// of UI command handlers and makes the state matrix testable without audio
/// hardware.
pub(crate) fn classify_native_listener_status(
    settings: &AppSettings,
    phase: WakeWordRuntimePhase,
    listener_active: bool,
    conversation_active: bool,
) -> WakeWordListenerStatus {
    if phase == WakeWordRuntimePhase::ShuttingDown {
        return WakeWordListenerStatus::ShuttingDown;
    }

    if listener_active {
        return match phase {
            WakeWordRuntimePhase::Listening => WakeWordListenerStatus::Active,
            WakeWordRuntimePhase::Triggered | WakeWordRuntimePhase::SuspendedTalking => {
                WakeWordListenerStatus::SuspendedForCommand
            }
            WakeWordRuntimePhase::Loading => WakeWordListenerStatus::Starting,
            WakeWordRuntimePhase::Error => WakeWordListenerStatus::FailedClosed,
            WakeWordRuntimePhase::Disabled => WakeWordListenerStatus::Active,
            WakeWordRuntimePhase::ShuttingDown => WakeWordListenerStatus::ShuttingDown,
        };
    }

    if !settings.wake_word_enabled || phase == WakeWordRuntimePhase::Disabled {
        return WakeWordListenerStatus::Stopped;
    }

    if !wake_word_asr_mode_supported(settings.asr_mode) || phase == WakeWordRuntimePhase::Error {
        return WakeWordListenerStatus::FailedClosed;
    }

    if conversation_active {
        return WakeWordListenerStatus::PendingUntilIdle;
    }

    match phase {
        WakeWordRuntimePhase::Loading => WakeWordListenerStatus::Starting,
        WakeWordRuntimePhase::Triggered | WakeWordRuntimePhase::SuspendedTalking => {
            WakeWordListenerStatus::SuspendedForCommand
        }
        WakeWordRuntimePhase::Listening => WakeWordListenerStatus::FailedClosed,
        WakeWordRuntimePhase::Error => WakeWordListenerStatus::FailedClosed,
        WakeWordRuntimePhase::Disabled => WakeWordListenerStatus::Stopped,
        WakeWordRuntimePhase::ShuttingDown => WakeWordListenerStatus::ShuttingDown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::AsrMode;

    fn wake_enabled_settings() -> AppSettings {
        AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn disabled_runtime_is_stopped_without_claiming_capture_release() {
        assert_eq!(
            classify_native_listener_status(
                &AppSettings::default(),
                WakeWordRuntimePhase::Disabled,
                false,
                false,
            ),
            WakeWordListenerStatus::Stopped
        );
        assert_eq!(
            classify_native_listener_status(
                &AppSettings::default(),
                WakeWordRuntimePhase::Disabled,
                true,
                false,
            ),
            WakeWordListenerStatus::Active
        );
    }

    #[test]
    fn listener_status_requires_physical_listener_for_active_listening() {
        let settings = wake_enabled_settings();

        assert_eq!(
            classify_native_listener_status(
                &settings,
                WakeWordRuntimePhase::Listening,
                false,
                false,
            ),
            WakeWordListenerStatus::FailedClosed
        );
        assert_eq!(
            classify_native_listener_status(
                &settings,
                WakeWordRuntimePhase::Listening,
                true,
                false,
            ),
            WakeWordListenerStatus::Active
        );
    }

    #[test]
    fn loading_and_active_conversation_are_pending_until_idle_without_listener() {
        let settings = wake_enabled_settings();

        assert_eq!(
            classify_native_listener_status(
                &settings,
                WakeWordRuntimePhase::Loading,
                false,
                true,
            ),
            WakeWordListenerStatus::PendingUntilIdle
        );
    }

    #[test]
    fn command_ownership_states_are_suspended_for_command() {
        let settings = wake_enabled_settings();

        for phase in [
            WakeWordRuntimePhase::Triggered,
            WakeWordRuntimePhase::SuspendedTalking,
        ] {
            assert_eq!(
                classify_native_listener_status(&settings, phase, false, false),
                WakeWordListenerStatus::SuspendedForCommand,
                "{phase:?}"
            );
            assert_eq!(
                classify_native_listener_status(&settings, phase, true, false),
                WakeWordListenerStatus::SuspendedForCommand,
                "{phase:?}"
            );
        }
    }

    #[test]
    fn unsupported_asr_and_runtime_errors_are_failed_closed() {
        let unsupported = AppSettings {
            wake_word_enabled: true,
            asr_mode: AsrMode::GeminiLiveAudio,
            ..Default::default()
        };
        let supported = wake_enabled_settings();

        assert_eq!(
            classify_native_listener_status(
                &unsupported,
                WakeWordRuntimePhase::Loading,
                false,
                false,
            ),
            WakeWordListenerStatus::FailedClosed
        );
        assert_eq!(
            classify_native_listener_status(
                &supported,
                WakeWordRuntimePhase::Error,
                false,
                false,
            ),
            WakeWordListenerStatus::FailedClosed
        );
    }

    #[test]
    fn shutdown_state_wins_over_listener_handle_presence() {
        let settings = wake_enabled_settings();

        assert_eq!(
            classify_native_listener_status(
                &settings,
                WakeWordRuntimePhase::ShuttingDown,
                true,
                false,
            ),
            WakeWordListenerStatus::ShuttingDown
        );
    }
}
