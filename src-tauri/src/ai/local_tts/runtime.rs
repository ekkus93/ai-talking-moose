use super::installer::{LocalTtsInstallErrorKind, LocalTtsInstaller};
use super::manifest::{local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform};
use super::runtime_verification::{LocalTtsRuntimeVerificationErrorKind, LocalTtsRuntimeVerifier};
use super::storage::{global_local_tts_storage, LocalTtsInstallState};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsRuntimePhase {
    Unloaded,
    Loading,
    Ready,
    Failed,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsRuntimeErrorKind {
    ShuttingDown,
    UnknownModel,
    ModelNotInstalled,
    Verification,
    UnsupportedPlatform,
    RuntimeUnavailable,
    ModelLoad,
    ModelDelete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTtsRuntimeError {
    pub kind: LocalTtsRuntimeErrorKind,
    pub message: &'static str,
}

impl LocalTtsRuntimeError {
    fn new(kind: LocalTtsRuntimeErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }

    fn shutting_down() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::ShuttingDown,
            "The Local TTS runtime is shutting down.",
        )
    }

    fn unknown_model() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::UnknownModel,
            "The selected Local TTS model is not in the supported catalog.",
        )
    }

    fn model_not_installed() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::ModelNotInstalled,
            "The selected Local TTS model is not installed and verified.",
        )
    }

    fn verification() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::Verification,
            "The installed Local TTS artifacts failed runtime-use verification.",
        )
    }

    fn unsupported_platform() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::UnsupportedPlatform,
            "Local TTS is not supported on this platform.",
        )
    }

    fn runtime_unavailable() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::RuntimeUnavailable,
            "The Local TTS inference engine is not integrated in this build yet.",
        )
    }

    fn model_load() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::ModelLoad,
            "The selected Local TTS model could not be loaded.",
        )
    }

    fn model_delete() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::ModelDelete,
            "The selected Local TTS model could not be deleted.",
        )
    }
}

impl std::fmt::Display for LocalTtsRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for LocalTtsRuntimeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LocalTtsRuntimeIdentity {
    pub(super) model_id: String,
    pub(super) model_revision: String,
    pub(super) runtime_compatibility_version: u32,
    pub(super) adapter_contract: String,
    pub(super) onnx_runtime_version: String,
    pub(super) g2p_source_revision: String,
    pub(super) platform: LocalTtsPlatform,
}

pub(super) trait LocalTtsRuntimeEngine: Send {
    fn load(
        &mut self,
        identity: &LocalTtsRuntimeIdentity,
        verified_artifact_paths: &[PathBuf],
    ) -> Result<(), LocalTtsRuntimeError>;
    fn unload(&mut self);
}

trait LocalTtsRuntimeEngineFactory: Send + Sync {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError>;
}

struct PendingLocalTtsRuntimeEngineFactory;

impl LocalTtsRuntimeEngineFactory for PendingLocalTtsRuntimeEngineFactory {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError> {
        Err(LocalTtsRuntimeError::runtime_unavailable())
    }
}

trait RuntimeArtifactVerifier: Send + Sync {
    fn verify(
        &self,
        model_id: &str,
        platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeError>;
}

#[derive(Default)]
struct GlobalRuntimeArtifactVerifier {
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

struct RuntimeState {
    engine: Option<Box<dyn LocalTtsRuntimeEngine>>,
    loaded: Option<LocalTtsRuntimeIdentity>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            engine: None,
            loaded: None,
        }
    }

    fn unload(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.unload();
        }
        self.loaded = None;
    }

    fn unload_if_model(&mut self, model_id: &str) {
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.model_id == model_id)
        {
            self.unload();
        }
    }

    fn shutdown(&mut self) {
        self.unload();
        self.engine = None;
    }
}

struct LocalTtsRuntimeInner {
    operation_lock: Arc<tokio::sync::Mutex<()>>,
    state: Arc<Mutex<RuntimeState>>,
    phase: RwLock<LocalTtsRuntimePhase>,
    last_error: RwLock<Option<LocalTtsRuntimeErrorKind>>,
    verifier: Arc<dyn RuntimeArtifactVerifier>,
    factory: Arc<dyn LocalTtsRuntimeEngineFactory>,
}

