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
    Verifying,
    Installed,
    Corrupt,
    Incompatible,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrModelDescriptor {
    /// Stable application-owned identifier, not an upstream implementation type.
    pub id: String,
    pub display_name: String,
    pub mode: AsrMode,
    pub install_state: AsrModelInstallState,
    pub revision: String,
    pub runtime_release: String,
    pub installed_bytes: Option<u64>,
    pub expected_bytes: u64,
    pub active: bool,
    pub error_message: Option<String>,
}

/// Live metrics owned by one active local-ASR pipeline.
///
/// Memory values are process RSS snapshots because the native Moonshine/ONNX
/// allocator is process-global and does not expose a reliable per-model RSS.
/// `baseline_resident_memory_bytes` is sampled before the native model is
/// opened, so callers can derive a session-local incremental footprint without
/// pretending the process-level measurement is model-exclusive.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LocalAsrRuntimeDiagnostics {
    pub input_sample_rate_hz: u32,
    pub streaming: bool,
    pub metrics_snapshot: bool,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub last_error: Option<AsrError>,
    pub first_partial_latency_ms: Option<u64>,
    pub first_final_latency_ms: Option<u64>,
    pub last_transcription_latency_ms: Option<u32>,
    pub processed_audio_ms: u64,
    pub inference_wall_time_ms: u64,
    pub real_time_factor: Option<f32>,
    pub process_cpu_time_ms: Option<u64>,
    pub average_cpu_utilization_percent: Option<f32>,
    pub baseline_resident_memory_bytes: Option<u64>,
    pub resident_memory_bytes: Option<u64>,
    pub peak_resident_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsrDiagnostics {
    pub selected_mode: AsrMode,
    pub engine_name: String,
    pub model_id: Option<String>,
    pub model_revision: Option<String>,
    pub install_state: Option<AsrModelInstallState>,
    pub input_sample_rate_hz: u32,
    pub streaming: bool,
    pub metrics_snapshot: bool,
    pub cpu_threads: Option<usize>,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub dropped_chunks: u64,
    pub last_error: Option<AsrError>,
    pub first_partial_latency_ms: Option<u64>,
    pub first_final_latency_ms: Option<u64>,
    pub last_transcription_latency_ms: Option<u32>,
    pub processed_audio_ms: u64,
    pub inference_wall_time_ms: u64,
    pub real_time_factor: Option<f32>,
    pub process_cpu_time_ms: Option<u64>,
    pub average_cpu_utilization_percent: Option<f32>,
    pub baseline_resident_memory_bytes: Option<u64>,
    pub resident_memory_bytes: Option<u64>,
    pub peak_resident_memory_bytes: Option<u64>,
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
