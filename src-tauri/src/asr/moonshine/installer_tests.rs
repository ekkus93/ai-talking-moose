use super::super::ffi::MOONSHINE_HEADER_VERSION;
use super::super::manifest::{
    MOONSHINE_ASSET_ARCHIVE_COMMIT, MOONSHINE_ASSET_ARCHIVE_REPOSITORY, MOONSHINE_RUNTIME_COMMIT,
    MOONSHINE_RUNTIME_RELEASE,
};
use super::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Mutex as StdMutex;
use tempfile::TempDir;

const FILE_A_BYTES: &[u8] = b"tiny adapter fixture\n";
const FILE_B_BYTES: &[u8] = b"tiny tokenizer fixture\n";
const FILE_A_SHA256: &str = "2ca29b7ac5ade45723b2109e119d6eaee2e12b8c6b27197e356faac4ac2511cd";
const FILE_B_SHA256: &str = "d36873f296f194088a2732a7cfdd8ff5fd6b51a819e4b7c6dbf106491a23dbc9";
const FILE_A_CRC32C: &str = "QciULA==";
const FILE_B_CRC32C: &str = "wPsozg==";

const TEST_FILES: [MoonshineModelFile; 2] = [
    MoonshineModelFile {
        name: "adapter.ort",
        bytes: FILE_A_BYTES.len() as u64,
        sha256: FILE_A_SHA256,
        upstream_crc32c_base64: FILE_A_CRC32C,
    },
    MoonshineModelFile {
        name: "tokenizer.bin",
        bytes: FILE_B_BYTES.len() as u64,
        sha256: FILE_B_SHA256,
        upstream_crc32c_base64: FILE_B_CRC32C,
    },
];

const TEST_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
    id: "moonshine-tiny-streaming-en",
    display_name: "Test Tiny",
    architecture: MoonshineModelArchitecture::TinyStreaming,
    revision: "test_revision",
    base_url: "https://download.moonshine.ai/model/tiny-streaming-en/test_revision",
    expected_bytes: (FILE_A_BYTES.len() + FILE_B_BYTES.len()) as u64,
    runtime: super::super::manifest::MoonshineRuntimeCompatibility {
        release: MOONSHINE_RUNTIME_RELEASE,
        source_commit: MOONSHINE_RUNTIME_COMMIT,
        c_header_version: MOONSHINE_HEADER_VERSION,
    },
    provenance: super::super::manifest::MoonshineModelProvenance {
        source_repository: "UsefulSensors/moonshine-streaming-tiny",
        source_commit: "f8e9dfd8c562c257c151a907b7b7f2fe8ff8511a",
        asset_archive_repository: MOONSHINE_ASSET_ARCHIVE_REPOSITORY,
        asset_archive_commit: MOONSHINE_ASSET_ARCHIVE_COMMIT,
        license: "MIT",
    },
    files: &TEST_FILES,
};

#[derive(Clone)]
struct FakeResponse {
    chunks: Vec<Vec<u8>>,
    content_length: Option<u64>,
    fail_after_chunks: Option<usize>,
    cancel_after_chunks: Option<usize>,
}

#[derive(Default)]
struct FakeTransport {
    responses: StdMutex<HashMap<String, FakeResponse>>,
    requests: StdMutex<Vec<String>>,
    active: AtomicUsize,
    max_active: AtomicUsize,
}

impl FakeTransport {
    fn with_fixture_manifest() -> Arc<Self> {
        let transport = Arc::new(Self::default());
        transport.add_response("adapter.ort", FILE_A_BYTES);
        transport.add_response("tokenizer.bin", FILE_B_BYTES);
        transport
    }

    fn add_response(&self, name: &str, body: &[u8]) {
        let url = format!("{}/{}", TEST_MANIFEST.base_url, name);
        self.responses.lock().unwrap().insert(
            url,
            FakeResponse {
                chunks: body.chunks(5).map(<[u8]>::to_vec).collect(),
                content_length: Some(body.len() as u64),
                fail_after_chunks: None,
                cancel_after_chunks: None,
            },
        );
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    fn partial_entries(root: &Path) -> Vec<PathBuf> {
        if !root.exists() {
            return Vec::new();
        }
        fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".partial"))
            })
            .collect()
    }
}

#[async_trait]
impl ModelDownloadTransport for FakeTransport {
    async fn stream(
        &self,
        url: &str,
        cancellation: &MoonshineModelInstallCancellation,
        sink: &mut dyn DownloadSink,
    ) -> Result<DownloadMetadata, MoonshineModelInstallError> {
        let active = self.active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        self.max_active.fetch_max(active, AtomicOrdering::SeqCst);
        self.requests.lock().unwrap().push(url.to_string());
        let response = self
            .responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(MoonshineModelInstallError::network)?;

        let result = async {
            for (index, chunk) in response.chunks.iter().enumerate() {
                cancellation.check()?;
                if response.fail_after_chunks == Some(index) {
                    return Err(MoonshineModelInstallError::network());
                }
                if response.cancel_after_chunks == Some(index) {
                    cancellation.cancel();
                    cancellation.check()?;
                }
                sink.write_chunk(chunk)?;
                tokio::task::yield_now().await;
            }
            Ok(DownloadMetadata {
                content_length: response.content_length,
            })
        }
        .await;
        self.active.fetch_sub(1, AtomicOrdering::SeqCst);
        result
    }
}

struct FakeDiskSpace {
    available: Option<u64>,
}

impl DiskSpaceProbe for FakeDiskSpace {
    fn available_bytes(&self, _path: &Path) -> std::io::Result<Option<u64>> {
        Ok(self.available)
    }
}

struct FailingDiskSpace;

impl DiskSpaceProbe for FailingDiskSpace {
    fn available_bytes(&self, _path: &Path) -> std::io::Result<Option<u64>> {
        Err(std::io::Error::other("test disk probe failure"))
    }
}

fn installer(temp: &TempDir, transport: Arc<FakeTransport>) -> MoonshineModelInstaller {
    MoonshineModelInstaller::with_dependencies(
        temp.path(),
        transport,
        Arc::new(FakeDiskSpace {
            available: Some(
                TEST_MANIFEST
                    .expected_bytes
                    .saturating_add(DISK_SPACE_HEADROOM_BYTES)
                    .saturating_add(1024),
            ),
        }),
    )
}

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
    let guard = install_operation_lock().lock().await;
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

#[test]
fn install_operation_locks_are_scoped_per_model() {
    let tiny_a = install_operation_lock("moonshine-tiny-streaming-en");
    let tiny_b = install_operation_lock("moonshine-tiny-streaming-en");
    let small = install_operation_lock("moonshine-small-streaming-en");

    assert!(Arc::ptr_eq(&tiny_a, &tiny_b));
    assert!(!Arc::ptr_eq(&tiny_a, &small));
}
