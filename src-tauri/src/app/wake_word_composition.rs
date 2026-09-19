use super::state::AppSettings;
use super::wake_word::runtime::{
    WakeWordRuntimeError, WakeWordRuntimeManager, WakeWordRuntimePhase, WakeWordRuntimeSnapshot,
};
use std::time::Instant;

/// Authoritative application-level owner for the Wake Word V1 runtime.
///
/// This composition boundary deliberately owns no microphone stream. Audio remains owned by
/// `AppState::audio_capture`; later WWR-300 wiring routes that one canonical stream into this
/// runtime rather than allowing Wake Word to open a competing capture device.
#[derive(Clone)]
pub struct WakeWordApplicationRuntime {
    manager: WakeWordRuntimeManager,
}

impl WakeWordApplicationRuntime {
    /// Build the production Wake Word owner from the already-normalized persisted settings.
    ///
    /// Enabled startup enters `Loading`; it must not claim `Listening` until the pinned KWS
    /// artifacts/runtime have actually loaded and the caller invokes `mark_loaded`.
    pub fn from_settings(settings: &AppSettings) -> Result<Self, WakeWordRuntimeError> {
        let manager = WakeWordRuntimeManager::new();
        if settings.wake_word_enabled {
            manager.begin_enable()?;
        }
        Ok(Self { manager })
    }

    pub fn manager(&self) -> &WakeWordRuntimeManager {
        &self.manager
    }

    pub fn snapshot(&self, now: Instant) -> WakeWordRuntimeSnapshot {
        self.manager.snapshot(now)
    }

    pub fn apply_enabled_setting(&self, enabled: bool) -> Result<(), WakeWordRuntimeError> {
        if enabled {
            self.manager.begin_enable()
        } else {
            self.manager.disable();
            Ok(())
        }
    }

    pub fn mark_loaded(&self) -> Result<(), WakeWordRuntimeError> {
        self.manager.mark_loaded()
    }

    /// Enter the Talking/TTS suspension boundary before any Moose audio can feed KWS.
    /// The manager clears retained pre-roll as part of this transition.
    pub fn suspend_for_talking(&self) -> Result<(), WakeWordRuntimeError> {
        self.manager.suspend_for_talking()
    }

    /// Resume after any terminal interaction/TTS outcome while honoring the latest setting.
    ///
    /// The explicit setting argument is intentional: disabling Wake Word while Talking must
    /// resolve to `Disabled`, never an unconditional resume to `Listening`.
    pub fn resume_after_interaction(
        &self,
        wake_word_enabled: bool,
    ) -> Result<(), WakeWordRuntimeError> {
        if wake_word_enabled {
            self.manager.resume_after_interaction()
        } else {
            self.manager.disable();
            Ok(())
        }
    }

    pub fn record_runtime_error(&self) {
        self.manager.record_runtime_error();
    }

    pub fn begin_shutdown(&self) {
        self.manager.begin_shutdown();
    }

    pub fn phase(&self) -> WakeWordRuntimePhase {
        self.snapshot(Instant::now()).phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_owner() -> WakeWordApplicationRuntime {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let owner = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        owner.mark_loaded().unwrap();
        owner
    }

    #[test]
    fn persisted_disabled_setting_constructs_disabled_owner() {
        let settings = AppSettings::default();
        let owner = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn persisted_enabled_setting_starts_loading_without_false_listening_claim() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let owner = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Loading);

        owner.mark_loaded().unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Listening);
    }

    #[test]
    fn disabling_owner_is_immediate_and_prevents_unintended_resume() {
        let owner = enabled_owner();
        owner.apply_enabled_setting(false).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn cloned_composition_shares_exactly_one_runtime_manager_state() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let owner = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        let clone = owner.clone();
        owner.mark_loaded().unwrap();
        assert_eq!(clone.phase(), WakeWordRuntimePhase::Listening);
    }

    #[test]
    fn talking_suspends_and_clears_retained_audio() {
        let owner = enabled_owner();
        assert!(owner.manager().append_listening_pcm(&[1, 2, 3]));
        assert_eq!(owner.snapshot(Instant::now()).ring_buffer_samples, 3);

        owner.suspend_for_talking().unwrap();

        let suspended = owner.snapshot(Instant::now());
        assert_eq!(suspended.phase, WakeWordRuntimePhase::SuspendedTalking);
        assert_eq!(suspended.ring_buffer_samples, 0);
        assert_eq!(suspended.handoff_pre_roll_samples, 0);
    }

    #[test]
    fn terminal_tts_outcome_resumes_listening_when_still_enabled() {
        let owner = enabled_owner();
        owner.suspend_for_talking().unwrap();
        owner.resume_after_interaction(true).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Listening);
    }

    #[test]
    fn disabling_during_talking_wins_over_resume() {
        let owner = enabled_owner();
        owner.suspend_for_talking().unwrap();
        owner.resume_after_interaction(false).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn recoverable_runtime_error_can_reenter_loading_without_affecting_manual_owner() {
        let owner = enabled_owner();
        owner.record_runtime_error();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Error);

        owner.apply_enabled_setting(true).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Loading);
    }
}
