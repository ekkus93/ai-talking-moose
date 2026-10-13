//! Explicit fail-closed adapters for provider choices whose runtime is not yet built.
//! Never substitute another model or voice when a selected provider is unavailable.
use crate::ai::traits::{SpeechSynthesizer, TextModel};
use crate::ai::types::{
    AudioStreamData, ProviderError, ProviderErrorKind, TextRequest, TextResponse, TtsRequest,
};
use async_trait::async_trait;

pub(crate) struct UnavailableTextModel;

#[async_trait]
impl TextModel for UnavailableTextModel {
    async fn generate(&self, _request: TextRequest) -> Result<TextResponse, ProviderError> {
        Err(ProviderError {
            kind: ProviderErrorKind::Setup,
            message: "The selected external text provider is not available in this build; no other provider was contacted.".into(),
            retryable: false,
        })
    }
}

pub(crate) struct UnavailableSpeechSynthesizer;

#[async_trait]
impl SpeechSynthesizer for UnavailableSpeechSynthesizer {
    async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        Err(ProviderError {
            kind: ProviderErrorKind::Setup,
            message: "System voice synthesis is not available in this build; no other voice provider was contacted.".into(),
            retryable: false,
        })
    }
}
