use super::*;
use crate::ai::local::catalog::LocalModelTemplateHint;
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
        _entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        _progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError> {
        tokio::fs::write(destination, self.bytes)
            .await
            .map_err(|_| LocalModelInstallError::io("write adversarial test transport output"))?;
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

#[test]
fn failed_atomic_promotion_never_creates_install_marker() {
    let dir = tempdir().unwrap();
    let missing_staging = dir.path().join(STAGING_DIR).join("missing.partial");

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &missing_staging).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::Promotion);
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    assert!(!revision_dir.join(TEST_ENTRY.artifact_filename).exists());
    assert!(!revision_dir.join(INSTALL_MARKER).exists());
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
fn staging_directory_symlink_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let sentinel = outside.path().join("keep.txt");
    fs::write(&sentinel, b"keep").unwrap();
    symlink(outside.path(), dir.path().join(STAGING_DIR)).unwrap();

    let error = LocalModelInstaller::new(dir.path().to_path_buf())
        .err()
        .expect("staging symlink must fail closed");

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn model_path_rejects_root_replaced_by_symlink_after_initialization() {
    use std::os::unix::fs::symlink;

    let parent = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let root = parent.path().join("llm-root");
    let installer = LocalModelInstaller::new(root.clone()).unwrap();
    fs::remove_dir_all(&root).unwrap();
    symlink(outside.path(), &root).unwrap();

    let error = installer
        .model_path(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID)
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
}

#[cfg(unix)]
#[test]
fn model_path_rejects_staging_replaced_by_symlink_after_initialization() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::remove_dir(&staging).unwrap();
    symlink(outside.path(), &staging).unwrap();

    let error = installer
        .model_path(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID)
        .unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
}

#[cfg(unix)]
#[test]
fn promotion_rejects_model_directory_symlink_without_root_escape() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir(&staging).unwrap();
    let staging_file = staging.join("verified.partial");
    fs::write(&staging_file, b"abc").unwrap();
    symlink(outside.path(), dir.path().join(TEST_ENTRY.id)).unwrap();

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &staging_file).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert!(!outside.path().join(TEST_ENTRY.revision).exists());
    assert_eq!(fs::read(&staging_file).unwrap(), b"abc");
}

#[cfg(unix)]
#[test]
fn promotion_rejects_revision_directory_symlink_without_root_escape() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir(&staging).unwrap();
    let staging_file = staging.join("verified.partial");
    fs::write(&staging_file, b"abc").unwrap();
    let model_dir = dir.path().join(TEST_ENTRY.id);
    fs::create_dir(&model_dir).unwrap();
    symlink(outside.path(), model_dir.join(TEST_ENTRY.revision)).unwrap();

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &staging_file).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert!(!outside.path().join(TEST_ENTRY.artifact_filename).exists());
    assert_eq!(fs::read(&staging_file).unwrap(), b"abc");
}

#[cfg(unix)]
#[test]
fn marker_symlink_failure_removes_promoted_artifact_and_preserves_target() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_marker = outside.path().join("marker.txt");
    fs::write(&outside_marker, b"keep").unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir(&staging).unwrap();
    let staging_file = staging.join("verified.partial");
    fs::write(&staging_file, b"abc").unwrap();
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    fs::create_dir_all(&revision_dir).unwrap();
    symlink(&outside_marker, revision_dir.join(INSTALL_MARKER)).unwrap();

    let error = promote_artifact(dir.path(), &TEST_ENTRY, &staging_file).unwrap_err();

    assert_eq!(error.kind, LocalModelInstallErrorKind::CorruptInstall);
    assert!(!revision_dir.join(TEST_ENTRY.artifact_filename).exists());
    assert_eq!(fs::read(&outside_marker).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn symlink_artifact_never_counts_as_installed() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::with_transport(
        dir.path().to_path_buf(),
        Arc::new(StaticBytesTransport {
            bytes: b"abc",
            wait_for_cancel_after_write: false,
        }),
    )
    .unwrap();
    let revision_dir = dir.path().join(TEST_ENTRY.id).join(TEST_ENTRY.revision);
    fs::create_dir_all(&revision_dir).unwrap();
    let real_artifact = dir.path().join("real.gguf");
    fs::write(&real_artifact, b"abc").unwrap();
    symlink(
        &real_artifact,
        revision_dir.join(TEST_ENTRY.artifact_filename),
    )
    .unwrap();
    write_test_marker(&revision_dir);

    assert!(!installer.install_is_valid(&TEST_ENTRY));
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
