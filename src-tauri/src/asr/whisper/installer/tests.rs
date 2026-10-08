use super::*;

#[test]
fn ensure_install_root_creates_clean_profile_directory() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp
        .path()
        .join("models")
        .join("whisper")
        .join("whisper-small");
    assert!(!root.exists());

    let installer = WhisperModelInstaller::new(&root).unwrap();
    installer.ensure_install_root().unwrap();

    assert!(root.is_dir());
    assert_eq!(installer.model_path(), root.join(MODEL_FILENAME));
}

#[test]
fn dropped_install_staging_guard_removes_partial_artifacts() {
    let temp = tempfile::TempDir::new().unwrap();
    let staging = temp.path().join("whisper_staging_interrupted");
    fs::create_dir(&staging).unwrap();
    fs::write(staging.join(MODEL_FILENAME), b"partial").unwrap();

    drop(StagingDirectory(staging.clone()));

    assert!(!staging.exists());
}

#[test]
fn verify_installed_classifies_existing_wrong_size_as_corrupt() {
    for artifact in [b"short".as_slice(), b"oversized".as_slice()] {
        let temp = tempfile::TempDir::new().unwrap();
        let installer = WhisperModelInstaller::new(temp.path()).unwrap();
        installer.ensure_install_root().unwrap();
        fs::write(installer.model_path(), artifact).unwrap();
        fs::write(
            installer.marker_path(),
            serde_json::to_string(&InstallMarker::new()).unwrap(),
        )
        .unwrap();

        let error = installer.verify_installed().unwrap_err();
        assert_eq!(error.kind, WhisperModelInstallErrorKind::CorruptInstall);
        assert_eq!(
            map_whisper_failure(WhisperFailure::Verification(error)).kind,
            AsrErrorKind::ModelCorrupt
        );
    }
}

#[test]
fn existing_model_without_marker_is_corrupt_not_missing() {
    let temp = tempfile::TempDir::new().unwrap();
    let installer = WhisperModelInstaller::new(temp.path()).unwrap();
    installer.ensure_install_root().unwrap();
    fs::write(installer.model_path(), b"partial model").unwrap();

    let error = installer.verify_installed().unwrap_err();
    assert_eq!(error.kind, WhisperModelInstallErrorKind::CorruptInstall);
}

#[cfg(unix)]
#[test]
fn installed_model_symlink_is_corrupt_without_following_target() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::TempDir::new().unwrap();
    let installer = WhisperModelInstaller::new(temp.path()).unwrap();
    installer.ensure_install_root().unwrap();
    let target = temp.path().join("outside-model");
    fs::write(&target, b"outside").unwrap();
    symlink(&target, installer.model_path()).unwrap();

    let error = installer.verify_installed().unwrap_err();

    assert_eq!(error.kind, WhisperModelInstallErrorKind::CorruptInstall);
    assert_eq!(fs::read(target).unwrap(), b"outside");
}

#[cfg(unix)]
#[test]
fn staged_marker_promotion_replaces_symlink_without_writing_through_it() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::TempDir::new().unwrap();
    let staging = temp.path().join("stage");
    fs::create_dir(&staging).unwrap();
    let external_target = temp.path().join("outside-marker");
    fs::write(&external_target, b"unchanged").unwrap();
    let marker_path = temp.path().join(INSTALL_MARKER_FILE);
    symlink(&external_target, &marker_path).unwrap();

    write_install_marker_staged(&staging, &marker_path).unwrap();

    assert_eq!(fs::read(external_target).unwrap(), b"unchanged");
    assert!(path_is_regular_file(&marker_path));
    assert!(
        serde_json::from_str::<InstallMarker>(&fs::read_to_string(marker_path).unwrap()).is_ok()
    );
}

#[test]
fn marker_v1_remains_compatible_for_verified_existing_install() {
    let marker = InstallMarker {
        schema_version: 1,
        model_id: manifest::WHISPER_SMALL_ID.to_string(),
        revision: manifest::WHISPER_SOURCE_COMMIT.to_string(),
        expected_bytes: manifest::WHISPER_MODEL_BYTES,
        runtime_release: manifest::WHISPER_RUNTIME_RELEASE.to_string(),
        runtime_commit: manifest::WHISPER_SOURCE_COMMIT.to_string(),
        source_commit: None,
    };
    assert!(marker.is_compatible());
    let current = InstallMarker::new();
    assert_eq!(current.revision, manifest::WHISPER_MODEL_REVISION);
    assert_eq!(
        current.source_commit.as_deref(),
        Some(manifest::WHISPER_SOURCE_COMMIT)
    );
}

#[test]
fn legacy_parent_layout_is_detected_only_for_canonical_whisper_small_root() {
    let temp = tempfile::TempDir::new().unwrap();
    let canonical = temp
        .path()
        .join("models")
        .join("whisper")
        .join("whisper-small");
    let installer = WhisperModelInstaller::new(&canonical).unwrap();
    assert_eq!(
        installer.legacy_model_path().unwrap(),
        temp.path()
            .join("models")
            .join("whisper")
            .join(MODEL_FILENAME)
    );

    let arbitrary = WhisperModelInstaller::new(temp.path().join("other")).unwrap();
    assert!(arbitrary.legacy_model_path().is_none());
}

