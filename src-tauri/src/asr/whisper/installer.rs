use super::manifest;
use parking_lot::Mutex as SyncMutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use uuid::Uuid;

mod delete;
mod disk;
mod integrity;
mod progress;
mod transport;

use crate::asr::types::{AsrError, AsrErrorKind};
use delete::delete_model_path;
use disk::{DiskSpaceProbe, SystemDiskSpaceProbe};
use progress::InstallerFileSink;
use transport::{ModelDownloadTransport, ReqwestModelDownloadTransport};

pub(crate) use progress::{
    WhisperModelInstallPhase, WhisperModelInstallProgress, WhisperModelInstallProgressCallback,
};

const MODEL_FILENAME: &str = "ggml-small.bin";
const INSTALL_MARKER_FILE: &str = ".talking-moose-model.json";
const INSTALL_MARKER_SCHEMA_VERSION: u32 = 2;

// --- Error types ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModelInstallErrorKind {
    InvalidManifest,
    InsufficientDiskSpace,
    Network,
    Http,
    Io,
    SizeMismatch,
    Sha256Mismatch,
    Cancelled,
    CorruptInstall,
    Promotion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhisperModelInstallError {
    pub kind: WhisperModelInstallErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl WhisperModelInstallError {
    fn new(
        kind: WhisperModelInstallErrorKind,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    pub fn invalid_manifest() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::InvalidManifest,
            "The bundled Whisper model manifest is invalid.",
            false,
        )
    }

    pub fn insufficient_disk_space(required: u64, available: u64) -> Self {
        Self::new(
            WhisperModelInstallErrorKind::InsufficientDiskSpace,
            format!(
                "Not enough free disk space to install the Whisper model (need {required} bytes, have {available} bytes)."
            ),
            true,
        )
    }

    pub fn network() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::Network,
            "The Whisper model download failed because of a network error.",
            true,
        )
    }

    pub fn http(status: u16) -> Self {
        let retryable = status == 408 || status == 429 || status >= 500;
        Self::new(
            WhisperModelInstallErrorKind::Http,
            format!("The Whisper model server returned HTTP status {status}."),
            retryable,
        )
    }

    pub fn io(operation: &'static str) -> Self {
        Self::new(
            WhisperModelInstallErrorKind::Io,
            format!("The Whisper model installer could not {operation}."),
            true,
        )
    }

    pub fn size_mismatch() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::SizeMismatch,
            "The downloaded Whisper artifact has the wrong size.",
            true,
        )
    }

    pub fn sha256_mismatch() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::Sha256Mismatch,
            "The downloaded Whisper artifact failed SHA-256 verification.",
            true,
        )
    }

    pub fn cancelled() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::Cancelled,
            "The Whisper model download was cancelled.",
            true,
        )
    }

    pub fn corrupt_install() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::CorruptInstall,
            "The installed Whisper model is incomplete or corrupt.",
            true,
        )
    }

    pub fn promotion() -> Self {
        Self::new(
            WhisperModelInstallErrorKind::Promotion,
            "The verified Whisper model could not be promoted into the install directory.",
            true,
        )
    }
}

impl fmt::Display for WhisperModelInstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WhisperModelInstallError {}

// --- Install outcome ------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModelInstallDisposition {
    Installed,
    AlreadyInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhisperModelInstallOutcome {
    pub disposition: WhisperModelInstallDisposition,
    pub model_id: String,
    /// Immutable model-artifact repository revision.
    pub revision: String,
    pub installed_bytes: u64,
    pub model_path: PathBuf,
}

// --- Cancellation ---------------------------------------------------------

#[derive(Clone)]
pub struct WhisperModelInstallCancellation {
    inner: tokio_util::sync::CancellationToken,
}

impl Default for WhisperModelInstallCancellation {
    fn default() -> Self {
        Self {
            inner: tokio_util::sync::CancellationToken::new(),
        }
    }
}

impl WhisperModelInstallCancellation {
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    async fn cancelled(&self) {
        self.inner.cancelled().await;
    }
}

