use super::*;
use crate::ai::local::catalog::LocalModelTemplateHint;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex as StdMutex;
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

fn assert_cancelled_install_is_not_installed(
    installer: &LocalModelInstaller,
    root: &Path,
    error: &LocalModelInstallError,
) {
    assert_eq!(error.kind, LocalModelInstallErrorKind::Cancelled);
    assert!(!installer.marker_shape_is_valid(&TEST_ENTRY));
    assert!(!test_marker_path(root).exists());
    assert!(staging_is_empty(root));
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

mod install_flow;
mod storage_defenses;
