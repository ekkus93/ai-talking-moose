use crate::ai::google::auth::{trace_google_provider_failure, GoogleAuth, GOOGLE_API_KEY_HEADER};
use crate::ai::google::config::{
    normalize_tts_model, normalize_tts_voice, validate_tts_model, validate_tts_voice,
};
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind, TtsRequest};
use async_trait::async_trait;
use base64::Engine;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::json;
use std::time::Duration;
use tracing::info;

const TTS_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub struct GoogleSpeechSynthesizer {
    auth: GoogleAuth,
    model: String,
    default_voice: String,
    client: Client,
}

impl GoogleSpeechSynthesizer {
    pub fn new(auth: GoogleAuth, model: String, default_voice: String) -> Self {
        Self {
            auth,
            model: normalize_tts_model(&model).to_string(),
            default_voice: normalize_tts_voice(&default_voice).to_string(),
            client: Client::new(),
        }
    }

    fn safe_error(kind: ProviderErrorKind) -> ProviderError {
        ProviderError::from_kind(kind)
    }

    fn classify_status(status: StatusCode) -> ProviderError {
        match status.as_u16() {
            401 | 403 => Self::safe_error(ProviderErrorKind::Auth),
            429 => Self::safe_error(ProviderErrorKind::Quota),
            404 => Self::safe_error(ProviderErrorKind::Model),
            400..=499 => Self::safe_error(ProviderErrorKind::Setup),
            _ => Self::safe_error(ProviderErrorKind::Network),
        }
    }

    fn generation_url(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        )
    }

    fn generation_request(&self, body: &serde_json::Value) -> RequestBuilder {
        self.client
            .post(self.generation_url())
            .header(GOOGLE_API_KEY_HEADER, &self.auth.api_key)
            .json(body)
    }

    fn performance_instruction(request: &TtsRequest) -> String {
        let rate = request.speaking_rate.unwrap_or(1.0);
        let pitch = request.pitch.unwrap_or(0.0);
        let pace = if rate < 0.75 {
            "very slowly"
        } else if rate < 0.95 {
            "a little slowly"
        } else if rate > 1.35 {
            "quickly"
        } else if rate > 1.05 {
            "a little briskly"
        } else {
            "at a natural, measured pace"
        };
        let register = if pitch <= -6.0 {
            "a noticeably lower vocal register"
        } else if pitch < -0.5 {
            "a slightly lower vocal register"
        } else if pitch >= 6.0 {
            "a noticeably higher vocal register"
        } else if pitch > 0.5 {
            "a slightly higher vocal register"
        } else {
            "your natural vocal register"
        };

        format!(
            "Perform this exact line as an original dry-witted cartoon moose. Speak {pace}, using {register}. Do not add, remove, or paraphrase words. Line: {}",
            request.text
        )
    }
}

#[async_trait]
impl SpeechSynthesizer for GoogleSpeechSynthesizer {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        if !self.auth.is_valid() {
            return Err(Self::safe_error(ProviderErrorKind::Auth));
        }
        validate_tts_model(&self.model).map_err(|_| Self::safe_error(ProviderErrorKind::Model))?;

        let voice = request
            .voice_name
            .clone()
            .unwrap_or_else(|| self.default_voice.clone());
        validate_tts_voice(&voice).map_err(|_| Self::safe_error(ProviderErrorKind::Setup))?;

        #[cfg(test)]
        if crate::test_support::network_denied() {
            return Err(Self::safe_error(ProviderErrorKind::Network));
        }

