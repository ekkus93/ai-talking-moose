use crate::app::state::AppSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub name: String,
    pub species: String,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            name: "Moose".to_string(),
            species: "moose".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalitySliders {
    pub dry: f32,
    pub sarcastic: f32,
    pub friendly: f32,
    pub absurd: f32,
    pub helpful: f32,
    pub verbosity: f32,
    pub talkativeness: f32,
}

impl Default for PersonalitySliders {
    fn default() -> Self {
        let settings = AppSettings::default();
        Self {
            dry: settings.dry,
            sarcastic: settings.sarcastic,
            friendly: settings.friendly,
            absurd: settings.absurd,
            helpful: settings.helpful,
            verbosity: settings.verbosity,
            talkativeness: settings.talkativeness,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechRules {
    pub max_sentences_ambient: usize,
    pub max_sentences_conversation_default: usize,
    pub avoid_assistant_language: bool,
    pub allow_self_deprecating_humor: bool,
}

impl Default for SpeechRules {
    fn default() -> Self {
        Self {
            max_sentences_ambient: 2,
            max_sentences_conversation_default: 3,
            avoid_assistant_language: true,
            allow_self_deprecating_humor: true,
        }
    }
}

/// Fixed V1 anti-annoyance floor between ambient remarks. This is deliberately
/// not a user preference: talkativeness and the hourly budget are configurable,
/// while this local cooldown remains a safety guardrail.
pub const V1_MIN_AMBIENT_COOLDOWN_SECONDS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub unsolicited_comments: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: u8, // e.g. 22 for 10 PM
    pub quiet_hours_end: u8,   // e.g. 8 for 8 AM
    pub max_comments_per_hour: u32,
    pub min_cooldown_seconds: u64,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        let settings = AppSettings::default();
        Self {
            unsolicited_comments: settings.unsolicited_comments,
            quiet_hours_enabled: settings.quiet_hours_enabled,
            quiet_hours_start: settings.quiet_hours_start,
            quiet_hours_end: settings.quiet_hours_end,
            max_comments_per_hour: settings.max_comments_per_hour,
            min_cooldown_seconds: V1_MIN_AMBIENT_COOLDOWN_SECONDS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterConfig {
    pub identity: IdentityConfig,
    pub personality: PersonalitySliders,
    pub speech: SpeechRules,
    pub behavior: BehaviorConfig,
    pub rules: Vec<String>,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            personality: PersonalitySliders::default(),
            speech: SpeechRules::default(),
            behavior: BehaviorConfig::default(),
            rules: vec![
                "You are an opinionated, dry-witted retro cartoon moose living inside the user's computer."
                    .to_string(),
                "Never say generic assistant phrases like 'How can I assist you today?' or 'Is there anything else I can help with?'."
                    .to_string(),
                "Prefer short, funny, deadpan observations over unsolicited offers of help."
                    .to_string(),
                "Speak in a warm, slightly dim-witted yet witty cartoon cadence. Never sound like customer support."
                    .to_string(),
                "Respect the user if they specifically ask you to be serious or quiet."
                    .to_string(),
            ],
        }
    }
}

impl CharacterConfig {
    pub fn from_yaml_str(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    pub fn to_yaml_str(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_config_json_round_trip_preserves_authoritative_defaults() {
        let expected = CharacterConfig::default();
        let json = serde_json::to_string(&expected).unwrap();
        let restored: CharacterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, expected);
    }

    #[test]
    fn character_defaults_follow_persisted_settings_defaults() {
        let settings = AppSettings::default();
        let character = CharacterConfig::default();

        assert_eq!(character.personality.dry, settings.dry);
        assert_eq!(character.personality.sarcastic, settings.sarcastic);
        assert_eq!(character.personality.friendly, settings.friendly);
        assert_eq!(character.personality.absurd, settings.absurd);
        assert_eq!(character.personality.helpful, settings.helpful);
        assert_eq!(character.personality.verbosity, settings.verbosity);
        assert_eq!(character.personality.talkativeness, settings.talkativeness);
        assert_eq!(
            character.behavior.unsolicited_comments,
            settings.unsolicited_comments
        );
        assert_eq!(
            character.behavior.quiet_hours_enabled,
            settings.quiet_hours_enabled
        );
        assert_eq!(
            character.behavior.quiet_hours_start,
            settings.quiet_hours_start
        );
        assert_eq!(character.behavior.quiet_hours_end, settings.quiet_hours_end);
        assert_eq!(
            character.behavior.max_comments_per_hour,
            settings.max_comments_per_hour
        );
    }
}
