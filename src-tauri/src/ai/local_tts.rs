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

fn local_tts_provider_error(kind: ProviderErrorKind, message: &'static str) -> ProviderError {
    let retryable = ProviderError::from_kind(kind).retryable;
    ProviderError {
        kind,
        message: message.to_string(),
        retryable,
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
    // Runtime errors are bounded static strings that intentionally exclude utterance text,
    // audio, credentials, and filesystem paths. Preserve that Local-TTS-specific guidance
    // instead of replacing it with generic conversation-provider copy.
    local_tts_provider_error(kind, error.message)
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
        validate_local_tts_model(&self.model_id).map_err(|_| {
            local_tts_provider_error(
                ProviderErrorKind::Model,
                "The selected Local TTS model is not in the supported catalog.",
            )
        })?;
        let voice_id = request
            .voice_name
            .unwrap_or_else(|| self.default_voice.clone());
        validate_local_tts_voice(&voice_id).map_err(|_| {
            local_tts_provider_error(
                ProviderErrorKind::Setup,
                "The selected Local TTS voice is unavailable.",
            )
        })?;

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

#[cfg(test)]
mod tests;
