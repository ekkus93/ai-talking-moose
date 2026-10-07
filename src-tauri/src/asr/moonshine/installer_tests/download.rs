use super::*;

#[test]
fn crc32c_implementation_matches_standard_check_vector() {
    let mut crc = Crc32c::default();
    crc.update(b"123456789");
    assert_eq!(crc.finalize(), 0xe306_9283);
    assert_eq!(crc32c_base64(0xe306_9283), "4waSgw==");
}

#[tokio::test]
async fn installs_fixture_atomically_and_never_uses_real_network() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport.clone());
    let cancellation = MoonshineModelInstallCancellation::default();

    let outcome = installer
        .install_manifest(&TEST_MANIFEST, &cancellation)
        .await
        .unwrap();

    assert_eq!(
        outcome.disposition,
        MoonshineModelInstallDisposition::Installed
    );
    assert_eq!(transport.request_count(), TEST_MANIFEST.files.len());
    assert_eq!(transport.max_active.load(AtomicOrdering::SeqCst), 1);
    assert!(outcome.model_path.join("adapter.ort").is_file());
    assert!(outcome.model_path.join("tokenizer.bin").is_file());
    assert!(outcome.model_path.join(INSTALL_MARKER_FILE).is_file());
    assert!(FakeTransport::partial_entries(outcome.model_path.parent().unwrap()).is_empty());
    installer
        .verify_manifest_at_path(&outcome.model_path, &TEST_MANIFEST)
        .unwrap();
}

#[tokio::test]
async fn known_good_install_is_not_replaced_or_redownloaded() {
    let temp = TempDir::new().unwrap();
    let first_transport = FakeTransport::with_fixture_manifest();
    let first_installer = installer(&temp, first_transport);
    let cancellation = MoonshineModelInstallCancellation::default();
    first_installer
        .install_manifest(&TEST_MANIFEST, &cancellation)
        .await
        .unwrap();

    let second_transport = FakeTransport::with_fixture_manifest();
    let second_installer = installer(&temp, second_transport.clone());
    let outcome = second_installer
        .install_manifest(&TEST_MANIFEST, &cancellation)
        .await
        .unwrap();

    assert_eq!(
        outcome.disposition,
        MoonshineModelInstallDisposition::AlreadyInstalled
    );
    assert_eq!(second_transport.request_count(), 0);
}

#[tokio::test]
async fn sha256_mismatch_cleans_staging_and_preserves_existing_install() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport.clone());
    let final_path = installer.model_path_for_manifest(&TEST_MANIFEST);
    fs::create_dir_all(&final_path).unwrap();
    fs::write(
        final_path.join("old-corrupt-file"),
        b"keep until replacement succeeds",
    )
    .unwrap();

    transport.add_response("adapter.ort", b"tiny adapter fIxture\n");
    let error = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::Sha256Mismatch);
    assert!(final_path.join("old-corrupt-file").is_file());
    assert!(FakeTransport::partial_entries(final_path.parent().unwrap()).is_empty());
}

#[tokio::test]
async fn crc32c_mismatch_is_rejected() {
    const BAD_CRC_FILES: [MoonshineModelFile; 2] = [
        MoonshineModelFile {
            upstream_crc32c_base64: "AAAAAA==",
            ..TEST_FILES[0]
        },
        TEST_FILES[1],
    ];
    const BAD_CRC_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
        files: &BAD_CRC_FILES,
        ..TEST_MANIFEST
    };
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);

    let error = installer
        .install_manifest(
            &BAD_CRC_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::Crc32cMismatch);
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    assert!(FakeTransport::partial_entries(&model_parent).is_empty());
}

