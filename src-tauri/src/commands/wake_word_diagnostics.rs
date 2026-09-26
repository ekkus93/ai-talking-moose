use crate::app::state::{AppSettings, AppState};
use crate::app::wake_word_composition::WakeWordApplicationRuntime;
use crate::app::wake_word_state;
use crate::asr::wake_word_diagnostics::{WakeWordDiagnostics, WakeWordListenerStatus};
use crate::asr::wake_word_runtime::WakeWordRuntimePhase;
use std::time::Instant;
use tauri::State;

fn wake_word_diagnostics_snapshot(
    runtime: &WakeWordApplicationRuntime,
    listener_status: WakeWordListenerStatus,
    now: Instant,
) -> WakeWordDiagnostics {
    WakeWordDiagnostics::from_runtime_and_listener(&runtime.snapshot(now), listener_status)
}

fn classify_native_listener_status(state: &AppState) -> WakeWordListenerStatus {
    classify_native_listener_status_parts(
        state.settings.read().clone(),
        state.wake_word_runtime.phase(),
        wake_word_state::native_wake_listener_is_active(),
        state.conversation_mgr.is_active(),
    )
}

fn classify_native_listener_status_parts(
    settings: AppSettings,
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

    if !wake_word_state::wake_word_asr_mode_supported(settings.asr_mode)
        || phase == WakeWordRuntimePhase::Error
    {
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

#[tauri::command]
pub fn get_wake_word_diagnostics(
    state: State<'_, AppState>,
) -> Result<WakeWordDiagnostics, String> {
    let runtime = wake_word_state::runtime_from_app_state(state.inner());
    let listener_status = classify_native_listener_status(state.inner());
    Ok(wake_word_diagnostics_snapshot(
        runtime,
        listener_status,
        Instant::now(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;
    use crate::asr::AsrMode;

    #[test]
    fn snapshot_reports_disabled_runtime_without_audio_content() {
        let runtime = WakeWordApplicationRuntime::from_settings(&AppSettings::default()).unwrap();
        let diagnostics = wake_word_diagnostics_snapshot(
            &runtime,
            WakeWordListenerStatus::Stopped,
            Instant::now(),
        );

        assert!(!diagnostics.enabled);
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(diagnostics.listener_status, WakeWordListenerStatus::Stopped);
        assert!(!diagnostics.listener_active);
        assert!(!diagnostics.listening);
        assert_eq!(diagnostics.ring_buffer_samples, 0);
        assert_eq!(diagnostics.handoff_pre_roll_samples, 0);

        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(!json.contains("pcm"));
        assert!(!json.contains("transcript"));
        assert!(!json.contains("credential"));
        assert!(!json.contains("path"));
    }

    #[test]
    fn snapshot_reports_listener_ownership_separately_from_runtime_phase() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        runtime.mark_loaded().unwrap();

        let diagnostics = wake_word_diagnostics_snapshot(
            &runtime,
            WakeWordListenerStatus::Active,
            Instant::now(),
        );

        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Listening);
        assert_eq!(diagnostics.listener_status, WakeWordListenerStatus::Active);
        assert!(diagnostics.listener_active);
        assert!(diagnostics.listening);
    }

    #[test]
    fn listener_status_does_not_treat_runtime_listening_as_active_listener() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };

        assert_eq!(
            classify_native_listener_status_parts(
                settings,
                WakeWordRuntimePhase::Listening,
                false,
                false,
            ),
            WakeWordListenerStatus::FailedClosed
        );
    }

    #[test]
    fn listener_status_reports_pending_until_idle() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };

        assert_eq!(
            classify_native_listener_status_parts(
                settings,
                WakeWordRuntimePhase::Loading,
                false,
                true,
            ),
            WakeWordListenerStatus::PendingUntilIdle
        );
    }

    #[test]
    fn listener_status_reports_unsupported_asr_as_failed_closed() {
        let settings = AppSettings {
            wake_word_enabled: true,
            asr_mode: AsrMode::GeminiLiveAudio,
            ..Default::default()
        };

        assert_eq!(
            classify_native_listener_status_parts(
                settings,
                WakeWordRuntimePhase::Loading,
                false,
                false,
            ),
            WakeWordListenerStatus::FailedClosed
        );
    }

    #[test]
    fn snapshot_exposes_sanitized_error_only() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        runtime.mark_loaded().unwrap();
        runtime.record_runtime_error();

        let diagnostics = wake_word_diagnostics_snapshot(
            &runtime,
            WakeWordListenerStatus::FailedClosed,
            Instant::now(),
        );

        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Error);
        assert_eq!(
            diagnostics.listener_status,
            WakeWordListenerStatus::FailedClosed
        );
        assert!(!diagnostics.listener_active);
        assert!(!diagnostics.listening);
        assert_eq!(
            diagnostics.last_error.as_deref(),
            Some("The Wake Word runtime encountered an internal error.")
        );
    }
}
