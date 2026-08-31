use super::llama::LlamaEngine;
use super::types::{
    LocalRuntimeDiagnostics, LocalRuntimeError, LocalRuntimeErrorKind, LocalRuntimeGenerateRequest,
    LocalRuntimeGeneration, LocalRuntimePhase, LocalRuntimePolicy, RuntimeModelIdentity,
    RuntimeModelSpec, MAX_PROMPT_BYTES, MAX_TEMPERATURE,
};
use crate::ai::local::catalog::{local_model_entry, LocalModelCatalogEntry};
use crate::ai::local::installer::{
    LocalModelInstallErrorKind, LocalModelInstallState, LocalModelInstaller,
};
use parking_lot::{Mutex, RwLock};
use std::fs;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(super) trait RuntimeEngine: Send {
    fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError>;
    fn unload_model(&mut self);
    fn generate(
        &mut self,
        spec: &RuntimeModelSpec,
        request: &LocalRuntimeGenerateRequest,
        cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError>;
}

pub(super) struct RuntimeState {
    pub(super) engine: Option<Box<dyn RuntimeEngine>>,
    pub(super) loaded: Option<RuntimeModelSpec>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            engine: None,
            loaded: None,
        }
    }

    pub(super) fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError> {
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.identity == spec.identity)
        {
            return Ok(());
        }

        if let Some(engine) = self.engine.as_mut() {
            engine.unload_model();
        }
        self.loaded = None;

        if self.engine.is_none() {
            self.engine = Some(Box::new(LlamaEngine::initialize()?));
        }
        let engine = self.engine.as_mut().expect("runtime engine initialized");
        engine.load_model(spec)?;
        self.loaded = Some(spec.clone());
        Ok(())
    }

    fn generate(
        &mut self,
        spec: &RuntimeModelSpec,
        request: &LocalRuntimeGenerateRequest,
        cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        self.load_model(spec)?;
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(LocalRuntimeError::model_not_loaded)?;
        engine.generate(spec, request, cancellation)
    }

    pub(super) fn unload_model(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.unload_model();
        }
        self.loaded = None;
    }

    fn unload_model_if(&mut self, model_id: &str) {
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.identity.id == model_id)
        {
            self.unload_model();
        }
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

#[derive(Debug, Default)]
struct RuntimeTelemetry {
    loaded: Option<RuntimeModelSpec>,
    generation_in_progress: bool,
    last_error_category: Option<LocalRuntimeErrorKind>,
    last_generation_duration_ms: Option<u64>,
    last_prompt_tokens: Option<u32>,
    last_output_tokens: Option<u32>,
    last_tokens_per_second: Option<f32>,
}

struct LocalRuntimeInner {
    policy: LocalRuntimePolicy,
    lifecycle: RwLock<LocalRuntimeLifecycle>,
    operation_lock: Arc<tokio::sync::Mutex<()>>,
    state: Arc<Mutex<RuntimeState>>,
    telemetry: RwLock<RuntimeTelemetry>,
}

/// Authoritative owner for the local inference runtime lifecycle.
///
/// The manager owns one lazily initialized llama.cpp backend and at most one loaded model. Native
/// contexts/samplers are intentionally not stored here because the selected binding makes those
/// values thread-affine. Every load, generation, unload, delete, and shutdown transition is
/// serialized through one operation gate.
pub(crate) struct LocalRuntimeManager {
    inner: Arc<LocalRuntimeInner>,
}

impl LocalRuntimeManager {
    pub(crate) fn new() -> Self {
        Self::with_policy(LocalRuntimePolicy::default())
    }

