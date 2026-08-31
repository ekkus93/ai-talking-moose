use super::manager::{LocalRuntimeManager, RuntimeEngine, RuntimeState};
use super::types::{
    LocalRuntimeError, LocalRuntimeErrorKind, LocalRuntimeGenerateRequest, LocalRuntimeGeneration,
    LocalRuntimePhase, LocalRuntimePolicy, RuntimeModelIdentity, RuntimeModelSpec,
    DEFAULT_CONTEXT_SIZE, MAX_DEFAULT_THREADS, MAX_PROMPT_BYTES,
};
use crate::ai::local::{LocalModelInstaller, DEFAULT_LOCAL_TEXT_MODEL_ID};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

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

    fn generate(
        &mut self,
        spec: &RuntimeModelSpec,
        _request: &LocalRuntimeGenerateRequest,
        cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        if cancellation.is_cancelled() {
            return Err(LocalRuntimeError::cancelled());
        }
        self.events
            .lock()
            .push(format!("generate:{}", spec.identity.id));
        Ok(LocalRuntimeGeneration {
            text: "fixture".to_string(),
            prompt_tokens: 3,
            output_tokens: 1,
            duration_ms: 5,
            tokens_per_second: Some(200.0),
        })
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
        max_output_tokens: 256,
    }
}

fn fake_state(events: Arc<Mutex<Vec<String>>>) -> RuntimeState {
    RuntimeState {
        engine: Some(Box::new(FakeEngine { events })),
        loaded: None,
    }
}

fn request(prompt: impl Into<String>) -> LocalRuntimeGenerateRequest {
    LocalRuntimeGenerateRequest {
        model_id: DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
        prompt: prompt.into(),
        temperature: 0.7,
        max_output_tokens: 32,
        seed: 7,
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
fn model_switch_unloads_before_replacement_and_never_falls_back() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut state = fake_state(events.clone());

    state.load_model(&test_spec("first")).unwrap();
    state.load_model(&test_spec("first")).unwrap();
    let error = state.load_model(&test_spec("fail")).unwrap_err();

    assert_eq!(error.kind, LocalRuntimeErrorKind::ModelLoad);
    assert!(state.loaded.is_none());
    assert_eq!(*events.lock(), vec!["load:first", "unload", "load:fail"]);
}

#[test]
fn request_bounds_fail_closed_before_native_inference() {
    let mut invalid = request("");
    assert_eq!(
        LocalRuntimeManager::validate_request(&invalid)
            .unwrap_err()
            .kind,
        LocalRuntimeErrorKind::InvalidRequest
    );

    invalid = request("x".repeat(MAX_PROMPT_BYTES + 1));
    assert!(LocalRuntimeManager::validate_request(&invalid).is_err());

    invalid = request("prompt");
    invalid.max_output_tokens = 0;
    assert!(LocalRuntimeManager::validate_request(&invalid).is_err());

    invalid = request("prompt");
    invalid.temperature = f32::NAN;
    assert!(LocalRuntimeManager::validate_request(&invalid).is_err());

    invalid = request("prompt");
    invalid.temperature = 2.01;
    assert!(LocalRuntimeManager::validate_request(&invalid).is_err());
}

#[tokio::test]
async fn missing_install_fails_before_native_backend_initialization() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
    let manager =
        LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(4));

    let error = manager
        .generate(installer, request("hello"), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind, LocalRuntimeErrorKind::ModelNotInstalled);
    assert!(manager.state().lock().engine.is_none());
}

#[tokio::test]
async fn precancelled_request_never_reaches_model_validation_or_backend_init() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
    let manager =
        LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(4));
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let error = manager
        .generate(installer, request("hello"), cancellation)
        .await
        .unwrap_err();
    assert_eq!(error.kind, LocalRuntimeErrorKind::Cancelled);
    assert!(manager.state().lock().engine.is_none());
}

#[tokio::test]
async fn deleting_loaded_model_unloads_before_installer_cleanup() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
    let manager =
        LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(4));
    let events = Arc::new(Mutex::new(Vec::new()));
    {
        let state = manager.state();
        let mut state = state.lock();
        *state = fake_state(events.clone());
        state.loaded = Some(test_spec(DEFAULT_LOCAL_TEXT_MODEL_ID));
    }

    manager
        .delete_model(installer, DEFAULT_LOCAL_TEXT_MODEL_ID.to_string())
        .await
        .unwrap();

    assert_eq!(*events.lock(), vec!["unload"]);
    assert!(manager.state().lock().loaded.is_none());
}

#[tokio::test]
async fn shutdown_is_idempotent_and_rejects_future_generation() {
    let manager =
        LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(8));
    manager.shutdown().await.unwrap();
    manager.shutdown().await.unwrap();
    assert_eq!(manager.phase(), LocalRuntimePhase::ShuttingDown);

    let dir = tempdir().unwrap();
    let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
    let error = manager
        .generate(installer, request("hello"), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind, LocalRuntimeErrorKind::ShuttingDown);
}

#[test]
fn diagnostics_shape_never_contains_prompt_or_generated_text() {
    let manager =
        LocalRuntimeManager::with_policy(LocalRuntimePolicy::for_available_parallelism(4));
    let diagnostics = manager.diagnostics(DEFAULT_LOCAL_TEXT_MODEL_ID.to_string());
    let value = serde_json::to_value(diagnostics).unwrap();
    let object = value.as_object().unwrap();

    assert!(!object.contains_key("prompt"));
    assert!(!object.contains_key("text"));
    assert!(!object.contains_key("output"));
    assert_eq!(
        object.get("selected_model_id").and_then(|value| value.as_str()),
        Some(DEFAULT_LOCAL_TEXT_MODEL_ID)
    );
}