#[tokio::test]
async fn mismatched_revision_is_rejected_before_network() {
    const WRONG_REVISION_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
        revision: "wrong_revision",
        ..TEST_MANIFEST
    };
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport.clone());

    let error = installer
        .install_manifest(
            &WRONG_REVISION_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::InvalidManifest);
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn interrupted_download_cleans_partial_and_can_be_retried_from_scratch() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let adapter_url = format!("{}/adapter.ort", TEST_MANIFEST.base_url);
    transport
        .responses
        .lock()
        .unwrap()
        .get_mut(&adapter_url)
        .unwrap()
        .fail_after_chunks = Some(1);
    let installer = installer(&temp, transport.clone());

    let first_error = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(first_error.kind, MoonshineModelInstallErrorKind::Network);
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    assert!(FakeTransport::partial_entries(&model_parent).is_empty());

    transport
        .responses
        .lock()
        .unwrap()
        .get_mut(&adapter_url)
        .unwrap()
        .fail_after_chunks = None;
    let outcome = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        outcome.disposition,
        MoonshineModelInstallDisposition::Installed
    );
}

#[tokio::test]
async fn untrusted_model_host_is_rejected_before_network() {
    const UNTRUSTED_HOST_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
        base_url: "https://example.invalid/model/tiny-streaming-en/test_revision",
        ..TEST_MANIFEST
    };
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport.clone());

    let error = installer
        .install_manifest(
            &UNTRUSTED_HOST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::InvalidManifest);
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn disk_space_probe_error_fails_closed_before_network() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = MoonshineModelInstaller::with_dependencies(
        temp.path(),
        transport.clone(),
        Arc::new(FailingDiskSpace),
    );

    let error = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::Io);
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn insufficient_disk_space_fails_before_network() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = MoonshineModelInstaller::with_dependencies(
        temp.path(),
        transport.clone(),
        Arc::new(FakeDiskSpace { available: Some(1) }),
    );

    let error = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(
        error.kind,
        MoonshineModelInstallErrorKind::InsufficientDiskSpace
    );
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn native_runtime_artifacts_are_rejected_before_network() {
    const RUNTIME_FILE: [MoonshineModelFile; 1] = [MoonshineModelFile {
        name: "libmoonshine.dylib",
        bytes: 1,
        sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        upstream_crc32c_base64: "AAAAAA==",
    }];
    const RUNTIME_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
        files: &RUNTIME_FILE,
        expected_bytes: 1,
        ..TEST_MANIFEST
    };
    let temp = TempDir::new().unwrap();
    let transport = Arc::new(FakeTransport::default());
    let installer = installer(&temp, transport.clone());

    let error = installer
        .install_manifest(
            &RUNTIME_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(
        error.kind,
        MoonshineModelInstallErrorKind::UnsupportedArtifact
    );
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn content_length_mismatch_is_rejected_and_staging_is_removed() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let adapter_url = format!("{}/adapter.ort", TEST_MANIFEST.base_url);
    transport
        .responses
        .lock()
        .unwrap()
        .get_mut(&adapter_url)
        .unwrap()
        .content_length = Some(999);
    let installer = installer(&temp, transport);

    let error = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::SizeMismatch);
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    assert!(FakeTransport::partial_entries(&model_parent).is_empty());
}

#[tokio::test]
async fn install_progress_reports_download_bytes_then_verification() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);
    let observed = Arc::new(StdMutex::new(Vec::<MoonshineModelInstallProgress>::new()));
    let observed_callback = observed.clone();
    let progress: MoonshineModelInstallProgressCallback = Arc::new(move |update| {
        observed_callback.lock().unwrap().push(update);
    });

    installer
        .install_manifest_with_progress(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
            Some(progress),
        )
        .await
        .unwrap();

    let observed = observed.lock().unwrap();
    assert!(observed.iter().any(|update| {
        update.phase == MoonshineModelInstallPhase::Downloading
            && update.downloaded_bytes > 0
            && update.downloaded_bytes <= TEST_MANIFEST.expected_bytes
    }));
    assert_eq!(
        observed.last(),
        Some(&MoonshineModelInstallProgress {
            phase: MoonshineModelInstallPhase::Verifying,
            downloaded_bytes: TEST_MANIFEST.expected_bytes,
            total_bytes: TEST_MANIFEST.expected_bytes,
            current_file: None,
        })
    );
}