/// Shared owner for the Local TTS runtime lifecycle.
///
/// KTT-300 owns lifecycle and admission semantics. KTT-301 supplies the real KittenTTS engine.
/// Every load and future synthesis operation passes through one serialized operation gate; a new
/// model/runtime identity must pass KTT-204 verification before the backend may load it.
pub struct LocalTtsRuntimeManager {
    inner: Arc<LocalTtsRuntimeInner>,
}

impl LocalTtsRuntimeManager {
    pub fn new() -> Self {
        Self::with_dependencies(
            Arc::new(GlobalRuntimeArtifactVerifier::default()),
            Arc::new(PendingLocalTtsRuntimeEngineFactory),
        )
    }

    fn with_dependencies(
        verifier: Arc<dyn RuntimeArtifactVerifier>,
        factory: Arc<dyn LocalTtsRuntimeEngineFactory>,
    ) -> Self {
        Self {
            inner: Arc::new(LocalTtsRuntimeInner {
                operation_lock: Arc::new(tokio::sync::Mutex::new(())),
                state: Arc::new(Mutex::new(RuntimeState::new())),
                phase: RwLock::new(LocalTtsRuntimePhase::Unloaded),
                last_error: RwLock::new(None),
                verifier,
                factory,
            }),
        }
    }

    fn ensure_running(&self) -> Result<(), LocalTtsRuntimeError> {
        if *self.inner.phase.read() == LocalTtsRuntimePhase::ShuttingDown {
            Err(LocalTtsRuntimeError::shutting_down())
        } else {
            Ok(())
        }
    }

    pub async fn ensure_loaded(&self, model_id: &str) -> Result<(), LocalTtsRuntimeError> {
        self.with_loaded_engine(model_id, |_| Ok(())).await
    }

    /// Run one runtime operation against a verified, loaded model while holding the authoritative
    /// Local TTS operation slot. KTT-301 uses this seam for real synthesis so concurrent
    /// utterances cannot create parallel model instances or bypass runtime identity checks.
    pub(super) async fn with_loaded_engine<T, F>(
        &self,
        model_id: &str,
        operation: F,
    ) -> Result<T, LocalTtsRuntimeError>
    where
        T: Send + 'static,
        F: FnOnce(&mut dyn LocalTtsRuntimeEngine) -> Result<T, LocalTtsRuntimeError>
            + Send
            + 'static,
    {
        let platform = current_platform()?;
        let manifest =
            local_tts_model_manifest(model_id).ok_or_else(LocalTtsRuntimeError::unknown_model)?;
        self.with_loaded_identity(runtime_identity(manifest, platform), operation)
            .await
    }

    async fn ensure_loaded_identity(
        &self,
        identity: LocalTtsRuntimeIdentity,
    ) -> Result<(), LocalTtsRuntimeError> {
        self.with_loaded_identity(identity, |_| Ok(())).await
    }

    async fn with_loaded_identity<T, F>(
        &self,
        identity: LocalTtsRuntimeIdentity,
        operation: F,
    ) -> Result<T, LocalTtsRuntimeError>
    where
        T: Send + 'static,
        F: FnOnce(&mut dyn LocalTtsRuntimeEngine) -> Result<T, LocalTtsRuntimeError>
            + Send
            + 'static,
    {
        self.ensure_running()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_running()?;

        let already_loaded = self.inner.state.lock().loaded.as_ref() == Some(&identity);
        if !already_loaded {
            *self.inner.phase.write() = LocalTtsRuntimePhase::Loading;
        }

        let state = self.inner.state.clone();
        let verifier = self.inner.verifier.clone();
        let factory = self.inner.factory.clone();
        let result = tokio::task::spawn_blocking(move || {
            let needs_load = state.lock().loaded.as_ref() != Some(&identity);
            if needs_load {
                {
                    let mut state = state.lock();
                    if state.loaded.is_some() {
                        state.unload();
                    }
                }

                let paths = verifier.verify(&identity.model_id, identity.platform)?;
                let mut state = state.lock();
                if state.engine.is_none() {
                    state.engine = Some(factory.create()?);
                }
                let engine = state
                    .engine
                    .as_mut()
                    .ok_or_else(LocalTtsRuntimeError::model_load)?;
                if let Err(error) = engine.load(&identity, &paths) {
                    engine.unload();
                    state.loaded = None;
                    return Err(error);
                }
                state.loaded = Some(identity);
            }

            let mut state = state.lock();
            let engine = state
                .engine
                .as_mut()
                .ok_or_else(LocalTtsRuntimeError::model_load)?;
            operation(engine.as_mut())
        })
        .await
        .map_err(|_| LocalTtsRuntimeError::model_load())
        .and_then(|result| result);

        match &result {
            Ok(_) => {
                *self.inner.phase.write() = LocalTtsRuntimePhase::Ready;
                *self.inner.last_error.write() = None;
            }
            Err(error) => {
                *self.inner.phase.write() = LocalTtsRuntimePhase::Failed;
                *self.inner.last_error.write() = Some(error.kind);
            }
        }
        result
    }

