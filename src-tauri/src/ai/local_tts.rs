pub mod audio;
pub mod installer;
pub mod manifest;
pub mod runtime;
pub mod runtime_verification;
pub mod storage;

use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind, TtsRequest};
use async_trait::async_trait;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub use audio::inference_output_to_audio_stream_data;
pub use runtime::{LocalTtsRuntimeManager, LocalTtsRuntimeStatus};

use runtime::{LocalTtsInferenceRequest, LocalTtsRuntimeError, LocalTtsRuntimeErrorKind};

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

fn provider_error_for_runtime(error: LocalTtsRuntimeError) -> ProviderError {
    let kind = match error.kind {
        LocalTtsRuntimeErrorKind::Cancelled => ProviderErrorKind::Cancelled,
        LocalTtsRuntimeErrorKind::ShuttingDown => ProviderErrorKind::Closed,
        LocalTtsRuntimeErrorKind::UnknownModel => ProviderErrorKind::Model,
        LocalTtsRuntimeErrorKind::ModelNotInstalled
        | LocalTtsRuntimeErrorKind::UnsupportedPlatform
        | LocalTtsRuntimeErrorKind::RuntimeUnavailable
        | LocalTtsRuntimeErrorKind::InvalidInput
        | LocalTtsRuntimeErrorKind::InvalidVoice
        | LocalTtsRuntimeErrorKind::UnsupportedConfig => ProviderErrorKind::Setup,
        LocalTtsRuntimeErrorKind::Verification | LocalTtsRuntimeErrorKind::ModelLoad => {
            ProviderErrorKind::Model
        }
        LocalTtsRuntimeErrorKind::Inference | LocalTtsRuntimeErrorKind::ModelDelete => {
            ProviderErrorKind::Internal
        }
    };
    ProviderError::from_kind(kind)
}

/// Production standalone Local TTS provider backed by the one shared `LocalTtsRuntimeManager`.
///
/// The provider owns only the selected model/voice snapshot. Runtime lifetime, verification,
/// serialization, warm reuse, cancellation, and model unload remain authoritative in the shared
/// manager owned by `AppState`.
pub struct LocalSpeechSynthesizer {
    runtime: Arc<LocalTtsRuntimeManager>,
    model_id: String,
    default_voice: String,
}

impl LocalSpeechSynthesizer {
    pub fn new(
        runtime: Arc<LocalTtsRuntimeManager>,
        model_id: String,
        default_voice: String,
    ) -> Self {
        Self {
            runtime,
            model_id,
            default_voice,
        }
    }

    fn inference_request(
        &self,
        request: TtsRequest,
    ) -> Result<LocalTtsInferenceRequest, ProviderError> {
        validate_local_tts_model(&self.model_id)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Model))?;
        let voice_id = request
            .voice_name
            .unwrap_or_else(|| self.default_voice.clone());
        validate_local_tts_voice(&voice_id)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Setup))?;

        Ok(LocalTtsInferenceRequest {
            text: request.text,
            voice_id,
            speaking_rate: request.speaking_rate.unwrap_or(1.0),
            pitch: request.pitch,
        })
    }
}

#[async_trait]
impl SpeechSynthesizer for LocalSpeechSynthesizer {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        let cancellation = CancellationToken::new();
        self.synthesize_cancellable(request, &cancellation).await
    }

    async fn synthesize_cancellable(
        &self,
        request: TtsRequest,
        cancellation: &CancellationToken,
    ) -> Result<AudioStreamData, ProviderError> {
        if cancellation.is_cancelled() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Cancelled));
        }
        let inference_request = self.inference_request(request)?;
        let output = self
            .runtime
            .synthesize_f32_cancellable(&self.model_id, inference_request, cancellation)
            .await
            .map_err(provider_error_for_runtime)?;
        inference_output_to_audio_stream_data(output)
    }
}

/// Fail-closed compatibility bridge retained only until every legacy direct caller has migrated to
/// `LocalSpeechSynthesizer`. Production standalone routing no longer uses this placeholder.
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

    async fn synthesize_cancellable(
        &self,
        request: TtsRequest,
        cancellation: &CancellationToken,
    ) -> Result<AudioStreamData, ProviderError> {
        if cancellation.is_cancelled() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Cancelled));
        }
        self.synthesize(request).await
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
    async fn production_local_provider_rejects_unknown_model_before_runtime_use() {
        let synthesizer = LocalSpeechSynthesizer::new(
            Arc::new(LocalTtsRuntimeManager::new()),
            "missing-local-model".to_string(),
            DEFAULT_LOCAL_TTS_VOICE.to_string(),
        );
        let error = synthesizer
            .synthesize(TtsRequest {
                text: "private local text".to_string(),
                voice_name: None,
                speaking_rate: Some(1.0),
                pitch: None,
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Model);
        assert!(!error.message.contains("private local text"));
    }

    #[tokio::test]
    async fn production_local_provider_rejects_non_kitten_voice_before_runtime_use() {
        let synthesizer = LocalSpeechSynthesizer::new(
            Arc::new(LocalTtsRuntimeManager::new()),
            DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),
            DEFAULT_LOCAL_TTS_VOICE.to_string(),
        );
        let error = synthesizer
            .synthesize(TtsRequest {
                text: "private local text".to_string(),
                voice_name: Some("Puck".to_string()),
                speaking_rate: Some(1.0),
                pitch: None,
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Setup);
        assert!(!error.message.contains("private local text"));
    }

    #[tokio::test]
    async fn production_local_provider_reports_provider_neutral_cancellation_before_runtime_use() {
        let synthesizer = LocalSpeechSynthesizer::new(
            Arc::new(LocalTtsRuntimeManager::new()),
            DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),
            DEFAULT_LOCAL_TTS_VOICE.to_string(),
        );
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let error = synthesizer
            .synthesize_cancellable(
                TtsRequest {
                    text: "cancelled local text".to_string(),
                    voice_name: None,
                    speaking_rate: Some(1.0),
                    pitch: None,
                },
                &cancellation,
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Cancelled);
        assert!(!error.retryable);
        assert!(!error.message.contains("cancelled local text"));
    }

    #[tokio::test]
    async fn pending_local_runtime_reports_provider_neutral_cancellation() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let error = PendingLocalSpeechSynthesizer
            .synthesize_cancellable(
                TtsRequest {
                    text: "cancelled local text".to_string(),
                    voice_name: Some(DEFAULT_LOCAL_TTS_VOICE.to_string()),
                    speaking_rate: Some(1.0),
                    pitch: None,
                },
                &cancellation,
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Cancelled);
        assert!(!error.retryable);
        assert!(!error.message.contains("cancelled local text"));
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
