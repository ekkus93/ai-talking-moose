use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterState {
    Hidden,
    Appearing,
    Idle,
    Listening,
    Thinking,
    Talking,
    Interrupted,
    Dismissed,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouthShape {
    Closed,
    Small,
    Medium,
    Wide,
}

impl CharacterState {
    /// Check if transition from `self` to `target` is valid according to state machine rules.
    pub fn can_transition_to(&self, target: &CharacterState) -> bool {
        match (self, target) {
            // Self transitions are always permitted (e.g. refresh/reaffirm).
            (a, b) if a == b => true,

            // Error, Hidden, and Dismissed are interrupt/terminal presentation states and
            // may be reached from any current presentation state. Mute is deliberately
            // not represented here; AppState::is_muted is the single mute authority.
            (_, CharacterState::Error | CharacterState::Hidden) => true,
            (_, CharacterState::Dismissed) => true,

            // From Hidden: can only appear or become idle.
            (CharacterState::Hidden, CharacterState::Appearing | CharacterState::Idle) => true,
            (CharacterState::Hidden, _) => false,

            // From Appearing: transitions into Idle or Listening.
            (CharacterState::Appearing, CharacterState::Idle | CharacterState::Listening) => true,
            (CharacterState::Appearing, _) => false,

            // From Idle: can listen, think, or talk for an ambient/text utterance.
            (
                CharacterState::Idle,
                CharacterState::Listening | CharacterState::Thinking | CharacterState::Talking,
            ) => true,

            // From Listening: can transition to Thinking, Talking, or Idle.
            (
                CharacterState::Listening,
                CharacterState::Thinking | CharacterState::Talking | CharacterState::Idle,
            ) => true,

            // From Thinking: can transition to Talking, Listening, or Idle.
            (
                CharacterState::Thinking,
                CharacterState::Talking | CharacterState::Listening | CharacterState::Idle,
            ) => true,

            // From Talking: can transition to Interrupted, Listening, or Idle.
            (
                CharacterState::Talking,
                CharacterState::Interrupted | CharacterState::Listening | CharacterState::Idle,
            ) => true,

            // From Interrupted: transitions directly to Listening or Idle.
            (CharacterState::Interrupted, CharacterState::Listening | CharacterState::Idle) => true,

            // From Dismissed: an explicit show can return to Idle.
            (CharacterState::Dismissed, CharacterState::Idle) => true,

            // From Error: explicit recovery can return to Idle.
            (CharacterState::Error, CharacterState::Idle) => true,

            // All other combinations are disallowed.
            _ => false,
        }
    }
}

/// Atomically validate and apply a character-state transition.
///
/// Every production state mutation should use this helper so the transition table is
/// authoritative rather than advisory.
pub fn transition_character_state(
    state: &RwLock<CharacterState>,
    target: CharacterState,
) -> Result<CharacterState, String> {
    let mut current = state.write();
    let previous = *current;
    if !previous.can_transition_to(&target) {
        return Err(format!(
            "invalid character state transition: {previous:?} -> {target:?}"
        ));
    }
    *current = target;
    Ok(previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        assert!(CharacterState::Hidden.can_transition_to(&CharacterState::Appearing));
        assert!(CharacterState::Appearing.can_transition_to(&CharacterState::Idle));
        assert!(CharacterState::Idle.can_transition_to(&CharacterState::Listening));
        assert!(CharacterState::Listening.can_transition_to(&CharacterState::Thinking));
        assert!(CharacterState::Thinking.can_transition_to(&CharacterState::Talking));
        assert!(CharacterState::Talking.can_transition_to(&CharacterState::Interrupted));
        assert!(CharacterState::Interrupted.can_transition_to(&CharacterState::Listening));
        assert!(CharacterState::Idle.can_transition_to(&CharacterState::Hidden));
        assert!(CharacterState::Thinking.can_transition_to(&CharacterState::Dismissed));
    }

    #[test]
    fn test_invalid_state_transitions() {
        assert!(!CharacterState::Hidden.can_transition_to(&CharacterState::Talking));
        assert!(!CharacterState::Hidden.can_transition_to(&CharacterState::Thinking));
        assert!(!CharacterState::Appearing.can_transition_to(&CharacterState::Thinking));
    }

    #[test]
    fn transition_helper_rejects_invalid_mutation_without_changing_state() {
        let state = RwLock::new(CharacterState::Hidden);
        let result = transition_character_state(&state, CharacterState::Talking);

        assert!(result.is_err());
        assert_eq!(*state.read(), CharacterState::Hidden);
    }

    #[test]
    fn transition_helper_applies_valid_mutation() {
        let state = RwLock::new(CharacterState::Idle);
        let previous = transition_character_state(&state, CharacterState::Listening).unwrap();

        assert_eq!(previous, CharacterState::Idle);
        assert_eq!(*state.read(), CharacterState::Listening);
    }
}
