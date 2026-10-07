use super::super::manifest::{LocalTtsModelManifest, LocalTtsPlatform};
use super::super::runtime_verification::{
    LocalTtsRuntimeVerificationErrorKind, LocalTtsRuntimeVerifier,
};
use super::super::storage::global_local_tts_storage;
use super::LocalTtsRuntimeError;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ai::local_tts) struct LocalTtsRuntimeIdentity {
    pub(super) model_id: String,
    pub(super) model_revision: String,
    pub(super) runtime_compatibility_version: u32,
    pub(super) adapter_contract: String,
    pub(super) onnx_runtime_version: String,
    pub(super) g2p_source_revision: String,
    pub(super) platform: LocalTtsPlatform,
}

pub(super) trait RuntimeArtifactVerifier: Send + Sync {
    fn verify(
        &self,
        model_id: &str,
        platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeError>;
}

#[derive(Default)]
pub(super) struct GlobalRuntimeArtifactVerifier {
    verifier: Mutex<Option<Arc<LocalTtsRuntimeVerifier>>>,
}

impl RuntimeArtifactVerifier for GlobalRuntimeArtifactVerifier {
    fn verify(
        &self,
        model_id: &str,
        platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeError> {
        let verifier = {
            let mut slot = self.verifier.lock();
            if let Some(verifier) = slot.as_ref() {
                verifier.clone()
            } else {
                let storage = global_local_tts_storage()
                    .map_err(|_| LocalTtsRuntimeError::model_not_installed())?;
                let verifier = Arc::new(LocalTtsRuntimeVerifier::new(storage));
                *slot = Some(verifier.clone());
                verifier
            }
        };
        verifier
            .verified_artifact_paths(model_id, platform)
            .map_err(|error| match error.kind {
                LocalTtsRuntimeVerificationErrorKind::UnknownModel => {
                    LocalTtsRuntimeError::unknown_model()
                }
                LocalTtsRuntimeVerificationErrorKind::CorruptInstall => {
                    LocalTtsRuntimeError::model_not_installed()
                }
                LocalTtsRuntimeVerificationErrorKind::Io
                | LocalTtsRuntimeVerificationErrorKind::Sha256Mismatch => {
                    LocalTtsRuntimeError::verification()
                }
            })
    }
}

pub(super) fn runtime_identity(
    manifest: &LocalTtsModelManifest,
    platform: LocalTtsPlatform,
) -> LocalTtsRuntimeIdentity {
    LocalTtsRuntimeIdentity {
        model_id: manifest.provider_model_id.to_string(),
        model_revision: manifest.model_source_revision.to_string(),
        runtime_compatibility_version: manifest.runtime.compatibility_version,
        adapter_contract: manifest.runtime.adapter_contract.to_string(),
        onnx_runtime_version: manifest.runtime.onnx_runtime_version.to_string(),
        g2p_source_revision: manifest.runtime.g2p_source_revision.to_string(),
        platform,
    }
}

pub(super) fn current_platform() -> Result<LocalTtsPlatform, LocalTtsRuntimeError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok(LocalTtsPlatform::LinuxX86_64);
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(LocalTtsPlatform::MacosArm64);
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(LocalTtsPlatform::MacosX86_64);
    }
    #[allow(unreachable_code)]
    Err(LocalTtsRuntimeError::unsupported_platform())
}
