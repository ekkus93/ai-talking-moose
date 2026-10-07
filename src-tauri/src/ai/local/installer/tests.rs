use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;

struct BytesTransport {
    bytes: Vec<u8>,
    calls: AtomicUsize,
}

#[async_trait]
impl LocalModelDownloadTransport for BytesTransport {
    async fn download(
        &self,
        _entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        _progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if cancellation.is_cancelled() {
            return Err(LocalModelInstallError::cancelled());
        }
        tokio::fs::write(destination, &self.bytes)
            .await
            .map_err(|_| LocalModelInstallError::io("write test transport output"))
    }
}

#[test]
fn verifier_rejects_wrong_size_and_wrong_hash() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("artifact.gguf");
    fs::write(&path, b"abc").unwrap();
    let wrong_size = verify_artifact(&path, 4, "00").unwrap_err();
    assert_eq!(wrong_size.kind, LocalModelInstallErrorKind::SizeMismatch);

    let wrong_hash = verify_artifact(
        &path,
        3,
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap_err();
    assert_eq!(wrong_hash.kind, LocalModelInstallErrorKind::Sha256Mismatch);
}

#[test]
fn stale_staging_files_and_directories_are_removed_on_startup() {
    let dir = tempdir().unwrap();
    let staging = dir.path().join(STAGING_DIR);
    fs::create_dir_all(staging.join("old-dir")).unwrap();
    fs::write(staging.join("old.partial"), b"stale").unwrap();
    let _installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
}

#[test]
fn model_paths_are_catalog_owned_and_revision_scoped() {
    let dir = tempdir().unwrap();
    let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
    let path = installer
        .model_path(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID)
        .unwrap();
    let entry = local_model_entry(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
    assert!(path.ends_with(
        Path::new(entry.id)
            .join(entry.revision)
            .join(entry.artifact_filename)
    ));
    assert_eq!(
        installer.model_path("../escape").unwrap_err().kind,
        LocalModelInstallErrorKind::UnknownModel
    );
}

#[test]
fn delete_never_follows_a_model_directory_symlink() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let sentinel = outside.path().join("keep.txt");
        fs::write(&sentinel, b"keep").unwrap();
        let installer = LocalModelInstaller::new(dir.path().to_path_buf()).unwrap();
        let entry = local_model_entry(super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
        symlink(outside.path(), dir.path().join(entry.id)).unwrap();
        installer.delete(entry.id).unwrap();
        assert!(sentinel.exists());
        assert!(!dir.path().join(entry.id).exists());
    }
}

#[tokio::test]
async fn duplicate_install_is_rejected_without_silent_parallel_work() {
    let dir = tempdir().unwrap();
    let transport = Arc::new(BytesTransport {
        bytes: vec![],
        calls: AtomicUsize::new(0),
    });
    let installer =
        LocalModelInstaller::with_transport(dir.path().to_path_buf(), transport).unwrap();
    let model_id = super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID;
    installer.in_flight.lock().insert(
        model_id.to_string(),
        InFlightInstall {
            cancellation: CancellationToken::new(),
            phase: LocalModelInstallPhase::Downloading,
        },
    );
    let error = installer.install(model_id, None).await.unwrap_err();
    assert_eq!(error.kind, LocalModelInstallErrorKind::Busy);
}

#[tokio::test]
async fn cancellation_handle_targets_only_the_requested_model() {
    let dir = tempdir().unwrap();
    let transport = Arc::new(BytesTransport {
        bytes: vec![1, 2, 3],
        calls: AtomicUsize::new(0),
    });
    let installer =
        LocalModelInstaller::with_transport(dir.path().to_path_buf(), transport).unwrap();
    let model_id = super::super::catalog::DEFAULT_LOCAL_TEXT_MODEL_ID;
    let token = CancellationToken::new();
    installer.in_flight.lock().insert(
        model_id.to_string(),
        InFlightInstall {
            cancellation: token.clone(),
            phase: LocalModelInstallPhase::Downloading,
        },
    );
    assert!(installer.cancel(model_id));
    assert!(token.is_cancelled());
    assert!(!installer.cancel("qwen3-0-6b-instruct-q4-k-m"));
}
