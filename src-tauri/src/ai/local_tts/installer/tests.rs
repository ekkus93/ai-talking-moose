use super::fs_ops::remove_model_dir;
use super::transport::{ArtifactProgressCallback, LocalTtsDownloadTransport};
use super::*;
use crate::ai::local_tts::manifest::{
    LocalTtsArtifact, LocalTtsArtifactKind, LocalTtsModelManifest, LocalTtsPlatformArtifact,
    LocalTtsRuntimeCompatibility, LocalTtsVoiceManifest,
};
use crate::ai::local_tts::storage::INSTALL_MARKER;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

const TEST_MODEL_BYTES: &[u8] = b"model";
const TEST_VOICES_BYTES: &[u8] = b"voices";
const TEST_G2P_BYTES: &[u8] = b"g2p";
const TEST_ORT_BYTES: &[u8] = b"ort";

static TEST_COMMON_ARTIFACTS: [LocalTtsArtifact; 3] = [
    LocalTtsArtifact {
        kind: LocalTtsArtifactKind::Model,
        filename: "model.onnx",
        source_url: "https://example.invalid/model.onnx",
        source_revision: "1111111111111111111111111111111111111111",
        expected_bytes: 5,
        sha256: "9372c470eeadd5ecd9c3c74c2b3cb633f8e2f2fad799250a0f70d652b6b825e4",
        license: "Apache-2.0",
        source_provenance: "test",
        purpose: "test model",
    },
    LocalTtsArtifact {
        kind: LocalTtsArtifactKind::Voices,
        filename: "voices.npz",
        source_url: "https://example.invalid/voices.npz",
        source_revision: "1111111111111111111111111111111111111111",
        expected_bytes: 6,
        sha256: "db9cf15a9a701d0583df98351f97d69c586e1e6f9c8e5c8daf915c6bfb09e762",
        license: "Apache-2.0",
        source_provenance: "test",
        purpose: "test voices",
    },
    LocalTtsArtifact {
        kind: LocalTtsArtifactKind::G2pData,
        filename: "g2p.json",
        source_url: "https://example.invalid/g2p.json",
        source_revision: "2222222222222222222222222222222222222222",
        expected_bytes: 3,
        sha256: "2e898001bc9a1f92fdff2792ed1520eba3d9d71bddabd144a2d9c0342da1c2fa",
        license: "BSD-3-Clause",
        source_provenance: "test",
        purpose: "test g2p",
    },
];

static TEST_PLATFORM_ARTIFACTS: [LocalTtsPlatformArtifact; 1] = [LocalTtsPlatformArtifact {
    platform: LocalTtsPlatform::LinuxX86_64,
    artifact: LocalTtsArtifact {
        kind: LocalTtsArtifactKind::OnnxRuntimeArchive,
        filename: "ort.tgz",
        source_url: "https://example.invalid/ort.tgz",
        source_revision: "v1.0.0",
        expected_bytes: 3,
        sha256: "8273ef957736e504caa03e5ef80f29bb0e8c81916d935cf1c8aabcc408221650",
        license: "MIT",
        source_provenance: "test",
        purpose: "test runtime",
    },
}];

static TEST_VOICES: [LocalTtsVoiceManifest; 1] = [LocalTtsVoiceManifest {
    id: "Test",
    embedding_key: "test",
}];

static TEST_MANIFEST: LocalTtsModelManifest = LocalTtsModelManifest {
    id: "test-local-tts",
    provider_model_id: "test/local-tts",
    display_name: "Test Local TTS",
    family: "Test",
    version: "1",
    language: "en",
    sample_rate_hz: 24_000,
    parameter_scale: "tiny",
    license: "Apache-2.0",
    model_source_revision: "1111111111111111111111111111111111111111",
    voices: &TEST_VOICES,
    runtime: LocalTtsRuntimeCompatibility {
        compatibility_version: 1,
        adapter_contract: "test-v1",
        ort_crate_version: "test",
        onnx_runtime_version: "1.0.0",
        g2p_crate_version: "test",
        g2p_source_revision: "2222222222222222222222222222222222222222",
        inference_threads: 1,
    },
    common_artifacts: &TEST_COMMON_ARTIFACTS,
    platform_artifacts: &TEST_PLATFORM_ARTIFACTS,
};

struct MemoryTransport {
    calls: AtomicUsize,
}

impl MemoryTransport {
    fn bytes_for(artifact: &LocalTtsArtifact) -> &'static [u8] {
        match artifact.filename {
            "model.onnx" => TEST_MODEL_BYTES,
            "voices.npz" => TEST_VOICES_BYTES,
            "g2p.json" => TEST_G2P_BYTES,
            "ort.tgz" => TEST_ORT_BYTES,
            other => panic!("unexpected test artifact: {other}"),
        }
    }
}

#[async_trait]
impl LocalTtsDownloadTransport for MemoryTransport {
    async fn download(
        &self,
        artifact: &'static LocalTtsArtifact,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&ArtifactProgressCallback>,
    ) -> Result<(), LocalTtsInstallError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if cancellation.is_cancelled() {
            return Err(LocalTtsInstallError::cancelled());
        }
        let bytes = Self::bytes_for(artifact);
        tokio::fs::write(destination, bytes)
            .await
            .map_err(|_| LocalTtsInstallError::io("write a test artifact"))?;
        if let Some(callback) = progress {
            callback(bytes.len() as u64);
        }
        Ok(())
    }
}

