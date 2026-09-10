use super::{LocalTtsInstallError, LocalTtsModelManifest, LocalTtsPlatform};
use crate::ai::local_tts::storage::{install_marker, INSTALL_MARKER};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) fn prepare_existing_target(
    root: &Path,
    manifest: &'static LocalTtsModelManifest,
) -> Result<(), LocalTtsInstallError> {
    let model_dir = root.join(manifest.id);
    match fs::symlink_metadata(&model_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(LocalTtsInstallError::corrupt_install())
        }
        Ok(_) => remove_model_dir(root, manifest),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(LocalTtsInstallError::io(
            "inspect the existing Local TTS model directory",
        )),
    }
}

pub(super) fn promote_operation_dir(
    root: &Path,
    manifest: &'static LocalTtsModelManifest,
    operation_dir: &Path,
) -> Result<PathBuf, LocalTtsInstallError> {
    let model_dir = root.join(manifest.id);
    ensure_plain_directory(&model_dir)?;
    let revision_dir = model_dir.join(manifest.model_source_revision);
    match fs::symlink_metadata(&revision_dir) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => return Err(LocalTtsInstallError::corrupt_install()),
        Err(_) => {
            return Err(LocalTtsInstallError::io(
                "inspect the Local TTS revision directory",
            ));
        }
    }
    fs::rename(operation_dir, &revision_dir).map_err(|_| LocalTtsInstallError::promotion())?;
    Ok(revision_dir)
}

pub(super) fn write_install_marker(
    revision_dir: &Path,
    manifest: &'static LocalTtsModelManifest,
    platform: LocalTtsPlatform,
    artifacts: &[&crate::ai::local_tts::manifest::LocalTtsArtifact],
) -> Result<(), LocalTtsInstallError> {
    let marker = install_marker(manifest, platform, artifacts);
    let marker_bytes = serde_json::to_vec_pretty(&marker)
        .map_err(|_| LocalTtsInstallError::io("serialize the Local TTS install marker"))?;
    let marker_tmp = revision_dir.join(format!("{INSTALL_MARKER}.{}.tmp", Uuid::new_v4()));
    let marker_path = revision_dir.join(INSTALL_MARKER);
    let mut marker_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker_tmp)
        .map_err(|_| LocalTtsInstallError::io("create the Local TTS install marker"))?;
    marker_file
        .write_all(&marker_bytes)
        .map_err(|_| LocalTtsInstallError::io("write the Local TTS install marker"))?;
    marker_file
        .sync_all()
        .map_err(|_| LocalTtsInstallError::io("sync the Local TTS install marker"))?;
    drop(marker_file);

    match fs::symlink_metadata(&marker_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => {
            let _ = fs::remove_file(&marker_tmp);
            return Err(LocalTtsInstallError::corrupt_install());
        }
        Err(_) => {
            let _ = fs::remove_file(&marker_tmp);
            return Err(LocalTtsInstallError::io(
                "inspect the Local TTS install marker",
            ));
        }
    }
    let result =
        fs::rename(&marker_tmp, &marker_path).map_err(|_| LocalTtsInstallError::promotion());
    if result.is_err() {
        let _ = fs::remove_file(&marker_tmp);
    }
    result
}

pub(super) fn remove_promoted_revision(
    root: &Path,
    manifest: &'static LocalTtsModelManifest,
) -> Result<(), LocalTtsInstallError> {
    let model_dir = root.join(manifest.id);
    let revision_dir = model_dir.join(manifest.model_source_revision);
    match fs::symlink_metadata(&revision_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(LocalTtsInstallError::corrupt_install());
        }
        Ok(_) => fs::remove_dir_all(&revision_dir)
            .map_err(|_| LocalTtsInstallError::io("remove cancelled Local TTS artifacts"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(LocalTtsInstallError::io(
                "inspect cancelled Local TTS artifacts",
            ));
        }
    }
    remove_empty_model_dir(&model_dir)
}

pub(super) fn remove_model_dir(
    root: &Path,
    manifest: &'static LocalTtsModelManifest,
) -> Result<(), LocalTtsInstallError> {
    let model_dir = root.join(manifest.id);
    match fs::symlink_metadata(&model_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(&model_dir)
            .map_err(|_| LocalTtsInstallError::io("remove the Local TTS model link"))?,
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(&model_dir)
            .map_err(|_| LocalTtsInstallError::io("delete the Local TTS model"))?,
        Ok(_) => return Err(LocalTtsInstallError::corrupt_install()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(LocalTtsInstallError::io(
                "inspect the Local TTS model directory",
            ));
        }
    }
    Ok(())
}

fn remove_empty_model_dir(model_dir: &Path) -> Result<(), LocalTtsInstallError> {
    match fs::remove_dir(model_dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(_) => Err(LocalTtsInstallError::io(
            "clean the Local TTS model directory",
        )),
    }
}

fn ensure_plain_directory(path: &Path) -> Result<(), LocalTtsInstallError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalTtsInstallError::corrupt_install());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|_| LocalTtsInstallError::io("create a Local TTS model directory"))?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| LocalTtsInstallError::io("inspect a Local TTS model directory"))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalTtsInstallError::corrupt_install());
            }
        }
        Err(_) => {
            return Err(LocalTtsInstallError::io(
                "inspect a Local TTS model directory",
            ));
        }
    }
    Ok(())
}

pub(super) fn cleanup_stale_staging(staging: &Path) -> Result<(), LocalTtsInstallError> {
    let metadata = fs::symlink_metadata(staging)
        .map_err(|_| LocalTtsInstallError::io("inspect the Local TTS staging directory"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalTtsInstallError::corrupt_install());
    }
    for entry in fs::read_dir(staging)
        .map_err(|_| LocalTtsInstallError::io("inspect the Local TTS staging directory"))?
    {
        let entry = entry.map_err(|_| LocalTtsInstallError::io("inspect a staging entry"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| LocalTtsInstallError::io("inspect a stale staging entry"))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(path)
                .map_err(|_| LocalTtsInstallError::io("remove a stale staging directory"))?;
        } else {
            fs::remove_file(path)
                .map_err(|_| LocalTtsInstallError::io("remove a stale staging file"))?;
        }
    }
    Ok(())
}
