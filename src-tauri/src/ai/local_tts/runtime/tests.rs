use super::*;
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