#[test]
fn legacy_layout_migration_rewrites_marker_and_preserves_model_bytes() {
    let temp = tempfile::TempDir::new().unwrap();
    let legacy_root = temp.path().join("models").join("whisper");
    let new_root = legacy_root.join("whisper-small");
    fs::create_dir_all(&legacy_root).unwrap();
    let legacy_model = legacy_root.join(MODEL_FILENAME);
    fs::write(&legacy_model, b"verified-fixture").unwrap();
    fs::write(
        legacy_root.join(INSTALL_MARKER_FILE),
        r#"{"schema_version":1,"model_id":"old","revision":"old","expected_bytes":1,"runtime_release":"old","runtime_commit":"old"}"#,
    )
    .unwrap();

    let installer = WhisperModelInstaller::new(&new_root).unwrap();
    let migrated = installer
        .migrate_legacy_layout_with_verifier(&legacy_root, |_path| Ok(()))
        .unwrap();

    assert!(migrated);
    assert!(!legacy_model.exists());
    assert_eq!(
        fs::read(installer.model_path()).unwrap(),
        b"verified-fixture"
    );
    assert!(!legacy_root.join(INSTALL_MARKER_FILE).exists());

    let marker: InstallMarker =
        serde_json::from_str(&fs::read_to_string(new_root.join(INSTALL_MARKER_FILE)).unwrap())
            .unwrap();
    assert_eq!(marker, InstallMarker::new());
}

#[test]
fn legacy_layout_migration_does_not_replace_existing_canonical_model() {
    let temp = tempfile::TempDir::new().unwrap();
    let legacy_root = temp.path().join("models").join("whisper");
    let new_root = legacy_root.join("whisper-small");
    fs::create_dir_all(&new_root).unwrap();
    fs::write(legacy_root.join(MODEL_FILENAME), b"legacy").unwrap();
    fs::write(new_root.join(MODEL_FILENAME), b"canonical").unwrap();
    fs::write(
        new_root.join(INSTALL_MARKER_FILE),
        serde_json::to_string(&InstallMarker::new()).unwrap(),
    )
    .unwrap();

    let installer = WhisperModelInstaller::new(&new_root).unwrap();
    let migrated = installer
        .migrate_legacy_layout_with_verifier(&legacy_root, |_path| Ok(()))
        .unwrap();

    assert!(!migrated);
    assert_eq!(fs::read(installer.model_path()).unwrap(), b"canonical");
    assert_eq!(
        fs::read(legacy_root.join(MODEL_FILENAME)).unwrap(),
        b"legacy"
    );
}

#[test]
fn legacy_migration_replaces_partial_canonical_and_is_retryable() {
    let temp = tempfile::TempDir::new().unwrap();
    let legacy_root = temp.path().join("models").join("whisper");
    let new_root = legacy_root.join("whisper-small");
    fs::create_dir_all(&new_root).unwrap();
    let legacy_model = legacy_root.join(MODEL_FILENAME);
    fs::write(&legacy_model, b"verified legacy fixture").unwrap();
    fs::write(new_root.join(MODEL_FILENAME), b"partial canonical").unwrap();

    let installer = WhisperModelInstaller::new(&new_root).unwrap();
    let migrated = installer
        .migrate_legacy_layout_with_verifier(&legacy_root, |path| {
            if fs::read(path).unwrap() == b"verified legacy fixture" {
                Ok(())
            } else {
                Err(WhisperModelInstallError::corrupt_install())
            }
        })
        .unwrap();

    assert!(migrated);
    assert_eq!(
        fs::read(installer.model_path()).unwrap(),
        b"verified legacy fixture"
    );
    assert!(!legacy_model.exists());
}

#[test]
fn legacy_migration_failure_retains_source_for_retry() {
    let temp = tempfile::TempDir::new().unwrap();
    let legacy_root = temp.path().join("models").join("whisper");
    let new_root = legacy_root.join("whisper-small");
    fs::create_dir_all(&new_root).unwrap();
    let legacy_model = legacy_root.join(MODEL_FILENAME);
    fs::write(&legacy_model, b"verified fixture").unwrap();
    let installer = WhisperModelInstaller::new(&new_root).unwrap();
    let marker_path = installer.marker_path();
    fs::create_dir(&marker_path).unwrap();

    let failed = installer
        .migrate_legacy_layout_with_verifier(&legacy_root, |_| Ok(()))
        .unwrap_err();
    assert_eq!(failed.kind, WhisperModelInstallErrorKind::Promotion);
    assert!(legacy_model.exists());
    assert!(!installer.model_path().exists());

    fs::remove_dir(&marker_path).unwrap();
    assert!(installer
        .migrate_legacy_layout_with_verifier(&legacy_root, |_| Ok(()))
        .unwrap());
    assert_eq!(
        fs::read(installer.model_path()).unwrap(),
        b"verified fixture"
    );
}

