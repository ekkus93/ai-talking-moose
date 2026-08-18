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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSessionConfig {
    pub model: String,
    pub voice_name: Option<String>,
    pub system_instruction: Option<String>,
    pub sample_rate_in: u32,
    pub sample_rate_out: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum LiveServerEvent {
    Connected,
    UserTranscript(String),
    ModelTranscript(String),
    AudioData(Vec<u8>), // PCM bytes (usually 24kHz)
    Interrupted,
    TurnComplete,
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    Error(String),
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
    pub output: serde_json::Value,
}
