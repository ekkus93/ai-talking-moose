use serde::{Deserialize, Serialize};

pub use super::wake_word::policy::DEFAULT_WAKE_PHRASE;
pub const WAKE_WORD_ENABLED_FIELD: &str = "wake_word_enabled";
pub const WAKE_WORD_PHRASE_FIELD: &str = "wake_word_phrase";

/// Engine-independent persisted Wake Word V1 configuration.
///
/// V1 intentionally fixes the phrase and sherpa tuning. Keeping these values in
/// a structured object gives settings a stable expansion point without exposing
/// arbitrary keyword editing before it has acoustic acceptance coverage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct WakeWordSettings {
    pub enabled: bool,
    pub phrase: String,
}

impl Default for WakeWordSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            phrase: DEFAULT_WAKE_PHRASE.to_string(),
        }
    }
}

impl WakeWordSettings {
    /// Build Wake Word settings from the authoritative flat AppSettings fields.
    ///
    /// Missing fields migrate to the V1 defaults. Present fields must have the
    /// expected JSON type and phrase normalization is applied before the value
    /// can reach persistence or runtime state.
    pub fn from_persisted_app_settings(value: &serde_json::Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| "application settings must be a JSON object".to_string())?;
        let enabled = match object.get(WAKE_WORD_ENABLED_FIELD) {
            Some(value) => value.as_bool().ok_or_else(|| {
                format!("{WAKE_WORD_ENABLED_FIELD} must be a boolean when present")
            })?,
            None => false,
        };
        let phrase = match object.get(WAKE_WORD_PHRASE_FIELD) {
            Some(value) => value.as_str().ok_or_else(|| {
                format!("{WAKE_WORD_PHRASE_FIELD} must be a string when present")
            })?,
            None => DEFAULT_WAKE_PHRASE,
        };
        Self::from_app_settings_fields(enabled, phrase)
    }

    /// Canonical validation path shared by persisted loads and live settings writes.
    pub fn from_app_settings_fields(enabled: bool, phrase: &str) -> Result<Self, String> {
        Self {
            enabled,
            phrase: phrase.to_string(),
        }
        .validate_and_normalize()
    }

    pub fn validate_and_normalize(mut self) -> Result<Self, String> {
        self.phrase = normalize_phrase(&self.phrase)?;
        Ok(self)
    }
}

fn normalize_phrase(value: &str) -> Result<String, String> {
    let normalized = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let canonical = DEFAULT_WAKE_PHRASE.to_ascii_lowercase();
    if normalized == canonical {
        Ok(DEFAULT_WAKE_PHRASE.to_string())
    } else {
        Err("Wake Word V1 only supports the default wake phrase".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_disabled_with_fixed_phrase() {
        let settings = WakeWordSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn canonical_phrase_is_accepted() {
        let settings = WakeWordSettings {
            enabled: true,
            phrase: "Hey, Moose".to_string(),
        }
        .validate_and_normalize()
        .unwrap();
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn phrase_case_and_whitespace_are_normalized() {
        let settings = WakeWordSettings {
            enabled: true,
            phrase: "  hey,   moose  ".to_string(),
        }
        .validate_and_normalize()
        .unwrap();
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn arbitrary_phrase_fails_closed() {
        let error = WakeWordSettings {
            enabled: true,
            phrase: "Hey Bruce".to_string(),
        }
        .validate_and_normalize()
        .unwrap_err();
        assert_eq!(error, "Wake Word V1 only supports the default wake phrase");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(serde_json::from_str::<WakeWordSettings>(
            r#"{"enabled":true,"phrase":"Hey, Moose","cloud":true}"#
        )
        .is_err());
    }

    #[test]
    fn app_settings_projection_defaults_missing_wake_fields() {
        let settings = WakeWordSettings::from_persisted_app_settings(&serde_json::json!({
            "asr_mode": "moonshine_tiny_streaming",
            "tts_provider": "google"
        }))
        .unwrap();
        assert_eq!(settings, WakeWordSettings::default());
    }

    #[test]
    fn app_settings_projection_preserves_enabled_state_and_normalizes_phrase() {
        let settings = WakeWordSettings::from_persisted_app_settings(&serde_json::json!({
            "wake_word_enabled": true,
            "wake_word_phrase": "  hey, moose  "
        }))
        .unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn app_settings_projection_rejects_invalid_types_and_phrase() {
        assert!(
            WakeWordSettings::from_persisted_app_settings(&serde_json::json!({
                "wake_word_enabled": "yes"
            }))
            .is_err()
        );
        assert!(
            WakeWordSettings::from_persisted_app_settings(&serde_json::json!({
                "wake_word_phrase": "Hey Bruce"
            }))
            .is_err()
        );
    }
}
