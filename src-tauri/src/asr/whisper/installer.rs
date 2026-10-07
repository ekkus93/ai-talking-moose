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
    revision: String,
    expected_bytes: u64,
    runtime_release: String,
    runtime_commit: String,
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
        let common = self.model_id == manifest::WHISPER_SMALL_ID
            && self.expected_bytes == manifest::WHISPER_MODEL_BYTES
            && self.runtime_release == manifest::WHISPER_RUNTIME_RELEASE
            && self.runtime_commit == manifest::WHISPER_SOURCE_COMMIT;
        match self.schema_version {
            1 => common && self.revision == manifest::WHISPER_SOURCE_COMMIT,
            INSTALL_MARKER_SCHEMA_VERSION => {
                common
                    && self.revision == manifest::WHISPER_MODEL_REVISION
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

    fn ensure_install_root(&self) -> Result<(), WhisperModelInstallError> {
        fs::create_dir_all(&self.install_root)
            .map_err(|_| WhisperModelInstallError::io("create the Whisper model directory"))
    }

    fn legacy_model_path(&self) -> Option<PathBuf> {
        if self.install_root.file_name()?.to_str()? != "whisper-small" {
            return None;
        }
        self.install_root
            .parent()
            .map(|parent| parent.join(MODEL_FILENAME))
    }

    pub(crate) fn migrate_legacy_layout_if_present(&self) -> Result<bool, WhisperModelInstallError> {
        if self.model_path().exists() {
            return Ok(false);
        }
        let Some(legacy_model_path) = self.legacy_model_path() else {
            return Ok(false);
        };
        if !legacy_model_path.is_file() {
            return Ok(false);
        }

        manifest::verify_model(&legacy_model_path)
            .map_err(|_| WhisperModelInstallError::corrupt_install())?;
        self.ensure_install_root()?;

        let canonical_model_path = self.model_path();
        fs::rename(&legacy_model_path, &canonical_model_path)
            .map_err(|_| WhisperModelInstallError::promotion())?;

        let marker_path = self.install_root.join(INSTALL_MARKER_FILE);
        let marker_json = serde_json::to_string(&InstallMarker::new())
            .map_err(|_| WhisperModelInstallError::invalid_manifest())?;
        if fs::write(&marker_path, marker_json).is_err() {
            let _ = fs::rename(&canonical_model_path, &legacy_model_path);
            return Err(WhisperModelInstallError::io(
                "write the migrated install marker",
            ));
        }

        if let Some(parent) = legacy_model_path.parent() {
            let _ = fs::remove_file(parent.join(INSTALL_MARKER_FILE));
        }
        Ok(true)
    }

    /// Check whether the installed model is intact and returns its size.
    pub fn verify_installed(
        &self,
    ) -> Result<Option<WhisperModelInstallOutcome>, WhisperModelInstallError> {
        let model_path = self.model_path();
        if !model_path.exists() {
            return Ok(None);
        }
        let marker_path = self.install_root.join(INSTALL_MARKER_FILE);
        let marker: InstallMarker = match fs::read_to_string(&marker_path) {
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
        if file_size != marker.expected_bytes {
            return Ok(None);
        }
        // Full integrity verification: size, SHA-256, magic prefix.
        if manifest::verify_model(&model_path).is_err() {
            return Err(WhisperModelInstallError::corrupt_install());
        }
        Ok(Some(WhisperModelInstallOutcome {
            disposition: WhisperModelInstallDisposition::Installed,
            model_id: marker.model_id,
            revision: manifest::WHISPER_MODEL_REVISION.to_string(),
            installed_bytes: file_size,
            model_path: model_path.clone(),
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
        self.migrate_legacy_layout_if_present()?;
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
        self.migrate_legacy_layout_if_present()?;

        // Fast path: already installed and intact. Refresh metadata while the
        // explicit install operation owns the lock so v1 markers become v2
        // without requiring a model redownload.
        if let Ok(Some(outcome)) = self.verify_installed() {
            let marker_path = self.install_root.join(INSTALL_MARKER_FILE);
            fs::write(
                &marker_path,
                serde_json::to_string(&InstallMarker::new()).unwrap_or_default(),
            )
            .map_err(|_| WhisperModelInstallError::io("refresh the install marker"))?;
            drop(guard);
            return Ok(WhisperModelInstallOutcome {
                disposition: WhisperModelInstallDisposition::AlreadyInstalled,
                ..outcome
            });
        }

        // Explicit installation owns creation of the canonical per-model root.
        // Descriptor queries remain side-effect free, while a clean profile does
        // not require workflow/test code to pre-create directories.
        self.ensure_install_root()?;

        // Disk space check.
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

        // Staging: use a unique subdirectory to avoid name collisions.
        let staging_dir_path = self
            .install_root
            .join(format!("whisper_staging_{}", Uuid::new_v4()));
        fs::create_dir(&staging_dir_path)
            .map_err(|_| WhisperModelInstallError::io("create the staging directory"))?;

        let staging_file_path = staging_dir_path.join(MODEL_FILENAME);
        let model_path = self.model_path();
        let marker_path = self.install_root.join(INSTALL_MARKER_FILE);

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
                } else {
                    if fs::rename(&staging_file_path, &model_path).is_err() {
                        fs::remove_dir_all(&staging_dir_path).ok();
                        drop(guard);
                        result = Err(WhisperModelInstallError::promotion());
                    } else {
                        if let Err(_error) = fs::write(
                            &marker_path,
                            serde_json::to_string(&InstallMarker::new()).unwrap_or_default(),
                        ) {
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
        let marker_path = self.install_root.join(INSTALL_MARKER_FILE);
        let _ = fs::remove_file(&marker_path);

        // Compatibility cleanup for profiles created before the canonical
        // per-model directory was introduced.
        if let Some(legacy_model_path) = self.legacy_model_path() {
            delete_model_path(&legacy_model_path)?;
            if let Some(parent) = legacy_model_path.parent() {
                let _ = fs::remove_file(parent.join(INSTALL_MARKER_FILE));
            }
        }
        Ok(())
    }
}

// --- Error mapping for the engine -----------------------------------------

pub fn map_install_error(error: WhisperModelInstallError) -> AsrError {
    match error.kind {
        WhisperModelInstallErrorKind::CorruptInstall
        | WhisperModelInstallErrorKind::SizeMismatch
        | WhisperModelInstallErrorKind::Sha256Mismatch
        => AsrError {
            kind: AsrErrorKind::ModelCorrupt,
            message: "The Whisper Small model is incomplete or corrupt. Reinstall it in Settings before starting local speech recognition. No microphone audio was sent to Google.".to_string(),
            retryable: true,
        },
        WhisperModelInstallErrorKind::Cancelled => AsrError {
            kind: AsrErrorKind::Cancelled,
            message: "The Whisper Small model verification was cancelled.".to_string(),
            retryable: true,
        },
        WhisperModelInstallErrorKind::InvalidManifest => AsrError {
            kind: AsrErrorKind::Internal,
            message: "The bundled Whisper Small model metadata is invalid. Update or reinstall the application.".to_string(),
            retryable: false,
        },
        _ => AsrError {
            kind: AsrErrorKind::ModelLoadFailed,
            message: format!(
                "Whisper Small could not be verified before local speech recognition started. {0}",
                error.message
            ),
            retryable: true,
        },
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
            temp.path().join("models").join("whisper").join(MODEL_FILENAME)
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
        assert_eq!(fs::read(installer.model_path()).unwrap(), b"verified-fixture");
        assert!(!legacy_root.join(INSTALL_MARKER_FILE).exists());

        let marker: InstallMarker = serde_json::from_str(
            &fs::read_to_string(new_root.join(INSTALL_MARKER_FILE)).unwrap(),
        )
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

    #[test]
    fn installer_struct_compiles() {
        // Just a compile test to ensure the struct is well-formed.
        let _ = std::mem::size_of::<WhisperModelInstaller>();
    }
}
