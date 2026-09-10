use super::*;
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::Barrier;

struct FakeVerifier {
    calls: AtomicUsize,
    fail: AtomicBool,
}

impl FakeVerifier {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
        }
    }
}

impl RuntimeArtifactVerifier for FakeVerifier {
    fn verify(
        &self,
        _model_id: &str,
        _platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(LocalTtsRuntimeError::verification());
        }
        Ok(vec![PathBuf::from("/verified/model-set")])
    }
}

#[derive(Default)]
struct EngineCounters {
    creates: AtomicUsize,
    loads: AtomicUsize,
    unloads: AtomicUsize,
}

struct FakeEngine {
    counters: Arc<EngineCounters>,
}

impl LocalTtsRuntimeEngine for FakeEngine {
    fn load(
        &mut self,
        _identity: &LocalTtsRuntimeIdentity,
        paths: &[PathBuf],
    ) -> Result<(), LocalTtsRuntimeError> {
        assert_eq!(paths, [PathBuf::from("/verified/model-set")]);
        self.counters.loads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn unload(&mut self) {
        self.counters.unloads.fetch_add(1, Ordering::SeqCst);
    }
}

struct FakeFactory {
    counters: Arc<EngineCounters>,
}

impl LocalTtsRuntimeEngineFactory for FakeFactory {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError> {
        self.counters.creates.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(FakeEngine {
            counters: self.counters.clone(),
        }))
    }
}

fn manager() -> (
    LocalTtsRuntimeManager,
    Arc<FakeVerifier>,
    Arc<EngineCounters>,
) {
    let verifier = Arc::new(FakeVerifier::new());
    let counters = Arc::new(EngineCounters::default());
    let manager = LocalTtsRuntimeManager::with_dependencies(
        verifier.clone(),
        Arc::new(FakeFactory {
            counters: counters.clone(),
        }),
    );
    (manager, verifier, counters)
}

#[test]
fn manager_is_lazy_and_starts_unloaded() {
    let (manager, verifier, counters) = manager();
    let status = manager.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string());
    assert_eq!(status.phase, LocalTtsRuntimePhase::Unloaded);
    assert!(status.loaded_model_id.is_none());
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
    assert_eq!(counters.creates.load(Ordering::SeqCst), 0);
}

#[test]
fn app_state_clone_reuses_one_local_tts_runtime_manager() {
    let state = crate::app::state::AppState::new_for_tests().unwrap();
    let cloned = state.clone();

    assert!(Arc::ptr_eq(
        &state.standalone_speech.local_tts_runtime(),
        &cloned.standalone_speech.local_tts_runtime()
    ));
}

#[tokio::test]
async fn warm_runtime_is_reused_and_duplicate_loads_are_serialized() {
    let (manager, verifier, counters) = manager();
    let manager = Arc::new(manager);
    let first = {
        let manager = manager.clone();
        tokio::spawn(async move { manager.ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID).await })
    };
    let second = {
        let manager = manager.clone();
        tokio::spawn(async move { manager.ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID).await })
    };
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    manager
        .ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID)
        .await
        .unwrap();
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert_eq!(counters.creates.load(Ordering::SeqCst), 1);
    assert_eq!(counters.loads.load(Ordering::SeqCst), 1);
    assert_eq!(counters.unloads.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_runtime_operations_are_serialized_on_one_warm_engine() {
    let (manager, verifier, counters) = manager();
    let manager = Arc::new(manager);
    let start = Arc::new(Barrier::new(3));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    let first = {
        let manager = manager.clone();
        let start = start.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        tokio::spawn(async move {
            start.wait().await;
            manager
                .with_loaded_engine(DEFAULT_LOCAL_TTS_MODEL_ID, move |_engine| {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(current, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(25));
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
        })
    };
    let second = {
        let manager = manager.clone();
        let start = start.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        tokio::spawn(async move {
            start.wait().await;
            manager
                .with_loaded_engine(DEFAULT_LOCAL_TTS_MODEL_ID, move |_engine| {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(current, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(25));
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
        })
    };

    start.wait().await;
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();

    assert_eq!(max_active.load(Ordering::SeqCst), 1);
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert_eq!(counters.creates.load(Ordering::SeqCst), 1);
    assert_eq!(counters.loads.load(Ordering::SeqCst), 1);
    assert_eq!(counters.unloads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn changed_runtime_identity_unloads_then_reloads() {
    let (manager, verifier, counters) = manager();
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let first = runtime_identity(manifest, LocalTtsPlatform::LinuxX86_64);
    manager.ensure_loaded_identity(first.clone()).await.unwrap();
    let mut changed = first;
    changed.runtime_compatibility_version += 1;
    manager.ensure_loaded_identity(changed).await.unwrap();
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 2);
    assert_eq!(counters.loads.load(Ordering::SeqCst), 2);
    assert_eq!(counters.unloads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn invalidation_unloads_and_verification_failure_cannot_reload() {
    let (manager, verifier, counters) = manager();
    manager
        .ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID)
        .await
        .unwrap();
    manager
        .invalidate_model(DEFAULT_LOCAL_TTS_MODEL_ID)
        .await
        .unwrap();
    assert_eq!(counters.unloads.load(Ordering::SeqCst), 1);
    assert_eq!(
        manager.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string()).phase,
        LocalTtsRuntimePhase::Unloaded
    );

    verifier.fail.store(true, Ordering::SeqCst);
    let error = manager
        .ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID)
        .await
        .unwrap_err();
    assert_eq!(error.kind, LocalTtsRuntimeErrorKind::Verification);
    assert_eq!(counters.loads.load(Ordering::SeqCst), 1);
    assert_eq!(
        manager.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string()).phase,
        LocalTtsRuntimePhase::Failed
    );
}

#[tokio::test]
async fn shutdown_unloads_and_rejects_future_loads() {
    let (manager, _verifier, counters) = manager();
    manager
        .ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID)
        .await
        .unwrap();
    manager.shutdown().await.unwrap();
    assert_eq!(counters.unloads.load(Ordering::SeqCst), 1);
    let error = manager
        .ensure_loaded(DEFAULT_LOCAL_TTS_MODEL_ID)
        .await
        .unwrap_err();
    assert_eq!(error.kind, LocalTtsRuntimeErrorKind::ShuttingDown);
}
