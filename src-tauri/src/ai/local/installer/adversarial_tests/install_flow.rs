use super::*;

#[test]
fn installer_diagnostics_report_the_latest_unresolved_error_chronologically() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();

    installer.record_install_error("first-model", LocalModelInstallError::network());
    installer.record_install_error("second-model", LocalModelInstallError::sha256_mismatch());
    assert_eq!(
        installer.diagnostics().last_error.unwrap().kind,
        LocalModelInstallErrorKind::Sha256Mismatch
    );

    installer.clear_install_error("second-model");
    assert_eq!(
        installer.diagnostics().last_error.unwrap().kind,
        LocalModelInstallErrorKind::Network
    );
}

#[cfg(unix)]
#[test]
fn runtime_verification_caches_unchanged_bytes_and_rejects_same_size_mutation() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    seed_test_shape_install(dir.path());
    assert!(installer.marker_shape_is_valid(&TEST_ENTRY));

    let hash_runs = Arc::new(AtomicUsize::new(0));
    let observed = hash_runs.clone();
    installer.set_runtime_verification_observer(Some(Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    })));

    let first = installer
        .verified_runtime_artifact_path(&TEST_ENTRY)
        .unwrap();
    let second = installer
        .verified_runtime_artifact_path(&TEST_ENTRY)
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(hash_runs.load(Ordering::SeqCst), 1);

    // Ensure ctime advances so this checks cache invalidation rather than the
    // filesystem timestamp resolution on a rapid same-size rewrite.
    std::thread::sleep(std::time::Duration::from_millis(2));
    fs::write(test_artifact_path(dir.path()), b"abd").unwrap();
    assert!(installer.marker_shape_is_valid(&TEST_ENTRY));

    let error = installer
        .verified_runtime_artifact_path(&TEST_ENTRY)
        .unwrap_err();
    assert_eq!(error.kind, LocalModelInstallErrorKind::Sha256Mismatch);
    assert_eq!(hash_runs.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn truncated_artifact_is_rejected_and_staging_is_cleaned() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::with_transport(
        dir.path().to_path_buf(),
        Arc::new(StaticBytesTransport {
            bytes: b"ab",
            wait_for_cancel_after_write: false,
        }),
    )
    .unwrap();
    let cancellation = CancellationToken::new();

    let error = installer
        .install_inner(&TEST_ENTRY, &cancellation, None)
        .await
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::SizeMismatch);
    assert!(staging_is_empty(dir.path()));
}

#[tokio::test]
async fn oversized_artifact_is_rejected_and_staging_is_cleaned() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::with_transport(
        dir.path().to_path_buf(),
        Arc::new(StaticBytesTransport {
            bytes: b"abcd",
            wait_for_cancel_after_write: false,
        }),
    )
    .unwrap();
    let cancellation = CancellationToken::new();

    let error = installer
        .install_inner(&TEST_ENTRY, &cancellation, None)
        .await
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::SizeMismatch);
    assert!(staging_is_empty(dir.path()));
}

#[tokio::test]
async fn same_size_wrong_hash_is_rejected_without_installing_artifact() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::with_transport(
        dir.path().to_path_buf(),
        Arc::new(StaticBytesTransport {
            bytes: b"abd",
            wait_for_cancel_after_write: false,
        }),
    )
    .unwrap();
    let cancellation = CancellationToken::new();

    let error = installer
        .install_inner(&TEST_ENTRY, &cancellation, None)
        .await
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::Sha256Mismatch);
    assert!(staging_is_empty(dir.path()));
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    assert!(!revision_dir.join(TEST_ENTRY.artifact_filename).exists());
    assert!(!revision_dir.join(INSTALL_MARKER).exists());
}

