//! Platform-neutral, fail-closed selection for macOS runtime voice identifiers.
use crate::ai::system_tts_voice_catalog::has_invisible_id_formatting;
use crate::ai::tts_catalog::TtsVoiceDescriptor;
use crate::ai::types::{ProviderError, ProviderErrorKind};

pub const SYSTEM_DEFAULT_VOICE_ID: &str = "system_default";

/// An absent saved voice is not silently replaced with another installed voice.
pub fn resolve_installed_system_voice<'a>(
    saved_id: &str,
    installed: &'a [TtsVoiceDescriptor],
) -> Result<Option<&'a TtsVoiceDescriptor>, ProviderError> {
    if saved_id == SYSTEM_DEFAULT_VOICE_ID {
        return Ok(None);
    }
    if saved_id.is_empty()
        || saved_id.len() > 128
        || saved_id.trim() != saved_id
        || saved_id.chars().any(char::is_control)
        || has_invisible_id_formatting(saved_id)
    {
        return Err(ProviderError::from_kind(ProviderErrorKind::Model));
    }
    installed
        .iter()
        .find(|voice| voice.id == saved_id)
        .map(Some)
        .ok_or_else(|| ProviderError::from_kind(ProviderErrorKind::Model))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_explicit_and_absent_saved_voice_never_falls_back() {
        let installed = vec![TtsVoiceDescriptor {
            id: "com.apple.voice.test".to_string(),
            display_name: "Test voice".to_string(),
            style: Some("en-US".to_string()),
        }];
        assert!(
            resolve_installed_system_voice(SYSTEM_DEFAULT_VOICE_ID, &installed)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            resolve_installed_system_voice("com.apple.voice.test", &installed)
                .unwrap()
                .unwrap()
                .display_name,
            "Test voice"
        );
        let missing =
            resolve_installed_system_voice("com.apple.voice.missing", &installed).unwrap_err();
        assert_eq!(missing.kind, ProviderErrorKind::Model);
        assert!(!missing.retryable);
        assert!(!missing.message.contains("com.apple.voice.missing"));
        assert!(resolve_installed_system_voice("com.apple.voice.test\n", &installed).is_err());
        assert!(resolve_installed_system_voice("", &installed).is_err());
        assert!(resolve_installed_system_voice(&"x".repeat(129), &installed).is_err());
        assert!(resolve_installed_system_voice("invalid\tvoice", &installed).is_err());
        assert!(resolve_installed_system_voice(" padded.voice", &installed).is_err());
        assert!(resolve_installed_system_voice("padded.voice ", &installed).is_err());
        assert!(resolve_installed_system_voice("com.apple.voice.test\r", &installed).is_err());
        assert!(
            resolve_installed_system_voice("com.apple.voice.\u{202e}spoof", &installed).is_err()
        );
        assert!(
            resolve_installed_system_voice("com.apple.voice.\u{200b}spoof", &installed).is_err()
        );
        assert!(
            resolve_installed_system_voice("com.apple.voice.\u{feff}spoof", &installed).is_err()
        );
    }
}
