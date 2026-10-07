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

#[path = "installer_tests/download.rs"]
mod download;
#[path = "installer_tests/filesystem.rs"]
mod filesystem;
#[path = "installer_tests/synchronization.rs"]
mod synchronization;
