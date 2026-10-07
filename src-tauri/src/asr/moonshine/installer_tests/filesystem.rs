use super::*;

#[test]
fn verification_of_missing_model_performs_no_network_io() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport.clone());

    let result = installer.verify_installed_manifest(&TEST_MANIFEST).unwrap();

    assert!(result.is_none());
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn successful_repair_replaces_corrupt_directory_only_after_verification() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);
    let final_path = installer.model_path_for_manifest(&TEST_MANIFEST);
    fs::create_dir_all(&final_path).unwrap();
    fs::write(final_path.join("old-corrupt-file"), b"corrupt").unwrap();

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
    assert!(!final_path.join("old-corrupt-file").exists());
    installer
        .verify_manifest_at_path(&final_path, &TEST_MANIFEST)
        .unwrap();
    let parent_entries = fs::read_dir(final_path.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    assert!(!parent_entries
        .iter()
        .any(|name| name.ends_with(".replaced")));
    assert!(!parent_entries.iter().any(|name| name.ends_with(".partial")));
}

#[tokio::test]
async fn stale_partial_directories_are_removed_before_retry() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    fs::create_dir_all(&model_parent).unwrap();
    let stale = model_parent.join(format!(".{}.stale.partial", TEST_MANIFEST.revision));
    fs::create_dir(&stale).unwrap();
    fs::write(stale.join("junk"), b"partial").unwrap();

    installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap();

    assert!(!stale.exists());
}

#[tokio::test]
async fn public_delete_is_architecture_scoped_and_idempotent() {
    let temp = TempDir::new().unwrap();
    let installer = MoonshineModelInstaller::new(temp.path()).unwrap();
    let tiny = installer.model_path(MoonshineModelArchitecture::TinyStreaming);
    let small = installer.model_path(MoonshineModelArchitecture::SmallStreaming);
    std::fs::create_dir_all(&tiny).unwrap();
    std::fs::create_dir_all(&small).unwrap();
    std::fs::write(tiny.join("sentinel"), b"tiny").unwrap();
    std::fs::write(small.join("sentinel"), b"small").unwrap();

    assert!(installer
        .delete_installed(MoonshineModelArchitecture::SmallStreaming)
        .await
        .unwrap());
    assert!(tiny.join("sentinel").is_file());
    assert!(!small.exists());
    assert!(!installer
        .delete_installed(MoonshineModelArchitecture::SmallStreaming)
        .await
        .unwrap());

    assert!(installer
        .delete_installed(MoonshineModelArchitecture::TinyStreaming)
        .await
        .unwrap());
    assert!(!tiny.exists());
}

#[tokio::test]
async fn stale_partial_and_replaced_directories_are_removed_before_retry() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    fs::create_dir_all(&model_parent).unwrap();
    let stale_partial = model_parent.join(format!(".{}.stale.partial", TEST_MANIFEST.revision));
    let stale_replaced = model_parent.join(format!(".{}.stale.replaced", TEST_MANIFEST.revision));
    let unrelated = model_parent.join("keep-me.replaced");
    for path in [&stale_partial, &stale_replaced, &unrelated] {
        fs::create_dir(path).unwrap();
        fs::write(path.join("junk"), b"stale").unwrap();
    }

    installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap();

    assert!(!stale_partial.exists());
    assert!(!stale_replaced.exists());
    assert!(unrelated.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn verification_does_not_follow_swapped_artifact_symlink() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);
    let outcome = installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap();

    let artifact = outcome.model_path.join("adapter.ort");
    let outside = temp.path().join("outside-adapter.ort");
    fs::write(&outside, FILE_A_BYTES).unwrap();
    fs::remove_file(&artifact).unwrap();
    symlink(&outside, &artifact).unwrap();

    let error = installer
        .verify_manifest_at_path(&outcome.model_path, &TEST_MANIFEST)
        .unwrap_err();
    assert_eq!(error.kind, MoonshineModelInstallErrorKind::CorruptInstall);
}
