use super::state::AppState;
use super::wake_word_composition::{
    application_wake_word_runtime, initialize_application_wake_word_runtime,
    WakeWordApplicationRuntime,
};

/// Resolve the process-wide Wake Word runtime from authoritative application state.
///
/// Initialization is seeded from the already-normalized persisted settings held by
/// `AppState`. The runtime owns lifecycle/KWS state only; physical microphone capture
/// remains exclusively owned by `AppState::audio_capture`.
#[expect(
    dead_code,
    reason = "WWR-300 introduces the AppState binding before the subsequent routing slice calls it from production startup"
)]
pub(crate) fn initialize_from_app_state(
    state: &AppState,
) -> Result<&'static WakeWordApplicationRuntime, String> {
    let settings = state.settings.read().clone();
    initialize_application_wake_word_runtime(&settings)
}

/// Access the process-wide Wake Word runtime after application composition.
#[expect(
    dead_code,
    reason = "WWR-300 exposes the authoritative state lookup before later lifecycle wiring consumes it"
)]
pub(crate) fn runtime_from_app_state(
    _state: &AppState,
) -> Result<&'static WakeWordApplicationRuntime, String> {
    application_wake_word_runtime()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::wake_word_runtime::WakeWordRuntimePhase;

    #[test]
    fn state_binding_uses_persisted_setting_without_owning_capture() {
        let state = AppState::new_for_tests().unwrap();
        assert!(!state.settings.read().wake_word_enabled);

        let runtime = initialize_from_app_state(&state).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
        assert_eq!(
            runtime_from_app_state(&state).unwrap().phase(),
            WakeWordRuntimePhase::Disabled
        );

        // The binding exposes no AudioCapture and therefore cannot create a competing
        // microphone stream. AppState remains the sole physical capture owner.
        assert!(!state.audio_capture.lock().diagnostics().active);
    }
}
