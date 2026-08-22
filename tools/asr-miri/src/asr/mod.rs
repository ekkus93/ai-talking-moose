#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrError {
    pub kind: AsrErrorKind,
    pub message: String,
    pub retryable: bool,
}

pub mod moonshine;
