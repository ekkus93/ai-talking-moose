use super::catalog::{local_model_entry, validate_local_model_catalog, LocalModelCatalogEntry};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio_util::sync::CancellationToken;
use transport::{LocalModelDownloadTransport, ReqwestLocalModelDownloadTransport};
use uuid::Uuid;

mod promotion;
mod storage;
mod transport;
mod types;
mod verification;

#[cfg(test)]
use promotion::promote_artifact;
use promotion::{promote_artifact_file, remove_pending_artifact, write_install_marker};
use storage::{
    cleanup_stale_staging, ensure_plain_directory, prepare_model_root, validate_storage_layout,
};
pub use types::{
    LocalModelDescriptor, LocalModelDiagnostics, LocalModelInstallError,
    LocalModelInstallErrorKind, LocalModelInstallOutcome, LocalModelInstallProgress,
    LocalModelInstallProgressCallback, LocalModelInstallState,
};
#[cfg(test)]
use verification::verify_artifact;
use verification::{verify_artifact_async, verify_artifact_cancellable};

const STAGING_DIR: &str = ".staging";
const INSTALL_MARKER: &str = ".talking-moose-local-llm.json";
const INSTALL_MARKER_VERSION: u32 = 1;
const VERIFY_BUFFER_BYTES: usize = 1024 * 1024;

static GLOBAL_LOCAL_MODEL_INSTALLER: OnceLock<Arc<LocalModelInstaller>> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
struct InstallMarker {
    schema_version: u32,
    model_id: String,
    revision: String,
    artifact_filename: String,
    expected_bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalModelInstallPhase {
    Downloading,
    Verifying,
    Promoting,
}

#[derive(Clone)]
struct InFlightInstall {
    cancellation: CancellationToken,
    phase: LocalModelInstallPhase,
}

#[derive(Debug, Clone)]
struct RecordedInstallError {
    sequence: u64,
    error: LocalModelInstallError,
}

#[derive(Debug, Default)]
struct InstallErrorState {
    next_sequence: u64,
    by_model: HashMap<String, RecordedInstallError>,
}

impl InstallErrorState {
    fn record(&mut self, model_id: &str, error: LocalModelInstallError) {
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.by_model.insert(
            model_id.to_string(),
            RecordedInstallError {
                sequence: self.next_sequence,
                error,
            },
        );
    }

    fn clear(&mut self, model_id: &str) {
        self.by_model.remove(model_id);
    }

    fn for_model(&self, model_id: &str) -> Option<LocalModelInstallError> {
        self.by_model
            .get(model_id)
            .map(|recorded| recorded.error.clone())
    }

