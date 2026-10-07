use super::storage::ensure_plain_directory;
use super::types::LocalModelInstallError;
use super::{InstallMarker, INSTALL_MARKER, INSTALL_MARKER_VERSION};
use crate::ai::local::catalog::LocalModelCatalogEntry;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) fn write_install_marker_file(
    marker_tmp: &Path,
    marker_path: &Path,
    marker_bytes: &[u8],
) -> Result<(), LocalModelInstallError> {
    let mut marker_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker_tmp)
        .map_err(|_| LocalModelInstallError::io("create the local model install marker"))?;
    marker_file
        .write_all(marker_bytes)
        .map_err(|_| LocalModelInstallError::io("write the local model install marker"))?;
    marker_file
        .sync_all()
        .map_err(|_| LocalModelInstallError::io("sync the local model install marker"))?;
    drop(marker_file);

    match fs::symlink_metadata(marker_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(LocalModelInstallError::corrupt_install());
        }
        Ok(_) => {
            fs::remove_file(marker_path).map_err(|_| {
                LocalModelInstallError::io("replace the local model install marker")
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(LocalModelInstallError::io(
                "inspect the local model install marker",
            ))
        }
    }
    fs::rename(marker_tmp, marker_path).map_err(|_| LocalModelInstallError::promotion())
}

pub(super) fn write_install_marker(
    revision_dir: &Path,
    entry: &'static LocalModelCatalogEntry,
) -> Result<(), LocalModelInstallError> {
    let marker = InstallMarker {
        schema_version: INSTALL_MARKER_VERSION,
        model_id: entry.id.to_string(),
        revision: entry.revision.to_string(),
        artifact_filename: entry.artifact_filename.to_string(),
        expected_bytes: entry.expected_bytes,
        sha256: entry.sha256.to_string(),
    };
    let marker_bytes = serde_json::to_vec_pretty(&marker)
        .map_err(|_| LocalModelInstallError::io("serialize the local model install marker"))?;
    let marker_tmp = revision_dir.join(format!("{INSTALL_MARKER}.{}.tmp", Uuid::new_v4()));
    let marker_path = revision_dir.join(INSTALL_MARKER);
    let result = write_install_marker_file(&marker_tmp, &marker_path, &marker_bytes);
    if result.is_err() {
        let _ = fs::remove_file(&marker_tmp);
    }
    result
}

pub(super) fn promote_artifact_file(
    root: &Path,
    entry: &'static LocalModelCatalogEntry,
    staging_path: &Path,
) -> Result<PathBuf, LocalModelInstallError> {
    let model_dir = root.join(entry.id);
    ensure_plain_directory(&model_dir, "create the local model directory")?;
    let revision_dir = model_dir.join(entry.revision);
    ensure_plain_directory(&revision_dir, "create the local model revision directory")?;

    let final_path = revision_dir.join(entry.artifact_filename);
    match fs::symlink_metadata(&final_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(LocalModelInstallError::corrupt_install());
        }
        Ok(_) => {
            fs::remove_file(&final_path).map_err(|_| {
                LocalModelInstallError::io("replace the previous local model artifact")
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(LocalModelInstallError::io(
                "inspect the previous local model artifact",
            ));
        }
    }
    fs::rename(staging_path, &final_path).map_err(|_| LocalModelInstallError::promotion())?;
    Ok(final_path)
}

pub(super) fn remove_pending_artifact(
    root: &Path,
    entry: &'static LocalModelCatalogEntry,
) -> Result<(), LocalModelInstallError> {
    let final_path = root
        .join(entry.id)
        .join(entry.revision)
        .join(entry.artifact_filename);
    match fs::symlink_metadata(&final_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LocalModelInstallError::corrupt_install())
        }
        Ok(_) => fs::remove_file(final_path)
            .map_err(|_| LocalModelInstallError::io("remove a cancelled local model artifact")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(LocalModelInstallError::io(
            "inspect a cancelled local model artifact",
        )),
    }
}

#[cfg(test)]
pub(super) fn promote_artifact(
    root: &Path,
    entry: &'static LocalModelCatalogEntry,
    staging_path: &Path,
) -> Result<(), LocalModelInstallError> {
    let final_path = promote_artifact_file(root, entry, staging_path)?;
    let revision_dir = root.join(entry.id).join(entry.revision);
    if let Err(error) = write_install_marker(&revision_dir, entry) {
        let _ = fs::remove_file(&final_path);
        return Err(error);
    }
    Ok(())
}
