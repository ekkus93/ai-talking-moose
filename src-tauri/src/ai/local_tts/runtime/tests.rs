use super::*;
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::Barrier;
use tokio_util::sync::CancellationToken;

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

    fn synthesize(
        &mut self,
        _request: &LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        Ok(LocalTtsInferenceOutput {
            samples: vec![0.0, 0.25, -0.25],
            sample_rate_hz: 24_000,
        })
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

struct BlockingCancellationEngine {
    entered: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
}

impl LocalTtsRuntimeEngine for BlockingCancellationEngine {
    fn load(
        &mut self,
        _identity: &LocalTtsRuntimeIdentity,
        _paths: &[PathBuf],
    ) -> Result<(), LocalTtsRuntimeError> {
        Ok(())
    }

    fn synthesize(
        &mut self,
        _request: &LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        Err(LocalTtsRuntimeError::inference())
    }

    fn synthesize_cancellable(
        &mut self,
        _request: &LocalTtsInferenceRequest,
        cancellation: &LocalTtsRuntimeCancellation,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        self.entered.store(true, Ordering::SeqCst);
        while !cancellation.is_cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        self.exited.store(true, Ordering::SeqCst);
        Err(LocalTtsRuntimeError::cancelled())
    }

    fn unload(&mut self) {}
}

struct BlockingCancellationFactory {
    entered: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
}

impl LocalTtsRuntimeEngineFactory for BlockingCancellationFactory {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError> {
        Ok(Box::new(BlockingCancellationEngine {
            entered: self.entered.clone(),
            exited: self.exited.clone(),
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

fn inference_request() -> LocalTtsInferenceRequest {
    LocalTtsInferenceRequest {
        text: "Cancellation probe".to_string(),
        voice_id: "Jasper".to_string(),
        speaking_rate: 1.0,
        pitch: None,
    }
}

async fn wait_until_true(flag: &AtomicBool) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while !flag.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("test condition should become true");
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
        &state.local_tts_runtime,
        &cloned.local_tts_runtime
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_while_waiting_for_runtime_slot_returns_promptly() {
    let (manager, _verifier, _counters) = manager();
    let manager = Arc::new(manager);
    let blocker_entered = Arc::new(AtomicBool::new(false));
    let blocker = {
        let manager = manager.clone();
        let blocker_entered = blocker_entered.clone();
        tokio::spawn(async move {
            manager
                .with_loaded_engine(DEFAULT_LOCAL_TTS_MODEL_ID, move |_engine| {
                    blocker_entered.store(true, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(200));
                    Ok(())
                })
                .await
        })
    };
    wait_until_true(&blocker_entered).await;

    let cancellation = CancellationToken::new();
    let synthesis = {
        let manager = manager.clone();
        let token = cancellation.clone();
        tokio::spawn(async move {
            manager
                .synthesize_f32_cancellable(
                    DEFAULT_LOCAL_TTS_MODEL_ID,
                    inference_request(),
                    &token,
                )
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(5)).await;
    cancellation.cancel();

    let error = tokio::time::timeout(Duration::from_millis(75), synthesis)
        .await
        .expect("cancelled waiter must not wait for the busy runtime slot")
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind, LocalTtsRuntimeErrorKind::Cancelled);
    blocker.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_inference_cancels_without_starving_unrelated_tokio_work() {
    let verifier = Arc::new(FakeVerifier::new());
    let entered = Arc::new(AtomicBool::new(false));
    let exited = Arc::new(AtomicBool::new(false));
    let manager = Arc::new(LocalTtsRuntimeManager::with_dependencies(
        verifier,
        Arc::new(BlockingCancellationFactory {
            entered: entered.clone(),
            exited: exited.clone(),
        }),
    ));
    let cancellation = CancellationToken::new();
    let synthesis = {
        let manager = manager.clone();
        let token = cancellation.clone();
        tokio::spawn(async move {
            manager
                .synthesize_f32_cancellable(
                    DEFAULT_LOCAL_TTS_MODEL_ID,
                    inference_request(),
                    &token,
                )
                .await
        })
    };
    wait_until_true(&entered).await;

    let unrelated = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        42_u8
    });
    cancellation.cancel();

    assert_eq!(
        tokio::time::timeout(Duration::from_millis(100), unrelated)
            .await
            .expect("blocking inference must not starve Tokio")
            .unwrap(),
        42
    );
    let error = tokio::time::timeout(Duration::from_millis(250), synthesis)
        .await
        .expect("cancelled blocking inference must drain promptly")
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind, LocalTtsRuntimeErrorKind::Cancelled);
    assert!(exited.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_cancels_active_synthesis_and_drains_worker() {
    let verifier = Arc::new(FakeVerifier::new());
    let entered = Arc::new(AtomicBool::new(false));
    let exited = Arc::new(AtomicBool::new(false));
    let manager = Arc::new(LocalTtsRuntimeManager::with_dependencies(
        verifier,
        Arc::new(BlockingCancellationFactory {
            entered: entered.clone(),
            exited: exited.clone(),
        }),
    ));
    let token = CancellationToken::new();
    let synthesis = {
        let manager = manager.clone();
        tokio::spawn(async move {
            manager
                .synthesize_f32_cancellable(
                    DEFAULT_LOCAL_TTS_MODEL_ID,
                    inference_request(),
                    &token,
                )
                .await
        })
    };
    wait_until_true(&entered).await;

    manager.begin_shutdown();
    tokio::time::timeout(Duration::from_millis(250), manager.shutdown())
        .await
        .expect("shutdown must drain a cancelled active worker")
        .unwrap();
    let error = synthesis.await.unwrap().unwrap_err();
    assert_eq!(error.kind, LocalTtsRuntimeErrorKind::Cancelled);
    assert!(exited.load(Ordering::SeqCst));
}

#[tokio::test]
async fn changed_runtime_identity_unloads_then_reloads() {
    let (manager, verifier, counters) = manager();
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let first = runtime_identity(manifest, LocalTtsPlatform::LinuxX86_64);
    manager
        .with_loaded_identity(first.clone(), |_| Ok(()))
        .await
        .unwrap();
    let mut changed = first;
    changed.runtime_compatibility_version += 1;
    manager
        .with_loaded_identity(changed, |_| Ok(()))
        .await
        .unwrap();
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
