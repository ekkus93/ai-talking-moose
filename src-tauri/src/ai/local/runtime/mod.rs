//! Local llama.cpp runtime ownership boundary.
//!
//! Binding-specific llama.cpp objects belong below this module boundary. Application state,
//! commands, and frontend-facing IPC types own or observe `LocalRuntimeManager`, never raw
//! `llama-cpp-2` model/context/sampler values.

mod llama;

use super::catalog::{local_model_entry, LocalModelCatalogEntry};
use super::installer::{LocalModelInstallState, LocalModelInstaller};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

const DEFAULT_CONTEXT_SIZE: u32 = 4_096;
const MAX_DEFAULT_THREADS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalRuntimePhase {
    Ready,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocalRuntimeErrorKind {
    ShuttingDown,
    UnknownModel,
    ModelNotInstalled,
    UnsafeArtifact,
    Initialization,
    ModelLoad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalRuntimeError {
    pub(crate) kind: LocalRuntimeErrorKind,
    pub(crate) message: &'static str,
}

impl LocalRuntimeError {
    fn new(kind: LocalRuntimeErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }

    fn shutting_down() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ShuttingDown,
            "The local model runtime is shutting down.",
        )
    }

    fn unknown_model() -> Self {
        Self::new(
            LocalRuntimeErrorKind::UnknownModel,
            "The selected local model is not in the supported catalog.",
        )
    }

    fn model_not_installed() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ModelNotInstalled,
            "The selected local model is not installed and verified.",
        )
    }

    fn unsafe_artifact() -> Self {
        Self::new(
            LocalRuntimeErrorKind::UnsafeArtifact,
            "The selected local model artifact is not a safe application-owned file.",
        )
    }

    fn initialization() -> Self {
        Self::new(
            LocalRuntimeErrorKind::Initialization,
            "The local model runtime could not be initialized.",
        )
    }

    fn model_load() -> Self {
        Self::new(
            LocalRuntimeErrorKind::ModelLoad,
            "The selected local model could not be loaded.",
        )
    }
}

impl fmt::Display for LocalRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for LocalRuntimeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalRuntimePolicy {
    context_size: u32,
    thread_count: usize,
}

impl LocalRuntimePolicy {
    fn for_available_parallelism(available_parallelism: usize) -> Self {
        let available_parallelism = available_parallelism.max(1);
        let thread_count = (available_parallelism / 2).max(1).min(MAX_DEFAULT_THREADS);
        Self {
            context_size: DEFAULT_CONTEXT_SIZE,
            thread_count,
        }
    }

    pub(crate) fn context_size(self) -> u32 {
        self.context_size
    }

    pub(crate) fn thread_count(self) -> usize {
        self.thread_count
    }
}

