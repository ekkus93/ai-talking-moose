use crate::app::state::AppState;
use crate::app::wake_word_composition::WakeWordApplicationRuntime;
use crate::app::wake_word_state;
use crate::asr::wake_word_diagnostics::WakeWordDiagnostics;
use std::time::Instant;
use tauri::State;

fn wake_word_diagnostics_snapshot(
    runtime: &WakeWordApplicationRuntime,
    now: Instant,
) -> WakeWordDiagnostics {
    WakeWordDiagnostics::from_runtime(&runtime.snapshot(now))
}

#[tauri::command]
pub fn get_wake_word_diagnostics(
    state: State<'_, AppState>,
) -> Result<WakeWordDiagnostics, String> {
    let runtime = wake_word_state::runtime_from_app_state(state.inner());
    Ok(wake_word_diagnostics_snapshot(runtime, Instant::now()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;
    use crate::asr::wake_word_runtime::WakeWordRuntimePhase;

    #[test]
    fn snapshot_reports_disabled_runtime_without_audio_content() {
        let runtime = WakeWordApplicationRuntime::from_settings(&AppSettings::default()).unwrap();
        let diagnostics = wake_word_diagnostics_snapshot(&runtime, Instant::now());

        assert!(!diagnostics.enabled);
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(diagnostics.ring_buffer_samples, 0);
        assert_eq!(diagnostics.handoff_pre_roll_samples, 0);

        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(!json.contains("pcm"));
        assert!(!json.contains("transcript"));
        assert!(!json.contains("credential"));
        assert!(!json.contains("path"));
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

        let diagnostics = wake_word_diagnostics_snapshot(&runtime, Instant::now());

        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Error);
        assert_eq!(
            diagnostics.last_error.as_deref(),
            Some("The Wake Word runtime encountered an internal error.")
        );
    }
}