fn installer() -> (
    tempfile::TempDir,
    Arc<LocalTtsInstaller>,
    Arc<MemoryTransport>,
) {
    let dir = tempdir().unwrap();
    let storage = Arc::new(LocalTtsStorage::new(dir.path().join("models").join("tts")).unwrap());
    let transport = Arc::new(MemoryTransport {
        calls: AtomicUsize::new(0),
    });
    let installer =
        Arc::new(LocalTtsInstaller::with_transport(storage, transport.clone()).unwrap());
    (dir, installer, transport)
}

#[tokio::test]
async fn successful_multi_file_install_commits_marker_last() {
    let (_dir, installer, transport) = installer();
    let outcome = installer
        .install_manifest(&TEST_MANIFEST, LocalTtsPlatform::LinuxX86_64, None)
        .await
        .unwrap();
    assert_eq!(outcome.installed_bytes, 17);
    assert_eq!(transport.calls.load(Ordering::SeqCst), 4);
    assert!(installer
        .storage
        .marker_shape_is_valid(&TEST_MANIFEST, LocalTtsPlatform::LinuxX86_64));
    let revision_dir = installer
        .root()
        .join(TEST_MANIFEST.id)
        .join(TEST_MANIFEST.model_source_revision);
    assert!(revision_dir.join(INSTALL_MARKER).is_file());
    assert!(installer
        .storage
        .staging_root()
        .read_dir()
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn progress_reports_download_verify_and_promote_states() {
    let (_dir, installer, _transport) = installer();
    let states = Arc::new(Mutex::new(Vec::new()));
    let captured = states.clone();
    let callback: LocalTtsInstallProgressCallback = Arc::new(move |progress| {
        captured.lock().push(progress.install_state);
    });
    installer
        .install_manifest(
            &TEST_MANIFEST,
            LocalTtsPlatform::LinuxX86_64,
            Some(callback),
        )
        .await
        .unwrap();
    let states = states.lock();
    assert!(states.contains(&LocalTtsInstallState::Downloading));
    assert!(states.contains(&LocalTtsInstallState::Verifying));
    assert!(states.contains(&LocalTtsInstallState::Promoting));
}

#[tokio::test]
async fn accepted_cancellation_before_marker_commit_cannot_leave_install_ready() {
    let (_dir, installer, _transport) = installer();
    let cancelling_installer = installer.clone();
    installer.set_promotion_observer(Some(Arc::new(move || {
        assert!(cancelling_installer.cancel(TEST_MANIFEST.provider_model_id));
    })));
    let error = installer
        .install_manifest(&TEST_MANIFEST, LocalTtsPlatform::LinuxX86_64, None)
        .await
        .unwrap_err();
    assert_eq!(error.kind, LocalTtsInstallErrorKind::Cancelled);
    assert!(!installer
        .storage
        .marker_shape_is_valid(&TEST_MANIFEST, LocalTtsPlatform::LinuxX86_64));
    assert!(!installer.root().join(TEST_MANIFEST.id).exists());
}

#[tokio::test]
async fn delete_then_reinstall_is_supported() {
    let (_dir, installer, transport) = installer();
    installer
        .install_manifest(&TEST_MANIFEST, LocalTtsPlatform::LinuxX86_64, None)
        .await
        .unwrap();
    remove_model_dir(installer.root(), &TEST_MANIFEST).unwrap();
    assert!(!installer.root().join(TEST_MANIFEST.id).exists());
    installer
        .install_manifest(&TEST_MANIFEST, LocalTtsPlatform::LinuxX86_64, None)
        .await
        .unwrap();
    assert_eq!(transport.calls.load(Ordering::SeqCst), 8);
}

#[test]
fn stale_staging_entries_are_removed_on_installer_startup() {
    let dir = tempdir().unwrap();
    let storage = Arc::new(LocalTtsStorage::new(dir.path().join("models").join("tts")).unwrap());
    fs::create_dir(storage.staging_root().join("old-dir")).unwrap();
    fs::write(storage.staging_root().join("old.partial"), b"stale").unwrap();
    let transport = Arc::new(MemoryTransport {
        calls: AtomicUsize::new(0),
    });
    let _installer = LocalTtsInstaller::with_transport(storage.clone(), transport).unwrap();
    assert_eq!(storage.staging_root().read_dir().unwrap().count(), 0);
}

#[test]
fn latest_error_is_chronological_not_lexical() {
    let (_dir, installer, _transport) = installer();
    installer.error_state.lock().record(
        "z-model",
        LocalTtsInstallError::new(LocalTtsInstallErrorKind::Network, "first", true),
    );
    installer.error_state.lock().record(
        "a-model",
        LocalTtsInstallError::new(LocalTtsInstallErrorKind::Io, "second", true),
    );
    assert_eq!(installer.latest_error().unwrap().message, "second");
}

#[test]
fn delete_never_follows_model_directory_symlink() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let (dir, installer, _transport) = installer();
        let outside = dir.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let sentinel = outside.join("keep.txt");
        fs::write(&sentinel, b"keep").unwrap();
        symlink(&outside, installer.root().join(TEST_MANIFEST.id)).unwrap();
        remove_model_dir(installer.root(), &TEST_MANIFEST).unwrap();
        assert!(sentinel.exists());
        assert!(!installer.root().join(TEST_MANIFEST.id).exists());
    }
}
