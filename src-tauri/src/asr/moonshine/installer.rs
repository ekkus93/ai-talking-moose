use super::manifest::{manifest_for_architecture, MoonshineModelFile, MoonshineModelManifest};
use super::runtime::MoonshineModelArchitecture;
#[cfg(test)]
use async_trait::async_trait;
use parking_lot::Mutex as SyncMutex;
use ring::digest::{Context as Sha256Context, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

mod delete;
mod disk;
mod integrity;
mod progress;
mod transport;

use delete::delete_model_path;
use disk::{DiskSpaceProbe, SystemDiskSpaceProbe};
use integrity::{crc32c_base64, digest_hex, Crc32c};
use progress::InstallerFileSink;
pub use progress::{
    MoonshineModelInstallPhase, MoonshineModelInstallProgress,
    MoonshineModelInstallProgressCallback,
};
#[cfg(test)]
use transport::{DownloadMetadata, DownloadSink};
use transport::{ModelDownloadTransport, ReqwestModelDownloadTransport};

const INSTALL_MARKER_FILE: &str = ".talking-moose-model.json";
const INSTALL_MARKER_SCHEMA_VERSION: u32 = 1;
const VERIFY_BUFFER_BYTES: usize = 1024 * 1024;
const DISK_SPACE_HEADROOM_BYTES: u64 = 8 * 1024 * 1024;
const ALLOWED_MODEL_FILES: [&str; 7] = [
    "adapter.ort",
    "cross_kv.ort",
    "decoder_kv.ort",
    "encoder.ort",
    "frontend.ort",
    "streaming_config.json",
    "tokenizer.bin",
];

/// A stable category for local model installation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoonshineModelInstallErrorKind {
    InvalidManifest,
    UnsupportedArtifact,
    InsufficientDiskSpace,
    Network,
    Http,
    Io,
    SizeMismatch,
    Sha256Mismatch,
    Crc32cMismatch,
    Cancelled,
    CorruptInstall,
    Promotion,
}

/// Sanitized install error safe to surface to the desktop UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonshineModelInstallError {
    pub kind: MoonshineModelInstallErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl MoonshineModelInstallError {
    fn new(
        kind: MoonshineModelInstallErrorKind,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    fn invalid_manifest() -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::InvalidManifest,
            "The bundled Moonshine model manifest is invalid.",
            false,
        )
    }

    fn unsupported_artifact(name: &str) -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::UnsupportedArtifact,
            format!("The Moonshine manifest contains an unsupported model artifact: {name}."),
            false,
        )
    }

    fn insufficient_disk_space(required: u64, available: u64) -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::InsufficientDiskSpace,
            format!(
                "Not enough free disk space to install the Moonshine model (need {required} bytes, have {available} bytes)."
            ),
            true,
        )
    }

    fn network() -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::Network,
            "The Moonshine model download failed because of a network error.",
            true,
        )
    }

    fn http(status: u16) -> Self {
        let retryable = status == 408 || status == 429 || status >= 500;
        Self::new(
            MoonshineModelInstallErrorKind::Http,
            format!("The Moonshine model server returned HTTP status {status}."),
            retryable,
        )
    }

    fn io(operation: &'static str) -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::Io,
            format!("The Moonshine model installer could not {operation}."),
            true,
        )
    }

    fn size_mismatch(name: &str) -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::SizeMismatch,
            format!("The downloaded Moonshine artifact has the wrong size: {name}."),
            true,
        )
    }

    fn sha256_mismatch(name: &str) -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::Sha256Mismatch,
            format!("The downloaded Moonshine artifact failed SHA-256 verification: {name}."),
            true,
        )
    }

    fn crc32c_mismatch(name: &str) -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::Crc32cMismatch,
            format!("The downloaded Moonshine artifact failed CRC32C verification: {name}."),
            true,
        )
    }

    fn cancelled() -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::Cancelled,
            "The Moonshine model download was cancelled.",
            true,
        )
    }

    fn corrupt_install() -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::CorruptInstall,
            "The installed Moonshine model is incomplete or corrupt.",
            true,
        )
    }

    fn promotion() -> Self {
        Self::new(
            MoonshineModelInstallErrorKind::Promotion,
            "The verified Moonshine model could not be promoted into the install directory.",
            true,
        )
    }
}

