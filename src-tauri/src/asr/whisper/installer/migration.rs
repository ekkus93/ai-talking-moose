use super::*;

impl WhisperModelInstaller {
    pub(super) fn legacy_model_path(&self) -> Option<PathBuf> {
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

    pub(super) fn migrate_legacy_layout(&self) -> Result<bool, WhisperModelInstallError> {
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

    pub(super) fn migrate_legacy_layout_with_verifier<F>(
        &self,
        legacy_root: &Path,
        verifier: F,
    ) -> Result<bool, WhisperModelInstallError>
    where
        F: Fn(&Path) -> Result<(), WhisperModelInstallError>,
    {
        self.migrate_legacy_layout_with_verifier_and_copier(
            legacy_root,
            verifier,
            |source, target| fs::copy(source, target),
        )
    }

    pub(super) fn migrate_legacy_layout_with_verifier_and_copier<F, C>(
        &self,
        legacy_root: &Path,
        verifier: F,
        copier: C,
    ) -> Result<bool, WhisperModelInstallError>
    where
        F: Fn(&Path) -> Result<(), WhisperModelInstallError>,
        C: FnOnce(&Path, &Path) -> std::io::Result<u64>,
    {
        let legacy_model = legacy_root.join(MODEL_FILENAME);
        if !path_is_regular_file(&legacy_model) {
            return Ok(false);
        }

        verifier(&legacy_model)?;
        self.ensure_install_root()?;

        let model_path = self.model_path();
        let marker_path = self.marker_path();
        if fs::symlink_metadata(&model_path).is_ok() {
            let canonical_marker_is_compatible = path_is_regular_file(&marker_path)
                && fs::read_to_string(&marker_path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<InstallMarker>(&text).ok())
                    .is_some_and(|marker| marker.is_compatible());
            if path_is_regular_file(&model_path)
                && canonical_marker_is_compatible
                && verifier(&model_path).is_ok()
            {
                return Ok(false);
            }
            fs::remove_file(&model_path).map_err(|_| WhisperModelInstallError::promotion())?;
        }
        match fs::symlink_metadata(&marker_path) {
            Ok(_) => {
                fs::remove_file(&marker_path).map_err(|_| WhisperModelInstallError::promotion())?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(WhisperModelInstallError::promotion()),
        }

        let staging_dir = self
            .install_root
            .join(format!("whisper_migration_{}", Uuid::new_v4()));
        fs::create_dir(&staging_dir)
            .map_err(|_| WhisperModelInstallError::io("create the migration staging directory"))?;
        let staged_model = staging_dir.join(MODEL_FILENAME);
        if copier(&legacy_model, &staged_model).is_err() {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(WhisperModelInstallError::promotion());
        }
        if let Err(error) = verifier(&staged_model) {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }
        if fs::rename(&staged_model, &model_path).is_err() {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(WhisperModelInstallError::promotion());
        }

        let staged_marker = staging_dir.join(INSTALL_MARKER_FILE);
        let marker_result = serde_json::to_string(&InstallMarker::new())
            .map_err(|_| WhisperModelInstallError::io("serialize the install marker"))
            .and_then(|text| {
                fs::write(&staged_marker, text)
                    .map_err(|_| WhisperModelInstallError::io("write the install marker"))
            })
            .and_then(|()| {
                fs::rename(&staged_marker, &marker_path)
                    .map_err(|_| WhisperModelInstallError::io("promote the install marker"))
            });
        if let Err(error) = marker_result {
            let _ = fs::remove_file(&model_path);
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }

        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_file(&legacy_model);
        let _ = fs::remove_file(legacy_root.join(INSTALL_MARKER_FILE));
        Ok(true)
    }
}
