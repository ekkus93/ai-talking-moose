use super::state::AppSettings;
use super::wake_word::engine::{
    NativeKwsSession, NativeKwsSessionPaths, SherpaKwsEngine, WakeWordError,
};
use super::wake_word::runtime::{
    WakeWordRuntimeError, WakeWordRuntimeManager, WakeWordRuntimePhase, WakeWordRuntimeSnapshot,
};
use super::wake_word_capture_consumer::WakeCapturePcmConsumer;
use super::wake_word_pcm_router::CanonicalWakePcmRouter;
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

    /// Build the production capture consumer from the authoritative runtime manager clone.
    ///
    /// The returned consumer owns no microphone stream. It routes the application's single
    /// canonical capture timeline into the shared Wake runtime manager and the supplied KWS
    /// engine so pre-roll retention, KWS inference, and wake→ASR handoff observe one ordered
    /// PCM stream.
    pub(crate) fn capture_consumer<E: SherpaKwsEngine>(
        &self,
        engine: E,
    ) -> WakeCapturePcmConsumer<E> {
        WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(self.manager.clone(), engine))
    }

    /// Build the production capture consumer with the real verified native sherpa KWS session.
    ///
    /// Test fakes remain available only through `capture_consumer(engine)` for deterministic unit
    /// coverage. The production constructor accepts explicit model/runtime roots, constructs the
    /// real native session, and fails closed before any microphone capture can start if verified
    /// artifacts or native runtime identities are missing or corrupt.
    pub(crate) fn native_capture_consumer(
        &self,
        paths: NativeKwsSessionPaths,
    ) -> Result<WakeCapturePcmConsumer<NativeKwsSession>, WakeWordError> {
        let engine = NativeKwsSession::new(paths)?;
        Ok(self.capture_consumer(engine))
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
    /// resolve to `Disabled`, never an unconditional resume to `Listening`. Conversely, if Wake
    /// Word was enabled while a manual interaction owned the microphone, the runtime can still be
    /// `Disabled`; terminal resolution starts its normal `Loading` path instead of failing the
    /// interaction teardown.
    pub fn resume_after_interaction(
        &self,
        wake_word_enabled: bool,
    ) -> Result<(), WakeWordRuntimeError> {
        if !wake_word_enabled {
            self.manager.disable();
            return Ok(());
        }

        match self.phase() {
            WakeWordRuntimePhase::Disabled | WakeWordRuntimePhase::Error => {
                self.manager.begin_enable()
            }
            WakeWordRuntimePhase::Loading => Ok(()),
            WakeWordRuntimePhase::Listening
            | WakeWordRuntimePhase::Triggered
            | WakeWordRuntimePhase::SuspendedTalking => self.manager.resume_after_interaction(),
            WakeWordRuntimePhase::ShuttingDown => self.manager.resume_after_interaction(),
        }
    }

    /// Fail Wake Word closed when the one authoritative microphone capture reports an error.
    ///
    /// This boundary deliberately does not reopen or replace `AudioCapture`: device reconnect
    /// remains the responsibility of the single application capture owner. Wake Word drops all
    /// retained PCM and enters a recoverable sanitized error state, avoiding retry spin and
    /// preventing a second capture stream from being created as an error-recovery side effect.
    pub fn record_capture_error(&self) {
        self.manager.record_runtime_error();
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
    fn native_capture_consumer_uses_real_session_and_fails_closed_without_verified_artifacts() {
        let owner = enabled_owner();
        let temp = tempfile::tempdir().unwrap();
        let paths = NativeKwsSessionPaths {
            model_dir: temp.path().join("model"),
            runtime_dir: temp.path().join("runtime"),
        };

        let result = owner.native_capture_consumer(paths);

        let Err(error) = result else {
            panic!("missing verified artifacts must fail before Wake capture starts");
        };
        assert_eq!(error.message, "missing required wake artifact");
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Listening);
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
    fn all_terminal_tts_outcomes_resume_through_same_enabled_policy() {
        for outcome in ["success", "cancellation", "recoverable_failure"] {
            let owner = enabled_owner();
            assert!(owner.manager().append_listening_pcm(&[10, 11, 12]));
            owner.suspend_for_talking().unwrap();

            owner
                .resume_after_interaction(true)
                .unwrap_or_else(|error| panic!("{outcome} should resume Wake Word: {error}"));

            let resumed = owner.snapshot(Instant::now());
            assert_eq!(resumed.phase, WakeWordRuntimePhase::Listening, "{outcome}");
            assert_eq!(resumed.ring_buffer_samples, 0, "{outcome}");
            assert_eq!(resumed.handoff_pre_roll_samples, 0, "{outcome}");
        }
    }

    #[test]
    fn repeated_terminal_resolution_does_not_leave_runtime_suspended() {
        let owner = enabled_owner();

        for _ in 0..3 {
            owner.suspend_for_talking().unwrap();
            assert_eq!(owner.phase(), WakeWordRuntimePhase::SuspendedTalking);
            owner.resume_after_interaction(true).unwrap();
            assert_eq!(owner.phase(), WakeWordRuntimePhase::Listening);
        }
    }

    #[test]
    fn disabled_during_trigger_or_talking_clears_audio_and_stays_disabled() {
        let owner = enabled_owner();
        assert!(owner.manager().append_listening_pcm(&[21, 22, 23]));
        assert!(owner.manager().accept_trigger(Instant::now()).unwrap());
        assert!(owner.snapshot(Instant::now()).handoff_pre_roll_samples > 0);

        owner.resume_after_interaction(false).unwrap();

        let disabled_after_trigger = owner.snapshot(Instant::now());
        assert_eq!(disabled_after_trigger.phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(disabled_after_trigger.ring_buffer_samples, 0);
        assert_eq!(disabled_after_trigger.handoff_pre_roll_samples, 0);

        owner.apply_enabled_setting(true).unwrap();
        owner.mark_loaded().unwrap();
        owner.suspend_for_talking().unwrap();
        owner.resume_after_interaction(false).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn terminal_interaction_starts_loading_when_wake_was_enabled_while_disabled() {
        let owner = WakeWordApplicationRuntime::from_settings(&AppSettings::default()).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Disabled);

        owner.resume_after_interaction(true).unwrap();

        assert_eq!(owner.phase(), WakeWordRuntimePhase::Loading);
    }

    #[test]
    fn terminal_interaction_preserves_loading_for_deferred_enable() {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let owner = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Loading);

        owner.resume_after_interaction(true).unwrap();

        assert_eq!(owner.phase(), WakeWordRuntimePhase::Loading);
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

    #[test]
    fn capture_error_fails_closed_without_restarting_or_retaining_audio() {
        let owner = enabled_owner();
        assert!(owner.manager().append_listening_pcm(&[7, 8, 9]));

        owner.record_capture_error();

        let failed = owner.snapshot(Instant::now());
        assert_eq!(failed.phase, WakeWordRuntimePhase::Error);
        assert_eq!(failed.ring_buffer_samples, 0);
        assert_eq!(failed.handoff_pre_roll_samples, 0);
        assert_eq!(
            failed.last_error,
            Some("The Wake Word runtime encountered an internal error.")
        );

        // Recovery is explicit and returns to Loading; this boundary never opens a stream.
        owner.apply_enabled_setting(true).unwrap();
        assert_eq!(owner.phase(), WakeWordRuntimePhase::Loading);
    }
}