        let body = json!({
            "contents": [
                {
                    "parts": [
                        { "text": Self::performance_instruction(&request) }
                    ]
                }
            ],
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "speechConfig": {
                    "voiceConfig": {
                        "prebuiltVoiceConfig": {
                            "voiceName": voice
                        }
                    }
                }
            }
        });

        let response = self
            .generation_request(&body)
            .timeout(TTS_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|_| {
                let error = Self::safe_error(ProviderErrorKind::Network);
                trace_google_provider_failure("tts", &error);
                error
            })?;
        if !response.status().is_success() {
            let error = Self::classify_status(response.status());
            trace_google_provider_failure("tts", &error);
            return Err(error);
        }

        let payload = response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| Self::safe_error(ProviderErrorKind::Protocol))?;
        let parts = payload["candidates"][0]["content"]["parts"]
            .as_array()
            .ok_or_else(|| Self::safe_error(ProviderErrorKind::Protocol))?;
        for part in parts {
            if let Some(encoded) = part["inlineData"]["data"].as_str() {
                let pcm_bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|_| Self::safe_error(ProviderErrorKind::Protocol))?;
                info!(
                    byte_count = pcm_bytes.len(),
                    "Gemini generated standalone TTS audio"
                );
                return Ok(AudioStreamData {
                    pcm_bytes,
                    sample_rate: 24_000,
                });
            }
        }

        Err(Self::safe_error(ProviderErrorKind::Protocol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn network_denial_harness_blocks_tts_before_http_send() {
        let _guard = crate::test_support::deny_network_for_scope();
        let synthesizer = GoogleSpeechSynthesizer::new(
            GoogleAuth::new("valid-test-key".to_string()),
            crate::ai::google::config::DEFAULT_TTS_MODEL.to_string(),
            "Fenrir".to_string(),
        );
        let error = synthesizer
            .synthesize(TtsRequest {
                text: "network must be denied".to_string(),
                voice_name: Some("Fenrir".to_string()),
                speaking_rate: Some(1.0),
                pitch: Some(0.0),
            })
            .await
            .expect_err("test network denial must stop TTS before HTTP I/O");
        assert_eq!(error.kind, ProviderErrorKind::Network);
    }

    #[test]
    fn performance_prompt_maps_rate_and_pitch_without_changing_text() {
        let request = TtsRequest {
            text: "Oh good. Another dialog box.".to_string(),
            voice_name: None,
            speaking_rate: Some(0.8),
            pitch: Some(-2.0),
        };
        let prompt = GoogleSpeechSynthesizer::performance_instruction(&request);
        assert!(prompt.contains("a little slowly"));
        assert!(prompt.contains("slightly lower vocal register"));
        assert!(prompt.contains(&request.text));
    }

    #[test]
    fn constructor_normalizes_legacy_model_and_unknown_voice() {
        let synthesizer = GoogleSpeechSynthesizer::new(
            GoogleAuth::new("test-key".to_string()),
            "en-US-Standard-B".to_string(),
            "not-a-real-voice".to_string(),
        );

        assert_eq!(
            synthesizer.model,
            crate::ai::google::config::DEFAULT_TTS_MODEL
        );
        assert_eq!(
            synthesizer.default_voice,
            crate::ai::google::config::DEFAULT_TTS_VOICE
        );
    }

    #[test]
    fn public_constructor_uses_selected_tts_model() {
        let synthesizer = GoogleSpeechSynthesizer::new(
            GoogleAuth::new("test-key".to_string()),
            crate::ai::google::config::DEFAULT_TTS_MODEL.to_string(),
            "Puck".to_string(),
        );
        assert!(synthesizer.generation_url().contains(&format!(
            "/models/{}:generateContent",
            crate::ai::google::config::DEFAULT_TTS_MODEL
        )));
    }

    #[test]
    fn generation_request_uses_api_key_header_and_secret_free_url() {
        const KEY: &str = "AIzaSyTTS_HEADER_ONLY_248a";
        let synthesizer = GoogleSpeechSynthesizer::new(
            GoogleAuth::new(KEY.to_string()),
            crate::ai::google::config::DEFAULT_TTS_MODEL.to_string(),
            "Puck".to_string(),
        );
        let request = synthesizer
            .generation_request(&json!({"contents": []}))
            .build()
            .unwrap();

        assert!(request.url().query().is_none());
        assert!(!request.url().as_str().contains(KEY));
        assert_eq!(
            request
                .headers()
                .get(GOOGLE_API_KEY_HEADER)
                .unwrap()
                .to_str()
                .unwrap(),
            KEY
        );
    }

    #[test]
    fn http_errors_map_to_structured_safe_provider_categories() {
        assert_eq!(
            GoogleSpeechSynthesizer::classify_status(StatusCode::UNAUTHORIZED).kind,
            ProviderErrorKind::Auth
        );
        assert_eq!(
            GoogleSpeechSynthesizer::classify_status(StatusCode::TOO_MANY_REQUESTS).kind,
            ProviderErrorKind::Quota
        );
        assert_eq!(
            GoogleSpeechSynthesizer::classify_status(StatusCode::NOT_FOUND).kind,
            ProviderErrorKind::Model
        );
        assert_eq!(
            GoogleSpeechSynthesizer::classify_status(StatusCode::BAD_GATEWAY).kind,
            ProviderErrorKind::Network
        );
    }
}
