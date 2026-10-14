//! Fail-closed validation shared by future macOS system speech adapters.
use crate::ai::system_tts_selection::resolve_installed_system_voice;
use crate::ai::tts_catalog::TtsVoiceDescriptor;
use crate::ai::types::{ProviderError, ProviderErrorKind, TtsRequest};

pub fn validate_system_tts_request(
    request: &TtsRequest,
    installed: &[TtsVoiceDescriptor],
) -> Result<(), ProviderError> {
    if request.text.trim().is_empty() || request.text.chars().count() > 4_000 {
        return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
    }
    if let Some(rate) = request.speaking_rate {
        if !rate.is_finite() || !(0.25..=4.0).contains(&rate) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Setup));
        }
    }
    let voice = request.voice_name.as_deref().unwrap_or("system_default");
    resolve_installed_system_voice(voice, installed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_voice_and_invalid_input_without_fallback() {
        let mut request = TtsRequest {
            text: "hello".into(),
            voice_name: Some("missing.voice".into()),
            speaking_rate: Some(1.0),
            pitch: None,
        };
        assert_eq!(
            validate_system_tts_request(&request, &[]).unwrap_err().kind,
            ProviderErrorKind::Model
        );
        request.voice_name = None;
        assert!(validate_system_tts_request(&request, &[]).is_ok());
        request.speaking_rate = Some(f32::NAN);
        assert!(validate_system_tts_request(&request, &[]).is_err());
        request.speaking_rate = Some(1.0);
        request.text = " ".into();
        assert!(validate_system_tts_request(&request, &[]).is_err());
    }
}