// --- Install marker -------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct InstallMarker {
    schema_version: u32,
    model_id: String,
    /// Immutable model-artifact repository revision.
    revision: String,
    expected_bytes: u64,
    /// Runtime release label shown to users; this is the native source commit.
    runtime_release: String,
    /// Native whisper.cpp source commit compiled into the runtime.
    runtime_commit: String,
    /// Native whisper.cpp source commit recorded separately from the model artifact revision.
    #[serde(default)]
    source_commit: Option<String>,
}

impl InstallMarker {
    fn new() -> Self {
        Self {
            schema_version: INSTALL_MARKER_SCHEMA_VERSION,
            model_id: manifest::WHISPER_SMALL_ID.to_string(),
            revision: manifest::WHISPER_MODEL_REVISION.to_string(),
            expected_bytes: manifest::WHISPER_MODEL_BYTES,
            runtime_release: manifest::WHISPER_RUNTIME_RELEASE.to_string(),
            runtime_commit: manifest::WHISPER_SOURCE_COMMIT.to_string(),
            source_commit: Some(manifest::WHISPER_SOURCE_COMMIT.to_string()),
        }
    }

    fn is_compatible(&self) -> bool {
        if self.model_id != manifest::WHISPER_SMALL_ID {
            return false;
        }
        if self.expected_bytes != manifest::WHISPER_MODEL_BYTES {
            return false;
        }
        if self.runtime_release != manifest::WHISPER_RUNTIME_RELEASE {
            return false;
        }
        if self.runtime_commit != manifest::WHISPER_SOURCE_COMMIT {
            return false;
        }

        match self.schema_version {
            1 => self.revision == manifest::WHISPER_SOURCE_COMMIT && self.source_commit.is_none(),
            INSTALL_MARKER_SCHEMA_VERSION => {
                self.revision == manifest::WHISPER_MODEL_REVISION
                    && self.source_commit.as_deref() == Some(manifest::WHISPER_SOURCE_COMMIT)
            }
            _ => false,
        }
    }
}

// --- Per-model operation lock ---------------------------------------------

static INSTALL_OPERATION_LOCKS: OnceLock<SyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    OnceLock::new();

