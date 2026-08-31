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
