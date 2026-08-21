use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveClientMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<LiveSetupConfig>,
    #[serde(rename = "realtimeInput", skip_serializing_if = "Option::is_none")]
    pub realtime_input: Option<LiveRealtimeInput>,
    #[serde(rename = "toolResponse", skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<LiveToolResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSetupConfig {
    pub model: String,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<LiveGenerationConfig>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<LiveContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "inputAudioTranscription",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_audio_transcription: Option<serde_json::Value>,
    #[serde(
        rename = "outputAudioTranscription",
        skip_serializing_if = "Option::is_none"
    )]
    pub output_audio_transcription: Option<serde_json::Value>,
    #[serde(rename = "sessionResumption", skip_serializing_if = "Option::is_none")]
    pub session_resumption: Option<LiveSessionResumptionConfig>,
    #[serde(
        rename = "contextWindowCompression",
        skip_serializing_if = "Option::is_none"
    )]
    pub context_window_compression: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveGenerationConfig {
    #[serde(rename = "responseModalities", skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<String>>,
    #[serde(rename = "speechConfig", skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<LiveSpeechConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSpeechConfig {
    #[serde(rename = "voiceConfig", skip_serializing_if = "Option::is_none")]
    pub voice_config: Option<LiveVoiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveVoiceConfig {
    #[serde(
        rename = "prebuiltVoiceConfig",
        skip_serializing_if = "Option::is_none"
    )]
    pub prebuilt_voice_config: Option<LivePrebuiltVoiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivePrebuiltVoiceConfig {
    #[serde(rename = "voiceName")]
    pub voice_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveContent {
    pub parts: Vec<LivePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<LiveBlob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBlob {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveRealtimeInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<LiveBlob>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveToolResponse {
    #[serde(rename = "functionResponses")]
    pub function_responses: Vec<LiveFunctionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFunctionResponse {
    pub id: String,
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveSessionResumptionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveServerMessage {
    #[serde(rename = "setupComplete")]
    pub setup_complete: Option<serde_json::Value>,
    #[serde(rename = "serverContent")]
    pub server_content: Option<LiveServerContent>,
    #[serde(rename = "toolCall")]
    pub tool_call: Option<LiveToolCall>,
    #[serde(rename = "toolCallCancellation")]
    pub tool_call_cancellation: Option<serde_json::Value>,
    #[serde(rename = "goAway")]
    pub go_away: Option<LiveGoAway>,
    #[serde(rename = "sessionResumptionUpdate")]
    pub session_resumption_update: Option<LiveSessionResumptionUpdate>,
    pub error: Option<LiveServerError>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveServerContent {
    #[serde(rename = "modelTurn")]
    pub model_turn: Option<LiveContent>,
    #[serde(rename = "turnComplete")]
    pub turn_complete: Option<bool>,
    #[serde(rename = "generationComplete")]
    pub generation_complete: Option<bool>,
    pub interrupted: Option<bool>,
    #[serde(rename = "inputTranscription")]
    pub input_transcription: Option<LiveTranscription>,
    #[serde(rename = "interimInputTranscription")]
    pub interim_input_transcription: Option<LiveTranscription>,
    #[serde(rename = "outputTranscription")]
    pub output_transcription: Option<LiveTranscription>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveTranscription {
    #[serde(default)]
    pub text: String,
    pub finished: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveToolCall {
    #[serde(rename = "functionCalls", default)]
    pub function_calls: Vec<LiveFunctionCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFunctionCall {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveGoAway {
    #[serde(rename = "timeLeft")]
    pub time_left: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveSessionResumptionUpdate {
    pub resumable: Option<bool>,
    #[serde(rename = "newHandle")]
    pub new_handle: Option<String>,
    pub token: Option<String>,
}

impl LiveSessionResumptionUpdate {
    pub fn handle(&self) -> Option<&str> {
        self.new_handle.as_deref().or(self.token.as_deref())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveServerError {
    pub code: Option<u16>,
    pub status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realtime_audio_uses_current_audio_field_not_deprecated_media_chunks() {
        let message = LiveClientMessage {
            setup: None,
            realtime_input: Some(LiveRealtimeInput {
                audio: Some(LiveBlob {
                    mime_type: "audio/pcm;rate=16000".to_string(),
                    data: "AA==".to_string(),
                }),
                text: None,
            }),
            tool_response: None,
        };
        let value = serde_json::to_value(message).unwrap();
        assert!(value["realtimeInput"].get("audio").is_some());
        assert!(value["realtimeInput"].get("mediaChunks").is_none());
    }

    #[test]
    fn representative_server_frame_tolerates_unknown_additive_fields() {
        let frame = serde_json::json!({
            "serverContent": {
                "outputTranscription": { "text": "hello" },
                "generationComplete": true,
                "futureField": { "safe": true }
            },
            "futureEnvelopeField": 123
        });
        let parsed: LiveServerMessage = serde_json::from_value(frame).unwrap();
        let content = parsed.server_content.unwrap();
        assert_eq!(content.output_transcription.unwrap().text, "hello");
        assert_eq!(content.generation_complete, Some(true));
    }

    #[test]
    fn resumption_update_accepts_current_and_legacy_handle_spellings() {
        let current: LiveSessionResumptionUpdate = serde_json::from_value(serde_json::json!({
            "resumable": true,
            "newHandle": "handle-a"
        }))
        .unwrap();
        assert_eq!(current.handle(), Some("handle-a"));

        let documented_token: LiveSessionResumptionUpdate =
            serde_json::from_value(serde_json::json!({ "token": "handle-b" })).unwrap();
        assert_eq!(documented_token.handle(), Some("handle-b"));
    }
}
