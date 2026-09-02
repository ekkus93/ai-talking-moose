use super::manager::{RuntimeEngine, RuntimeState};
use super::types::{
    LocalRuntimeError, LocalRuntimeGenerateRequest, LocalRuntimeGeneration, RuntimeModelIdentity,
    RuntimeModelSpec,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct CountingEngine {
    events: Arc<Mutex<Vec<String>>>,
    active_generations: Arc<AtomicUsize>,
    max_active_generations: Arc<AtomicUsize>,
}

impl RuntimeEngine for CountingEngine {
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
        let active = self.active_generations.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active_generations
            .fetch_max(active, Ordering::SeqCst);
        self.events
            .lock()
            .push(format!("generate:{}", spec.identity.id));
        thread::sleep(Duration::from_millis(20));
        self.active_generations.fetch_sub(1, Ordering::SeqCst);
        Ok(LocalRuntimeGeneration {
            text: "fixture".to_string(),
            prompt_tokens: 1,
            output_tokens: 1,
            duration_ms: 20,
            tokens_per_second: Some(50.0),
        })
    }
}

fn spec(id: &str) -> RuntimeModelSpec {
    RuntimeModelSpec {
        identity: RuntimeModelIdentity {
            id: id.to_string(),
            revision: "revision".to_string(),
            quantization: "Q4_K_M".to_string(),
        },
        path: PathBuf::from("unused.gguf"),
        context_size: 1024,
        thread_count: 2,
        max_output_tokens: 32,
    }
}

fn request(model_id: &str) -> LocalRuntimeGenerateRequest {
    LocalRuntimeGenerateRequest {
        model_id: model_id.to_string(),
        system_instruction: None,
        prompt: "test".to_string(),
        temperature: 0.2,
        max_output_tokens: 8,
        seed: 1,
    }
}

fn state_with_counting_engine(
    events: Arc<Mutex<Vec<String>>>,
    active_generations: Arc<AtomicUsize>,
    max_active_generations: Arc<AtomicUsize>,
) -> RuntimeState {
    RuntimeState {
        engine: Some(Box::new(CountingEngine {
            events,
            active_generations,
            max_active_generations,
        })),
        loaded: None,
    }
}

#[test]
fn same_model_is_reused_until_explicit_unload_then_reloads() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut state = state_with_counting_engine(
        events.clone(),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    );
    let selected = spec("smollm2");

    state.load_model(&selected).unwrap();
    state.load_model(&selected).unwrap();
    assert_eq!(*events.lock(), vec!["load:smollm2"]);

    state.unload_model();
    state.load_model(&selected).unwrap();
    assert_eq!(
        *events.lock(),
        vec!["load:smollm2", "unload", "load:smollm2"]
    );
}

#[test]
fn runtime_state_mutex_serializes_native_generation_access() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let active_generations = Arc::new(AtomicUsize::new(0));
    let max_active_generations = Arc::new(AtomicUsize::new(0));
    let selected = spec("smollm2");
    let state = Arc::new(Mutex::new(state_with_counting_engine(
        events,
        active_generations,
        max_active_generations.clone(),
    )));
    state.lock().load_model(&selected).unwrap();

    let mut workers = Vec::new();
    for _ in 0..2 {
        let state = state.clone();
        let selected = selected.clone();
        workers.push(thread::spawn(move || {
            let mut state = state.lock();
            let engine = state.engine.as_mut().expect("test engine installed");
            engine
                .generate(&selected, &request("smollm2"), &CancellationToken::new())
                .unwrap();
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }

    assert_eq!(
        max_active_generations.load(Ordering::SeqCst),
        1,
        "the runtime state lock must never permit overlapping native generation"
    );
}