    fn latest(&self) -> Option<LocalModelInstallError> {
        self.by_model
            .values()
            .max_by_key(|recorded| recorded.sequence)
            .map(|recorded| recorded.error.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeArtifactFingerprint {
    canonical_path: PathBuf,
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanoseconds: i64,
}

impl RuntimeArtifactFingerprint {
    fn from_metadata(canonical_path: PathBuf, metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            canonical_path,
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(unix)]
            changed_seconds: metadata.ctime(),
            #[cfg(unix)]
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

type VerificationObserver = Arc<dyn Fn() + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromotionCheckpoint {
    BeforeRename,
    AfterRename,
    BeforeMarkerCommit,
}

#[cfg(test)]
type PromotionObserver = Arc<dyn Fn(PromotionCheckpoint) + Send + Sync>;

pub struct LocalModelInstaller {
    root: PathBuf,
    transport: Arc<dyn LocalModelDownloadTransport>,
    in_flight: Mutex<HashMap<String, InFlightInstall>>,
    error_state: Mutex<InstallErrorState>,
    runtime_verifications: Mutex<HashMap<String, RuntimeArtifactFingerprint>>,
    #[cfg(test)]
    verification_observer: Mutex<Option<VerificationObserver>>,
    #[cfg(test)]
    runtime_verification_observer: Mutex<Option<VerificationObserver>>,
    #[cfg(test)]
    promotion_observer: Mutex<Option<PromotionObserver>>,
}

impl LocalModelInstaller {
    pub fn new(root: PathBuf) -> Result<Self, LocalModelInstallError> {
        validate_local_model_catalog().map_err(|_| LocalModelInstallError::invalid_catalog())?;
        prepare_model_root(&root)?;
        let staging = root.join(STAGING_DIR);
        ensure_plain_directory(&staging, "create the model staging directory")?;
        cleanup_stale_staging(&staging)?;
        Ok(Self {
            root,
            transport: Arc::new(ReqwestLocalModelDownloadTransport::new()?),
            in_flight: Mutex::new(HashMap::new()),
            error_state: Mutex::new(InstallErrorState::default()),
            runtime_verifications: Mutex::new(HashMap::new()),
            #[cfg(test)]
            verification_observer: Mutex::new(None),
            #[cfg(test)]
            runtime_verification_observer: Mutex::new(None),
            #[cfg(test)]
            promotion_observer: Mutex::new(None),
        })
    }

    #[cfg(test)]
    fn with_transport(
        root: PathBuf,
        transport: Arc<dyn LocalModelDownloadTransport>,
    ) -> Result<Self, LocalModelInstallError> {
        validate_local_model_catalog().map_err(|_| LocalModelInstallError::invalid_catalog())?;
        prepare_model_root(&root)?;
        ensure_plain_directory(
            &root.join(STAGING_DIR),
            "create the model staging directory",
        )?;
        Ok(Self {
            root,
            transport,
            in_flight: Mutex::new(HashMap::new()),
            error_state: Mutex::new(InstallErrorState::default()),
            runtime_verifications: Mutex::new(HashMap::new()),
            #[cfg(test)]
            verification_observer: Mutex::new(None),
            #[cfg(test)]
            runtime_verification_observer: Mutex::new(None),
            #[cfg(test)]
            promotion_observer: Mutex::new(None),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn set_in_flight_phase(&self, model_id: &str, phase: LocalModelInstallPhase) {
        if let Some(in_flight) = self.in_flight.lock().get_mut(model_id) {
            in_flight.phase = phase;
        }
    }

    fn in_flight_install_state(&self, model_id: &str) -> Option<LocalModelInstallState> {
        self.in_flight
            .lock()
            .get(model_id)
            .map(|in_flight| match in_flight.phase {
                LocalModelInstallPhase::Downloading => LocalModelInstallState::Downloading,
                LocalModelInstallPhase::Verifying => LocalModelInstallState::Verifying,
                LocalModelInstallPhase::Promoting => LocalModelInstallState::Promoting,
            })
    }

    #[cfg(test)]
    fn set_verification_observer(&self, observer: Option<VerificationObserver>) {
        *self.verification_observer.lock() = observer;
    }

    #[cfg(test)]
    fn set_runtime_verification_observer(&self, observer: Option<VerificationObserver>) {
        *self.runtime_verification_observer.lock() = observer;
    }

    #[cfg(test)]
    fn set_promotion_observer(&self, observer: Option<PromotionObserver>) {
        *self.promotion_observer.lock() = observer;
    }

    fn verification_observer(&self) -> Option<VerificationObserver> {
        #[cfg(test)]
        {
            self.verification_observer.lock().clone()
        }
        #[cfg(not(test))]
        {
            None
        }
    }

    fn notify_runtime_verification(&self) {
        #[cfg(test)]
        if let Some(observer) = self.runtime_verification_observer.lock().clone() {
            observer();
        }
    }

    fn notify_promotion_checkpoint(&self, checkpoint: PromotionCheckpoint) {
        #[cfg(test)]
        if let Some(observer) = self.promotion_observer.lock().clone() {
            observer(checkpoint);
        }
        #[cfg(not(test))]
        let _ = checkpoint;
    }

    pub fn model_path(&self, model_id: &str) -> Result<PathBuf, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let entry =
            local_model_entry(model_id).ok_or_else(LocalModelInstallError::unknown_model)?;
        Ok(self
            .root
            .join(entry.id)
            .join(entry.revision)
            .join(entry.artifact_filename))
    }

    pub fn descriptors(&self, selected_model_id: &str) -> Vec<LocalModelDescriptor> {
        super::catalog::LOCAL_MODEL_CATALOG
            .iter()
            .map(|entry| self.descriptor(entry, selected_model_id))
            .collect()
    }

    pub fn diagnostics(&self) -> LocalModelDiagnostics {
        let last_error = self.error_state.lock().latest();
        LocalModelDiagnostics {
            model_root_ready: validate_storage_layout(&self.root).is_ok(),
            installs_in_progress: self.in_flight.lock().len(),
            last_error,
        }
    }

    fn record_install_error(&self, model_id: &str, error: LocalModelInstallError) {
        self.error_state.lock().record(model_id, error);
    }

    fn clear_install_error(&self, model_id: &str) {
        self.error_state.lock().clear(model_id);
    }

    fn clear_runtime_verification(&self, model_id: &str) {
        self.runtime_verifications.lock().remove(model_id);
    }

    pub fn cancel(&self, model_id: &str) -> bool {
        let in_flight = self.in_flight.lock();
        if let Some(install) = in_flight.get(model_id) {
            install.cancellation.cancel();
            true
        } else {
            false
        }
    }

    pub async fn install(
        &self,
        model_id: &str,
        progress: Option<LocalModelInstallProgressCallback>,
    ) -> Result<LocalModelInstallOutcome, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let entry =
            local_model_entry(model_id).ok_or_else(LocalModelInstallError::unknown_model)?;
        self.install_entry(entry, progress).await
    }

    async fn install_entry(
        &self,
        entry: &'static LocalModelCatalogEntry,
        progress: Option<LocalModelInstallProgressCallback>,
    ) -> Result<LocalModelInstallOutcome, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        if self.marker_shape_is_valid(entry) {
            return Ok(LocalModelInstallOutcome {
                model_id: entry.id.to_string(),
                revision: entry.revision.to_string(),
                installed_bytes: entry.expected_bytes,
            });
        }
        self.clear_runtime_verification(entry.id);

        let cancellation = {
            let mut in_flight = self.in_flight.lock();
            if in_flight.contains_key(entry.id) {
                return Err(LocalModelInstallError::busy());
            }
            let cancellation = CancellationToken::new();
            in_flight.insert(
                entry.id.to_string(),
                InFlightInstall {
                    cancellation: cancellation.clone(),
                    phase: LocalModelInstallPhase::Downloading,
                },
            );
            cancellation
        };

        let pending_result = self
            .install_inner(entry, &cancellation, progress.as_ref())
            .await;
        let result = match pending_result {
            Ok(outcome) => {
                // The install marker is the durable commit point. Keep the in-flight mutex
                // locked while deciding whether to write it so cancel() has a single,
                // truthful linearization point: either cancellation is accepted before the
                // marker commit, or the operation is no longer cancellable.
                self.notify_promotion_checkpoint(PromotionCheckpoint::BeforeMarkerCommit);
                let mut in_flight = self.in_flight.lock();
                let cancelled = in_flight
                    .get(entry.id)
                    .map(|install| install.cancellation.is_cancelled())
                    .unwrap_or(true);
                let finalized = if cancelled {
                    let _ = remove_pending_artifact(&self.root, entry);
                    Err(LocalModelInstallError::cancelled())
                } else {
                    let revision_dir = self.root.join(entry.id).join(entry.revision);
                    match write_install_marker(&revision_dir, entry) {
                        Ok(()) => Ok(outcome),
                        Err(error) => {
                            let _ = remove_pending_artifact(&self.root, entry);
                            Err(error)
                        }
                    }
                };
                in_flight.remove(entry.id);
                finalized
            }
            Err(error) => {
                self.in_flight.lock().remove(entry.id);
                Err(error)
            }
        };

        match &result {
            Ok(_) => self.clear_install_error(entry.id),
            Err(error) => self.record_install_error(entry.id, error.clone()),
        }
        result
    }

    pub fn delete(&self, model_id: &str) -> Result<(), LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let entry =
            local_model_entry(model_id).ok_or_else(LocalModelInstallError::unknown_model)?;
        if self.in_flight.lock().contains_key(model_id) {
            return Err(LocalModelInstallError::busy());
        }
        let model_dir = self.root.join(entry.id);
        match fs::symlink_metadata(&model_dir) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                fs::remove_file(&model_dir)
                    .map_err(|_| LocalModelInstallError::io("remove the local model link"))?;
            }
            Ok(metadata) if metadata.is_dir() => {
                fs::remove_dir_all(&model_dir)
                    .map_err(|_| LocalModelInstallError::io("delete the local model"))?;
            }
            Ok(_) => return Err(LocalModelInstallError::corrupt_install()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(LocalModelInstallError::io(
                    "inspect the local model directory",
                ))
            }
        }
        self.clear_install_error(model_id);
        self.clear_runtime_verification(model_id);
        Ok(())
    }

    async fn install_inner(
        &self,
        entry: &'static LocalModelCatalogEntry,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<LocalModelInstallOutcome, LocalModelInstallError> {
        validate_storage_layout(&self.root)?;
        let staging_path =
            self.root
                .join(STAGING_DIR)
                .join(format!("{}-{}.partial", entry.id, Uuid::new_v4()));
        let result = async {
            self.set_in_flight_phase(entry.id, LocalModelInstallPhase::Downloading);
            self.transport
                .download(entry, &staging_path, cancellation, progress)
                .await?;
            if cancellation.is_cancelled() {
                return Err(LocalModelInstallError::cancelled());
            }

            self.set_in_flight_phase(entry.id, LocalModelInstallPhase::Verifying);
            if let Some(callback) = progress {
                callback(LocalModelInstallProgress {
                    model_id: entry.id.to_string(),
                    install_state: LocalModelInstallState::Verifying,
                    downloaded_bytes: entry.expected_bytes,
                    total_bytes: entry.expected_bytes,
                });
            }
            verify_artifact_async(
                staging_path.clone(),
                entry.expected_bytes,
                entry.sha256.to_string(),
                cancellation.clone(),
                self.verification_observer(),
            )
            .await?;
            if cancellation.is_cancelled() {
                return Err(LocalModelInstallError::cancelled());
            }

            validate_storage_layout(&self.root)?;
            self.set_in_flight_phase(entry.id, LocalModelInstallPhase::Promoting);
            self.notify_promotion_checkpoint(PromotionCheckpoint::BeforeRename);
            if cancellation.is_cancelled() {
                return Err(LocalModelInstallError::cancelled());
            }
            let final_path = promote_artifact_file(&self.root, entry, &staging_path)?;
            self.notify_promotion_checkpoint(PromotionCheckpoint::AfterRename);
            if cancellation.is_cancelled() {
                let _ = fs::remove_file(&final_path);
                return Err(LocalModelInstallError::cancelled());
            }

            Ok(LocalModelInstallOutcome {
                model_id: entry.id.to_string(),
                revision: entry.revision.to_string(),
                installed_bytes: entry.expected_bytes,
            })
        }
        .await;
        if staging_path.exists() {
            let _ = fs::remove_file(&staging_path);
        }
        result
    }

    fn descriptor(
        &self,
        entry: &'static LocalModelCatalogEntry,
        selected_model_id: &str,
    ) -> LocalModelDescriptor {
        let error = self.error_state.lock().for_model(entry.id);
        let in_flight_state = self.in_flight_install_state(entry.id);
        let installed = self.marker_shape_is_valid(entry);
        let install_state = if let Some(in_flight_state) = in_flight_state {
            in_flight_state
        } else if installed {
            LocalModelInstallState::Installed
        } else if error.is_some() {
            LocalModelInstallState::Failed
        } else {
            LocalModelInstallState::NotInstalled
        };
        let installed_bytes = installed.then_some(entry.expected_bytes);
        LocalModelDescriptor {
            id: entry.id.to_string(),
            display_name: entry.display_name.to_string(),
            family: entry.family.to_string(),
            parameter_scale: entry.parameter_scale.to_string(),
            quantization: entry.quantization.to_string(),
            revision: entry.revision.to_string(),
            expected_bytes: entry.expected_bytes,
            installed_bytes,
            license: entry.license.to_string(),
            context_limit: entry.context_limit,
            recommended_max_output: entry.recommended_max_output,
            install_state,
            active: entry.id == selected_model_id,
            error,
        }
    }

    fn runtime_artifact_fingerprint(
        &self,
        entry: &'static LocalModelCatalogEntry,
    ) -> Result<(PathBuf, RuntimeArtifactFingerprint), LocalModelInstallError> {
        if !self.marker_shape_is_valid(entry) {
            return Err(LocalModelInstallError::corrupt_install());
        }

        let artifact_path = self
            .root
            .join(entry.id)
            .join(entry.revision)
            .join(entry.artifact_filename);
        let canonical_root =
            fs::canonicalize(&self.root).map_err(|_| LocalModelInstallError::corrupt_install())?;
        let canonical_path = fs::canonicalize(&artifact_path)
            .map_err(|_| LocalModelInstallError::corrupt_install())?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(LocalModelInstallError::corrupt_install());
        }

        let metadata = fs::symlink_metadata(&artifact_path)
            .map_err(|_| LocalModelInstallError::corrupt_install())?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() != entry.expected_bytes
        {
            return Err(LocalModelInstallError::corrupt_install());
        }
        let fingerprint =
            RuntimeArtifactFingerprint::from_metadata(canonical_path.clone(), &metadata);
        Ok((canonical_path, fingerprint))
    }

