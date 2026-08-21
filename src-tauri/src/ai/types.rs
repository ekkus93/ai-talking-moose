use crate::tools::policy::ToolDeclaration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRequest {
    pub prompt: String,
    pub system_instruction: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResponse {
    pub text: String,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice_name: Option<String>,
    pub speaking_rate: Option<f32>,
    pub pitch: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamData {
    pub pcm_bytes: Vec<u8>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorKind {
    Auth,
    Quota,
    Network,
    Protocol,
    Setup,
    Model,
    Closed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl ProviderError {
    pub fn from_kind(kind: ProviderErrorKind) -> Self {
        let (message, retryable) = match kind {
            ProviderErrorKind::Auth => (
                "Provider authentication failed. Check the configured API credential.",
                false,
            ),
            ProviderErrorKind::Quota => (
                "Provider quota or rate limit was reached. Try again later or review your quota.",
                true,
            ),
            ProviderErrorKind::Network => (
                "The conversation service could not be reached. Check your network connection and try again.",
                true,
            ),
            ProviderErrorKind::Protocol => (
                "The conversation service returned an unexpected response. Try again.",
                false,
            ),
            ProviderErrorKind::Setup => (
                "The conversation session could not be configured. Check the selected model and voice.",
                false,
            ),
            ProviderErrorKind::Model => (
                "The selected conversation model is unavailable or unsupported.",
                false,
            ),
            ProviderErrorKind::Closed => (
                "The conversation service closed the session. Try starting a new conversation.",
                true,
            ),
            ProviderErrorKind::Internal => (
                "The conversation session failed unexpectedly. Try again.",
                false,
            ),
        };
        Self {
            kind,
            message: message.to_string(),
            retryable,
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ProviderError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSessionConfig {
    pub model: String,
    pub voice_name: Option<String>,
    pub system_instruction: Option<String>,
    pub sample_rate_in: u32,
    pub sample_rate_out: u32,
    #[serde(default)]
    pub tools: Vec<ToolDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptUpdate {
    pub text: String,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum LiveServerEvent {
    Connected,
    UserTranscript(TranscriptUpdate),
    ModelTranscript(TranscriptUpdate),
    AudioData(Vec<u8>), // PCM bytes (usually 24kHz)
    Interrupted,
    TurnComplete,
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    Error(ProviderError),
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub id: String,
    pub name: String,
    pub output: serde_json::Value,
}

#[cfg(test)]
mod provider_error_tests {
    use super::*;

    #[test]
    fn provider_errors_have_stable_safe_messages_and_retry_policy() {
        let auth = ProviderError::from_kind(ProviderErrorKind::Auth);
        assert_eq!(auth.kind, ProviderErrorKind::Auth);
        assert!(!auth.retryable);
        assert!(auth.message.contains("credential"));

        let network = ProviderError::from_kind(ProviderErrorKind::Network);
        assert!(network.retryable);
        assert!(network.message.contains("network"));
    }

    #[test]
    fn provider_error_serialization_exposes_only_structured_safe_fields() {
        let error = ProviderError::from_kind(ProviderErrorKind::Quota);
        let json = serde_json::to_value(error).unwrap();
        assert_eq!(json["kind"], "quota");
        assert_eq!(json["retryable"].as_bool(), Some(true));
        assert!(json["message"].as_str().unwrap().contains("quota"));
    }
}
