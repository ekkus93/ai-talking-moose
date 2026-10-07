use super::types::LocalModelInstallError;
use super::STAGING_DIR;
use std::fs;
use std::path::Path;

pub(super) fn prepare_model_root(root: &Path) -> Result<(), LocalModelInstallError> {
    let parent = root
        .parent()
        .ok_or_else(LocalModelInstallError::corrupt_install)?;
    let anchor = parent
        .parent()
        .ok_or_else(LocalModelInstallError::corrupt_install)?;
    fs::create_dir_all(anchor)
        .map_err(|_| LocalModelInstallError::io("create the local model parent directory"))?;
    ensure_plain_directory(parent, "create the local model parent directory")?;
    ensure_plain_directory(root, "create the model root")
}

pub(super) fn validate_storage_layout(root: &Path) -> Result<(), LocalModelInstallError> {
    let parent = root
        .parent()
        .ok_or_else(LocalModelInstallError::corrupt_install)?;
    let staging = root.join(STAGING_DIR);
    for directory in [parent, root, staging.as_path()] {
        let metadata = fs::symlink_metadata(directory)
            .map_err(|_| LocalModelInstallError::corrupt_install())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LocalModelInstallError::corrupt_install());
        }
    }
    Ok(())
}

pub(super) fn ensure_plain_directory(
    path: &Path,
    create_operation: &'static str,
) -> Result<(), LocalModelInstallError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalModelInstallError::corrupt_install());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| LocalModelInstallError::io(create_operation))?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| LocalModelInstallError::io(create_operation))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LocalModelInstallError::corrupt_install());
            }
        }
        Err(_) => return Err(LocalModelInstallError::io(create_operation)),
    }
    Ok(())
}

pub(super) fn cleanup_stale_staging(staging: &Path) -> Result<(), LocalModelInstallError> {
    let metadata = fs::symlink_metadata(staging)
        .map_err(|_| LocalModelInstallError::io("inspect the model staging directory"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalModelInstallError::corrupt_install());
    }
    for entry in fs::read_dir(staging)
        .map_err(|_| LocalModelInstallError::io("inspect the model staging directory"))?
    {
        let entry = entry.map_err(|_| LocalModelInstallError::io("inspect a staging entry"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| LocalModelInstallError::io("inspect a stale staging entry"))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(path)
                .map_err(|_| LocalModelInstallError::io("remove a stale staging directory"))?;
        } else {
            fs::remove_file(path)
                .map_err(|_| LocalModelInstallError::io("remove a stale staging file"))?;
        }
    }
    Ok(())
}
