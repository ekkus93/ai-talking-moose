use serde::{Deserialize, Serialize};

/// User-selectable speech-recognition path.
///
/// Local Moonshine modes keep microphone audio on-device. `GeminiLiveAudio`
/// sends microphone audio to the configured Google Gemini Live session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AsrMode {
    #[default]
    MoonshineTinyStreaming,
    MoonshineSmallStreaming,
    GeminiLiveAudio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsrErrorKind {
    ModelNotInstalled,
    ModelCorrupt,
    RuntimeUnavailable,
    ModelLoadFailed,
    AudioInput,
    Inference,
    InvalidState,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrError {
    pub kind: AsrErrorKind,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AsrEvent {
    SpeechStarted { monotonic_ms: Option<u64> },
    PartialTranscript { text: String },
    FinalTranscript { text: String },
    SpeechEnded { monotonic_ms: Option<u64> },
    Error { error: AsrError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsrModelInstallState {
    NotInstalled,
    Downloading,
    Installed,
    Corrupt,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrModelDescriptor {
    /// Stable application-owned identifier, not an upstream implementation type.
    pub id: String,
    pub display_name: String,
    pub mode: AsrMode,
    pub install_state: AsrModelInstallState,
    pub installed_revision: Option<String>,
    pub installed_bytes: Option<u64>,
    pub expected_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrDiagnostics {
    pub selected_mode: AsrMode,
    pub engine_name: String,
    pub model_id: Option<String>,
    pub model_revision: Option<String>,
    pub install_state: Option<AsrModelInstallState>,
    pub input_sample_rate_hz: u32,
    pub streaming: bool,
    pub cpu_threads: Option<usize>,
    pub queue_depth: usize,
    pub dropped_chunks: u64,
    pub last_error: Option<AsrError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_mode_default_is_moonshine_tiny_streaming() {
        assert_eq!(AsrMode::default(), AsrMode::MoonshineTinyStreaming);
    }

    #[test]
    fn asr_mode_has_stable_serialized_names() {
        assert_eq!(
            serde_json::to_string(&AsrMode::MoonshineTinyStreaming).unwrap(),
            r#""moonshine_tiny_streaming""#
        );
        assert_eq!(
            serde_json::to_string(&AsrMode::MoonshineSmallStreaming).unwrap(),
            r#""moonshine_small_streaming""#
        );
        assert_eq!(
            serde_json::to_string(&AsrMode::GeminiLiveAudio).unwrap(),
            r#""gemini_live_audio""#
        );
    }
}
