use super::installer::{LocalTtsInstallErrorKind, LocalTtsInstaller};
use super::manifest::{local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform};
use super::runtime_verification::{LocalTtsRuntimeVerificationErrorKind, LocalTtsRuntimeVerifier};
use super::storage::{global_local_tts_storage, LocalTtsInstallState};
use ort::session::RunOptions;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

mod engine;
mod npz;
mod tokenize;

use engine::KittenTtsRuntimeEngineFactory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsRuntimePhase {
    Unloaded,
    Loading,
    Generating,
    Ready,
    Failed,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTtsRuntimeErrorKind {
    ShuttingDown,
    Cancelled,
    UnknownModel,
    ModelNotInstalled,
    Verification,
    UnsupportedPlatform,
    RuntimeUnavailable,
    InvalidInput,
    InvalidVoice,
    UnsupportedConfig,
    Inference,
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

    fn cancelled() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::Cancelled,
            "Local TTS synthesis was cancelled.",
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

    fn invalid_input() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::InvalidInput,
            "The Local TTS input is empty, too long, or unsupported.",
        )
    }

    fn invalid_voice() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::InvalidVoice,
            "The selected Local TTS voice is unavailable.",
        )
    }

    fn unsupported_config() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::UnsupportedConfig,
            "The selected Local TTS configuration is unsupported.",
        )
    }

    fn inference() -> Self {
        Self::new(
            LocalTtsRuntimeErrorKind::Inference,
            "Local TTS inference failed.",
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

#[derive(Debug, Clone)]
pub struct LocalTtsInferenceRequest {
    pub text: String,
    pub voice_id: String,
    pub speaking_rate: f32,
    pub pitch: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct LocalTtsInferenceOutput {
    pub samples: Vec<f32>,
    pub sample_rate_hz: u32,
}

#[derive(Default)]
struct LocalTtsRuntimeCancellationInner {
    cancelled: AtomicBool,
    run_options: Mutex<Option<Arc<RunOptions>>>,
}

#[derive(Clone, Default)]
pub(super) struct LocalTtsRuntimeCancellation {
    inner: Arc<LocalTtsRuntimeCancellationInner>,
}

impl LocalTtsRuntimeCancellation {
    fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        let run_options = self.inner.run_options.lock().clone();
        if let Some(run_options) = run_options {
            let _ = run_options.terminate();
        }
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub(super) fn check_cancelled(&self) -> Result<(), LocalTtsRuntimeError> {
        if self.is_cancelled() {
            Err(LocalTtsRuntimeError::cancelled())
        } else {
            Ok(())
        }
    }

    pub(super) fn install_run_options(&self, run_options: Arc<RunOptions>) {
        *self.inner.run_options.lock() = Some(run_options.clone());
        if self.is_cancelled() {
            let _ = run_options.terminate();
        }
    }

    pub(super) fn clear_run_options(&self) {
        *self.inner.run_options.lock() = None;
    }

    #[cfg(test)]
    pub(super) fn has_run_options(&self) -> bool {
        self.inner.run_options.lock().is_some()
    }
}

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
    fn synthesize(
        &mut self,
        request: &LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError>;
    fn synthesize_cancellable(
        &mut self,
        request: &LocalTtsInferenceRequest,
        cancellation: &LocalTtsRuntimeCancellation,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        cancellation.check_cancelled()?;
        self.synthesize(request)
    }
    fn unload(&mut self);
}

trait LocalTtsRuntimeEngineFactory: Send + Sync {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError>;
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

#[derive(Debug, Clone, Default)]
struct RuntimeTelemetry {
    last_model_load_duration_ms: Option<u64>,
    last_synthesis_duration_ms: Option<u64>,
    last_generated_audio_duration_ms: Option<f64>,
    last_real_time_factor: Option<f64>,
}

impl RuntimeTelemetry {
    fn record_model_load(&mut self, duration: Duration) {
        self.last_model_load_duration_ms = Some(duration.as_millis().try_into().unwrap_or(u64::MAX));
    }

    fn record_synthesis(
        &mut self,
        result: &Result<LocalTtsInferenceOutput, LocalTtsRuntimeError>,
        duration: Duration,
    ) {
        self.last_synthesis_duration_ms =
            Some(duration.as_millis().try_into().unwrap_or(u64::MAX));
        match result {
            Ok(output) if output.sample_rate_hz > 0 && !output.samples.is_empty() => {
                let audio_duration_ms =
                    output.samples.len() as f64 * 1_000.0 / f64::from(output.sample_rate_hz);
                self.last_generated_audio_duration_ms = Some(audio_duration_ms);
                self.last_real_time_factor =
                    Some(duration.as_secs_f64() * 1_000.0 / audio_duration_ms);
            }
            _ => {
                self.last_generated_audio_duration_ms = None;
                self.last_real_time_factor = None;
            }
        }
    }
}

struct LocalTtsRuntimeInner {
    operation_lock: Arc<tokio::sync::Mutex<()>>,
    state: Arc<Mutex<RuntimeState>>,
    phase: RwLock<LocalTtsRuntimePhase>,
    last_error: RwLock<Option<LocalTtsRuntimeErrorKind>>,
    telemetry: RwLock<RuntimeTelemetry>,
    active_synthesis: Mutex<Option<LocalTtsRuntimeCancellation>>,
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
            Arc::new(KittenTtsRuntimeEngineFactory),
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
                telemetry: RwLock::new(RuntimeTelemetry::default()),
                active_synthesis: Mutex::new(None),
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

    pub async fn synthesize_f32(
        &self,
        model_id: &str,
        request: LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        let inner = self.inner.clone();
        self.with_loaded_engine(model_id, move |engine| {
            *inner.phase.write() = LocalTtsRuntimePhase::Generating;
            let started = Instant::now();
            let result = engine.synthesize(&request);
            inner
                .telemetry
                .write()
                .record_synthesis(&result, started.elapsed());
            result
        })
        .await
    }

    pub async fn synthesize_f32_cancellable(
        &self,
        model_id: &str,
        request: LocalTtsInferenceRequest,
        cancellation: &CancellationToken,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        self.ensure_running()?;
        if cancellation.is_cancelled() {
            return Err(LocalTtsRuntimeError::cancelled());
        }

        let platform = current_platform()?;
        let manifest =
            local_tts_model_manifest(model_id).ok_or_else(LocalTtsRuntimeError::unknown_model)?;
        let identity = runtime_identity(manifest, platform);
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = tokio::select! {
            () = cancellation.cancelled() => {
                return Err(LocalTtsRuntimeError::cancelled());
            }
            operation = operation_lock.lock_owned() => operation,
        };
        self.ensure_running()?;
        if cancellation.is_cancelled() {
            return Err(LocalTtsRuntimeError::cancelled());
        }

        let runtime_cancellation = LocalTtsRuntimeCancellation::default();
        *self.inner.active_synthesis.lock() = Some(runtime_cancellation.clone());
        let watcher_cancellation = runtime_cancellation.clone();
        let token = cancellation.clone();
        let watcher = tokio::spawn(async move {
            token.cancelled().await;
            watcher_cancellation.cancel();
        });

        let worker_cancellation = runtime_cancellation.clone();
        let inner = self.inner.clone();
        let result = self
            .with_loaded_identity_locked(identity, move |engine| {
                worker_cancellation.check_cancelled()?;
                *inner.phase.write() = LocalTtsRuntimePhase::Generating;
                let started = Instant::now();
                let result = engine.synthesize_cancellable(&request, &worker_cancellation);
                inner
                    .telemetry
                    .write()
                    .record_synthesis(&result, started.elapsed());
                result
            })
            .await;

        watcher.abort();
        self.inner.active_synthesis.lock().take();
        if cancellation.is_cancelled() {
            *self.inner.last_error.write() = Some(LocalTtsRuntimeErrorKind::Cancelled);
            return Err(LocalTtsRuntimeError::cancelled());
        }
        result
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
        self.with_loaded_identity_locked(identity, operation).await
    }

    async fn with_loaded_identity_locked<T, F>(
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
        let already_loaded = self.inner.state.lock().loaded.as_ref() == Some(&identity);
        if !already_loaded {
            *self.inner.phase.write() = LocalTtsRuntimePhase::Loading;
        }

        let state = self.inner.state.clone();
        let verifier = self.inner.verifier.clone();
        let factory = self.inner.factory.clone();
        let inner = self.inner.clone();
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
                let load_started = Instant::now();
                let load_result = engine.load(&identity, &paths);
                inner
                    .telemetry
                    .write()
                    .record_model_load(load_started.elapsed());
                if let Err(error) = load_result {
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
            Err(error) if error.kind == LocalTtsRuntimeErrorKind::Cancelled => {
                *self.inner.phase.write() = if self.inner.state.lock().loaded.is_some() {
                    LocalTtsRuntimePhase::Ready
                } else {
                    LocalTtsRuntimePhase::Unloaded
                };
                *self.inner.last_error.write() = Some(LocalTtsRuntimeErrorKind::Cancelled);
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
            let status = installer.status(&model_id, platform).map_err(|error| match error.kind {
                LocalTtsInstallErrorKind::UnknownModel => LocalTtsRuntimeError::unknown_model(),
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
        let manifest = loaded
            .as_ref()
            .and_then(|identity| local_tts_model_manifest(&identity.model_id))
            .or_else(|| local_tts_model_manifest(&selected_model_id));
        let telemetry = self.inner.telemetry.read().clone();
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
            sample_rate_hz: manifest.map(|manifest| manifest.sample_rate_hz),
            inference_thread_count: manifest
                .map(|manifest| u32::from(manifest.runtime.inference_threads)),
            last_model_load_duration_ms: telemetry.last_model_load_duration_ms,
            last_synthesis_duration_ms: telemetry.last_synthesis_duration_ms,
            last_generated_audio_duration_ms: telemetry.last_generated_audio_duration_ms,
            last_real_time_factor: telemetry.last_real_time_factor,
            last_error_category: *self.inner.last_error.read(),
        }
    }

    pub fn begin_shutdown(&self) {
        *self.inner.phase.write() = LocalTtsRuntimePhase::ShuttingDown;
        if let Some(cancellation) = self.inner.active_synthesis.lock().as_ref() {
            cancellation.cancel();
        }
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LocalTtsRuntimeStatus {
    pub selected_model_id: String,
    pub loaded_model_id: Option<String>,
    pub loaded_revision: Option<String>,
    pub runtime_compatibility_version: Option<u32>,
    pub phase: LocalTtsRuntimePhase,
    pub sample_rate_hz: Option<u32>,
    pub inference_thread_count: Option<u32>,
    pub last_model_load_duration_ms: Option<u64>,
    pub last_synthesis_duration_ms: Option<u64>,
    pub last_generated_audio_duration_ms: Option<f64>,
    pub last_real_time_factor: Option<f64>,
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
mod telemetry_tests;
#[cfg(test)]
mod tests;
