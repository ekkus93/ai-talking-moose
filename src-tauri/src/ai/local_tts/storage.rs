use super::manifest::{
    local_tts_model_manifest, local_tts_platform_artifact, validate_local_tts_model_catalog,
    LocalTtsArtifact, LocalTtsModelManifest, LocalTtsPlatform,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

const STAGING_DIR: &str = ".staging";
pub(super) const INSTALL_MARKER: &str = ".talking-moose-local-tts.json";
const INSTALL_MARKER_SCHEMA_VERSION: u32 = 1;
const MAX_INSTALL_MARKER_BYTES: u64 = 64 * 1024;

static GLOBAL_LOCAL_TTS_STORAGE: OnceLock<Arc<LocalTtsStorage>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsInstallState {
    NotInstalled,
    Downloading,
    Verifying,
    Promoting,
    Installed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTtsStatusError {
    pub message: String,
    pub retryable: bool,
}

impl LocalTtsStatusError {
    fn invalid_catalog() -> Self {
        Self {
            message: "The bundled Local TTS artifact catalog is invalid.".to_string(),
            retryable: false,
        }
    }

    fn unknown_model() -> Self {
        Self {
            message: "The selected Local TTS model is not in the supported catalog.".to_string(),
            retryable: false,
        }
    }

    fn corrupt_install() -> Self {
        Self {
            message: "The Local TTS installation is incomplete or invalid.".to_string(),
            retryable: true,
        }
    }

    fn storage() -> Self {
        Self {
            message: "The Local TTS model storage is unavailable or unsafe.".to_string(),
            retryable: false,
        }
    }
}

impl fmt::Display for LocalTtsStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LocalTtsStatusError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTtsModelStatus {
    pub model_id: String,
    pub storage_id: String,
    pub revision: String,
    pub expected_bytes: u64,
    pub installed_bytes: Option<u64>,
    pub install_state: LocalTtsInstallState,
    pub error: Option<LocalTtsStatusError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct InstallMarkerArtifact {
    filename: String,
    expected_bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct InstallMarker {
    schema_version: u32,
    storage_id: String,
    provider_model_id: String,
    model_source_revision: String,
    runtime_compatibility_version: u32,
    platform: String,
    artifacts: Vec<InstallMarkerArtifact>,
}

pub struct LocalTtsStorage {
    root: PathBuf,
}

impl LocalTtsStorage {
    pub fn new(root: PathBuf) -> Result<Self, LocalTtsStatusError> {
        validate_local_tts_model_catalog().map_err(|_| LocalTtsStatusError::invalid_catalog())?;
        prepare_storage_root(&root)?;
        ensure_plain_directory(&root.join(STAGING_DIR))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn staging_root(&self) -> PathBuf {
        self.root.join(STAGING_DIR)
    }

    pub fn model_revision_dir(
        &self,
        provider_model_id: &str,
    ) -> Result<PathBuf, LocalTtsStatusError> {
        let manifest = local_tts_model_manifest(provider_model_id)
            .ok_or_else(LocalTtsStatusError::unknown_model)?;
        Ok(self
            .root
            .join(manifest.id)
            .join(manifest.model_source_revision))
    }

    pub fn status(
        &self,
        provider_model_id: &str,
        platform: LocalTtsPlatform,
    ) -> Result<LocalTtsModelStatus, LocalTtsStatusError> {
        let manifest = local_tts_model_manifest(provider_model_id)
            .ok_or_else(LocalTtsStatusError::unknown_model)?;
        let expected_bytes = expected_install_bytes(manifest, platform)?;
        let model_dir = self.root.join(manifest.id);

        if validate_storage_layout(&self.root).is_err() {
            return Ok(status_descriptor(
                manifest,
                expected_bytes,
                None,
                LocalTtsInstallState::Failed,
                Some(LocalTtsStatusError::storage()),
            ));
        }

        match fs::symlink_metadata(&model_dir) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(status_descriptor(
                    manifest,
                    expected_bytes,
                    None,
                    LocalTtsInstallState::NotInstalled,
                    None,
                ));
            }
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => {}
            Ok(_) => {
                return Ok(status_descriptor(
                    manifest,
                    expected_bytes,
                    None,
                    LocalTtsInstallState::Failed,
                    Some(LocalTtsStatusError::corrupt_install()),
                ));
            }
            Err(_) => {
                return Ok(status_descriptor(
                    manifest,
                    expected_bytes,
                    None,
                    LocalTtsInstallState::Failed,
                    Some(LocalTtsStatusError::storage()),
                ));
            }
        }

        if self.marker_shape_is_valid(manifest, platform) {
            return Ok(status_descriptor(
                manifest,
                expected_bytes,
                Some(expected_bytes),
                LocalTtsInstallState::Installed,
                None,
            ));
        }

        Ok(status_descriptor(
            manifest,
            expected_bytes,
            None,
            LocalTtsInstallState::Failed,
            Some(LocalTtsStatusError::corrupt_install()),
        ))
    }

    /// Fast status-only validation. This deliberately checks marker identity, file type, and
    /// expected byte counts without hashing model data. KTT-204 owns cryptographic verification
    /// before first runtime use.
    pub(super) fn marker_shape_is_valid(
        &self,
        manifest: &'static LocalTtsModelManifest,
        platform: LocalTtsPlatform,
    ) -> bool {
        if validate_storage_layout(&self.root).is_err() {
            return false;
        }

        let model_dir = self.root.join(manifest.id);
        let revision_dir = model_dir.join(manifest.model_source_revision);
        for directory in [&model_dir, &revision_dir] {
            let Ok(metadata) = fs::symlink_metadata(directory) else {
                return false;
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return false;
            }
        }

        let Ok(expected_artifacts) = expected_artifacts(manifest, platform) else {
            return false;
        };
        for artifact in &expected_artifacts {
            let path = revision_dir.join(artifact.filename);
            let Ok(metadata) = fs::symlink_metadata(path) else {
                return false;
            };
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() != artifact.expected_bytes
            {
                return false;
            }
        }

        let marker_path = revision_dir.join(INSTALL_MARKER);
        let Ok(marker_metadata) = fs::symlink_metadata(&marker_path) else {
            return false;
        };
        if marker_metadata.file_type().is_symlink()
            || !marker_metadata.is_file()
            || marker_metadata.len() == 0
            || marker_metadata.len() > MAX_INSTALL_MARKER_BYTES
        {
            return false;
        }
        let Ok(marker_bytes) = fs::read(marker_path) else {
            return false;
        };
        let Ok(marker): Result<InstallMarker, _> = serde_json::from_slice(&marker_bytes) else {
            return false;
        };

        marker == install_marker(manifest, platform, &expected_artifacts)
    }
}

pub fn initialize_global_local_tts_storage(
    root: PathBuf,
) -> Result<Arc<LocalTtsStorage>, LocalTtsStatusError> {
    if let Some(existing) = GLOBAL_LOCAL_TTS_STORAGE.get() {
        if existing.root() == root {
            return Ok(existing.clone());
        }
        return Err(LocalTtsStatusError::storage());
    }

    let storage = Arc::new(LocalTtsStorage::new(root)?);
    match GLOBAL_LOCAL_TTS_STORAGE.set(storage.clone()) {
        Ok(()) => Ok(storage),
        Err(_) => GLOBAL_LOCAL_TTS_STORAGE
            .get()
            .cloned()
            .ok_or_else(LocalTtsStatusError::storage),
    }
}

pub fn global_local_tts_storage() -> Result<Arc<LocalTtsStorage>, LocalTtsStatusError> {
    GLOBAL_LOCAL_TTS_STORAGE
        .get()
        .cloned()
        .ok_or_else(LocalTtsStatusError::storage)
}

fn status_descriptor(
    manifest: &LocalTtsModelManifest,
    expected_bytes: u64,
    installed_bytes: Option<u64>,
    install_state: LocalTtsInstallState,
    error: Option<LocalTtsStatusError>,
) -> LocalTtsModelStatus {
    LocalTtsModelStatus {
        model_id: manifest.provider_model_id.to_string(),
        storage_id: manifest.id.to_string(),
        revision: manifest.model_source_revision.to_string(),
        expected_bytes,
        installed_bytes,
        install_state,
        error,
    }
}

pub(super) fn expected_artifacts(
    manifest: &'static LocalTtsModelManifest,
    platform: LocalTtsPlatform,
) -> Result<Vec<&'static LocalTtsArtifact>, LocalTtsStatusError> {
    let runtime = local_tts_platform_artifact(manifest, platform)
        .ok_or_else(LocalTtsStatusError::invalid_catalog)?;
    let mut artifacts = Vec::with_capacity(manifest.common_artifacts.len() + 1);
    artifacts.extend(manifest.common_artifacts.iter());
    artifacts.push(runtime);
    Ok(artifacts)
}

fn expected_install_bytes(
    manifest: &'static LocalTtsModelManifest,
    platform: LocalTtsPlatform,
) -> Result<u64, LocalTtsStatusError> {
    expected_artifacts(manifest, platform)?
        .into_iter()
        .try_fold(0_u64, |total, artifact| {
            total
                .checked_add(artifact.expected_bytes)
                .ok_or_else(LocalTtsStatusError::invalid_catalog)
        })
}

pub(super) fn install_marker(
    manifest: &LocalTtsModelManifest,
    platform: LocalTtsPlatform,
    artifacts: &[&LocalTtsArtifact],
) -> InstallMarker {
    InstallMarker {
        schema_version: INSTALL_MARKER_SCHEMA_VERSION,
        storage_id: manifest.id.to_string(),
        provider_model_id: manifest.provider_model_id.to_string(),
        model_source_revision: manifest.model_source_revision.to_string(),
        runtime_compatibility_version: manifest.runtime.compatibility_version,
        platform: platform_id(platform).to_string(),
        artifacts: artifacts
            .iter()
            .map(|artifact| InstallMarkerArtifact {
                filename: artifact.filename.to_string(),
                expected_bytes: artifact.expected_bytes,
                sha256: artifact.sha256.to_string(),
            })
            .collect(),
    }
}

fn platform_id(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "linux-x86_64",
        LocalTtsPlatform::MacosArm64 => "macos-arm64",
        LocalTtsPlatform::MacosX86_64 => "macos-x86_64",
    }
}

fn prepare_storage_root(root: &Path) -> Result<(), LocalTtsStatusError> {
    let models_dir = root.parent().ok_or_else(LocalTtsStatusError::storage)?;
    let app_data_dir = models_dir
        .parent()
        .ok_or_else(LocalTtsStatusError::storage)?;
    fs::create_dir_all(app_data_dir).map_err(|_| LocalTtsStatusError::storage())?;
    ensure_plain_directory(models_dir)?;
    ensure_plain_directory(root)
}

pub(super) fn validate_storage_layout(root: &Path) -> Result<(), LocalTtsStatusError> {
    let models_dir = root.parent().ok_or_else(LocalTtsStatusError::storage)?;
    let staging = root.join(STAGING_DIR);
    for directory in [models_dir, root, staging.as_path()] {
        let metadata =
            fs::symlink_metadata(directory).map_err(|_| LocalTtsStatusError::storage())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LocalTtsStatusError::storage());
        }
    }
    Ok(())
}

fn ensure_plain_directory(path: &Path) -> Result<(), LocalTtsStatusError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalTtsStatusError::storage());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| LocalTtsStatusError::storage())?;
            let metadata =
                fs::symlink_metadata(path).map_err(|_| LocalTtsStatusError::storage())?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalTtsStatusError::storage());
            }
        }
        Err(_) => return Err(LocalTtsStatusError::storage()),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
