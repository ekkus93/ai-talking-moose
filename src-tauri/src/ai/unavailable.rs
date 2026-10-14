//! Explicit fail-closed adapters for provider choices whose runtime is not yet built.
//! Never substitute another model or voice when a selected provider is unavailable.
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind, TtsRequest};
use async_trait::async_trait;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_system_voice_does_not_fallback() {
        let error = UnavailableSpeechSynthesizer
            .synthesize(TtsRequest {
                text: "hello".into(),
                voice_name: None,
                speaking_rate: None,
                pitch: None,
            })
            .await
            .unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Setup);
        assert!(error.message.contains("no other voice provider"));
        assert!(!error.retryable);
        assert!(!error.message.contains("hello"));
    }
}