#[test]
fn interrupted_legacy_copy_is_cleaned_and_can_be_retried() {
    let temp = tempfile::TempDir::new().unwrap();
    let legacy_root = temp.path().join("models").join("whisper");
    let new_root = legacy_root.join("whisper-small");
    fs::create_dir_all(&new_root).unwrap();
    let legacy_model = legacy_root.join(MODEL_FILENAME);
    fs::write(&legacy_model, b"verified source remains available").unwrap();
    let installer = WhisperModelInstaller::new(&new_root).unwrap();

    let failed = installer
        .migrate_legacy_layout_with_verifier_and_copier(
            &legacy_root,
            |_| Ok(()),
            |_source, target| {
                fs::write(target, b"partial copy")?;
                Err(std::io::Error::other("injected interrupted copy"))
            },
        )
        .unwrap_err();
    assert_eq!(failed.kind, WhisperModelInstallErrorKind::Promotion);
    assert_eq!(
        fs::read(&legacy_model).unwrap(),
        b"verified source remains available"
    );
    assert!(!installer.model_path().exists());
    assert!(fs::read_dir(&new_root).unwrap().next().is_none());

    assert!(installer
        .migrate_legacy_layout_with_verifier(&legacy_root, |_| Ok(()))
        .unwrap());
    assert_eq!(
        fs::read(installer.model_path()).unwrap(),
        b"verified source remains available"
    );
}

#[tokio::test]
async fn active_verified_model_lease_blocks_delete_until_released() {
    let temp = tempfile::TempDir::new().unwrap();
    let installer = Arc::new(WhisperModelInstaller::new(temp.path()).unwrap());
    fs::write(installer.model_path(), b"leased model fixture").unwrap();
    let operation_lock = install_operation_lock(manifest::WHISPER_SMALL_ID);
    let lease = WhisperVerifiedModelLease {
        model_path: installer.model_path(),
        _operation_guard: operation_lock.clone().lock_owned().await,
    };

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let delete_installer = installer.clone();
    let delete_task = tokio::spawn(async move {
        let _ = started_tx.send(());
        delete_installer.delete().await
    });
    started_rx.await.unwrap();

    assert!(installer.model_path().exists());
    assert!(
        !delete_task.is_finished(),
        "delete must wait for the active model lease"
    );
    drop(lease);
    delete_task.await.unwrap().unwrap();
    assert!(!installer.model_path().exists());
}

#[test]
fn public_error_mapping_covers_all_whisper_engine_failure_classes() {
    for (failure, expected) in [
        (
            WhisperFailure::ModelNotInstalled,
            AsrErrorKind::ModelNotInstalled,
        ),
        (
            WhisperFailure::RuntimeUnavailable,
            AsrErrorKind::RuntimeUnavailable,
        ),
        (
            WhisperFailure::ModelLoad("load failed".to_string()),
            AsrErrorKind::ModelLoadFailed,
        ),
        (WhisperFailure::AudioInput, AsrErrorKind::AudioInput),
        (
            WhisperFailure::Inference("inference failed".to_string()),
            AsrErrorKind::Inference,
        ),
        (
            WhisperFailure::InvalidState("invalid state".to_string()),
            AsrErrorKind::InvalidState,
        ),
        (
            WhisperFailure::Internal("internal failure".to_string()),
            AsrErrorKind::Internal,
        ),
    ] {
        assert_eq!(map_whisper_failure(failure).kind, expected);
    }

    for kind in [
        WhisperModelInstallErrorKind::CorruptInstall,
        WhisperModelInstallErrorKind::SizeMismatch,
        WhisperModelInstallErrorKind::Sha256Mismatch,
        WhisperModelInstallErrorKind::IncompatibleInstall,
    ] {
        let mapped = map_whisper_failure(WhisperFailure::Verification(
            WhisperModelInstallError::new(kind, "integrity", true),
        ));
        assert_eq!(mapped.kind, AsrErrorKind::ModelCorrupt);
    }

    let cancelled = map_whisper_failure(WhisperFailure::Verification(
        WhisperModelInstallError::cancelled(),
    ));
    assert_eq!(cancelled.kind, AsrErrorKind::Internal);

    let invalid = map_whisper_failure(WhisperFailure::Verification(
        WhisperModelInstallError::invalid_manifest(),
    ));
    assert_eq!(invalid.kind, AsrErrorKind::Internal);

    let io = map_whisper_failure(WhisperFailure::Verification(WhisperModelInstallError::io(
        "read the model",
    )));
    assert_eq!(io.kind, AsrErrorKind::Internal);

    for kind in [
        WhisperModelInstallErrorKind::Network,
        WhisperModelInstallErrorKind::Http,
        WhisperModelInstallErrorKind::Promotion,
        WhisperModelInstallErrorKind::InsufficientDiskSpace,
    ] {
        let mapped = map_whisper_failure(WhisperFailure::Verification(
            WhisperModelInstallError::new(kind, "unexpected", true),
        ));
        assert_eq!(mapped.kind, AsrErrorKind::Internal);
    }
}

#[test]
fn installer_struct_compiles() {
    // Just a compile test to ensure the struct is well-formed.
    let _ = std::mem::size_of::<WhisperModelInstaller>();
}