impl fmt::Display for MoonshineModelInstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MoonshineModelInstallError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoonshineModelInstallDisposition {
    Installed,
    AlreadyInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonshineModelInstallOutcome {
    pub disposition: MoonshineModelInstallDisposition,
    pub model_id: String,
    pub revision: String,
    pub installed_bytes: u64,
    pub model_path: PathBuf,
}

/// Cooperative cancellation handle for one explicit model-install request.
#[derive(Clone)]
pub struct MoonshineModelInstallCancellation {
    inner: CancellationToken,
}

impl Default for MoonshineModelInstallCancellation {
    fn default() -> Self {
        Self {
            inner: CancellationToken::new(),
        }
    }
}

impl MoonshineModelInstallCancellation {
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    async fn cancelled(&self) {
        self.inner.cancelled().await;
    }

    fn check(&self) -> Result<(), MoonshineModelInstallError> {
        if self.is_cancelled() {
            Err(MoonshineModelInstallError::cancelled())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct InstallMarker {
    schema_version: u32,
    model_id: String,
    revision: String,
    expected_bytes: u64,
    runtime_release: String,
    runtime_commit: String,
    runtime_header_version: i32,
}

impl InstallMarker {
    fn from_manifest(manifest: &MoonshineModelManifest) -> Self {
        Self {
            schema_version: INSTALL_MARKER_SCHEMA_VERSION,
            model_id: manifest.id.to_string(),
            revision: manifest.revision.to_string(),
            expected_bytes: manifest.expected_bytes,
            runtime_release: manifest.runtime.release.to_string(),
            runtime_commit: manifest.runtime.source_commit.to_string(),
            runtime_header_version: manifest.runtime.c_header_version,
        }
    }

    fn matches_manifest(&self, manifest: &MoonshineModelManifest) -> bool {
        self == &Self::from_manifest(manifest)
    }
}

struct StagingDirectory {
    path: PathBuf,
    keep: bool,
}

impl StagingDirectory {
    fn create(path: PathBuf) -> Result<Self, MoonshineModelInstallError> {
        fs::create_dir(&path)
            .map_err(|_| MoonshineModelInstallError::io("create the staging directory"))?;
        Ok(Self { path, keep: false })
    }

    fn mark_promoted(&mut self) {
        self.keep = true;
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Explicit, serialized installer for pinned Moonshine model payloads.
///
/// Creating or querying this object performs no network activity. Network I/O
/// only occurs when `install` is called by an explicit user action.
pub struct MoonshineModelInstaller {
    install_root: PathBuf,
    transport: Arc<dyn ModelDownloadTransport>,
    disk_space: Arc<dyn DiskSpaceProbe>,
}

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

/// Holds the per-model mutation lock from verified-path resolution through native load.
///
/// The engine opens models on its dedicated OS inference worker, so the synchronous
/// blocking lock acquisition here cannot block a Tokio executor thread.
pub(crate) struct MoonshineVerifiedModelLease {
    model_path: PathBuf,
    _operation_guard: OwnedMutexGuard<()>,
}

impl MoonshineVerifiedModelLease {
    pub(crate) fn model_path(&self) -> &Path {
        &self.model_path
    }
}

impl MoonshineModelInstaller {
    pub fn new(install_root: impl Into<PathBuf>) -> Result<Self, MoonshineModelInstallError> {
        Ok(Self {
            install_root: install_root.into(),
            transport: Arc::new(ReqwestModelDownloadTransport::new()?),
            disk_space: Arc::new(SystemDiskSpaceProbe),
        })
    }

    #[cfg(test)]
    fn with_dependencies(
        install_root: impl Into<PathBuf>,
        transport: Arc<dyn ModelDownloadTransport>,
        disk_space: Arc<dyn DiskSpaceProbe>,
    ) -> Self {
        Self {
            install_root: install_root.into(),
            transport,
            disk_space,
        }
    }

    pub fn model_path(&self, architecture: MoonshineModelArchitecture) -> PathBuf {
        let manifest = manifest_for_architecture(architecture);
        self.model_path_for_manifest(manifest)
    }

    pub fn verify_installed(
        &self,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Option<MoonshineModelInstallOutcome>, MoonshineModelInstallError> {
        let manifest = manifest_for_architecture(architecture);
        manifest
            .validate()
            .map_err(|_| MoonshineModelInstallError::invalid_manifest())?;
        self.verify_installed_manifest(manifest)
    }

    /// Resolve a verified model while retaining the same per-model lock used by
    /// install/delete. The returned lease must remain alive until native loading
    /// has finished so deletion cannot invalidate the verified path in between.
    pub(crate) fn acquire_verified_model_lease(
        &self,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Option<MoonshineVerifiedModelLease>, MoonshineModelInstallError> {
        self.acquire_verified_model_lease_for_manifest(manifest_for_architecture(architecture))
    }

    fn acquire_verified_model_lease_for_manifest(
        &self,
        manifest: &MoonshineModelManifest,
    ) -> Result<Option<MoonshineVerifiedModelLease>, MoonshineModelInstallError> {
        manifest
            .validate()
            .map_err(|_| MoonshineModelInstallError::invalid_manifest())?;
        let operation_guard = install_operation_lock(manifest.id).blocking_lock_owned();
        let Some(installed) = self.verify_installed_manifest(manifest)? else {
            return Ok(None);
        };
        Ok(Some(MoonshineVerifiedModelLease {
            model_path: installed.model_path,
            _operation_guard: operation_guard,
        }))
    }

    pub async fn install(
        &self,
        architecture: MoonshineModelArchitecture,
        cancellation: &MoonshineModelInstallCancellation,
    ) -> Result<MoonshineModelInstallOutcome, MoonshineModelInstallError> {
        let manifest = manifest_for_architecture(architecture);
        self.install_manifest_with_progress(manifest, cancellation, None)
            .await
    }

    pub async fn install_with_progress(
        &self,
        architecture: MoonshineModelArchitecture,
        cancellation: &MoonshineModelInstallCancellation,
        progress: MoonshineModelInstallProgressCallback,
    ) -> Result<MoonshineModelInstallOutcome, MoonshineModelInstallError> {
        let manifest = manifest_for_architecture(architecture);
        self.install_manifest_with_progress(manifest, cancellation, Some(progress))
            .await
    }

    pub async fn delete_installed(
        &self,
        architecture: MoonshineModelArchitecture,
    ) -> Result<bool, MoonshineModelInstallError> {
        self.delete_installed_manifest(manifest_for_architecture(architecture))
            .await
    }

    async fn delete_installed_manifest(
        &self,
        manifest: &MoonshineModelManifest,
    ) -> Result<bool, MoonshineModelInstallError> {
        let operation_lock = install_operation_lock(manifest.id);
        let _operation_guard = operation_lock.lock().await;
        manifest
            .validate()
            .map_err(|_| MoonshineModelInstallError::invalid_manifest())?;
        delete_model_path(&self.model_path_for_manifest(manifest))
    }

    #[cfg(test)]
    async fn install_manifest(
        &self,
        manifest: &'static MoonshineModelManifest,
        cancellation: &MoonshineModelInstallCancellation,
    ) -> Result<MoonshineModelInstallOutcome, MoonshineModelInstallError> {
        self.install_manifest_with_progress(manifest, cancellation, None)
            .await
    }

    async fn install_manifest_with_progress(
        &self,
        manifest: &'static MoonshineModelManifest,
        cancellation: &MoonshineModelInstallCancellation,
        progress: Option<MoonshineModelInstallProgressCallback>,
    ) -> Result<MoonshineModelInstallOutcome, MoonshineModelInstallError> {
        let operation_lock = install_operation_lock(manifest.id);
        let _operation_guard = tokio::select! {
            () = cancellation.cancelled() => return Err(MoonshineModelInstallError::cancelled()),
            guard = operation_lock.lock() => guard,
        };
        cancellation.check()?;
        self.validate_install_manifest(manifest)?;

        fs::create_dir_all(&self.install_root)
            .map_err(|_| MoonshineModelInstallError::io("create the model install root"))?;
        let model_parent = self.install_root.join(manifest.id);
        fs::create_dir_all(&model_parent)
            .map_err(|_| MoonshineModelInstallError::io("create the model directory"))?;
        self.cleanup_stale_partials(&model_parent, manifest.revision)?;

        match self.verify_installed_manifest(manifest) {
            Ok(Some(existing)) => {
                return Ok(MoonshineModelInstallOutcome {
                    disposition: MoonshineModelInstallDisposition::AlreadyInstalled,
                    ..existing
                });
            }
            Ok(None) => {}
            Err(error) if error.kind == MoonshineModelInstallErrorKind::CorruptInstall => {
                // Keep the corrupt target until the verified replacement is ready.
            }
            Err(error) => return Err(error),
        }

        let required_disk_bytes = manifest
            .expected_bytes
            .saturating_add(DISK_SPACE_HEADROOM_BYTES);
        match self.disk_space.available_bytes(&model_parent) {
            Ok(Some(available)) if available < required_disk_bytes => {
                return Err(MoonshineModelInstallError::insufficient_disk_space(
                    required_disk_bytes,
                    available,
                ));
            }
            Ok(Some(_)) => {}
            Ok(None) => {
                warn!("Free-space preflight is unavailable on this platform");
            }
            Err(_) => {
                return Err(MoonshineModelInstallError::io(
                    "check free disk space for the model installation",
                ));
            }
        }

        let staging_path =
            model_parent.join(format!(".{}.{}.partial", manifest.revision, Uuid::new_v4()));
        let mut staging = StagingDirectory::create(staging_path)?;

        let mut downloaded_bytes = 0_u64;
        for manifest_file in manifest.files {
            cancellation.check()?;
            let url = format!("{}/{}", manifest.base_url, manifest_file.name);
            if !url.starts_with("https://") {
                return Err(MoonshineModelInstallError::invalid_manifest());
            }
            let staged_file = staging.path.join(manifest_file.name);
            let mut sink = InstallerFileSink::create(
                &staged_file,
                manifest_file,
                downloaded_bytes,
                manifest.expected_bytes,
                progress.clone(),
            )?;
            let metadata = self.transport.stream(&url, cancellation, &mut sink).await?;
            if metadata
                .content_length
                .is_some_and(|length| length != manifest_file.bytes)
            {
                return Err(MoonshineModelInstallError::size_mismatch(
                    manifest_file.name,
                ));
            }
            sink.finish()?;
            downloaded_bytes = downloaded_bytes.saturating_add(manifest_file.bytes);
        }

        cancellation.check()?;
        if let Some(progress) = progress.as_ref() {
            progress(MoonshineModelInstallProgress {
                phase: MoonshineModelInstallPhase::Verifying,
                downloaded_bytes: manifest.expected_bytes,
                total_bytes: manifest.expected_bytes,
                current_file: None,
            });
        }
        self.write_install_marker(&staging.path, manifest)?;
        cancellation.check()?;
        let final_path = self.model_path_for_manifest(manifest);
        self.promote_staging(&mut staging, &final_path, manifest)?;
        self.verify_manifest_at_path(&final_path, manifest)?;

        Ok(MoonshineModelInstallOutcome {
            disposition: MoonshineModelInstallDisposition::Installed,
            model_id: manifest.id.to_string(),
            revision: manifest.revision.to_string(),
            installed_bytes: manifest.expected_bytes,
            model_path: final_path,
        })
    }

    fn validate_install_manifest(
        &self,
        manifest: &MoonshineModelManifest,
    ) -> Result<(), MoonshineModelInstallError> {
        manifest
            .validate()
            .map_err(|_| MoonshineModelInstallError::invalid_manifest())?;
        let base_url = reqwest::Url::parse(manifest.base_url)
            .map_err(|_| MoonshineModelInstallError::invalid_manifest())?;
        let revision_suffix = format!("/{}", manifest.revision);
        if base_url.scheme() != "https"
            || base_url.host_str() != Some("download.moonshine.ai")
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || !base_url
                .path()
                .trim_end_matches('/')
                .ends_with(&revision_suffix)
        {
            return Err(MoonshineModelInstallError::invalid_manifest());
        }
        for file in manifest.files {
            if !ALLOWED_MODEL_FILES.contains(&file.name) {
                return Err(MoonshineModelInstallError::unsupported_artifact(file.name));
            }
        }
        Ok(())
    }

    fn model_path_for_manifest(&self, manifest: &MoonshineModelManifest) -> PathBuf {
        self.install_root.join(manifest.id).join(manifest.revision)
    }

    fn verify_installed_manifest(
        &self,
        manifest: &MoonshineModelManifest,
    ) -> Result<Option<MoonshineModelInstallOutcome>, MoonshineModelInstallError> {
        let path = self.model_path_for_manifest(manifest);
        match fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(MoonshineModelInstallError::io(
                    "inspect the installed Moonshine model",
                ));
            }
        }
        self.verify_manifest_at_path(&path, manifest)?;
        Ok(Some(MoonshineModelInstallOutcome {
            disposition: MoonshineModelInstallDisposition::AlreadyInstalled,
            model_id: manifest.id.to_string(),
            revision: manifest.revision.to_string(),
            installed_bytes: manifest.expected_bytes,
            model_path: path,
        }))
    }

    fn verify_manifest_at_path(
        &self,
        path: &Path,
        manifest: &MoonshineModelManifest,
    ) -> Result<(), MoonshineModelInstallError> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(MoonshineModelInstallError::corrupt_install());
            }
            Err(_) => {
                return Err(MoonshineModelInstallError::io(
                    "inspect the installed Moonshine model directory",
                ));
            }
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(MoonshineModelInstallError::corrupt_install());
        }

        let expected_names: HashSet<&str> = manifest.files.iter().map(|file| file.name).collect();
        for entry in fs::read_dir(path)
            .map_err(|_| MoonshineModelInstallError::io("read the installed model directory"))?
        {
            let entry = entry.map_err(|_| {
                MoonshineModelInstallError::io("read an installed model directory entry")
            })?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return Err(MoonshineModelInstallError::corrupt_install());
            };
            if name != INSTALL_MARKER_FILE && !expected_names.contains(name) {
                return Err(MoonshineModelInstallError::corrupt_install());
            }
        }

        for manifest_file in manifest.files {
            self.verify_file(&path.join(manifest_file.name), manifest_file)?;
        }
        self.verify_install_marker(path, manifest)
    }

    fn open_regular_file_no_follow(
        &self,
        path: &Path,
        io_context: &'static str,
    ) -> Result<File, MoonshineModelInstallError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }

