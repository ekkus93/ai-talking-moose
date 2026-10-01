use super::*;
use crate::ai::local::catalog::LocalModelTemplateHint;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tempfile::tempdir;

static TEST_ENTRY: LocalModelCatalogEntry = LocalModelCatalogEntry {
    id: "test-local-model",
    display_name: "Test Local Model",
    family: "Test",
    parameter_scale: "tiny",
    quantization: "test",
    artifact_filename: "test-model.gguf",
    source_url: "https://example.invalid/test-model.gguf",
    revision: "0123456789012345678901234567890123456789",
    expected_bytes: 3,
    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    license: "Apache-2.0",
    context_limit: 32,
    recommended_max_output: 8,
    template_hint: LocalModelTemplateHint::SmolLm2,
};

struct StaticBytesTransport {
    bytes: &'static [u8],
    wait_for_cancel_after_write: bool,
}

#[async_trait]
impl LocalModelDownloadTransport for StaticBytesTransport {
    async fn download(
        &self,
        entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError> {
        tokio::fs::write(destination, self.bytes)
            .await
            .map_err(|_| LocalModelInstallError::io("write adversarial test transport output"))?;
        if let Some(callback) = progress {
            callback(LocalModelInstallProgress {
                model_id: entry.id.to_string(),
                install_state: LocalModelInstallState::Downloading,
                downloaded_bytes: self.bytes.len() as u64,
                total_bytes: entry.expected_bytes,
            });
        }
        if self.wait_for_cancel_after_write {
            cancellation.cancelled().await;
            return Err(LocalModelInstallError::cancelled());
        }
        Ok(())
    }
}

fn staging_is_empty(root: &Path) -> bool {
    fs::read_dir(root.join(STAGING_DIR))
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}

fn test_revision_dir(root: &Path) -> PathBuf {
    root.join(TEST_ENTRY.id).join(TEST_ENTRY.revision)
}

fn test_artifact_path(root: &Path) -> PathBuf {
    test_revision_dir(root).join(TEST_ENTRY.artifact_filename)
}

fn test_marker_path(root: &Path) -> PathBuf {
    test_revision_dir(root).join(INSTALL_MARKER)
}

fn write_test_marker(revision_dir: &Path) {
    let marker = InstallMarker {
        schema_version: INSTALL_MARKER_VERSION,
        model_id: TEST_ENTRY.id.to_string(),
        revision: TEST_ENTRY.revision.to_string(),
        artifact_filename: TEST_ENTRY.artifact_filename.to_string(),
        expected_bytes: TEST_ENTRY.expected_bytes,
        sha256: TEST_ENTRY.sha256.to_string(),
    };
    fs::write(
        revision_dir.join(INSTALL_MARKER),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();
}

fn seed_test_shape_install(root: &Path) {
    let revision_dir = test_revision_dir(root);
    fs::create_dir_all(&revision_dir).unwrap();
    fs::write(test_artifact_path(root), b"abc").unwrap();
    write_test_marker(&revision_dir);
}

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
    installer
        .verified_runtime_artifact_path(&TEST_ENTRY)
        .unwrap();
    installer.verified_runtime_artifact_path(&TEST_ENTRY).unwrap();
    assert_eq!(hash_runs.load(Ordering::SeqCst), 1);
    fs::write(test_artifact_path(dir.path()), b"abd").unwrap();
    let error = installer
        .verified_runtime_artifact_path(&TEST_ENTRY)
        .unwrap_err();
    assert_eq!(
        error.kind,
        LocalModelInstallErrorKind::Sha256Mismatch
    );
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
    let error = installer
        .install_inner(&TEST_ENTRY, &CancellationToken::new(), None)
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
    let error = installer.install_inner(&TEST_ENTRY, &CancellationToken::new(), None).await.unwrap_err();
    assert_eq!(error.kind, LocalModelInstallErrorKind::Sha256Mismatch);
    assert!(staging_is_empty(dir.path()));
    assert!(!test_artifact_path(dir.path()).exists());
    assert!(!test_marker_path(dir.path()).exists());
}

#[tokio::test]
async fn cancellation_during_download_never_installs() {
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
    assert_eq!(error.kind, LocalModelInstallErrorKind::Cancelled);
    assert!(!installer.marker_shape_is_valid(&TEST_ENTRY));
    assert!(staging_is_empty(dir.path()));
}

#[test]
fn failed_atomic_promotion_never_creates_install_marker() {
    let dir = tempdir().unwrap();
    let missing_staging = dir.path().join(STAGING_DIR).join("missing.partial");
    let error = promote_artifact(dir.path(), &TEST_ENTRY, &missing_staging).unwrap_err();
    assert_eq!(error.kind, LocalModelInstallErrorKind::Promotion);
    assert!(!test_artifact_path(dir.path()).exists());
    assert!(!test_marker_path(dir.path()).exists());
}

#[cfg(unix)]
#[test]
fn model_root_symlink_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;
    let parent = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let sentinel = outside.path().join("keep.txt");
    fs::write(&sentinel, b"keep").unwrap();
    let root = parent.path().join("llm-root");
    symlink(outside.path(), &root).unwrap();
    let error = LocalModelInstaller::new(root)
        .err()
        .expect("model-root symlink must fail closed");
    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn deleting_model_tree_does_not_follow_revision_symlink() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let sentinel = outside.path().join("keep.txt");
    fs::write(&sentinel, b"keep").unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    let entry = local_model_entry(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
    let model_dir = dir.path().join(entry.id);
    fs::create_dir(&model_dir).unwrap();
    symlink(outside.path(), model_dir.join(entry.revision)).unwrap();
    installer.delete(entry.id).unwrap();
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
    assert!(!model_dir.exists());
}