    pub(super) fn with_policy(policy: LocalRuntimePolicy) -> Self {
        Self {
            inner: Arc::new(LocalRuntimeInner {
                policy,
                lifecycle: RwLock::new(LocalRuntimeLifecycle {
                    phase: LocalRuntimePhase::Ready,
                }),
                operation_lock: Arc::new(tokio::sync::Mutex::new(())),
                state: Arc::new(Mutex::new(RuntimeState::new())),
                telemetry: RwLock::new(RuntimeTelemetry::default()),
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

    pub(super) fn validate_request(
        request: &LocalRuntimeGenerateRequest,
    ) -> Result<(), LocalRuntimeError> {
        if request.prompt.is_empty()
            || request.prompt.len() > MAX_PROMPT_BYTES
            || request.max_output_tokens == 0
            || !request.temperature.is_finite()
            || !(0.0..=MAX_TEMPERATURE).contains(&request.temperature)
        {
            return Err(LocalRuntimeError::invalid_request());
        }
        Ok(())
    }

    // P5 establishes the native generation entry point before P6 wires it into TextModel.
    #[allow(dead_code)]
    pub(crate) async fn generate(
        &self,
        installer: Arc<LocalModelInstaller>,
        request: LocalRuntimeGenerateRequest,
        cancellation: CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        Self::validate_request(&request)?;
        self.ensure_ready()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_ready()?;
        if cancellation.is_cancelled() {
            return Err(LocalRuntimeError::cancelled());
        }

        self.inner.telemetry.write().generation_in_progress = true;
        let state = self.inner.state.clone();
        let policy = self.inner.policy;
        let result = tokio::task::spawn_blocking(move || {
            let entry = local_model_entry(&request.model_id)
                .ok_or_else(LocalRuntimeError::unknown_model)?;
            let spec = RuntimeModelSpec::for_installed_entry(&installer, entry, policy)?;
            if request.max_output_tokens > spec.max_output_tokens {
                return Err(LocalRuntimeError::invalid_request());
            }
            state.lock().generate(&spec, &request, &cancellation)
        })
        .await
        .map_err(|_| LocalRuntimeError::initialization())
        .and_then(|result| result);

        self.finish_generation(&result);
        result
    }

    pub(crate) async fn delete_model(
        &self,
        installer: Arc<LocalModelInstaller>,
        model_id: String,
    ) -> Result<(), LocalRuntimeError> {
        self.ensure_ready()?;
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        self.ensure_ready()?;

        let state = self.inner.state.clone();
        let result = tokio::task::spawn_blocking(move || {
            if local_model_entry(&model_id).is_none() {
                return Err(LocalRuntimeError::unknown_model());
            }
            let descriptors = installer.descriptors(&model_id);
            if descriptors.iter().any(|descriptor| {
                descriptor.id == model_id
                    && matches!(
                        descriptor.install_state,
                        LocalModelInstallState::Downloading | LocalModelInstallState::Verifying
                    )
            }) {
                return Err(LocalRuntimeError::new(
                    LocalRuntimeErrorKind::ModelDelete,
                    "The local model cannot be deleted while installation is in progress.",
                ));
            }

            state.lock().unload_model_if(&model_id);
            installer
                .delete(&model_id)
                .map_err(|error| match error.kind {
                    LocalModelInstallErrorKind::UnknownModel => LocalRuntimeError::unknown_model(),
                    LocalModelInstallErrorKind::Busy => LocalRuntimeError::new(
                        LocalRuntimeErrorKind::ModelDelete,
                        "The local model cannot be deleted while installation is in progress.",
                    ),
                    _ => LocalRuntimeError::model_delete(),
                })
        })
        .await
        .map_err(|_| LocalRuntimeError::initialization())
        .and_then(|result| result);

        self.finish_non_generation(&result);
        result
    }

    // P5 exposes sanitized runtime telemetry here; the frontend IPC shape is wired separately.
    #[allow(dead_code)]
    pub(crate) fn diagnostics(&self, selected_model_id: String) -> LocalRuntimeDiagnostics {
        let phase = self.inner.lifecycle.read().phase;
        let telemetry = self.inner.telemetry.read();
        let loaded = telemetry.loaded.as_ref();
        LocalRuntimeDiagnostics {
            selected_model_id,
            loaded_model_id: loaded.map(|spec| spec.identity.id.clone()),
            loaded_revision: loaded.map(|spec| spec.identity.revision.clone()),
            loaded_quantization: loaded.map(|spec| spec.identity.quantization.clone()),
            loaded: loaded.is_some(),
            phase,
            thread_count: loaded
                .map_or(self.inner.policy.thread_count(), |spec| spec.thread_count)
                .try_into()
                .unwrap_or(u32::MAX),
            context_size: loaded.map_or(self.inner.policy.context_size(), |spec| spec.context_size),
            generation_in_progress: telemetry.generation_in_progress,
            last_error_category: telemetry.last_error_category,
            last_generation_duration_ms: telemetry.last_generation_duration_ms,
            last_prompt_tokens: telemetry.last_prompt_tokens,
            last_output_tokens: telemetry.last_output_tokens,
            last_tokens_per_second: telemetry.last_tokens_per_second,
        }
    }

    pub(crate) fn begin_shutdown(&self) {
        self.inner.lifecycle.write().phase = LocalRuntimePhase::ShuttingDown;
    }

    pub(crate) async fn shutdown(&self) -> Result<(), LocalRuntimeError> {
        self.begin_shutdown();
        let operation_lock = self.inner.operation_lock.clone();
        let _operation = operation_lock.lock_owned().await;
        let state = self.inner.state.clone();
        let result = tokio::task::spawn_blocking(move || state.lock().shutdown())
            .await
            .map_err(|_| LocalRuntimeError::initialization());
        self.finish_non_generation(&result);
        result
    }

    fn finish_generation(&self, result: &Result<LocalRuntimeGeneration, LocalRuntimeError>) {
        let loaded = self.inner.state.lock().loaded.clone();
        let mut telemetry = self.inner.telemetry.write();
        telemetry.generation_in_progress = false;
        telemetry.loaded = loaded;
        match result {
            Ok(generation) => {
                telemetry.last_error_category = None;
                telemetry.last_generation_duration_ms = Some(generation.duration_ms);
                telemetry.last_prompt_tokens = Some(generation.prompt_tokens);
                telemetry.last_output_tokens = Some(generation.output_tokens);
                telemetry.last_tokens_per_second = generation.tokens_per_second;
            }
            Err(error) => telemetry.last_error_category = Some(error.kind),
        }
    }

    fn finish_non_generation(&self, result: &Result<(), LocalRuntimeError>) {
        let loaded = self.inner.state.lock().loaded.clone();
        let mut telemetry = self.inner.telemetry.write();
        telemetry.loaded = loaded;
        match result {
            Ok(()) => telemetry.last_error_category = None,
            Err(error) => telemetry.last_error_category = Some(error.kind),
        }
    }

    #[cfg(test)]
    pub(super) fn phase(&self) -> LocalRuntimePhase {
        self.inner.lifecycle.read().phase
    }

    #[cfg(test)]
    pub(super) fn state(&self) -> Arc<Mutex<RuntimeState>> {
        self.inner.state.clone()
    }
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
            max_output_tokens: entry.recommended_max_output,
        })
    }
}

impl Default for LocalRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}