#[tokio::test]
async fn interrupted_download_cleans_partial_staging_file() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::with_transport(
        dir.path().to_path_buf(),
        Arc::new(StaticBytesTransport {
            bytes: b"a",
            wait_for_cancel_after_write: true,
        }),
    )
    .unwrap();
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();

    let install = installer.install_inner(&TEST_ENTRY, &cancellation, None);
    let interrupt = async move {
        tokio::task::yield_now().await;
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(install, interrupt);

    let error = result.unwrap_err();
    assert_eq!(error.kind, LocalModelInstallErrorKind::Cancelled);
    assert!(staging_is_empty(dir.path()));
}

#[tokio::test]
async fn accepted_cancellation_during_download_never_installs() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: true,
            }),
        )
        .unwrap(),
    );
    let accepted = Arc::new(AtomicBool::new(false));
    let callback_installer = installer.clone();
    let callback_accepted = accepted.clone();
    let progress: LocalModelInstallProgressCallback = Arc::new(move |update| {
        if update.install_state == LocalModelInstallState::Downloading {
            callback_accepted.store(callback_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
        }
    });

    let error = installer
        .install_entry(&TEST_ENTRY, Some(progress))
        .await
        .unwrap_err();

    assert!(accepted.load(Ordering::SeqCst));
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &error);
    assert!(!test_artifact_path(dir.path()).exists());
}

#[tokio::test]
async fn accepted_cancellation_after_download_before_verify_never_installs() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: false,
            }),
        )
        .unwrap(),
    );
    let accepted = Arc::new(AtomicBool::new(false));
    let observed_state = Arc::new(StdMutex::new(None));
    let callback_installer = installer.clone();
    let callback_accepted = accepted.clone();
    let callback_state = observed_state.clone();
    let progress: LocalModelInstallProgressCallback = Arc::new(move |update| {
        if update.install_state == LocalModelInstallState::Verifying {
            let descriptor = callback_installer.descriptor(&TEST_ENTRY, TEST_ENTRY.id);
            *callback_state.lock().unwrap() = Some(descriptor.install_state);
            callback_accepted.store(callback_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
        }
    });

    let error = installer
        .install_entry(&TEST_ENTRY, Some(progress))
        .await
        .unwrap_err();

    assert!(accepted.load(Ordering::SeqCst));
    assert_eq!(
        *observed_state.lock().unwrap(),
        Some(LocalModelInstallState::Verifying)
    );
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &error);
    assert!(!test_artifact_path(dir.path()).exists());
}

#[tokio::test(flavor = "current_thread")]
async fn verification_runs_off_executor_and_is_cancellable_mid_hash() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: false,
            }),
        )
        .unwrap(),
    );
    let executor_thread = std::thread::current().id();
    let worker_thread = Arc::new(StdMutex::new(None));
    let observed_state = Arc::new(StdMutex::new(None));
    let accepted = Arc::new(AtomicBool::new(false));
    let observer_installer = installer.clone();
    let observer_worker_thread = worker_thread.clone();
    let observer_state = observed_state.clone();
    let observer_accepted = accepted.clone();
    installer.set_verification_observer(Some(Arc::new(move || {
        *observer_worker_thread.lock().unwrap() = Some(std::thread::current().id());
        let descriptor = observer_installer.descriptor(&TEST_ENTRY, TEST_ENTRY.id);
        *observer_state.lock().unwrap() = Some(descriptor.install_state);
        observer_accepted.store(observer_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
    })));

    let error = installer
        .install_entry(&TEST_ENTRY, None)
        .await
        .unwrap_err();

    assert!(accepted.load(Ordering::SeqCst));
    assert_ne!(*worker_thread.lock().unwrap(), Some(executor_thread));
    assert_eq!(
        *observed_state.lock().unwrap(),
        Some(LocalModelInstallState::Verifying)
    );
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &error);
    assert!(!test_artifact_path(dir.path()).exists());
}

