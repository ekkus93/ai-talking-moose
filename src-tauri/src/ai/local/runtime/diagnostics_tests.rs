use super::manager::{LocalRuntimeManager, RuntimeEngine, RuntimeState};
use super::types::{
    LocalRuntimeError, LocalRuntimeGenerateRequest, LocalRuntimeGeneration, LocalRuntimePolicy,
    RuntimeModelSpec,
};
use crate::ai::local::{local_model_entry, LocalModelInstaller, DEFAULT_LOCAL_TEXT_MODEL_ID};
use parking_lot::Mutex;
use std::fs;
use std::sync::{Arc, Barrier};
use tempfile::tempdir;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

struct DiagnosticsBlockingEngine {
    generation_started: Option<oneshot::Sender<()>>,
    release_generation: Arc<Barrier>,
    observed_prompt: Arc<Mutex<Option<String>>>,
    observed_system_instruction: Arc<Mutex<Option<String>>>,
    generation_text: String,
}

impl RuntimeEngine for DiagnosticsBlockingEngine {
    fn load_model(&mut self, _spec: &RuntimeModelSpec) -> Result<(), LocalRuntimeError> {
        Ok(())
    }

    fn unload_model(&mut self) {}

    fn generate(
        &mut self,
        _spec: &RuntimeModelSpec,
        request: &LocalRuntimeGenerateRequest,
        _cancellation: &CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        *self.observed_prompt.lock() = Some(request.prompt.clone());
        *self.observed_system_instruction.lock() = request.system_instruction.clone();
        if let Some(started) = self.generation_started.take() {
            let _ = started.send(());
        }
        self.release_generation.wait();
        Ok(LocalRuntimeGeneration {
            text: self.generation_text.clone(),
            prompt_tokens: 3,
            output_tokens: 1,
            duration_ms: 5,
            tokens_per_second: Some(200.0),
        })
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
    installer
        .seed_runtime_verification_cache_for_test(entry)
        .expect("runtime fixture cache seed must remain shape-valid");
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

#[tokio::test]
async fn diagnostics_are_live_and_payload_free_during_generation() {
    const PROMPT_SENTINEL: &str = "PROMPT_SENTINEL_P3_PRIVATE";
    const CONTEXT_SENTINEL: &str = "MEMORY_TRANSCRIPT_SENTINEL_P3_PRIVATE";
    const OUTPUT_SENTINEL: &str = "OUTPUT_SENTINEL_P3_PRIVATE";

    let dir = tempdir().unwrap();
    let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
    seed_installed_catalog_artifact(&installer, DEFAULT_LOCAL_TEXT_MODEL_ID);

    let observed_prompt: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let observed_system_instruction: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let (generation_started_tx, generation_started_rx) = oneshot::channel();
    let release_generation = Arc::new(Barrier::new(2));
    let manager = Arc::new(LocalRuntimeManager::with_policy(
        LocalRuntimePolicy::for_available_parallelism(4),
    ));
    {
        let state = manager.state();
        *state.lock() = RuntimeState {
            engine: Some(Box::new(DiagnosticsBlockingEngine {
                generation_started: Some(generation_started_tx),
                release_generation: release_generation.clone(),
                observed_prompt: observed_prompt.clone(),
                observed_system_instruction: observed_system_instruction.clone(),
                generation_text: OUTPUT_SENTINEL.to_string(),
            })),
            loaded: None,
        };
    }

    let before = manager.diagnostics(DEFAULT_LOCAL_TEXT_MODEL_ID.to_string());
    assert!(!before.loaded);
    assert!(!before.generation_in_progress);

    let mut generation_request = request(PROMPT_SENTINEL);
    generation_request.system_instruction = Some(CONTEXT_SENTINEL.to_string());
    let generation_manager = manager.clone();
    let generation_installer = installer.clone();
    let generation = tokio::spawn(async move {
        generation_manager
            .generate(
                generation_installer,
                generation_request,
                CancellationToken::new(),
            )
            .await
    });

    generation_started_rx
        .await
        .expect("blocking engine must report generation start");
    assert_eq!(observed_prompt.lock().as_deref(), Some(PROMPT_SENTINEL));
    assert_eq!(
        observed_system_instruction.lock().as_deref(),
        Some(CONTEXT_SENTINEL)
    );

    let during = manager.diagnostics(DEFAULT_LOCAL_TEXT_MODEL_ID.to_string());
    assert!(during.loaded);
    assert!(during.generation_in_progress);
    assert_eq!(
        during.loaded_model_id.as_deref(),
        Some(DEFAULT_LOCAL_TEXT_MODEL_ID)
    );
    let during_json = serde_json::to_string(&during).unwrap();
    for sentinel in [PROMPT_SENTINEL, CONTEXT_SENTINEL, OUTPUT_SENTINEL] {
        assert!(!during_json.contains(sentinel));
    }

    release_generation.wait();
    let generated = generation.await.unwrap().unwrap();
    assert_eq!(generated.text, OUTPUT_SENTINEL);

    let after = manager.diagnostics(DEFAULT_LOCAL_TEXT_MODEL_ID.to_string());
    assert!(after.loaded);
    assert!(!after.generation_in_progress);
    assert_eq!(after.last_generation_duration_ms, Some(5));
    assert_eq!(after.last_prompt_tokens, Some(3));
    assert_eq!(after.last_output_tokens, Some(1));
    assert_eq!(after.last_tokens_per_second, Some(200.0));
    let after_json = serde_json::to_string(&after).unwrap();
    for sentinel in [PROMPT_SENTINEL, CONTEXT_SENTINEL, OUTPUT_SENTINEL] {
        assert!(!after_json.contains(sentinel));
    }

    manager
        .delete_model(installer, DEFAULT_LOCAL_TEXT_MODEL_ID.to_string())
        .await
        .unwrap();
    let unloaded = manager.diagnostics(DEFAULT_LOCAL_TEXT_MODEL_ID.to_string());
    assert!(!unloaded.loaded);
    assert!(!unloaded.generation_in_progress);
}
