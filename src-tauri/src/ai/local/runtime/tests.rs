use super::manager::{LocalRuntimeManager, RuntimeEngine, RuntimeState};
use super::types::{
    LocalRuntimeError, LocalRuntimeErrorKind, LocalRuntimeGenerateRequest, LocalRuntimeGeneration,
    LocalRuntimePhase, LocalRuntimePolicy, RuntimeModelIdentity, RuntimeModelSpec,
    DEFAULT_CONTEXT_SIZE, MAX_DEFAULT_THREADS, MAX_PROMPT_BYTES,
};
use crate::ai::local::{
    local_model_entry, LocalModelInstallState, LocalModelInstaller, DEFAULT_LOCAL_TEXT_MODEL_ID,
};
use parking_lot::Mutex;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::time::Duration;
use tempfile::tempdir;
use tokio::sync::oneshot;
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

struct BlockingEngine {
    events: Arc<Mutex<Vec<String>>>,
    generation_started: Option<oneshot::Sender<()>>,
    release_generation: Arc<Barrier>,
}

impl RuntimeEngine for BlockingEngine {
    fn load_model(&mut self, spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError> {
        self.events
            .lock()
            .push(format!("load:{}", spec.identity.id));
        Ok(())
    }

    fn unload_model(&mut self) {
        self.events.lock().push("unload".to_string());
    }

    fn generate(
        &mut self,
        spec: &RuntimeModelSpec,
        _request: &LocalRuntimeGenerateRequest,
        _cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        self.events
            .lock()
            .push(format!("generate-start:{}", spec.identity.id));
        if let Some(started) = self.generation_started.take() {
            let _ = started.send(());
        }
        self.release_generation.wait();
        self.events
            .lock()
            .push(format!("generate-end:{}", spec.identity.id));
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
        system_instruction: None,
        prompt: prompt.into(),
        temperature: 0.7,
        max_output_tokens: 32,
        seed: 7,
    }
}

fn seed_installed_catalog_artifact(installer: &LocalModelInstaller, model_id: &str) {
    let entry = local_model_entry(model_id).expect("test model must exist in catalog");
    let artifact = installer.model_path(model_id).unwrap();
    let revision_dir = artifact
        .parent()
        .expect("catalog artifact has revision dir");
    fs::create_dir_all(revision_dir).unwrap();
    let file = fs::File::create(&artifact).unwrap();
    file.set_len(entry.expected_bytes).unwrap();
    let marker = serde_json::json!({
        "schema_version": 1,
        "model_id": entry.id,
        "revision": entry.revision,
        "artifact_filename": entry.artifact_filename,
        "expected_bytes": entry.expected_bytes,
        "sha256": entry.sha256,
    });
    fs::write(
        revision_dir.join(".talking-moose-local-llm.json"),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .unwrap();

    let descriptor = installer
        .descriptors(model_id)
        .into_iter()
        .find(|descriptor| descriptor.id == model_id)
        .expect("seeded model descriptor must exist");
    assert_eq!(descriptor.install_state, LocalModelInstallState::Installed);
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

    invalid = request("user");
    invalid.system_instruction = Some("x".repeat(MAX_PROMPT_BYTES));
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
async fn generation_and_delete_are_serialized_without_runtime_corruption() {
    let dir = tempdir().unwrap();
    let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
    seed_installed_catalog_artifact(&installer, DEFAULT_LOCAL_TEXT_MODEL_ID);

    let events = Arc::new(Mutex::new(Vec::new()));
    let (generation_started_tx, generation_started_rx) = oneshot::channel();
    let release_generation = Arc::new(Barrier::new(2));
    let manager = Arc::new(LocalRuntimeManager::with_policy(
        LocalRuntimePolicy::for_available_parallelism(4),
    ));
    {
        let state = manager.state();
        *state.lock() = RuntimeState {
            engine: Some(Box::new(BlockingEngine {
                events: events.clone(),
                generation_started: Some(generation_started_tx),
                release_generation: release_generation.clone(),
            })),
            loaded: None,
        };
    }

    let generation_manager = manager.clone();
    let generation_installer = installer.clone();
    let generation = tokio::spawn(async move {
        generation_manager
            .generate(
                generation_installer,
                request("generation must finish before delete"),
                CancellationToken::new(),
            )
            .await
    });
    generation_started_rx
        .await
        .expect("blocking engine must report generation start");

    let delete_manager = manager.clone();
    let delete_installer = installer.clone();
    let (delete_started_tx, delete_started_rx) = oneshot::channel();
    let deletion = tokio::spawn(async move {
        let _ = delete_started_tx.send(());
        delete_manager
            .delete_model(delete_installer, DEFAULT_LOCAL_TEXT_MODEL_ID.to_string())
            .await
    });
    delete_started_rx
        .await
        .expect("delete task must reach the runtime manager");
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(
        !deletion.is_finished(),
        "delete must wait while native generation owns the operation gate"
    );

    release_generation.wait();
    let generated = generation.await.unwrap().unwrap();
    assert_eq!(generated.text, "fixture");
    deletion.await.unwrap().unwrap();

    assert_eq!(
        *events.lock(),
        vec![
            format!("load:{DEFAULT_LOCAL_TEXT_MODEL_ID}"),
            format!("generate-start:{DEFAULT_LOCAL_TEXT_MODEL_ID}"),
            format!("generate-end:{DEFAULT_LOCAL_TEXT_MODEL_ID}"),
            "unload".to_string(),
        ]
    );
    assert!(manager.state().lock().loaded.is_none());
    let descriptor = installer
        .descriptors(DEFAULT_LOCAL_TEXT_MODEL_ID)
        .into_iter()
        .find(|descriptor| descriptor.id == DEFAULT_LOCAL_TEXT_MODEL_ID)
        .unwrap();
    assert_eq!(
        descriptor.install_state,
        LocalModelInstallState::NotInstalled
    );
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
        object
            .get("selected_model_id")
            .and_then(|value| value.as_str()),
        Some(DEFAULT_LOCAL_TEXT_MODEL_ID)
    );
}
