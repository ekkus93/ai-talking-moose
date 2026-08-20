use super::MoonshineModelInstallError;
use std::fs;
use std::path::Path;

pub(super) fn delete_model_path(path: &Path) -> Result<bool, MoonshineModelInstallError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => {
            return Err(MoonshineModelInstallError::io(
                "inspect the installed Moonshine model before deletion",
            ));
        }
    };

    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
            .map_err(|_| MoonshineModelInstallError::io("delete the Moonshine model"))?;
    } else {
        fs::remove_file(path)
            .map_err(|_| MoonshineModelInstallError::io("delete the Moonshine model"))?;
    }

    if let Some(parent) = path.parent() {
        if fs::read_dir(parent)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
        {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(true)
}
