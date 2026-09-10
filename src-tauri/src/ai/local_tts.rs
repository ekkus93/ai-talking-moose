pub mod manifest;
pub mod storage;

use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind, TtsRequest};
use async_trait::async_trait;

pub const DEFAULT_LOCAL_TTS_MODEL_ID: &str = "KittenML/kitten-tts-mini-0.8";
pub const DEFAULT_LOCAL_TTS_VOICE: &str = "Bella";

pub const LOCAL_TTS_MODEL_IDS: &[&str] = &[DEFAULT_LOCAL_TTS_MODEL_ID];
pub const LOCAL_TTS_VOICE_IDS: &[&str] = &[
    "Bella", "Jasper", "Luna", "Bruno", "Rosie", "Hugo", "Kiki", "Leo",
];

pub fn validate_local_tts_model(model_id: &str) -> Result<(), String> {
    if LOCAL_TTS_MODEL_IDS.contains(&model_id) {
        Ok(())
    } else {
        Err("unsupported local TTS model".to_string())
    }
}

pub fn validate_local_tts_voice(voice_id: &str) -> Result<(), String> {
    if LOCAL_TTS_VOICE_IDS.contains(&voice_id) {
        Ok(())
    } else {
        Err("unsupported local TTS voice".to_string())
    }
}

/// Fail-closed bridge used only until KTT-400 installs the real KittenTTS synthesizer.
///
/// Persisting `tts_provider = local` must never cause a silent fallback to Google. This
/// provider therefore returns a local setup error without inspecting or transmitting text.
pub struct PendingLocalSpeechSynthesizer;

#[async_trait]
impl SpeechSynthesizer for PendingLocalSpeechSynthesizer {
    async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        Err(ProviderError {
            kind: ProviderErrorKind::Setup,
            message: "Local TTS runtime is not available in this build yet.".to_string(),
            retryable: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_tts_defaults_are_valid_catalog_values() {
        assert!(validate_local_tts_model(DEFAULT_LOCAL_TTS_MODEL_ID).is_ok());
        assert!(validate_local_tts_voice(DEFAULT_LOCAL_TTS_VOICE).is_ok());
    }

    #[test]
    fn local_tts_catalog_rejects_unknown_values() {
        assert!(validate_local_tts_model("latest").is_err());
        assert!(validate_local_tts_voice("Fenrir").is_err());
    }

    #[tokio::test]
    async fn pending_local_runtime_fails_closed_without_manufacturing_audio() {
        let error = PendingLocalSpeechSynthesizer
            .synthesize(TtsRequest {
                text: "this text must stay local".to_string(),
                voice_name: Some(DEFAULT_LOCAL_TTS_VOICE.to_string()),
                speaking_rate: Some(1.0),
                pitch: None,
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Setup);
        assert!(!error.retryable);
        assert!(!error.message.contains("this text must stay local"));
    }
}
