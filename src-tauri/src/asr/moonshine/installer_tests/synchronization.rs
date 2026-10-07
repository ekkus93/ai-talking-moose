use super::*;

#[tokio::test]
async fn cancellation_cleans_partial_download() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport);
    let cancellation = MoonshineModelInstallCancellation::default();
    cancellation.cancel();

    let error = installer
        .install_manifest(&TEST_MANIFEST, &cancellation)
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::Cancelled);
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    assert!(FakeTransport::partial_entries(&model_parent).is_empty());
}

#[tokio::test]
async fn cancellation_during_stream_cleans_partial_download() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let adapter_url = format!("{}/adapter.ort", TEST_MANIFEST.base_url);
    transport
        .responses
        .lock()
        .unwrap()
        .get_mut(&adapter_url)
        .unwrap()
        .cancel_after_chunks = Some(1);
    let installer = installer(&temp, transport);
    let cancellation = MoonshineModelInstallCancellation::default();

    let error = installer
        .install_manifest(&TEST_MANIFEST, &cancellation)
        .await
        .unwrap_err();

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::Cancelled);
    assert!(cancellation.is_cancelled());
    let model_parent = temp.path().join(TEST_MANIFEST.id);
    assert!(FakeTransport::partial_entries(&model_parent).is_empty());
}

#[tokio::test]
async fn cancellation_while_waiting_for_global_install_lock_returns_without_network() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = installer(&temp, transport.clone());
    let cancellation = MoonshineModelInstallCancellation::default();
    let operation_lock = install_operation_lock(TEST_MANIFEST.id);
    let guard = operation_lock.lock().await;
    cancellation.cancel();

    let error = installer
        .install_manifest(&TEST_MANIFEST, &cancellation)
        .await
        .unwrap_err();
    drop(guard);

    assert_eq!(error.kind, MoonshineModelInstallErrorKind::Cancelled);
    assert_eq!(transport.request_count(), 0);
}

#[tokio::test]
async fn concurrent_install_requests_are_serialized() {
    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let first_installer = installer(&temp, transport.clone());
    let second_installer = installer(&temp, transport.clone());
    let first_cancel = MoonshineModelInstallCancellation::default();
    let second_cancel = MoonshineModelInstallCancellation::default();

    let (first, second) = tokio::join!(
        first_installer.install_manifest(&TEST_MANIFEST, &first_cancel),
        second_installer.install_manifest(&TEST_MANIFEST, &second_cancel),
    );
    let first = first.unwrap();
    let second = second.unwrap();
    let dispositions = [first.disposition, second.disposition];

    assert!(dispositions.contains(&MoonshineModelInstallDisposition::Installed));
    assert!(dispositions.contains(&MoonshineModelInstallDisposition::AlreadyInstalled));
    assert_eq!(transport.max_active.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(transport.request_count(), TEST_MANIFEST.files.len());
}

#[test]
fn operation_locks_are_scoped_per_model_id() {
    let tiny_a = install_operation_lock("moonshine-tiny-streaming-en");
    let tiny_b = install_operation_lock("moonshine-tiny-streaming-en");
    let small = install_operation_lock("moonshine-small-streaming-en");

    assert!(Arc::ptr_eq(&tiny_a, &tiny_b));
    assert!(!Arc::ptr_eq(&tiny_a, &small));
}

#[tokio::test]
async fn verified_model_lease_blocks_delete_until_native_load_finishes() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let temp = TempDir::new().unwrap();
    let transport = FakeTransport::with_fixture_manifest();
    let installer = Arc::new(installer(&temp, transport));
    installer
        .install_manifest(
            &TEST_MANIFEST,
            &MoonshineModelInstallCancellation::default(),
        )
        .await
        .unwrap();

    let (lease_acquired_tx, lease_acquired_rx) = mpsc::channel();
    let (release_lease_tx, release_lease_rx) = mpsc::channel();
    let lease_installer = installer.clone();
    let lease_thread = thread::spawn(move || {
        let lease = lease_installer
            .acquire_verified_model_lease_for_manifest(&TEST_MANIFEST)
            .unwrap()
            .expect("fixture install must verify");
        lease_acquired_tx.send(()).unwrap();
        release_lease_rx.recv().unwrap();
        drop(lease);
    });
    lease_acquired_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("verified-model lease should be acquired");

    let delete_installer = installer.clone();
    let delete = tokio::spawn(async move {
        delete_installer
            .delete_installed_manifest(&TEST_MANIFEST)
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !delete.is_finished(),
        "delete must wait while verified model is leased for native load"
    );

    release_lease_tx.send(()).unwrap();
    lease_thread.join().unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(5), delete)
        .await
        .expect("delete should proceed after verified-model lease releases")
        .unwrap()
        .unwrap());
    assert!(!installer.model_path_for_manifest(&TEST_MANIFEST).exists());
}
