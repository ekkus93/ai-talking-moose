use serde::{Deserialize, Serialize};

pub const DEFAULT_WAKE_PHRASE: &str = "Hey, Moose";
pub const V1_WAKE_THRESHOLD: f32 = 0.25;
pub const V1_WAKE_SCORE: f32 = 1.0;
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
    /// Both persisted-load migration and live settings updates must pass through
    /// this constructor so they share one V1 phrase validation/normalization rule.
    pub fn from_app_settings_fields(
        enabled: bool,
        phrase: impl Into<String>,
    ) -> Result<Self, &'static str> {
        Self {
            enabled,
            phrase: phrase.into(),
        }
        .validate_and_normalize()
    }

    pub fn validate_and_normalize(mut self) -> Result<Self, &'static str> {
        let normalized = self.phrase.trim();
        if !normalized.eq_ignore_ascii_case(DEFAULT_WAKE_PHRASE) {
            return Err("Wake Word V1 only supports the default wake phrase");
        }
        self.phrase = DEFAULT_WAKE_PHRASE.to_string();
        Ok(self)
    }

    /// Project the Wake Word settings from the authoritative flat AppSettings JSON shape.
    /// Missing fields intentionally fail closed to the V1 defaults. This helper keeps
    /// migration semantics independent of the sherpa engine and gives AppSettings a
    /// single normalization boundary when the fields are wired into its persisted schema.
    pub fn from_persisted_app_settings(value: &serde_json::Value) -> Result<Self, &'static str> {
        let enabled = match value.get(WAKE_WORD_ENABLED_FIELD) {
            None => false,
            Some(value) => value
                .as_bool()
                .ok_or("wake_word_enabled must be a boolean")?,
        };
        let phrase = match value.get(WAKE_WORD_PHRASE_FIELD) {
            None => DEFAULT_WAKE_PHRASE.to_string(),
            Some(value) => value
                .as_str()
                .ok_or("wake_word_phrase must be a string")?
                .to_string(),
        };
        Self::from_app_settings_fields(enabled, phrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_disabled_and_use_canonical_phrase() {
        let settings = WakeWordSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.phrase, "Hey, Moose");
        assert_eq!(V1_WAKE_THRESHOLD, 0.25);
        assert_eq!(V1_WAKE_SCORE, 1.0);
    }

    #[test]
    fn missing_fields_default_fail_closed() {
        let settings: WakeWordSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, WakeWordSettings::default());
    }

    #[test]
    fn enabled_state_round_trips() {
        let settings = WakeWordSettings {
            enabled: true,
            ..Default::default()
        };
        let decoded: WakeWordSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(decoded.enabled);
        assert_eq!(decoded.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn field_constructor_is_the_canonical_live_and_persisted_boundary() {
        let settings =
            WakeWordSettings::from_app_settings_fields(true, "  hey, moose  ").unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
        assert!(WakeWordSettings::from_app_settings_fields(true, "Hey Bruce").is_err());
    }

    #[test]
    fn canonical_phrase_normalizes_case_and_whitespace() {
        let settings = WakeWordSettings {
            enabled: true,
            phrase: "  hey, moose  ".to_string(),
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