        let file = match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(MoonshineModelInstallError::corrupt_install());
            }
            #[cfg(unix)]
            Err(error) if error.raw_os_error() == Some(libc::ELOOP) => {
                return Err(MoonshineModelInstallError::corrupt_install());
            }
            Err(_) => return Err(MoonshineModelInstallError::io(io_context)),
        };

        let metadata = file
            .metadata()
            .map_err(|_| MoonshineModelInstallError::io(io_context))?;
        if !metadata.is_file() {
            return Err(MoonshineModelInstallError::corrupt_install());
        }

        // `O_NOFOLLOW` closes the check/open race on the macOS/Linux release
        // path. Keep an explicit post-open symlink check on other platforms so
        // a static symlink is still rejected rather than silently accepted.
        #[cfg(not(unix))]
        {
            let path_metadata = fs::symlink_metadata(path)
                .map_err(|_| MoonshineModelInstallError::io(io_context))?;
            if path_metadata.file_type().is_symlink() {
                return Err(MoonshineModelInstallError::corrupt_install());
            }
        }

        Ok(file)
    }

    fn verify_file(
        &self,
        path: &Path,
        manifest_file: &MoonshineModelFile,
    ) -> Result<(), MoonshineModelInstallError> {
        let file = self.open_regular_file_no_follow(path, "open an installed model artifact")?;
        let metadata = file
            .metadata()
            .map_err(|_| MoonshineModelInstallError::io("inspect an installed model artifact"))?;
        if metadata.len() != manifest_file.bytes {
            return Err(MoonshineModelInstallError::corrupt_install());
        }

        let mut reader = BufReader::with_capacity(VERIFY_BUFFER_BYTES, file);
        let mut buffer = vec![0_u8; VERIFY_BUFFER_BYTES];
        let mut sha256 = Sha256Context::new(&SHA256);
        let mut crc32c = Crc32c::default();
        loop {
            let count = reader.read(&mut buffer).map_err(|_| {
                MoonshineModelInstallError::io("verify an installed model artifact")
            })?;
            if count == 0 {
                break;
            }
            sha256.update(&buffer[..count]);
            crc32c.update(&buffer[..count]);
        }
        if !digest_hex(sha256.finish().as_ref()).eq_ignore_ascii_case(manifest_file.sha256)
            || crc32c_base64(crc32c.finalize()) != manifest_file.upstream_crc32c_base64
        {
            return Err(MoonshineModelInstallError::corrupt_install());
        }
        Ok(())
    }

    fn write_install_marker(
        &self,
        staging_path: &Path,
        manifest: &MoonshineModelManifest,
    ) -> Result<(), MoonshineModelInstallError> {
        let marker_path = staging_path.join(INSTALL_MARKER_FILE);
        let marker = InstallMarker::from_manifest(manifest);
        let marker_bytes = serde_json::to_vec_pretty(&marker)
            .map_err(|_| MoonshineModelInstallError::io("serialize the install marker"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(marker_path)
            .map_err(|_| MoonshineModelInstallError::io("create the install marker"))?;
        file.write_all(&marker_bytes)
            .and_then(|()| file.flush())
            .and_then(|()| file.sync_all())
            .map_err(|_| MoonshineModelInstallError::io("write the install marker"))
    }

    fn verify_install_marker(
        &self,
        path: &Path,
        manifest: &MoonshineModelManifest,
    ) -> Result<(), MoonshineModelInstallError> {
        let marker_path = path.join(INSTALL_MARKER_FILE);
        let mut marker_file =
            self.open_regular_file_no_follow(&marker_path, "open the Moonshine install marker")?;
        let mut marker_bytes = Vec::new();
        marker_file
            .read_to_end(&mut marker_bytes)
            .map_err(|_| MoonshineModelInstallError::io("read the Moonshine install marker"))?;
        let marker: InstallMarker = serde_json::from_slice(&marker_bytes)
            .map_err(|_| MoonshineModelInstallError::corrupt_install())?;
        if !marker.matches_manifest(manifest) {
            return Err(MoonshineModelInstallError::corrupt_install());
        }
        Ok(())
    }

    fn promote_staging(
        &self,
        staging: &mut StagingDirectory,
        final_path: &Path,
        manifest: &MoonshineModelManifest,
    ) -> Result<(), MoonshineModelInstallError> {
        let parent = final_path
            .parent()
            .ok_or_else(MoonshineModelInstallError::promotion)?;
        let backup_path = parent.join(format!(
            ".{}.{}.replaced",
            manifest.revision,
            Uuid::new_v4()
        ));
        let had_existing = match fs::symlink_metadata(final_path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(MoonshineModelInstallError::promotion()),
        };

        if had_existing {
            fs::rename(final_path, &backup_path)
                .map_err(|_| MoonshineModelInstallError::promotion())?;
        }

        if fs::rename(&staging.path, final_path).is_err() {
            if had_existing {
                let _ = fs::rename(&backup_path, final_path);
            }
            return Err(MoonshineModelInstallError::promotion());
        }
        staging.mark_promoted();

        if had_existing {
            if let Err(error_value) = fs::remove_dir_all(&backup_path) {
                warn!(
                    error = %error_value,
                    "Failed to remove replaced Moonshine model directory"
                );
            }
        }
        Ok(())
    }

    fn cleanup_stale_partials(
        &self,
        model_parent: &Path,
        revision: &str,
    ) -> Result<(), MoonshineModelInstallError> {
        let prefix = format!(".{revision}.");
        for entry in fs::read_dir(model_parent)
            .map_err(|_| MoonshineModelInstallError::io("inspect the model directory"))?
        {
            let entry =
                entry.map_err(|_| MoonshineModelInstallError::io("inspect the model directory"))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let stale_partial = name.ends_with(".partial");
            let stale_replaced = name.ends_with(".replaced");
            if !name.starts_with(&prefix) || (!stale_partial && !stale_replaced) {
                continue;
            }
            let metadata = entry
                .file_type()
                .map_err(|_| MoonshineModelInstallError::io("inspect stale model staging data"))?;
            if metadata.is_dir() {
                fs::remove_dir_all(entry.path()).map_err(|_| {
                    MoonshineModelInstallError::io("remove stale model staging data")
                })?;
            } else {
                fs::remove_file(entry.path()).map_err(|_| {
                    MoonshineModelInstallError::io("remove stale model staging data")
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "installer_tests.rs"]
mod tests;
