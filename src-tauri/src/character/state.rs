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
    Annoyed,
    Sleeping,
    Dismissed,
    Muted,
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
    /// Check if transition from `self` to `target` is valid according to state machine rules
    pub fn can_transition_to(&self, target: &CharacterState) -> bool {
        match (self, target) {
            // Self transitions are always permitted (e.g. refresh/reaffirm)
            (a, b) if a == b => true,

            // Error or Hidden can be reached from almost anywhere
            (_, CharacterState::Error) => true,
            (_, CharacterState::Hidden) => true,
            (_, CharacterState::Muted) => true,

            // From Hidden: can only appear or error
            (CharacterState::Hidden, CharacterState::Appearing) => true,
            (CharacterState::Hidden, CharacterState::Idle) => true,
            (CharacterState::Hidden, _) => false,

            // From Appearing: transitions into Idle or Listening
            (CharacterState::Appearing, CharacterState::Idle) => true,
            (CharacterState::Appearing, CharacterState::Listening) => true,
            (CharacterState::Appearing, _) => false,

            // From Idle: can listen, think, talk (ambient), sleep, get annoyed, dismiss
            (CharacterState::Idle, CharacterState::Listening) => true,
            (CharacterState::Idle, CharacterState::Thinking) => true,
            (CharacterState::Idle, CharacterState::Talking) => true,
            (CharacterState::Idle, CharacterState::Sleeping) => true,
            (CharacterState::Idle, CharacterState::Annoyed) => true,
            (CharacterState::Idle, CharacterState::Dismissed) => true,

            // From Listening: can transition to Thinking, Talking, Idle, Annoyed
            (CharacterState::Listening, CharacterState::Thinking) => true,
            (CharacterState::Listening, CharacterState::Talking) => true,
            (CharacterState::Listening, CharacterState::Idle) => true,
            (CharacterState::Listening, CharacterState::Annoyed) => true,
            (CharacterState::Listening, CharacterState::Dismissed) => true,

            // From Thinking: can transition to Talking, Listening, Idle, Annoyed
            (CharacterState::Thinking, CharacterState::Talking) => true,
            (CharacterState::Thinking, CharacterState::Listening) => true,
            (CharacterState::Thinking, CharacterState::Idle) => true,
            (CharacterState::Thinking, CharacterState::Annoyed) => true,

            // From Talking: can transition to Interrupted, Listening, Idle, Dismissed
            (CharacterState::Talking, CharacterState::Interrupted) => true,
            (CharacterState::Talking, CharacterState::Listening) => true,
            (CharacterState::Talking, CharacterState::Idle) => true,
            (CharacterState::Talking, CharacterState::Dismissed) => true,

            // From Interrupted: transitions directly to Listening or Annoyed then Idle
            (CharacterState::Interrupted, CharacterState::Listening) => true,
            (CharacterState::Interrupted, CharacterState::Annoyed) => true,
            (CharacterState::Interrupted, CharacterState::Idle) => true,

            // From Annoyed / Sleeping / Dismissed: can return to Idle
            (CharacterState::Annoyed, CharacterState::Idle) => true,
            (CharacterState::Annoyed, CharacterState::Listening) => true,
            (CharacterState::Sleeping, CharacterState::Idle) => true,
            (CharacterState::Sleeping, CharacterState::Listening) => true,
            (CharacterState::Dismissed, CharacterState::Idle) => true,

            // From Muted: can return to Idle
            (CharacterState::Muted, CharacterState::Idle) => true,

            // From Error: can return to Idle
            (CharacterState::Error, CharacterState::Idle) => true,

            // All other combinations are disallowed
            _ => false,
        }
    }
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
    }

    #[test]
    fn test_invalid_state_transitions() {
        assert!(!CharacterState::Hidden.can_transition_to(&CharacterState::Talking));
        assert!(!CharacterState::Hidden.can_transition_to(&CharacterState::Thinking));
        assert!(!CharacterState::Appearing.can_transition_to(&CharacterState::Sleeping));
    }
}