    pub(crate) fn verified_runtime_artifact_path(
        &self,
        entry: &'static LocalModelCatalogEntry,
    ) -> Result<PathBuf, LocalModelInstallError> {
        let (canonical_path, fingerprint) = match self.runtime_artifact_fingerprint(entry) {
            Ok(result) => result,
            Err(error) => {
                self.clear_runtime_verification(entry.id);
                return Err(error);
            }
        };
        if self
            .runtime_verifications
            .lock()
            .get(entry.id)
            .is_some_and(|cached| cached == &fingerprint)
        {
            return Ok(canonical_path);
        }

        self.notify_runtime_verification();
        if let Err(error) = verify_artifact_cancellable(
            &canonical_path,
            entry.expected_bytes,
            entry.sha256,
            &CancellationToken::new(),
            None,
        ) {
            self.clear_runtime_verification(entry.id);
            return Err(error);
        }

        let (canonical_after, fingerprint_after) = match self.runtime_artifact_fingerprint(entry) {
            Ok(result) => result,
            Err(error) => {
                self.clear_runtime_verification(entry.id);
                return Err(error);
            }
        };
        if fingerprint_after != fingerprint {
            self.clear_runtime_verification(entry.id);
            return Err(LocalModelInstallError::corrupt_install());
        }

        self.runtime_verifications
            .lock()
            .insert(entry.id.to_string(), fingerprint_after);
        Ok(canonical_after)
    }

