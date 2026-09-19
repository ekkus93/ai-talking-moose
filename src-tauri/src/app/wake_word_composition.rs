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
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let owner = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        owner.mark_loaded().unwrap();
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
}