#[tokio::test]
async fn accepted_cancellation_after_verify_before_promotion_never_installs() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: false,
            }),
        )
        .unwrap(),
    );
    let accepted = Arc::new(AtomicBool::new(false));
    let observed_state = Arc::new(StdMutex::new(None));
    let observer_installer = installer.clone();
    let observer_accepted = accepted.clone();
    let observer_state = observed_state.clone();
    installer.set_promotion_observer(Some(Arc::new(move |checkpoint| {
        if checkpoint == PromotionCheckpoint::BeforeRename {
            let descriptor = observer_installer.descriptor(&TEST_ENTRY, TEST_ENTRY.id);
            *observer_state.lock().unwrap() = Some(descriptor.install_state);
            observer_accepted.store(observer_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
        }
    })));

    let error = installer
        .install_entry(&TEST_ENTRY, None)
        .await
        .unwrap_err();

    assert!(accepted.load(Ordering::SeqCst));
    assert_eq!(
        *observed_state.lock().unwrap(),
        Some(LocalModelInstallState::Promoting)
    );
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &error);
    assert!(!test_artifact_path(dir.path()).exists());
}

#[tokio::test]
async fn accepted_cancellation_at_marker_commit_boundary_never_installs() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: false,
            }),
        )
        .unwrap(),
    );
    let accepted = Arc::new(AtomicBool::new(false));
    let boundary_observed = Arc::new(AtomicBool::new(false));
    let root = dir.path().to_path_buf();
    let observer_installer = installer.clone();
    let observer_accepted = accepted.clone();
    let observer_boundary = boundary_observed.clone();
    installer.set_promotion_observer(Some(Arc::new(move |checkpoint| {
        if checkpoint == PromotionCheckpoint::BeforeMarkerCommit {
            observer_boundary.store(
                test_artifact_path(&root).is_file() && !test_marker_path(&root).exists(),
                Ordering::SeqCst,
            );
            observer_accepted.store(observer_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
        }
    })));

    let error = installer
        .install_entry(&TEST_ENTRY, None)
        .await
        .unwrap_err();

    assert!(boundary_observed.load(Ordering::SeqCst));
    assert!(accepted.load(Ordering::SeqCst));
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &error);
    assert!(!test_artifact_path(dir.path()).exists());
}

#[tokio::test]
async fn cancelled_install_can_be_retried_successfully() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: false,
            }),
        )
        .unwrap(),
    );
    let observer_installer = installer.clone();
    installer.set_verification_observer(Some(Arc::new(move || {
        assert!(observer_installer.cancel(TEST_ENTRY.id));
    })));

    let first_error = installer
        .install_entry(&TEST_ENTRY, None)
        .await
        .unwrap_err();
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &first_error);

    installer.set_verification_observer(None);
    installer.set_promotion_observer(None);
    let outcome = installer.install_entry(&TEST_ENTRY, None).await.unwrap();

    assert_eq!(outcome.model_id, TEST_ENTRY.id);
    assert!(installer.marker_shape_is_valid(&TEST_ENTRY));
    assert_eq!(fs::read(test_artifact_path(dir.path())).unwrap(), b"abc");
    assert!(test_marker_path(dir.path()).is_file());
    assert!(staging_is_empty(dir.path()));
}

#[tokio::test]
async fn duplicate_cancellation_requests_are_idempotent_while_active() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(
        LocalModelInstaller::with_transport(
            dir.path().to_path_buf(),
            Arc::new(StaticBytesTransport {
                bytes: b"abc",
                wait_for_cancel_after_write: true,
            }),
        )
        .unwrap(),
    );
    let first = Arc::new(AtomicBool::new(false));
    let second = Arc::new(AtomicBool::new(false));
    let callback_installer = installer.clone();
    let callback_first = first.clone();
    let callback_second = second.clone();
    let progress: LocalModelInstallProgressCallback = Arc::new(move |update| {
        if update.install_state == LocalModelInstallState::Downloading {
            callback_first.store(callback_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
            callback_second.store(callback_installer.cancel(TEST_ENTRY.id), Ordering::SeqCst);
        }
    });

    let error = installer
        .install_entry(&TEST_ENTRY, Some(progress))
        .await
        .unwrap_err();

    assert!(first.load(Ordering::SeqCst));
    assert!(second.load(Ordering::SeqCst));
    assert_cancelled_install_is_not_installed(&installer, dir.path(), &error);
}
