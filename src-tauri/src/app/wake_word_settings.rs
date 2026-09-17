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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeWordSettingsError {
    InvalidPhrase,
}

impl std::fmt::Display for WakeWordSettingsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPhrase => formatter.write_str("Wake Word V1 only supports 'Hey, Moose'"),
        }
    }
}

impl std::error::Error for WakeWordSettingsError {}

impl WakeWordSettings {
    /// Canonical settings boundary for both persisted-load and live updates.
    pub fn from_app_settings_fields(
        enabled: bool,
        phrase: &str,
    ) -> Result<Self, WakeWordSettingsError> {
        let mut settings = Self {
            enabled,
            phrase: phrase.to_string(),
        };
        settings.validate_and_normalize()?;
        Ok(settings)
    }

    pub fn validate_and_normalize(&mut self) -> Result<(), WakeWordSettingsError> {
        let normalized = normalize_phrase(&self.phrase);
        if normalized != "hey, moose" {
            return Err(WakeWordSettingsError::InvalidPhrase);
        }
        self.phrase = DEFAULT_WAKE_PHRASE.to_string();
        Ok(())
    }
}

fn normalize_phrase(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_disabled_and_fixed_phrase() {
        let settings = WakeWordSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.phrase, "Hey, Moose");
    }

    #[test]
    fn canonical_phrase_is_accepted() {
        let settings = WakeWordSettings::from_app_settings_fields(true, "Hey, Moose").unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn whitespace_and_case_are_normalized() {
        let settings = WakeWordSettings::from_app_settings_fields(true, "  hey,   MOOSE  ").unwrap();
        assert_eq!(settings.phrase, DEFAULT_WAKE_PHRASE);
    }

    #[test]
    fn arbitrary_phrase_is_rejected() {
        let error = WakeWordSettings::from_app_settings_fields(true, "Hey Bruce").unwrap_err();
        assert_eq!(error, WakeWordSettingsError::InvalidPhrase);
    }
}