impl Default for LocalRuntimePolicy {
    fn default() -> Self {
        let available_parallelism = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        Self::for_available_parallelism(available_parallelism)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeModelIdentity {
    id: String,
    revision: String,
    quantization: String,
}

#[derive(Debug, Clone)]
struct RuntimeModelSpec {
    identity: RuntimeModelIdentity,
    path: PathBuf,
    context_size: u32,
    thread_count: usize,
}

impl RuntimeModelSpec {
    fn for_installed_entry(
        installer: &LocalModelInstaller,
        entry: &'static LocalModelCatalogEntry,
        policy: LocalRuntimePolicy,
    ) -> Result<Self, LocalRuntimeError> {
        let descriptor = installer
            .descriptors(entry.id)
            .into_iter()
            .find(|descriptor| descriptor.id == entry.id)
            .ok_or_else(LocalRuntimeError::unknown_model)?;
        if descriptor.install_state != LocalModelInstallState::Installed {
            return Err(LocalRuntimeError::model_not_installed());
        }

        let model_path = installer
            .model_path(entry.id)
            .map_err(|_| LocalRuntimeError::unknown_model())?;
        let canonical_root =
            fs::canonicalize(installer.root()).map_err(|_| LocalRuntimeError::unsafe_artifact())?;
        let canonical_path =
            fs::canonicalize(model_path).map_err(|_| LocalRuntimeError::model_not_installed())?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(LocalRuntimeError::unsafe_artifact());
        }
        let metadata =
            fs::metadata(&canonical_path).map_err(|_| LocalRuntimeError::unsafe_artifact())?;
        if !metadata.is_file() || metadata.len() != entry.expected_bytes {
            return Err(LocalRuntimeError::unsafe_artifact());
        }

        Ok(Self {
            identity: RuntimeModelIdentity {
                id: entry.id.to_string(),
                revision: entry.revision.to_string(),
                quantization: entry.quantization.to_string(),
            },
            path: canonical_path,
            context_size: policy.context_size().min(entry.context_limit),
            thread_count: policy.thread_count(),
        })
    }
}

trait RuntimeEngine: Send {
    fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError>;
    fn unload_model(&mut self);
}

struct RuntimeState {
    engine: Option<Box<dyn RuntimeEngine>>,
    loaded: Option<RuntimeModelIdentity>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            engine: None,
            loaded: None,
        }
    }

    fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError> {
        if self.loaded.as_ref() == Some(&spec.identity) {
            return Ok(());
        }

        if let Some(engine) = self.engine.as_mut() {
            engine.unload_model();
        }
        self.loaded = None;

        if self.engine.is_none() {
            self.engine = Some(Box::new(llama::LlamaEngine::initialize()?));
        }
        let engine = self.engine.as_mut().expect("runtime engine initialized");
        engine.load_model(spec)?;
        self.loaded = Some(spec.identity.clone());
        Ok(())
    }

    fn unload_model(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.unload_model();
        }
        self.loaded = None;
    }

    fn shutdown(&mut self) {
        self.unload_model();
        self.engine = None;
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct LocalRuntimeLifecycle {
    phase: LocalRuntimePhase,
}

struct LocalRuntimeInner {
    policy: LocalRuntimePolicy,
    lifecycle: RwLock<LocalRuntimeLifecycle>,
    operation_lock: Arc<tokio::sync::Mutex<()>>,
    state: Arc<Mutex<RuntimeState>>,
}

/// Authoritative owner for the local inference runtime lifecycle.
///
/// The manager owns one lazily initialized llama.cpp backend and at most one loaded model. Native
/// contexts/samplers are intentionally not stored here because the selected binding makes those
/// values thread-affine. Future generation creates and destroys them inside one blocking scope.
pub(crate) struct LocalRuntimeManager {
    inner: Arc<LocalRuntimeInner>,
}

impl LocalRuntimeManager {
    pub(crate) fn new() -> Self {
        Self::with_policy(LocalRuntimePolicy::default())
    }

    fn with_policy(policy: LocalRuntimePolicy) -> Self {
        Self {
            inner: Arc::new(LocalRuntimeInner {
                policy,
                lifecycle: RwLock::new(LocalRuntimeLifecycle {
                    phase: LocalRuntimePhase::Ready,
                }),
                operation_lock: Arc::new(tokio::sync::Mutex::new(())),
                state: Arc::new(Mutex::new(RuntimeState::new())),
            }),
        }
    }

    fn ensure_ready(&self) -> Result<(), LocalRuntimeError> {
        if self.inner.lifecycle.read().phase == LocalRuntimePhase::Ready {
            Ok(())
        } else {
            Err(LocalRuntimeError::shutting_down())
        }
    }

    pub(crate) async fn load_installed_model(
        &self,
        installer: Arc<LocalModelInstaller>,
        model_id: String,
    ) -> Result<(), LocalRuntimeError> {
        self.ensure_ready()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_ready()?;

        let state = self.inner.state.clone();
        let policy = self.inner.policy;
        tokio::task::spawn_blocking(move || {
            let entry =
                local_model_entry(&model_id).ok_or_else(LocalRuntimeError::unknown_model)?;
            let spec = RuntimeModelSpec::for_installed_entry(&installer, entry, policy)?;
            state.lock().load_model(&spec)
        })
        .await
        .map_err(|_| LocalRuntimeError::initialization())?
    }

    pub(crate) async fn unload_model(&self) -> Result<(), LocalRuntimeError> {
        self.ensure_ready()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_ready()?;
        let state = self.inner.state.clone();
        tokio::task::spawn_blocking(move || state.lock().unload_model())
            .await
            .map_err(|_| LocalRuntimeError::initialization())?;
        Ok(())
    }

    pub(crate) fn begin_shutdown(&self) {
        self.inner.lifecycle.write().phase = LocalRuntimePhase::ShuttingDown;
    }

    pub(crate) async fn shutdown(&self) -> Result<(), LocalRuntimeError> {
        self.begin_shutdown();
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        let state = self.inner.state.clone();
        tokio::task::spawn_blocking(move || state.lock().shutdown())
            .await
            .map_err(|_| LocalRuntimeError::initialization())?;
        Ok(())
    }

    #[cfg(test)]
    fn phase(&self) -> LocalRuntimePhase {
        self.inner.lifecycle.read().phase
    }
}

impl Default for LocalRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[derive(Clone)]
    struct FakeEngine {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl RuntimeEngine for FakeEngine {
        fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError> {
            self.events
                .lock()
                .push(format!("load:{}", spec.identity.id));
            if spec.identity.id == "fail" {
                Err(LocalRuntimeError::model_load())
            } else {
                Ok(())
            }
        }

        fn unload_model(&mut self) {
            self.events.lock().push("unload".to_string());
        }
    }

    fn test_spec(id: &str) -> RuntimeModelSpec {
        RuntimeModelSpec {
            identity: RuntimeModelIdentity {
                id: id.to_string(),
                revision: "revision".to_string(),
                quantization: "Q4_K_M".to_string(),
            },
            path: PathBuf::from("unused.gguf"),
            context_size: DEFAULT_CONTEXT_SIZE,
            thread_count: 2,
        }
    }

    fn fake_state(events: Arc<Mutex<Vec<String>>>) -> RuntimeState {
        RuntimeState {
            engine: Some(Box::new(FakeEngine { events })),
            loaded: None,
        }
    }

    #[test]
    fn runtime_policy_is_cpu_conservative_and_bounded() {
        let one = LocalRuntimePolicy::for_available_parallelism(1);
        assert_eq!(one.context_size(), DEFAULT_CONTEXT_SIZE);
        assert_eq!(one.thread_count(), 1);
        assert_eq!(
            LocalRuntimePolicy::for_available_parallelism(4).thread_count(),
            2
        );
        assert_eq!(
            LocalRuntimePolicy::for_available_parallelism(16).thread_count(),
            MAX_DEFAULT_THREADS
        );
        assert_eq!(
            LocalRuntimePolicy::for_available_parallelism(128).thread_count(),
            MAX_DEFAULT_THREADS
        );
    }

    #[test]
    fn model_switch_unloads_before_loading_replacement_and_never_falls_back() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut state = fake_state(events.clone());

        state.load_model(&test_spec("first")).unwrap();
        state.load_model(&test_spec("first")).unwrap();
        let error = state.load_model(&test_spec("fail")).unwrap_err();

        assert_eq!(error.kind, LocalRuntimeErrorKind::ModelLoad);
        assert_eq!(state.loaded, None);
        assert_eq!(*events.lock(), vec!["load:first", "unload", "load:fail"]);
    }

    #[tokio::test]
    async fn missing_install_fails_before_native_backend_initialization() {
        let dir = tempdir().unwrap();
        let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
        let manager =
            LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(4));

        let error = manager
            .load_installed_model(
                installer,
                super::super::DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.kind, LocalRuntimeErrorKind::ModelNotInstalled);
        assert!(manager.inner.state.lock().engine.is_none());
    }

    #[tokio::test]
    async fn shutdown_is_idempotent_and_rejects_future_loads() {
        let manager =
            LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(8));
        manager.shutdown().await.unwrap();
        manager.shutdown().await.unwrap();
        assert_eq!(manager.phase(), LocalRuntimePhase::ShuttingDown);

        let dir = tempdir().unwrap();
        let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
        let error = manager
            .load_installed_model(
                installer,
                super::super::DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.kind, LocalRuntimeErrorKind::ShuttingDown);
    }
}