    #[cfg(test)]
    pub(crate) fn seed_runtime_verification_cache_for_test(
        &self,
        entry: &'static LocalModelCatalogEntry,
    ) -> Result<(), LocalModelInstallError> {
        let (_, fingerprint) = self.runtime_artifact_fingerprint(entry)?;
        self.runtime_verifications
            .lock()
            .insert(entry.id.to_string(), fingerprint);
        Ok(())
    }

    // This is deliberately a fast marker/shape check for UI/status refreshes. Runtime use has a
    // separate cryptographic verification path in `verified_runtime_artifact_path`.
    pub(crate) fn marker_shape_is_valid(&self, entry: &'static LocalModelCatalogEntry) -> bool {
        if validate_storage_layout(&self.root).is_err() {
            return false;
        }
        let model_dir = self.root.join(entry.id);
        let revision_dir = model_dir.join(entry.revision);
        let artifact = revision_dir.join(entry.artifact_filename);
        let marker_path = revision_dir.join(INSTALL_MARKER);

        for directory in [&model_dir, &revision_dir] {
            let Ok(metadata) = fs::symlink_metadata(directory) else {
                return false;
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return false;
            }
        }

        let Ok(artifact_metadata) = fs::symlink_metadata(&artifact) else {
            return false;
        };
        if artifact_metadata.file_type().is_symlink()
            || !artifact_metadata.is_file()
            || artifact_metadata.len() != entry.expected_bytes
        {
            return false;
        }

        let Ok(marker_metadata) = fs::symlink_metadata(&marker_path) else {
            return false;
        };
        if marker_metadata.file_type().is_symlink() || !marker_metadata.is_file() {
            return false;
        }
        let Ok(marker_bytes) = fs::read(marker_path) else {
            return false;
        };
        let Ok(marker): Result<InstallMarker, _> = serde_json::from_slice(&marker_bytes) else {
            return false;
        };
        marker.schema_version == INSTALL_MARKER_VERSION
            && marker.model_id == entry.id
            && marker.revision == entry.revision
            && marker.artifact_filename == entry.artifact_filename
            && marker.expected_bytes == entry.expected_bytes
            && marker.sha256 == entry.sha256
    }
}

pub fn initialize_global_local_model_installer(
    root: PathBuf,
) -> Result<Arc<LocalModelInstaller>, LocalModelInstallError> {
    if let Some(existing) = GLOBAL_LOCAL_MODEL_INSTALLER.get() {
        if existing.root() == root {
            return Ok(existing.clone());
        }
        return Err(LocalModelInstallError::new(
            LocalModelInstallErrorKind::Io,
            "The local model root was already initialized to a different application directory.",
            false,
        ));
    }
    let installer = Arc::new(LocalModelInstaller::new(root)?);
    match GLOBAL_LOCAL_MODEL_INSTALLER.set(installer.clone()) {
        Ok(()) => Ok(installer),
        Err(_) => GLOBAL_LOCAL_MODEL_INSTALLER
            .get()
            .cloned()
            .ok_or_else(|| LocalModelInstallError::io("initialize the local model installer")),
    }
}

pub fn global_local_model_installer() -> Result<Arc<LocalModelInstaller>, LocalModelInstallError> {
    GLOBAL_LOCAL_MODEL_INSTALLER.get().cloned().ok_or_else(|| {
        LocalModelInstallError::new(
            LocalModelInstallErrorKind::Io,
            "The local model installer is not initialized.",
            false,
        )
    })
}

#[cfg(test)]
#[path = "installer/adversarial_tests/mod.rs"]
mod adversarial_tests;

#[cfg(test)]
#[path = "installer/tests.rs"]
mod tests;
