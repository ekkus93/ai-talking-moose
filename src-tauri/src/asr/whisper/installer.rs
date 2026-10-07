        WhisperModelInstallErrorKind::Cancelled => AsrError {
            kind: AsrErrorKind::Cancelled,
            message: "The Whisper Small model verification was cancelled.".to_string(),
            retryable: true,
        },
        WhisperModelInstallErrorKind::InvalidManifest => AsrError {
            kind: AsrErrorKind::Internal,
            message: "The bundled Whisper Small model metadata is invalid. Update or reinstall the application.".to_string(),
            retryable: false,
        },
        _ => AsrError {
            kind: AsrErrorKind::ModelLoadFailed,
            message: format!(
                "Whisper Small could not be verified before local speech recognition started. {0}",
                error.message
            ),
            retryable: true,
        },
    }
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
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
            temp.path().join("models").join("whisper").join(MODEL_FILENAME)
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
        assert_eq!(fs::read(installer.model_path()).unwrap(), b"verified-fixture");
        assert!(!legacy_root.join(INSTALL_MARKER_FILE).exists());

        let marker: InstallMarker = serde_json::from_str(
            &fs::read_to_string(new_root.join(INSTALL_MARKER_FILE)).unwrap(),
        )
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
    fn public_error_mapping_preserves_integrity_cancellation_and_internal_classes() {
        for kind in [
            WhisperModelInstallErrorKind::CorruptInstall,
            WhisperModelInstallErrorKind::SizeMismatch,
            WhisperModelInstallErrorKind::Sha256Mismatch,
        ] {
            let mapped = map_install_error(WhisperModelInstallError::new(kind, "integrity", true));
            assert_eq!(mapped.kind, AsrErrorKind::ModelCorrupt);
        }

        let cancelled = map_install_error(WhisperModelInstallError::cancelled());
        assert_eq!(cancelled.kind, AsrErrorKind::Cancelled);

        let invalid = map_install_error(WhisperModelInstallError::invalid_manifest());
        assert_eq!(invalid.kind, AsrErrorKind::Internal);

        for kind in [
            WhisperModelInstallErrorKind::Network,
            WhisperModelInstallErrorKind::Http,
            WhisperModelInstallErrorKind::Io,
            WhisperModelInstallErrorKind::Promotion,
            WhisperModelInstallErrorKind::InsufficientDiskSpace,
        ] {
            let mapped = map_install_error(WhisperModelInstallError::new(kind, "load", true));
            assert_eq!(mapped.kind, AsrErrorKind::ModelLoadFailed);
        }
    }

    #[test]
    fn installer_struct_compiles() {
        // Just a compile test to ensure the struct is well-formed.
        let _ = std::mem::size_of::<WhisperModelInstaller>();
    }
}