    pub async fn invalidate_model(&self, model_id: &str) -> Result<(), LocalTtsRuntimeError> {
        self.ensure_running()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_running()?;
        let state = self.inner.state.clone();
        let model_id = model_id.to_string();
        tokio::task::spawn_blocking(move || state.lock().unload_if_model(&model_id))
            .await
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        self.sync_phase_after_unload();
        Ok(())
    }

    pub async fn delete_model(
        &self,
        installer: Arc<LocalTtsInstaller>,
        model_id: String,
    ) -> Result<(), LocalTtsRuntimeError> {
        self.ensure_running()?;
        let platform = current_platform()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_running()?;
        let state = self.inner.state.clone();
        let result = tokio::task::spawn_blocking(move || {
            if local_tts_model_manifest(&model_id).is_none() {
                return Err(LocalTtsRuntimeError::unknown_model());
            }
            let status =
                installer
                    .status(&model_id, platform)
                    .map_err(|error| match error.kind {
                        LocalTtsInstallErrorKind::UnknownModel => {
                            LocalTtsRuntimeError::unknown_model()
                        }
                        _ => LocalTtsRuntimeError::model_delete(),
                    })?;
            if matches!(
                status.install_state,
                LocalTtsInstallState::Downloading
                    | LocalTtsInstallState::Verifying
                    | LocalTtsInstallState::Promoting
            ) {
                return Err(LocalTtsRuntimeError::model_delete());
            }
            state.lock().unload_if_model(&model_id);
            installer
                .delete(&model_id)
                .map_err(|_| LocalTtsRuntimeError::model_delete())
        })
        .await
        .map_err(|_| LocalTtsRuntimeError::model_delete())
        .and_then(|result| result);

        match &result {
            Ok(()) => self.sync_phase_after_unload(),
            Err(error) => *self.inner.last_error.write() = Some(error.kind),
        }
        result
    }

    pub fn status(&self, selected_model_id: String) -> LocalTtsRuntimeStatus {
        let loaded = self.inner.state.lock().loaded.clone();
        LocalTtsRuntimeStatus {
            selected_model_id,
            loaded_model_id: loaded.as_ref().map(|identity| identity.model_id.clone()),
            loaded_revision: loaded
                .as_ref()
                .map(|identity| identity.model_revision.clone()),
            runtime_compatibility_version: loaded
                .as_ref()
                .map(|identity| identity.runtime_compatibility_version),
            phase: *self.inner.phase.read(),
            last_error_category: *self.inner.last_error.read(),
        }
    }

    pub fn begin_shutdown(&self) {
        *self.inner.phase.write() = LocalTtsRuntimePhase::ShuttingDown;
    }

    pub async fn shutdown(&self) -> Result<(), LocalTtsRuntimeError> {
        self.begin_shutdown();
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        let state = self.inner.state.clone();
        tokio::task::spawn_blocking(move || state.lock().shutdown())
            .await
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        Ok(())
    }

    fn sync_phase_after_unload(&self) {
        *self.inner.last_error.write() = None;
        if self.inner.state.lock().loaded.is_none() {
            *self.inner.phase.write() = LocalTtsRuntimePhase::Unloaded;
        }
    }
}

impl Default for LocalTtsRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LocalTtsRuntimeStatus {
    pub selected_model_id: String,
    pub loaded_model_id: Option<String>,
    pub loaded_revision: Option<String>,
    pub runtime_compatibility_version: Option<u32>,
    pub phase: LocalTtsRuntimePhase,
    pub last_error_category: Option<LocalTtsRuntimeErrorKind>,
}

fn runtime_identity(
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

fn current_platform() -> Result<LocalTtsPlatform, LocalTtsRuntimeError> {
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

#[cfg(test)]
mod tests;
