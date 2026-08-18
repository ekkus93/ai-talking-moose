use serde::{Deserialize, Serialize};

// --- Client to Server Messages ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveClientMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<LiveSetupConfig>,
    #[serde(rename = "realtimeInput", skip_serializing_if = "Option::is_none")]
    pub realtime_input: Option<LiveRealtimeInput>,
    #[serde(rename = "toolResponse", skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<LiveToolResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveSetupConfig {
    pub model: String,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<LiveGenerationConfig>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<LiveContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveGenerationConfig {
    #[serde(rename = "responseModalities", skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<String>>,
    #[serde(rename = "speechConfig", skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<LiveSpeechConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveSpeechConfig {
    #[serde(rename = "voiceConfig", skip_serializing_if = "Option::is_none")]
    pub voice_config: Option<LiveVoiceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveVoiceConfig {
    #[serde(
        rename = "prebuiltVoiceConfig",
        skip_serializing_if = "Option::is_none"
    )]
    pub prebuilt_voice_config: Option<LivePrebuiltVoiceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LivePrebuiltVoiceConfig {
    #[serde(rename = "voiceName")]
    pub voice_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveContent {
    pub parts: Vec<LivePart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LivePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<LiveBlob>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveBlob {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String, // base64 encoded
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveRealtimeInput {
    #[serde(rename = "mediaChunks")]
    pub media_chunks: Vec<LiveBlob>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveToolResponse {
    #[serde(rename = "functionResponses")]
    pub function_responses: Vec<LiveFunctionResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveFunctionResponse {
    pub id: String,
    pub response: serde_json::Value,
}

// --- Server to Client Messages ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveServerMessage {
    #[serde(rename = "serverContent")]
    pub server_content: Option<LiveServerContent>,
    #[serde(rename = "toolCall")]
    pub tool_call: Option<LiveToolCall>,
    #[serde(rename = "toolCallCancellation")]
    pub tool_call_cancellation: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveServerContent {
    #[serde(rename = "modelTurn")]
    pub model_turn: Option<LiveContent>,
    #[serde(rename = "turnComplete")]
    pub turn_complete: Option<bool>,
    pub interrupted: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveToolCall {
    #[serde(rename = "functionCalls")]
    pub function_calls: Vec<LiveFunctionCall>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveFunctionCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}
