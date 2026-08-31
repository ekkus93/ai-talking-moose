use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

pub(super) const DEFAULT_CONTEXT_SIZE: u32 = 4_096;
pub(super) const MAX_DEFAULT_THREADS: usize = 8;
pub(super) const MAX_PROMPT_BYTES: usize = 64 * 1_024;
pub(super) const MAX_TEMPERATURE: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocalRuntimePhase {
    Ready,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocalRuntimeErrorKind {
    ShuttingDown,
    UnknownModel,
    ModelNotInstalled,
    UnsafeArtifact,
    Initialization,
    ModelLoad,
    ModelNotLoaded,
    InvalidRequest,
    PromptTooLong,
    ContextCreation,
    Tokenization,
    Decode,
    OutputDecode,
    Cancelled,
    ModelDelete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalRuntimeError {
    pub(crate) kind: LocalRuntimeErrorKind,
    pub(crate) message: &'static str,
}

impl LocalRuntimeError {
    pub(super) fn new(kind: LocalRuntimeErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }

    pub(super) fn shutting_down() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ShuttingDown,
            "The local model runtime is shutting down.",
        )
    }

    pub(super) fn unknown_model() -> Self {
        Self::new(
            LocalRuntimeErrorKind::UnknownModel,
            "The selected local model is not in the supported catalog.",
        )
    }

    pub(super) fn model_not_installed() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ModelNotInstalled,
            "The selected local model is not installed and verified.",
        )
    }

    pub(super) fn unsafe_artifact() -> Self {
        Self::new(
            LocalRuntimeErrorKind::UnsafeArtifact,
            "The selected local model artifact is not a safe application-owned file.",
        )
    }

    pub(super) fn initialization() -> Self {
        Self::new(
            LocalRuntimeErrorKind::Initialization,
            "The local model runtime could not be initialized.",
        )
    }

    pub(super) fn model_load() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ModelLoad,
            "The selected local model could not be loaded.",
        )
    }

    pub(super) fn model_not_loaded() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ModelNotLoaded,
            "No verified local model is loaded.",
        )
    }

    pub(super) fn invalid_request() -> Self {
        Self::new(
            LocalRuntimeErrorKind::InvalidRequest,
            "The local model request is outside the supported runtime bounds.",
        )
    }

    pub(super) fn prompt_too_long() -> Self {
        Self::new(
            LocalRuntimeErrorKind::PromptTooLong,
            "The local model prompt does not fit within the bounded context window.",
        )
    }

    pub(super) fn context_creation() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ContextCreation,
            "The local model context could not be created.",
        )
    }

    pub(super) fn tokenization() -> Self {
        Self::new(
            LocalRuntimeErrorKind::Tokenization,
            "The local model prompt could not be tokenized.",
        )
    }

    pub(super) fn decode() -> Self {
        Self::new(
            LocalRuntimeErrorKind::Decode,
            "Local model inference failed while decoding tokens.",
        )
    }

    pub(super) fn output_decode() -> Self {
        Self::new(
            LocalRuntimeErrorKind::OutputDecode,
            "The local model output could not be decoded safely.",
        )
    }

    pub(super) fn cancelled() -> Self {
        Self::new(
            LocalRuntimeErrorKind::Cancelled,
            "The local model request was cancelled.",
        )
    }

    pub(super) fn model_delete() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ModelDelete,
            "The local model could not be deleted.",
        )
    }
}

impl fmt::Display for LocalRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for LocalRuntimeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LocalRuntimePolicy {
    context_size: u32,
    thread_count: usize,
}

impl LocalRuntimePolicy {
    pub(super) fn for_available_parallelism(available_parallelism: usize) -> Self {
        let available_parallelism = available_parallelism.max(1);
        let thread_count = (available_parallelism / 2).max(1).min(MAX_DEFAULT_THREADS);
        Self {
            context_size: DEFAULT_CONTEXT_SIZE,
            thread_count,
        }
    }

    pub(super) fn context_size(self) -> u32 {
        self.context_size
    }

    pub(super) fn thread_count(self) -> usize {
        self.thread_count
    }
}

impl Default for LocalRuntimePolicy {
    fn default() -> Self {
        let available_parallelism = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        Self::for_available_parallelism(available_parallelism)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeModelIdentity {
    pub(super) id: String,
    pub(super) revision: String,
    pub(super) quantization: String,
}

#[derive(Debug, Clone)]
pub(super) struct RuntimeModelSpec {
    pub(super) identity: RuntimeModelIdentity,
    pub(super) path: PathBuf,
    pub(super) context_size: u32,
    pub(super) thread_count: usize,
    pub(super) max_output_tokens: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalRuntimeGenerateRequest {
    pub(crate) model_id: String,
    pub(crate) prompt: String,
    pub(crate) temperature: f32,
    pub(crate) max_output_tokens: u32,
    pub(crate) seed: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LocalRuntimeGeneration {
    pub(crate) text: String,
    pub(crate) prompt_tokens: u32,
    pub(crate) output_tokens: u32,
    pub(crate) duration_ms: u64,
    pub(crate) tokens_per_second: Option<f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct LocalRuntimeDiagnostics {
    pub selected_model_id: String,
    pub loaded_model_id: Option<String>,
    pub loaded_revision: Option<String>,
    pub loaded_quantization: Option<String>,
    pub loaded: bool,
    pub phase: LocalRuntimePhase,
    pub thread_count: u32,
    pub context_size: u32,
    pub generation_in_progress: bool,
    pub last_error_category: Option<LocalRuntimeErrorKind>,
    pub last_generation_duration_ms: Option<u64>,
    pub last_prompt_tokens: Option<u32>,
    pub last_output_tokens: Option<u32>,
    pub last_tokens_per_second: Option<f32>,
}
