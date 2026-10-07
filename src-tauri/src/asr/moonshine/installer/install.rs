use super::*;

impl MoonshineModelInstaller {
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

    pub(super) async fn delete_installed_manifest(
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
    pub(super) async fn install_manifest(
        &self,
        manifest: &'static MoonshineModelManifest,
        cancellation: &MoonshineModelInstallCancellation,
    ) -> Result<MoonshineModelInstallOutcome, MoonshineModelInstallError> {
        self.install_manifest_with_progress(manifest, cancellation, None)
            .await
    }

    pub(super) async fn install_manifest_with_progress(
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
}
