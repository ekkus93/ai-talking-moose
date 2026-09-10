use crate::ai::local_tts::storage::{LocalTtsInstallState, LocalTtsStatusError};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsInstallErrorKind {
    InvalidCatalog,
    UnknownModel,
    Busy,
    Network,
    Http,
    Io,
    SizeMismatch,
    Sha256Mismatch,
    Cancelled,
    Promotion,
    CorruptInstall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTtsInstallError {
    pub kind: LocalTtsInstallErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl LocalTtsInstallError {
    pub(super) fn new(
        kind: LocalTtsInstallErrorKind,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    pub(super) fn invalid_catalog() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::InvalidCatalog,
            "The bundled Local TTS artifact catalog is invalid.",
            false,
        )
    }

    pub(super) fn unknown_model() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::UnknownModel,
            "The selected Local TTS model is not in the supported catalog.",
            false,
        )
    }

    pub(super) fn busy() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Busy,
            "The selected Local TTS model already has an install or delete operation in progress.",
            true,
        )
    }

    pub(super) fn network() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Network,
            "The Local TTS download failed because of a network error.",
            true,
        )
    }

    pub(super) fn http(status: u16) -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Http,
            format!("The Local TTS server returned HTTP status {status}."),
            status == 408 || status == 429 || status >= 500,
        )
    }

    pub(super) fn io(operation: &'static str) -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Io,
            format!("The Local TTS installer could not {operation}."),
            true,
        )
    }

    pub(super) fn size_mismatch() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::SizeMismatch,
            "A downloaded Local TTS artifact has the wrong byte count.",
            true,
        )
    }

    pub(super) fn sha256_mismatch() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Sha256Mismatch,
            "A downloaded Local TTS artifact failed SHA-256 verification.",
            true,
        )
    }

    pub(super) fn cancelled() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Cancelled,
            "The Local TTS installation was cancelled.",
            true,
        )
    }

    pub(super) fn promotion() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::Promotion,
            "The verified Local TTS artifacts could not be promoted into model storage.",
            true,
        )
    }

    pub(super) fn corrupt_install() -> Self {
        Self::new(
            LocalTtsInstallErrorKind::CorruptInstall,
            "The Local TTS model storage contains an unsafe or corrupt path.",
            false,
        )
    }

    pub(super) fn from_status(error: LocalTtsStatusError) -> Self {
        Self::new(
            LocalTtsInstallErrorKind::CorruptInstall,
            error.message,
            error.retryable,
        )
    }
}

impl fmt::Display for LocalTtsInstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LocalTtsInstallError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTtsInstallProgress {
    pub model_id: String,
    pub artifact_filename: Option<String>,
    pub install_state: LocalTtsInstallState,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

pub type LocalTtsInstallProgressCallback = Arc<dyn Fn(LocalTtsInstallProgress) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTtsInstallOutcome {
    pub model_id: String,
    pub revision: String,
    pub installed_bytes: u64,
}