fn install_operation_lock(model_id: &str) -> Arc<AsyncMutex<()>> {
    let locks = INSTALL_OPERATION_LOCKS.get_or_init(|| SyncMutex::new(HashMap::new()));
    let mut locks = locks.lock();
    locks
        .entry(model_id.to_string())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

// --- Verified model lease --------------------------------------------------

pub(crate) struct WhisperVerifiedModelLease {
    model_path: PathBuf,
    _operation_guard: OwnedMutexGuard<()>,
}

impl WhisperVerifiedModelLease {
    pub(crate) fn model_path(&self) -> &Path {
        &self.model_path
    }
}

// --- Installer -------------------------------------------------------------

/// Explicit, serialized installer for the pinned Whisper model payload.
///
/// Creating or querying this object performs no network activity. Network I/O
/// only occurs when `install` is called by an explicit user action.
pub struct WhisperModelInstaller {
    install_root: PathBuf,
    transport: Arc<dyn ModelDownloadTransport>,
    disk_space: Arc<dyn DiskSpaceProbe>,
}

impl WhisperModelInstaller {
    pub fn new(install_root: impl Into<PathBuf>) -> Result<Self, WhisperModelInstallError> {
        let transport = Arc::new(ReqwestModelDownloadTransport::new()?);
        Ok(Self {
            install_root: install_root.into(),
            transport,
            disk_space: Arc::new(SystemDiskSpaceProbe),
        })
    }

    pub fn model_path(&self) -> PathBuf {
        self.install_root.join(MODEL_FILENAME)
    }

    fn marker_path(&self) -> PathBuf {
        self.install_root.join(INSTALL_MARKER_FILE)
    }

    fn ensure_install_root(&self) -> Result<(), WhisperModelInstallError> {
        fs::create_dir_all(&self.install_root)
            .map_err(|_| WhisperModelInstallError::io("create the Whisper model directory"))
    }

    fn legacy_model_path(&self) -> Option<PathBuf> {
        let model_root_name = self.install_root.file_name()?.to_string_lossy();
        if model_root_name != "whisper-small" {
            return None;
        }
        let legacy_root = self.install_root.parent()?;
        let legacy_root_name = legacy_root.file_name()?.to_string_lossy();
        if legacy_root_name != "whisper" {
            return None;
        }
        Some(legacy_root.join(MODEL_FILENAME))
    }

    fn migrate_legacy_layout(&self) -> Result<bool, WhisperModelInstallError> {
        let Some(legacy_model_path) = self.legacy_model_path() else {
            return Ok(false);
        };
        let Some(legacy_root) = legacy_model_path.parent() else {
            return Ok(false);
        };
        self.migrate_legacy_layout_with_verifier(legacy_root, |path| {
            manifest::verify_model(path).map_err(|_| WhisperModelInstallError::corrupt_install())
        })
    }

    pub(crate) fn migrate_legacy_layout_if_present(
        &self,
    ) -> Result<bool, WhisperModelInstallError> {
        self.migrate_legacy_layout()
    }

    fn migrate_legacy_layout_with_verifier<F>(
        &self,
        legacy_root: &Path,
        verifier: F,
    ) -> Result<bool, WhisperModelInstallError>
    where
        F: FnOnce(&Path) -> Result<(), WhisperModelInstallError>,
    {
        let legacy_model = legacy_root.join(MODEL_FILENAME);
        if self.model_path().exists() || !legacy_model.exists() {
            return Ok(false);
        }

        verifier(&legacy_model)?;
        self.ensure_install_root()?;

        let model_path = self.model_path();
        match fs::rename(&legacy_model, &model_path) {
            Ok(()) => {}
            Err(_) => {
                fs::copy(&legacy_model, &model_path)
                    .map_err(|_| WhisperModelInstallError::promotion())?;
                fs::remove_file(&legacy_model)
                    .map_err(|_| WhisperModelInstallError::promotion())?;
            }
        }

        fs::write(
            self.marker_path(),
            serde_json::to_string(&InstallMarker::new()).unwrap_or_default(),
        )
        .map_err(|_| WhisperModelInstallError::io("write the install marker"))?;
        let _ = fs::remove_file(legacy_root.join(INSTALL_MARKER_FILE));
        Ok(true)
    }

    /// Check whether the installed model is intact and returns its size.
    pub fn verify_installed(
        &self,
    ) -> Result<Option<WhisperModelInstallOutcome>, WhisperModelInstallError> {
        let _ = self.migrate_legacy_layout()?;

        let model_path = self.model_path();
        if !model_path.exists() {
            return Ok(None);
        }

        let marker: InstallMarker = match fs::read_to_string(self.marker_path()) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|_| WhisperModelInstallError::corrupt_install())?,
            Err(_) => return Ok(None),
        };
        if !marker.is_compatible() {
            return Ok(None);
        }

        let metadata =
            fs::metadata(&model_path).map_err(|_| WhisperModelInstallError::corrupt_install())?;
        let file_size = metadata.len();
        if file_size != manifest::WHISPER_MODEL_BYTES {
            return Ok(None);
        }

        // Full integrity verification: size, SHA-256, magic prefix. This is a
        // bounded streaming verification implemented in `manifest::verify_model`.
        if manifest::verify_model(&model_path).is_err() {
            return Err(WhisperModelInstallError::corrupt_install());
        }

        Ok(Some(WhisperModelInstallOutcome {
            disposition: WhisperModelInstallDisposition::Installed,
            model_id: manifest::WHISPER_SMALL_ID.to_string(),
            revision: manifest::WHISPER_MODEL_REVISION.to_string(),
            installed_bytes: file_size,
            model_path,
        }))
    }

    /// Acquire the verified model while holding the per-model operation lock.
    ///
    /// The returned lease must remain alive until native loading has finished
    /// so deletion cannot invalidate the verified path in between.
    pub(crate) fn acquire_verified_model_lease(
        &self,
    ) -> Result<Option<WhisperVerifiedModelLease>, WhisperModelInstallError> {
        let model_id = manifest::WHISPER_SMALL_ID;
        let guard = install_operation_lock(model_id).blocking_lock_owned();
        let outcome = match self.verify_installed() {
            Ok(Some(outcome)) => outcome,
            Ok(None) => {
                drop(guard);
                return Ok(None);
            }
            Err(error) => {
                drop(guard);
                return Err(error);
            }
        };
        Ok(Some(WhisperVerifiedModelLease {
            model_path: outcome.model_path,
            _operation_guard: guard,
        }))
    }

    /// Install or verify the pinned Whisper model payload.
    ///
    /// Returns `AlreadyInstalled` when the local file already matches the manifest.
    /// Otherwise downloads, verifies, and atomically promotes into the install root.
    pub async fn install(
        &self,
        cancellation: &mut WhisperModelInstallCancellation,
        progress_callback: &Option<WhisperModelInstallProgressCallback>,
    ) -> Result<WhisperModelInstallOutcome, WhisperModelInstallError> {
        let model_id = manifest::WHISPER_SMALL_ID.to_string();
        let lock = install_operation_lock(&model_id);
        let guard = lock.lock().await;

        self.ensure_install_root()?;

        // Fast path: already installed and intact.
        if let Ok(Some(outcome)) = self.verify_installed() {
            drop(guard);
            return Ok(WhisperModelInstallOutcome {
                disposition: WhisperModelInstallDisposition::AlreadyInstalled,
                ..outcome
            });
        }

        // Disk space check. The install root exists before this probe so clean
        // profiles do not fail merely because the canonical model directory was
        // not pre-created by a harness.
        match self.disk_space.available_bytes(&self.install_root) {
            Ok(Some(available)) if available < manifest::WHISPER_MODEL_BYTES => {
                drop(guard);
                return Err(WhisperModelInstallError::insufficient_disk_space(
                    manifest::WHISPER_MODEL_BYTES,
                    available,
                ));
            }
            _ => {}
        }

        if cancellation.is_cancelled() {
            drop(guard);
            return Err(WhisperModelInstallError::cancelled());
        }

        // Staging: use a unique subdirectory under the canonical per-model root
        // to avoid name collisions and to keep marker/artifact metadata together.
        let staging_dir_path = self
            .install_root
            .join(format!("whisper_staging_{}", Uuid::new_v4()));
        fs::create_dir(&staging_dir_path)
            .map_err(|_| WhisperModelInstallError::io("create the staging directory"))?;

        let staging_file_path = staging_dir_path.join(MODEL_FILENAME);
        let model_path = self.model_path();
        let marker_path = self.marker_path();

        let callback: Option<WhisperModelInstallProgressCallback> =
            progress_callback.as_ref().map(|cb| cb.clone());

        // Create the verifying file sink.
        let mut sink = InstallerFileSink::create(&staging_file_path, callback)?;

        // Download with the transport.
        let transport_result = self
            .transport
            .stream(manifest::WHISPER_MODEL_URL, cancellation, &mut sink)
            .await;

        let result: Result<WhisperModelInstallOutcome, WhisperModelInstallError>;

        match transport_result {
            Ok(()) => {
                if let Err(error) = sink.finish() {
                    fs::remove_dir_all(&staging_dir_path).ok();
                    drop(guard);
                    result = Err(error);
                } else if fs::rename(&staging_file_path, &model_path).is_err() {
                    fs::remove_dir_all(&staging_dir_path).ok();
                    drop(guard);
                    result = Err(WhisperModelInstallError::promotion());
                } else if fs::write(
                    &marker_path,
                    serde_json::to_string(&InstallMarker::new()).unwrap_or_default(),
                )
                .is_err()
                {
                    fs::remove_file(&model_path).ok();
                    fs::remove_dir_all(&staging_dir_path).ok();
                    drop(guard);
                    result = Err(WhisperModelInstallError::io("write the install marker"));
                } else {
                    fs::remove_dir_all(&staging_dir_path).ok();
                    drop(guard);
                    result = Ok(WhisperModelInstallOutcome {
                        disposition: WhisperModelInstallDisposition::Installed,
                        model_id: manifest::WHISPER_SMALL_ID.to_string(),
                        revision: manifest::WHISPER_MODEL_REVISION.to_string(),
                        installed_bytes: manifest::WHISPER_MODEL_BYTES,
                        model_path,
                    });
                }
            }
            Err(error) => {
                fs::remove_dir_all(&staging_dir_path).ok();
                drop(guard);
                result = Err(error);
            }
        }

        result
    }

    /// Delete the installed Whisper model.
    pub async fn delete(&self) -> Result<(), WhisperModelInstallError> {
        let model_id = manifest::WHISPER_SMALL_ID.to_string();
        let lock = install_operation_lock(&model_id);
        let _guard = lock.lock().await;
        let model_path = self.model_path();
        delete_model_path(&model_path)?;
        let _ = fs::remove_file(self.marker_path());
        Ok(())
    }
}

