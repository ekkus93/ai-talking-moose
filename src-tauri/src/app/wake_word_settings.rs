use serde::{Deserialize, Serialize};

pub const DEFAULT_WAKE_PHRASE: &str = "Hey, Moose";
pub const V1_WAKE_THRESHOLD: f32 = 0.25;
pub const V1_WAKE_SCORE: f32 = 1.0;

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
    pub fn validate_and_normalize(mut self) -> Result<Self, &'static str> {
        let normalized = self.phrase.trim();
        if !normalized.eq_ignore_ascii_case(DEFAULT_WAKE_PHRASE) {
            return Err("Wake Word V1 only supports the default wake phrase");
        }
        self.phrase = DEFAULT_WAKE_PHRASE.to_string();
        Ok(self)
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
}
