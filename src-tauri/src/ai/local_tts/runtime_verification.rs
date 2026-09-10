use super::manifest::{local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform};
use super::storage::{expected_artifacts, validate_storage_layout, LocalTtsStorage};
use parking_lot::Mutex;
use ring::digest::{Context as Sha256Context, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const VERIFY_BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsRuntimeVerificationErrorKind {
    UnknownModel,
    CorruptInstall,
    Io,
    Sha256Mismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTtsRuntimeVerificationError {
    pub kind: LocalTtsRuntimeVerificationErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl LocalTtsRuntimeVerificationError {
    fn unknown_model() -> Self {
        Self {
            kind: LocalTtsRuntimeVerificationErrorKind::UnknownModel,
            message: "The selected Local TTS model is not in the supported catalog.".to_string(),
            retryable: false,
        }
    }

    fn corrupt_install() -> Self {
        Self {
            kind: LocalTtsRuntimeVerificationErrorKind::CorruptInstall,
            message: "The Local TTS installation is incomplete or invalid.".to_string(),
            retryable: true,
        }
    }

    fn io() -> Self {
        Self {
            kind: LocalTtsRuntimeVerificationErrorKind::Io,
            message: "The Local TTS installation could not be verified for runtime use."
                .to_string(),
            retryable: true,
        }
    }

    fn sha256_mismatch() -> Self {
        Self {
            kind: LocalTtsRuntimeVerificationErrorKind::Sha256Mismatch,
            message: "A Local TTS runtime artifact failed SHA-256 verification.".to_string(),
            retryable: true,
        }
    }
}

impl fmt::Display for LocalTtsRuntimeVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LocalTtsRuntimeVerificationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeArtifactFingerprint {
    canonical_path: PathBuf,
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanoseconds: i64,
}

impl RuntimeArtifactFingerprint {
    fn from_metadata(canonical_path: PathBuf, metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            canonical_path,
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(unix)]
            changed_seconds: metadata.ctime(),
            #[cfg(unix)]
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeVerificationFingerprint {
    artifacts: Vec<RuntimeArtifactFingerprint>,
}

#[cfg(test)]
type VerificationObserver = Arc<dyn Fn() + Send + Sync>;

/// Cryptographic admission gate for installed Local TTS artifacts.
///
/// Fast UI status remains marker/shape based. Before KTT-300 loads a model set, the runtime
/// manager must obtain paths through this verifier. A successful verification is cached only while
/// the canonical paths and conservative file metadata/identity fingerprint remain unchanged.
pub struct LocalTtsRuntimeVerifier {
    storage: Arc<LocalTtsStorage>,
    verified: Mutex<HashMap<(String, LocalTtsPlatform), RuntimeVerificationFingerprint>>,
    #[cfg(test)]
    observer: Mutex<Option<VerificationObserver>>,
}

impl LocalTtsRuntimeVerifier {
    pub fn new(storage: Arc<LocalTtsStorage>) -> Self {
        Self {
            storage,
            verified: Mutex::new(HashMap::new()),
            #[cfg(test)]
            observer: Mutex::new(None),
        }
    }

    pub fn verified_artifact_paths(
        &self,
        model_id: &str,
        platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeVerificationError> {
        validate_storage_layout(self.storage.root())
            .map_err(|_| LocalTtsRuntimeVerificationError::corrupt_install())?;
        let manifest = local_tts_model_manifest(model_id)
            .ok_or_else(LocalTtsRuntimeVerificationError::unknown_model)?;
        self.verified_artifact_paths_for_manifest(manifest, platform)
    }

    fn verified_artifact_paths_for_manifest(
        &self,
        manifest: &'static LocalTtsModelManifest,
        platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeVerificationError> {
        let cache_key = (manifest.provider_model_id.to_string(), platform);
        let (canonical_paths, fingerprint) = match self.fingerprint(manifest, platform) {
            Ok(result) => result,
            Err(error) => {
                self.verified.lock().remove(&cache_key);
                return Err(error);
            }
        };

        if self
            .verified
            .lock()
            .get(&cache_key)
            .is_some_and(|cached| cached == &fingerprint)
        {
            return Ok(canonical_paths);
        }

        self.notify_verification();
        let artifacts = expected_artifacts(manifest, platform)
            .map_err(|_| LocalTtsRuntimeVerificationError::corrupt_install())?;
        for (artifact, path) in artifacts.iter().zip(canonical_paths.iter()) {
            if let Err(error) = verify_artifact(path, artifact.expected_bytes, artifact.sha256) {
                self.verified.lock().remove(&cache_key);
                return Err(error);
            }
        }

        let (canonical_after, fingerprint_after) = match self.fingerprint(manifest, platform) {
            Ok(result) => result,
            Err(error) => {
                self.verified.lock().remove(&cache_key);
                return Err(error);
            }
        };
        if fingerprint_after != fingerprint {
            self.verified.lock().remove(&cache_key);
            return Err(LocalTtsRuntimeVerificationError::corrupt_install());
        }

        self.verified.lock().insert(cache_key, fingerprint_after);
        Ok(canonical_after)
    }

    fn fingerprint(
        &self,
        manifest: &'static LocalTtsModelManifest,
        platform: LocalTtsPlatform,
    ) -> Result<(Vec<PathBuf>, RuntimeVerificationFingerprint), LocalTtsRuntimeVerificationError> {
        if !self.storage.marker_shape_is_valid(manifest, platform) {
            return Err(LocalTtsRuntimeVerificationError::corrupt_install());
        }

        let artifacts = expected_artifacts(manifest, platform)
            .map_err(|_| LocalTtsRuntimeVerificationError::corrupt_install())?;
        let revision_dir = self
            .storage
            .root()
            .join(manifest.id)
            .join(manifest.model_source_revision);
        let canonical_root = fs::canonicalize(self.storage.root())
            .map_err(|_| LocalTtsRuntimeVerificationError::corrupt_install())?;

        let mut canonical_paths = Vec::with_capacity(artifacts.len());
        let mut fingerprints = Vec::with_capacity(artifacts.len());
        for artifact in artifacts {
            let artifact_path = revision_dir.join(artifact.filename);
            let metadata = fs::symlink_metadata(&artifact_path)
                .map_err(|_| LocalTtsRuntimeVerificationError::corrupt_install())?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() != artifact.expected_bytes
            {
                return Err(LocalTtsRuntimeVerificationError::corrupt_install());
            }

            let canonical_path = fs::canonicalize(&artifact_path)
                .map_err(|_| LocalTtsRuntimeVerificationError::corrupt_install())?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(LocalTtsRuntimeVerificationError::corrupt_install());
            }

            fingerprints.push(RuntimeArtifactFingerprint::from_metadata(
                canonical_path.clone(),
                &metadata,
            ));
            canonical_paths.push(canonical_path);
        }

        Ok((
            canonical_paths,
            RuntimeVerificationFingerprint {
                artifacts: fingerprints,
            },
        ))
    }

    fn notify_verification(&self) {
        #[cfg(test)]
        if let Some(observer) = self.observer.lock().clone() {
            observer();
        }
    }

    #[cfg(test)]
    fn set_observer(&self, observer: Option<VerificationObserver>) {
        *self.observer.lock() = observer;
    }
}

fn verify_artifact(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<(), LocalTtsRuntimeVerificationError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LocalTtsRuntimeVerificationError::io())?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() != expected_bytes
    {
        return Err(LocalTtsRuntimeVerificationError::corrupt_install());
    }

    let file = fs::File::open(path).map_err(|_| LocalTtsRuntimeVerificationError::io())?;
    let mut reader = BufReader::with_capacity(VERIFY_BUFFER_BYTES, file);
    let mut context = Sha256Context::new(&SHA256);
    let mut buffer = vec![0_u8; VERIFY_BUFFER_BYTES];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| LocalTtsRuntimeVerificationError::io())?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }

    let actual = context.finish();
    let actual_hex = actual
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual_hex != expected_sha256 {
        return Err(LocalTtsRuntimeVerificationError::sha256_mismatch());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::local_tts::manifest::{
        LocalTtsArtifact, LocalTtsArtifactKind, LocalTtsPlatformArtifact,
        LocalTtsRuntimeCompatibility, LocalTtsVoiceManifest,
    };
    use crate::ai::local_tts::storage::{install_marker, INSTALL_MARKER};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;

    const MODEL_BYTES: &[u8] = b"model";
    const VOICES_BYTES: &[u8] = b"voices";
    const G2P_BYTES: &[u8] = b"g2p";
    const ORT_BYTES: &[u8] = b"ort";

    static COMMON_ARTIFACTS: [LocalTtsArtifact; 3] = [
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

    static PLATFORM_ARTIFACTS: [LocalTtsPlatformArtifact; 1] = [LocalTtsPlatformArtifact {
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

    static VOICES: [LocalTtsVoiceManifest; 1] = [LocalTtsVoiceManifest {
        id: "Test",
        embedding_key: "test",
    }];

    static MANIFEST: LocalTtsModelManifest = LocalTtsModelManifest {
        id: "test-runtime-verification",
        provider_model_id: "test/runtime-verification",
        display_name: "Test Runtime Verification",
        family: "Test",
        version: "1",
        language: "en",
        sample_rate_hz: 24_000,
        parameter_scale: "tiny",
        license: "Apache-2.0",
        model_source_revision: "1111111111111111111111111111111111111111",
        voices: &VOICES,
        runtime: LocalTtsRuntimeCompatibility {
            compatibility_version: 1,
            adapter_contract: "test-v1",
            ort_crate_version: "test",
            onnx_runtime_version: "1.0.0",
            g2p_crate_version: "test",
            g2p_source_revision: "2222222222222222222222222222222222222222",
            inference_threads: 1,
        },
        common_artifacts: &COMMON_ARTIFACTS,
        platform_artifacts: &PLATFORM_ARTIFACTS,
    };

    fn fixture() -> (tempfile::TempDir, Arc<LocalTtsStorage>, LocalTtsRuntimeVerifier) {
        let dir = tempdir().unwrap();
        let storage = Arc::new(
            LocalTtsStorage::new(dir.path().join("models").join("tts")).unwrap(),
        );
        let revision_dir = storage.root().join(MANIFEST.id).join(MANIFEST.model_source_revision);
        fs::create_dir_all(&revision_dir).unwrap();
        for (filename, bytes) in [
            ("model.onnx", MODEL_BYTES),
            ("voices.npz", VOICES_BYTES),
            ("g2p.json", G2P_BYTES),
            ("ort.tgz", ORT_BYTES),
        ] {
            fs::write(revision_dir.join(filename), bytes).unwrap();
        }
        let artifacts = expected_artifacts(&MANIFEST, LocalTtsPlatform::LinuxX86_64).unwrap();
        let marker = install_marker(&MANIFEST, LocalTtsPlatform::LinuxX86_64, &artifacts);
        fs::write(
            revision_dir.join(INSTALL_MARKER),
            serde_json::to_vec_pretty(&marker).unwrap(),
        )
        .unwrap();
        let verifier = LocalTtsRuntimeVerifier::new(storage.clone());
        (dir, storage, verifier)
    }

    #[test]
    fn unchanged_artifact_set_is_hashed_once_then_served_from_cache() {
        let (_dir, _storage, verifier) = fixture();
        let hash_runs = Arc::new(AtomicUsize::new(0));
        let observed = hash_runs.clone();
        verifier.set_observer(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        })));

        let first = verifier
            .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
            .unwrap();
        let second = verifier
            .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 4);
        assert!(first.iter().all(|path| path.is_absolute()));
        assert_eq!(hash_runs.load(Ordering::SeqCst), 1);
    }

    #[cfg(unix)]
    #[test]
    fn same_size_mutation_of_every_runtime_artifact_is_rehashed_and_rejected() {
        let mutations: [(&str, &[u8]); 4] = [
            ("model.onnx", b"m0del"),
            ("voices.npz", b"v0ices"),
            ("g2p.json", b"gXp"),
            ("ort.tgz", b"0rt"),
        ];

        for (filename, replacement) in mutations {
            let (_dir, storage, verifier) = fixture();
            verifier
                .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
                .unwrap();

            let revision_dir = storage
                .root()
                .join(MANIFEST.id)
                .join(MANIFEST.model_source_revision);
            fs::write(revision_dir.join(filename), replacement).unwrap();
            assert!(storage.marker_shape_is_valid(&MANIFEST, LocalTtsPlatform::LinuxX86_64));

            let error = verifier
                .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
                .unwrap_err();
            assert_eq!(
                error.kind,
                LocalTtsRuntimeVerificationErrorKind::Sha256Mismatch
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn same_bytes_replacement_changes_file_identity_and_forces_rehash() {
        let (_dir, storage, verifier) = fixture();
        let hash_runs = Arc::new(AtomicUsize::new(0));
        let observed = hash_runs.clone();
        verifier.set_observer(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        })));
        verifier
            .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
            .unwrap();

        let revision_dir = storage.root().join(MANIFEST.id).join(MANIFEST.model_source_revision);
        let replacement = revision_dir.join("voices.npz.replacement");
        fs::write(&replacement, VOICES_BYTES).unwrap();
        fs::rename(&replacement, revision_dir.join("voices.npz")).unwrap();

        verifier
            .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
            .unwrap();
        assert_eq!(hash_runs.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn size_change_invalidates_cache_before_hashing() {
        let (_dir, storage, verifier) = fixture();
        verifier
            .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
            .unwrap();

        let revision_dir = storage.root().join(MANIFEST.id).join(MANIFEST.model_source_revision);
        fs::write(revision_dir.join("g2p.json"), b"longer").unwrap();

        let error = verifier
            .verified_artifact_paths_for_manifest(&MANIFEST, LocalTtsPlatform::LinuxX86_64)
            .unwrap_err();
        assert_eq!(
            error.kind,
            LocalTtsRuntimeVerificationErrorKind::CorruptInstall
        );
    }
}