// --- Error mapping for the engine -----------------------------------------

pub(crate) enum WhisperFailure {
    ModelNotInstalled,
    RuntimeUnavailable,
    ModelLoad(String),
    AudioInput,
    Inference(String),
    InvalidState(String),
    Internal(String),
    Verification(WhisperModelInstallError),
}

/// The sole Whisper-internal to public ASR error mapping boundary.
pub(crate) fn map_whisper_failure(failure: WhisperFailure) -> AsrError {
    let (kind, message, retryable) = match failure {
        WhisperFailure::ModelNotInstalled => (
            AsrErrorKind::ModelNotInstalled,
            "The Whisper Small model is not installed. Install it in Settings before starting local speech recognition.".to_string(),
            false,
        ),
        WhisperFailure::RuntimeUnavailable => (
            AsrErrorKind::RuntimeUnavailable,
            manifest::WHISPER_RUNTIME_UNBUILT_MESSAGE.to_string(),
            false,
        ),
        WhisperFailure::ModelLoad(message) => {
            (AsrErrorKind::ModelLoadFailed, message, false)
        }
        WhisperFailure::AudioInput => (
            AsrErrorKind::AudioInput,
            "Whisper Small local ASR received invalid PCM samples.".to_string(),
            true,
        ),
        WhisperFailure::Inference(message) => (AsrErrorKind::Inference, message, true),
        WhisperFailure::InvalidState(message) => (AsrErrorKind::InvalidState, message, false),
        WhisperFailure::Internal(message) => (AsrErrorKind::Internal, message, false),
        WhisperFailure::Verification(error) => match error.kind {
        WhisperModelInstallErrorKind::CorruptInstall
        | WhisperModelInstallErrorKind::SizeMismatch
        | WhisperModelInstallErrorKind::Sha256Mismatch => (
            AsrErrorKind::ModelCorrupt,
            "The Whisper Small model is incomplete or corrupt. Reinstall it in Settings before starting local speech recognition. No microphone audio was sent to Google.".to_string(),
            true,
        ),
        WhisperModelInstallErrorKind::InvalidManifest => (
            AsrErrorKind::Internal,
            "The bundled Whisper Small model metadata is invalid. Update or reinstall the application.".to_string(),
            false,
        ),
        WhisperModelInstallErrorKind::Io => (
            AsrErrorKind::Internal,
            format!("Whisper Small model verification could not access the installed artifact. {}", error.message),
            error.retryable,
        ),
        WhisperModelInstallErrorKind::InsufficientDiskSpace
        | WhisperModelInstallErrorKind::Network
        | WhisperModelInstallErrorKind::Http
        | WhisperModelInstallErrorKind::Promotion
        | WhisperModelInstallErrorKind::Cancelled => (
            AsrErrorKind::Internal,
            "An installation-only Whisper error reached model startup unexpectedly.".to_string(),
            false,
        ),
        },
    };
    AsrError {
        kind,
        message,
        retryable,
    }
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_install_root_creates_clean_profile_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp
            .path()
            .join("models")
            .join("whisper")
            .join("whisper-small");
        assert!(!root.exists());

        let installer = WhisperModelInstaller::new(&root).unwrap();
        installer.ensure_install_root().unwrap();

        assert!(root.is_dir());
        assert_eq!(installer.model_path(), root.join(MODEL_FILENAME));
    }

    #[test]
    fn marker_v1_remains_compatible_for_verified_existing_install() {
        let marker = InstallMarker {
            schema_version: 1,
            model_id: manifest::WHISPER_SMALL_ID.to_string(),
            revision: manifest::WHISPER_SOURCE_COMMIT.to_string(),
            expected_bytes: manifest::WHISPER_MODEL_BYTES,
            runtime_release: manifest::WHISPER_RUNTIME_RELEASE.to_string(),
            runtime_commit: manifest::WHISPER_SOURCE_COMMIT.to_string(),
            source_commit: None,
        };
        assert!(marker.is_compatible());
        let current = InstallMarker::new();
        assert_eq!(current.revision, manifest::WHISPER_MODEL_REVISION);
        assert_eq!(
            current.source_commit.as_deref(),
            Some(manifest::WHISPER_SOURCE_COMMIT)
        );
    }

    #[test]
    fn legacy_parent_layout_is_detected_only_for_canonical_whisper_small_root() {
        let temp = tempfile::TempDir::new().unwrap();
        let canonical = temp
            .path()
            .join("models")
            .join("whisper")
            .join("whisper-small");
        let installer = WhisperModelInstaller::new(&canonical).unwrap();
        assert_eq!(
            installer.legacy_model_path().unwrap(),
            temp.path()
                .join("models")
                .join("whisper")
                .join(MODEL_FILENAME)
        );

        let arbitrary = WhisperModelInstaller::new(temp.path().join("other")).unwrap();
        assert!(arbitrary.legacy_model_path().is_none());
    }

    #[test]
    fn legacy_layout_migration_rewrites_marker_and_preserves_model_bytes() {
        let temp = tempfile::TempDir::new().unwrap();
        let legacy_root = temp.path().join("models").join("whisper");
        let new_root = legacy_root.join("whisper-small");
        fs::create_dir_all(&legacy_root).unwrap();
        let legacy_model = legacy_root.join(MODEL_FILENAME);
        fs::write(&legacy_model, b"verified-fixture").unwrap();
        fs::write(
            legacy_root.join(INSTALL_MARKER_FILE),
            r#"{"schema_version":1,"model_id":"old","revision":"old","expected_bytes":1,"runtime_release":"old","runtime_commit":"old"}"#,
        )
        .unwrap();

        let installer = WhisperModelInstaller::new(&new_root).unwrap();
        let migrated = installer
            .migrate_legacy_layout_with_verifier(&legacy_root, |_path| Ok(()))
            .unwrap();

        assert!(migrated);
        assert!(!legacy_model.exists());
        assert_eq!(
            fs::read(installer.model_path()).unwrap(),
            b"verified-fixture"
        );
        assert!(!legacy_root.join(INSTALL_MARKER_FILE).exists());

        let marker: InstallMarker =
            serde_json::from_str(&fs::read_to_string(new_root.join(INSTALL_MARKER_FILE)).unwrap())
                .unwrap();
        assert_eq!(marker, InstallMarker::new());
    }

    #[test]
    fn legacy_layout_migration_does_not_replace_existing_canonical_model() {
        let temp = tempfile::TempDir::new().unwrap();
        let legacy_root = temp.path().join("models").join("whisper");
        let new_root = legacy_root.join("whisper-small");
        fs::create_dir_all(&new_root).unwrap();
        fs::write(legacy_root.join(MODEL_FILENAME), b"legacy").unwrap();
        fs::write(new_root.join(MODEL_FILENAME), b"canonical").unwrap();

        let installer = WhisperModelInstaller::new(&new_root).unwrap();
        let migrated = installer
            .migrate_legacy_layout_with_verifier(&legacy_root, |_path| Ok(()))
            .unwrap();

        assert!(!migrated);
        assert_eq!(fs::read(installer.model_path()).unwrap(), b"canonical");
        assert_eq!(
            fs::read(legacy_root.join(MODEL_FILENAME)).unwrap(),
            b"legacy"
        );
    }

    #[tokio::test]
    async fn active_verified_model_lease_blocks_delete_until_released() {
        let temp = tempfile::TempDir::new().unwrap();
        let installer = Arc::new(WhisperModelInstaller::new(temp.path()).unwrap());
        fs::write(installer.model_path(), b"leased model fixture").unwrap();
        let operation_lock = install_operation_lock(manifest::WHISPER_SMALL_ID);
        let lease = WhisperVerifiedModelLease {
            model_path: installer.model_path(),
            _operation_guard: operation_lock.clone().lock_owned().await,
        };

        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let delete_installer = installer.clone();
        let delete_task = tokio::spawn(async move {
            let _ = started_tx.send(());
            delete_installer.delete().await
        });
        started_rx.await.unwrap();

        assert!(installer.model_path().exists());
        assert!(
            !delete_task.is_finished(),
            "delete must wait for the active model lease"
        );
        drop(lease);
        delete_task.await.unwrap().unwrap();
        assert!(!installer.model_path().exists());
    }

    #[test]
    fn public_error_mapping_covers_all_whisper_engine_failure_classes() {
        for (failure, expected) in [
            (
                WhisperFailure::ModelNotInstalled,
                AsrErrorKind::ModelNotInstalled,
            ),
            (
                WhisperFailure::RuntimeUnavailable,
                AsrErrorKind::RuntimeUnavailable,
            ),
            (
                WhisperFailure::ModelLoad("load failed".to_string()),
                AsrErrorKind::ModelLoadFailed,
            ),
            (WhisperFailure::AudioInput, AsrErrorKind::AudioInput),
            (
                WhisperFailure::Inference("inference failed".to_string()),
                AsrErrorKind::Inference,
            ),
            (
                WhisperFailure::InvalidState("invalid state".to_string()),
                AsrErrorKind::InvalidState,
            ),
            (
                WhisperFailure::Internal("internal failure".to_string()),
                AsrErrorKind::Internal,
            ),
        ] {
            assert_eq!(map_whisper_failure(failure).kind, expected);
        }

        for kind in [
            WhisperModelInstallErrorKind::CorruptInstall,
            WhisperModelInstallErrorKind::SizeMismatch,
            WhisperModelInstallErrorKind::Sha256Mismatch,
        ] {
            let mapped = map_whisper_failure(WhisperFailure::Verification(
                WhisperModelInstallError::new(kind, "integrity", true),
            ));
            assert_eq!(mapped.kind, AsrErrorKind::ModelCorrupt);
        }

        let cancelled = map_whisper_failure(WhisperFailure::Verification(
            WhisperModelInstallError::cancelled(),
        ));
        assert_eq!(cancelled.kind, AsrErrorKind::Internal);

        let invalid = map_whisper_failure(WhisperFailure::Verification(
            WhisperModelInstallError::invalid_manifest(),
        ));
        assert_eq!(invalid.kind, AsrErrorKind::Internal);

        let io = map_whisper_failure(WhisperFailure::Verification(WhisperModelInstallError::io(
            "read the model",
        )));
        assert_eq!(io.kind, AsrErrorKind::Internal);

        for kind in [
            WhisperModelInstallErrorKind::Network,
            WhisperModelInstallErrorKind::Http,
            WhisperModelInstallErrorKind::Promotion,
            WhisperModelInstallErrorKind::InsufficientDiskSpace,
        ] {
            let mapped = map_whisper_failure(WhisperFailure::Verification(
                WhisperModelInstallError::new(kind, "unexpected", true),
            ));
            assert_eq!(mapped.kind, AsrErrorKind::Internal);
        }
    }

    #[test]
    fn installer_struct_compiles() {
        // Just a compile test to ensure the struct is well-formed.
        let _ = std::mem::size_of::<WhisperModelInstaller>();
    }
}
