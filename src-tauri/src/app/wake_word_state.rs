use super::state::AppState;
use super::wake_word_composition::WakeWordApplicationRuntime;

/// Access the one Wake Word runtime owned directly by authoritative application state.
///
/// Physical microphone capture remains exclusively owned by `AppState::audio_capture`;
/// the Wake runtime owns lifecycle/KWS state only.
pub(crate) fn runtime_from_app_state(state: &AppState) -> &WakeWordApplicationRuntime {
    &state.wake_word_runtime
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::wake_word_runtime::WakeWordRuntimePhase;

    #[test]
    fn app_state_directly_owns_wake_runtime_without_owning_capture() {
        let state = AppState::new_for_tests().unwrap();
        assert!(!state.settings.read().wake_word_enabled);
        assert_eq!(
            runtime_from_app_state(&state).phase(),
            WakeWordRuntimePhase::Disabled
        );
        assert!(!state.audio_capture.lock().diagnostics().active);
    }
}
