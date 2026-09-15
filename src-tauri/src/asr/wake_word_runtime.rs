use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeWordRuntimePhase {
    Disabled,
    Loading,
    Listening,
    Triggered,
    SuspendedTalking,
    Error,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeWordRuntimeErrorKind {
    ShuttingDown,
    Disabled,
    Busy,
    InvalidTransition,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeWordRuntimeError {
    pub kind: WakeWordRuntimeErrorKind,
    pub message: &'static str,
}

impl WakeWordRuntimeError {
    fn new(kind: WakeWordRuntimeErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }

    fn shutting_down() -> Self {
        Self::new(
            WakeWordRuntimeErrorKind::ShuttingDown,
            "The Wake Word runtime is shutting down.",
        )
    }

    fn disabled() -> Self {
        Self::new(
            WakeWordRuntimeErrorKind::Disabled,
            "Wake Word listening is disabled.",
        )
    }

    fn busy() -> Self {
        Self::new(
            WakeWordRuntimeErrorKind::Busy,
            "A Wake Word interaction is already in progress.",
        )
    }

    fn invalid_transition() -> Self {
        Self::new(
            WakeWordRuntimeErrorKind::InvalidTransition,
            "The Wake Word runtime cannot perform that transition from its current state.",
        )
    }

    fn runtime() -> Self {
        Self::new(
            WakeWordRuntimeErrorKind::Runtime,
            "The Wake Word runtime encountered an internal error.",
        )
    }
}

impl std::fmt::Display for WakeWordRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for WakeWordRuntimeError {}

#[derive(Debug, Clone)]
struct WakeWordRuntimeState {
    phase: WakeWordRuntimePhase,
    trigger_count: u64,
    last_trigger_at: Option<Instant>,
    last_error: Option<&'static str>,
}

impl Default for WakeWordRuntimeState {
    fn default() -> Self {
        Self {
            phase: WakeWordRuntimePhase::Disabled,
            trigger_count: 0,
            last_trigger_at: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeWordRuntimeSnapshot {
    pub phase: WakeWordRuntimePhase,
    pub trigger_count: u64,
    pub last_trigger_age: Option<Duration>,
    pub last_error: Option<&'static str>,
}

#[derive(Clone, Default)]
pub struct WakeWordRuntimeManager {
    state: Arc<Mutex<WakeWordRuntimeState>>,
    shutting_down: Arc<AtomicBool>,
}

impl WakeWordRuntimeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_enable(&self) -> Result<(), WakeWordRuntimeError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(WakeWordRuntimeError::shutting_down());
        }
        let mut state = self.state.lock();
        match state.phase {
            WakeWordRuntimePhase::Disabled | WakeWordRuntimePhase::Error => {
                state.phase = WakeWordRuntimePhase::Loading;
                state.last_error = None;
                Ok(())
            }
            WakeWordRuntimePhase::Loading | WakeWordRuntimePhase::Listening => Ok(()),
            WakeWordRuntimePhase::Triggered | WakeWordRuntimePhase::SuspendedTalking => {
                Err(WakeWordRuntimeError::busy())
            }
            WakeWordRuntimePhase::ShuttingDown => Err(WakeWordRuntimeError::shutting_down()),
        }
    }

    pub fn mark_loaded(&self) -> Result<(), WakeWordRuntimeError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(WakeWordRuntimeError::shutting_down());
        }
        let mut state = self.state.lock();
        if state.phase != WakeWordRuntimePhase::Loading {
            return Err(WakeWordRuntimeError::invalid_transition());
        }
        state.phase = WakeWordRuntimePhase::Listening;
        Ok(())
    }

    pub fn disable(&self) {
        if self.shutting_down.load(Ordering::SeqCst) {
            return;
        }
        let mut state = self.state.lock();
        state.phase = WakeWordRuntimePhase::Disabled;
        state.last_error = None;
    }

    pub fn accept_trigger(&self, now: Instant) -> Result<bool, WakeWordRuntimeError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(WakeWordRuntimeError::shutting_down());
        }
        let mut state = self.state.lock();
        match state.phase {
            WakeWordRuntimePhase::Listening => {
                state.phase = WakeWordRuntimePhase::Triggered;
                state.trigger_count = state.trigger_count.saturating_add(1);
                state.last_trigger_at = Some(now);
                Ok(true)
            }
            WakeWordRuntimePhase::Triggered | WakeWordRuntimePhase::SuspendedTalking => Ok(false),
            WakeWordRuntimePhase::Disabled => Err(WakeWordRuntimeError::disabled()),
            WakeWordRuntimePhase::Loading | WakeWordRuntimePhase::Error => {
                Err(WakeWordRuntimeError::invalid_transition())
            }
            WakeWordRuntimePhase::ShuttingDown => Err(WakeWordRuntimeError::shutting_down()),
        }
    }

    pub fn suspend_for_talking(&self) -> Result<(), WakeWordRuntimeError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(WakeWordRuntimeError::shutting_down());
        }
        let mut state = self.state.lock();
        match state.phase {
            WakeWordRuntimePhase::Triggered | WakeWordRuntimePhase::Listening => {
                state.phase = WakeWordRuntimePhase::SuspendedTalking;
                Ok(())
            }
            WakeWordRuntimePhase::SuspendedTalking => Ok(()),
            WakeWordRuntimePhase::Disabled => Err(WakeWordRuntimeError::disabled()),
            WakeWordRuntimePhase::Loading | WakeWordRuntimePhase::Error => {
                Err(WakeWordRuntimeError::invalid_transition())
            }
            WakeWordRuntimePhase::ShuttingDown => Err(WakeWordRuntimeError::shutting_down()),
        }
    }

    pub fn resume_after_interaction(&self) -> Result<(), WakeWordRuntimeError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(WakeWordRuntimeError::shutting_down());
        }
        let mut state = self.state.lock();
        match state.phase {
            WakeWordRuntimePhase::Triggered | WakeWordRuntimePhase::SuspendedTalking => {
                state.phase = WakeWordRuntimePhase::Listening;
                Ok(())
            }
            WakeWordRuntimePhase::Listening => Ok(()),
            WakeWordRuntimePhase::Disabled => Err(WakeWordRuntimeError::disabled()),
            WakeWordRuntimePhase::Loading | WakeWordRuntimePhase::Error => {
                Err(WakeWordRuntimeError::invalid_transition())
            }
            WakeWordRuntimePhase::ShuttingDown => Err(WakeWordRuntimeError::shutting_down()),
        }
    }

    pub fn record_runtime_error(&self) {
        if self.shutting_down.load(Ordering::SeqCst) {
            return;
        }
        let mut state = self.state.lock();
        state.phase = WakeWordRuntimePhase::Error;
        state.last_error = Some(WakeWordRuntimeError::runtime().message);
    }

    pub fn begin_shutdown(&self) {
        if self.shutting_down.swap(true, Ordering::SeqCst) {
            return;
        }
        let mut state = self.state.lock();
        state.phase = WakeWordRuntimePhase::ShuttingDown;
    }

    pub fn snapshot(&self, now: Instant) -> WakeWordRuntimeSnapshot {
        let state = self.state.lock();
        WakeWordRuntimeSnapshot {
            phase: state.phase,
            trigger_count: state.trigger_count,
            last_trigger_age: state
                .last_trigger_at
                .and_then(|triggered_at| now.checked_duration_since(triggered_at)),
            last_error: state.last_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_starts_disabled_and_enables_through_loading() {
        let manager = WakeWordRuntimeManager::new();
        let now = Instant::now();
        assert_eq!(manager.snapshot(now).phase, WakeWordRuntimePhase::Disabled);

        manager.begin_enable().unwrap();
        assert_eq!(manager.snapshot(now).phase, WakeWordRuntimePhase::Loading);
        manager.mark_loaded().unwrap();
        assert_eq!(manager.snapshot(now).phase, WakeWordRuntimePhase::Listening);
    }

    #[test]
    fn repeated_positive_frames_create_one_trigger_until_interaction_resets() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        let first = Instant::now();

        assert!(manager.accept_trigger(first).unwrap());
        assert!(!manager.accept_trigger(first + Duration::from_millis(20)).unwrap());
        let triggered = manager.snapshot(first + Duration::from_millis(20));
        assert_eq!(triggered.phase, WakeWordRuntimePhase::Triggered);
        assert_eq!(triggered.trigger_count, 1);

        manager.resume_after_interaction().unwrap();
        assert!(manager
            .accept_trigger(first + Duration::from_secs(1))
            .unwrap());
        assert_eq!(
            manager.snapshot(first + Duration::from_secs(1)).trigger_count,
            2
        );
    }

    #[test]
    fn talking_suspension_blocks_activation_until_resume() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        let now = Instant::now();

        manager.suspend_for_talking().unwrap();
        assert_eq!(
            manager.snapshot(now).phase,
            WakeWordRuntimePhase::SuspendedTalking
        );
        assert!(!manager.accept_trigger(now).unwrap());
        manager.resume_after_interaction().unwrap();
        assert!(manager.accept_trigger(now).unwrap());
    }

    #[test]
    fn disable_restores_fail_closed_manual_only_state() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        manager.disable();

        let error = manager.accept_trigger(Instant::now()).unwrap_err();
        assert_eq!(error.kind, WakeWordRuntimeErrorKind::Disabled);
        assert_eq!(manager.snapshot(Instant::now()).phase, WakeWordRuntimePhase::Disabled);
    }

    #[test]
    fn shutdown_is_idempotent_and_rejects_later_activation() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        manager.begin_shutdown();
        manager.begin_shutdown();

        assert_eq!(
            manager.snapshot(Instant::now()).phase,
            WakeWordRuntimePhase::ShuttingDown
        );
        assert_eq!(
            manager.begin_enable().unwrap_err().kind,
            WakeWordRuntimeErrorKind::ShuttingDown
        );
    }

    #[test]
    fn runtime_error_is_sanitized_and_recoverable_through_enable() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        manager.record_runtime_error();
        let snapshot = manager.snapshot(Instant::now());
        assert_eq!(snapshot.phase, WakeWordRuntimePhase::Error);
        assert_eq!(
            snapshot.last_error,
            Some("The Wake Word runtime encountered an internal error.")
        );

        manager.begin_enable().unwrap();
        assert_eq!(
            manager.snapshot(Instant::now()).phase,
            WakeWordRuntimePhase::Loading
        );
        assert_eq!(manager.snapshot(Instant::now()).last_error, None);
    }
}
