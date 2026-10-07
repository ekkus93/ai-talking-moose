use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelInstallState {
    NotInstalled,
    Downloading,
    Verifying,
    Promoting,
    Installed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelInstallErrorKind {
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
pub struct LocalModelInstallError {
    pub kind: LocalModelInstallErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl LocalModelInstallError {
    pub(super) fn new(
        kind: LocalModelInstallErrorKind,
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
            LocalModelInstallErrorKind::InvalidCatalog,
            "The bundled local model catalog is invalid.",
            false,
        )
    }

    pub(super) fn unknown_model() -> Self {
        Self::new(
            LocalModelInstallErrorKind::UnknownModel,
            "The selected local model is not in the supported catalog.",
            false,
        )
    }

    pub(super) fn busy() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Busy,
            "The selected local model already has an install or delete operation in progress.",
            true,
        )
    }

    pub(super) fn network() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Network,
            "The local model download failed because of a network error.",
            true,
        )
    }

    pub(super) fn http(status: u16) -> Self {
        Self::new(
            LocalModelInstallErrorKind::Http,
            format!("The local model server returned HTTP status {status}."),
            status == 408 || status == 429 || status >= 500,
        )
    }

    pub(super) fn io(operation: &'static str) -> Self {
        Self::new(
            LocalModelInstallErrorKind::Io,
            format!("The local model installer could not {operation}."),
            true,
        )
    }

    pub(super) fn size_mismatch() -> Self {
        Self::new(
            LocalModelInstallErrorKind::SizeMismatch,
            "The downloaded local model has the wrong byte count.",
            true,
        )
    }

    pub(super) fn sha256_mismatch() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Sha256Mismatch,
            "The downloaded local model failed SHA-256 verification.",
            true,
        )
    }

    pub(super) fn cancelled() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Cancelled,
            "The local model download was cancelled.",
            true,
        )
    }

    pub(super) fn promotion() -> Self {
        Self::new(
            LocalModelInstallErrorKind::Promotion,
            "The verified local model could not be promoted into the model directory.",
            true,
        )
    }

    pub(super) fn corrupt_install() -> Self {
        Self::new(
            LocalModelInstallErrorKind::CorruptInstall,
            "The local model storage contains an unsafe or corrupt path.",
            false,
        )
    }
}

impl fmt::Display for LocalModelInstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LocalModelInstallError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelInstallProgress {
    pub model_id: String,
    pub install_state: LocalModelInstallState,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

pub type LocalModelInstallProgressCallback = Arc<dyn Fn(LocalModelInstallProgress) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelInstallOutcome {
    pub model_id: String,
    pub revision: String,
    pub installed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub family: String,
    pub parameter_scale: String,
    pub quantization: String,
    pub revision: String,
    pub expected_bytes: u64,
    pub installed_bytes: Option<u64>,
    pub license: String,
    pub context_limit: u32,
    pub recommended_max_output: u32,
    pub install_state: LocalModelInstallState,
    pub active: bool,
    pub error: Option<LocalModelInstallError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelDiagnostics {
    pub model_root_ready: bool,
    pub installs_in_progress: usize,
    pub last_error: Option<LocalModelInstallError>,
}
