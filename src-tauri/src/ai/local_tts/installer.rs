mod fs_ops;
mod transport;
mod types;
mod verification;

use super::manifest::{local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform};
use super::storage::{
    expected_artifacts, global_local_tts_storage, validate_storage_layout, LocalTtsInstallState,
    LocalTtsModelStatus, LocalTtsStatusError, LocalTtsStorage,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use fs_ops::{
    cleanup_stale_staging, prepare_existing_target, promote_operation_dir, remove_model_dir,
    remove_promoted_revision, write_install_marker,
};
use transport::{LocalTtsDownloadTransport, ReqwestLocalTtsDownloadTransport};
pub use types::{
    LocalTtsInstallError, LocalTtsInstallErrorKind, LocalTtsInstallOutcome,
    LocalTtsInstallProgress, LocalTtsInstallProgressCallback,
};
use verification::verify_artifact_async;

static GLOBAL_LOCAL_TTS_INSTALLER: OnceLock<Arc<LocalTtsInstaller>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalTtsInstallPhase {
    Downloading,
    Verifying,
    Promoting,
}

#[derive(Clone)]
struct InFlightInstall {
    cancellation: CancellationToken,
    phase: LocalTtsInstallPhase,
}

#[derive(Debug, Clone)]
struct RecordedInstallError {
    sequence: u64,
    error: LocalTtsInstallError,
}

#[derive(Debug, Default)]
struct InstallErrorState {
    next_sequence: u64,
    by_model: HashMap<String, RecordedInstallError>,
}

impl InstallErrorState {
    fn record(&mut self, model_id: &str, error: LocalTtsInstallError) {
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

    fn for_model(&self, model_id: &str) -> Option<LocalTtsInstallError> {
        self.by_model
            .get(model_id)
            .map(|recorded| recorded.error.clone())
    }

    fn latest(&self) -> Option<LocalTtsInstallError> {
        self.by_model
            .values()
            .max_by_key(|recorded| recorded.sequence)
            .map(|recorded| recorded.error.clone())
    }
}

#[derive(Debug)]
struct PromotedInstall {
    outcome: LocalTtsInstallOutcome,
    revision_dir: PathBuf,
}

#[cfg(test)]
type PromotionObserver = Arc<dyn Fn() + Send + Sync>;

pub struct LocalTtsInstaller {
    storage: Arc<LocalTtsStorage>,
    transport: Arc<dyn LocalTtsDownloadTransport>,
    in_flight: Mutex<HashMap<String, InFlightInstall>>,
    error_state: Mutex<InstallErrorState>,
    #[cfg(test)]
    promotion_observer: Mutex<Option<PromotionObserver>>,
}

impl LocalTtsInstaller {
    pub fn new(storage: Arc<LocalTtsStorage>) -> Result<Self, LocalTtsInstallError> {
        validate_storage_layout(storage.root()).map_err(LocalTtsInstallError::from_status)?;
        cleanup_stale_staging(&storage.staging_root())?;
        Ok(Self {
            storage,
            transport: Arc::new(ReqwestLocalTtsDownloadTransport::new()?),
            in_flight: Mutex::new(HashMap::new()),
            error_state: Mutex::new(InstallErrorState::default()),
            #[cfg(test)]
            promotion_observer: Mutex::new(None),
        })
    }

    #[cfg(test)]
    fn with_transport(
        storage: Arc<LocalTtsStorage>,
        transport: Arc<dyn LocalTtsDownloadTransport>,
    ) -> Result<Self, LocalTtsInstallError> {
        validate_storage_layout(storage.root()).map_err(LocalTtsInstallError::from_status)?;
        cleanup_stale_staging(&storage.staging_root())?;
        Ok(Self {
            storage,
            transport,
            in_flight: Mutex::new(HashMap::new()),
            error_state: Mutex::new(InstallErrorState::default()),
            promotion_observer: Mutex::new(None),
        })
    }

    pub fn root(&self) -> &Path {
        self.storage.root()
    }

    pub fn latest_error(&self) -> Option<LocalTtsInstallError> {
        self.error_state.lock().latest()
    }

    /// Return the most recently recorded safe installer error for one model only.
    ///
    /// Diagnostics use this instead of `latest_error()` so a failure for another model can never
    /// be attributed to the selected Local TTS model. The returned error is already reduced to the
    /// typed/safe installer boundary and contains no response body, path, credential, or utterance.
    pub fn error_for_model(&self, model_id: &str) -> Option<LocalTtsInstallError> {
        self.error_state.lock().for_model(model_id)
    }

    pub fn status(
        &self,
        model_id: &str,
        platform: LocalTtsPlatform,
    ) -> Result<LocalTtsModelStatus, LocalTtsInstallError> {
        let mut status = self
            .storage
            .status(model_id, platform)
            .map_err(LocalTtsInstallError::from_status)?;
        if let Some(state) = self.in_flight_install_state(model_id) {
            status.install_state = state;
            status.installed_bytes = None;
            status.error = None;
            return Ok(status);
        }
        if let Some(error) = self.error_state.lock().for_model(model_id) {
            status.install_state = LocalTtsInstallState::Failed;
            status.installed_bytes = None;
            status.error = Some(LocalTtsStatusError {
                message: error.message,
                retryable: error.retryable,
            });
        }
        Ok(status)
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
        platform: LocalTtsPlatform,
        progress: Option<LocalTtsInstallProgressCallback>,
    ) -> Result<LocalTtsInstallOutcome, LocalTtsInstallError> {
        validate_storage_layout(self.storage.root()).map_err(LocalTtsInstallError::from_status)?;
        let manifest =
            local_tts_model_manifest(model_id).ok_or_else(LocalTtsInstallError::unknown_model)?;
        self.install_manifest(manifest, platform, progress).await
    }

    async fn install_manifest(
        &self,
        manifest: &'static LocalTtsModelManifest,
        platform: LocalTtsPlatform,
        progress: Option<LocalTtsInstallProgressCallback>,
    ) -> Result<LocalTtsInstallOutcome, LocalTtsInstallError> {
        validate_storage_layout(self.storage.root()).map_err(LocalTtsInstallError::from_status)?;
        let artifacts = expected_artifacts(manifest, platform)
            .map_err(|_| LocalTtsInstallError::invalid_catalog())?;
        let total_bytes = total_artifact_bytes(&artifacts)?;
        if self.storage.marker_shape_is_valid(manifest, platform) {
            return Ok(LocalTtsInstallOutcome {
                model_id: manifest.provider_model_id.to_string(),
                revision: manifest.model_source_revision.to_string(),
                installed_bytes: total_bytes,
            });
        }

        let cancellation = {
            let mut in_flight = self.in_flight.lock();
            if in_flight.contains_key(manifest.provider_model_id) {
                return Err(LocalTtsInstallError::busy());
            }
            let cancellation = CancellationToken::new();
            in_flight.insert(
                manifest.provider_model_id.to_string(),
                InFlightInstall {
                    cancellation: cancellation.clone(),
                    phase: LocalTtsInstallPhase::Downloading,
                },
            );
            cancellation
        };

        let pending_result = self
            .install_inner(
                manifest,
                &artifacts,
                total_bytes,
                &cancellation,
                progress.as_ref(),
            )
            .await;

        let result = match pending_result {
            Ok(promoted) => {
                self.notify_before_marker_commit();
                let mut in_flight = self.in_flight.lock();
                let cancelled = in_flight
                    .get(manifest.provider_model_id)
                    .map(|install| install.cancellation.is_cancelled())
                    .unwrap_or(true);
                let finalized = if cancelled {
                    let _ = remove_promoted_revision(self.storage.root(), manifest);
                    Err(LocalTtsInstallError::cancelled())
                } else {
                    match write_install_marker(
                        &promoted.revision_dir,
                        manifest,
                        platform,
                        &artifacts,
                    ) {
                        Ok(()) => Ok(promoted.outcome),
                        Err(error) => {
                            let _ = remove_promoted_revision(self.storage.root(), manifest);
                            Err(error)
                        }
                    }
                };
                in_flight.remove(manifest.provider_model_id);
                finalized
            }
            Err(error) => {
                self.in_flight.lock().remove(manifest.provider_model_id);
                Err(error)
            }
        };

        match &result {
            Ok(_) => self.error_state.lock().clear(manifest.provider_model_id),
            Err(error) => self
                .error_state
                .lock()
                .record(manifest.provider_model_id, error.clone()),
        }
        result
    }

    pub fn delete(&self, model_id: &str) -> Result<(), LocalTtsInstallError> {
        validate_storage_layout(self.storage.root()).map_err(LocalTtsInstallError::from_status)?;
        let manifest =
            local_tts_model_manifest(model_id).ok_or_else(LocalTtsInstallError::unknown_model)?;
        if self.in_flight.lock().contains_key(model_id) {
            return Err(LocalTtsInstallError::busy());
        }
        remove_model_dir(self.storage.root(), manifest)?;
        self.error_state.lock().clear(model_id);
        Ok(())
    }

    async fn install_inner(
        &self,
        manifest: &'static LocalTtsModelManifest,
        artifacts: &[&'static super::manifest::LocalTtsArtifact],
        total_bytes: u64,
        cancellation: &CancellationToken,
        progress: Option<&LocalTtsInstallProgressCallback>,
    ) -> Result<PromotedInstall, LocalTtsInstallError> {
        validate_storage_layout(self.storage.root()).map_err(LocalTtsInstallError::from_status)?;
        prepare_existing_target(self.storage.root(), manifest)?;
        let operation_dir =
            self.storage
                .staging_root()
                .join(format!("{}-{}.partial", manifest.id, Uuid::new_v4()));
        fs::create_dir(&operation_dir)
            .map_err(|_| LocalTtsInstallError::io("create the Local TTS staging directory"))?;

        let result = async {
            self.set_in_flight_phase(
                manifest.provider_model_id,
                LocalTtsInstallPhase::Downloading,
            );
            let mut completed_bytes = 0_u64;
            for artifact in artifacts {
                if cancellation.is_cancelled() {
                    return Err(LocalTtsInstallError::cancelled());
                }
                let destination = operation_dir.join(artifact.filename);
                let base = completed_bytes;
                let callback = progress.cloned();
                let model_id = manifest.provider_model_id.to_string();
                let filename = artifact.filename.to_string();
                let artifact_total = artifact.expected_bytes;
                let total = total_bytes;
                let on_artifact_progress = callback.map(|callback| {
                    Arc::new(move |artifact_downloaded: u64| {
                        callback(LocalTtsInstallProgress {
                            model_id: model_id.clone(),
                            artifact_filename: Some(filename.clone()),
                            install_state: LocalTtsInstallState::Downloading,
                            downloaded_bytes: base
                                .saturating_add(artifact_downloaded.min(artifact_total)),
                            total_bytes: total,
                        });
                    }) as Arc<dyn Fn(u64) + Send + Sync>
                });
                self.transport
                    .download(
                        artifact,
                        &destination,
                        cancellation,
                        on_artifact_progress.as_ref(),
                    )
                    .await?;
                completed_bytes = completed_bytes
                    .checked_add(artifact.expected_bytes)
                    .ok_or_else(LocalTtsInstallError::size_mismatch)?;
            }

            if cancellation.is_cancelled() {
                return Err(LocalTtsInstallError::cancelled());
            }
            self.set_in_flight_phase(manifest.provider_model_id, LocalTtsInstallPhase::Verifying);
            emit_phase_progress(
                progress,
                manifest.provider_model_id,
                LocalTtsInstallState::Verifying,
                total_bytes,
            );
            for artifact in artifacts {
                verify_artifact_async(
                    operation_dir.join(artifact.filename),
                    artifact.expected_bytes,
                    artifact.sha256.to_string(),
                    cancellation.clone(),
                )
                .await?;
            }
            if cancellation.is_cancelled() {
                return Err(LocalTtsInstallError::cancelled());
            }

            validate_storage_layout(self.storage.root())
                .map_err(LocalTtsInstallError::from_status)?;
            self.set_in_flight_phase(manifest.provider_model_id, LocalTtsInstallPhase::Promoting);
            emit_phase_progress(
                progress,
                manifest.provider_model_id,
                LocalTtsInstallState::Promoting,
                total_bytes,
            );
            if cancellation.is_cancelled() {
                return Err(LocalTtsInstallError::cancelled());
            }
            let revision_dir =
                promote_operation_dir(self.storage.root(), manifest, &operation_dir)?;
            if cancellation.is_cancelled() {
                let _ = remove_promoted_revision(self.storage.root(), manifest);
                return Err(LocalTtsInstallError::cancelled());
            }

            Ok(PromotedInstall {
                outcome: LocalTtsInstallOutcome {
                    model_id: manifest.provider_model_id.to_string(),
                    revision: manifest.model_source_revision.to_string(),
                    installed_bytes: total_bytes,
                },
                revision_dir,
            })
        }
        .await;

        if operation_dir.exists() {
            let _ = fs::remove_dir_all(&operation_dir);
        }
        result
    }

    fn set_in_flight_phase(&self, model_id: &str, phase: LocalTtsInstallPhase) {
        if let Some(in_flight) = self.in_flight.lock().get_mut(model_id) {
            in_flight.phase = phase;
        }
    }

    fn in_flight_install_state(&self, model_id: &str) -> Option<LocalTtsInstallState> {
        self.in_flight
            .lock()
            .get(model_id)
            .map(|in_flight| match in_flight.phase {
                LocalTtsInstallPhase::Downloading => LocalTtsInstallState::Downloading,
                LocalTtsInstallPhase::Verifying => LocalTtsInstallState::Verifying,
                LocalTtsInstallPhase::Promoting => LocalTtsInstallState::Promoting,
            })
    }

    fn notify_before_marker_commit(&self) {
        #[cfg(test)]
        if let Some(observer) = self.promotion_observer.lock().clone() {
            observer();
        }
    }

    #[cfg(test)]
    fn set_promotion_observer(&self, observer: Option<PromotionObserver>) {
        *self.promotion_observer.lock() = observer;
    }
}

pub fn initialize_global_local_tts_installer(
    storage: Arc<LocalTtsStorage>,
) -> Result<Arc<LocalTtsInstaller>, LocalTtsInstallError> {
    if let Some(existing) = GLOBAL_LOCAL_TTS_INSTALLER.get() {
        if existing.root() == storage.root() {
            return Ok(existing.clone());
        }
        return Err(LocalTtsInstallError::corrupt_install());
    }
    let installer = Arc::new(LocalTtsInstaller::new(storage)?);
    match GLOBAL_LOCAL_TTS_INSTALLER.set(installer.clone()) {
        Ok(()) => Ok(installer),
        Err(_) => GLOBAL_LOCAL_TTS_INSTALLER
            .get()
            .cloned()
            .ok_or_else(|| LocalTtsInstallError::io("initialize the Local TTS installer")),
    }
}

pub fn global_local_tts_installer() -> Result<Arc<LocalTtsInstaller>, LocalTtsInstallError> {
    if let Some(existing) = GLOBAL_LOCAL_TTS_INSTALLER.get() {
        return Ok(existing.clone());
    }
    let storage = global_local_tts_storage().map_err(LocalTtsInstallError::from_status)?;
    initialize_global_local_tts_installer(storage)
}

fn emit_phase_progress(
    progress: Option<&LocalTtsInstallProgressCallback>,
    model_id: &str,
    state: LocalTtsInstallState,
    total_bytes: u64,
) {
    if let Some(callback) = progress {
        callback(LocalTtsInstallProgress {
            model_id: model_id.to_string(),
            artifact_filename: None,
            install_state: state,
            downloaded_bytes: total_bytes,
            total_bytes,
        });
    }
}

fn total_artifact_bytes(
    artifacts: &[&super::manifest::LocalTtsArtifact],
) -> Result<u64, LocalTtsInstallError> {
    artifacts.iter().try_fold(0_u64, |total, artifact| {
        total
            .checked_add(artifact.expected_bytes)
            .ok_or_else(LocalTtsInstallError::invalid_catalog)
    })
}

#[cfg(test)]
mod tests;
