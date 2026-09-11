use super::*;
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

struct FakeVerifier;

impl RuntimeArtifactVerifier for FakeVerifier {
    fn verify(
        &self,
        _model_id: &str,
        _platform: LocalTtsPlatform,
    ) -> Result<Vec<PathBuf>, LocalTtsRuntimeError> {
        Ok(vec![PathBuf::from("/verified/model-set")])
    }
}

#[derive(Default)]
struct Counters {
    loads: AtomicUsize,
}

struct FakeEngine {
    counters: Arc<Counters>,
}

impl LocalTtsRuntimeEngine for FakeEngine {
    fn load(
        &mut self,
        _identity: &LocalTtsRuntimeIdentity,
        _verified_artifact_paths: &[PathBuf],
    ) -> Result<(), LocalTtsRuntimeError> {
        self.counters.loads.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }

    fn synthesize(
        &mut self,
        _request: &LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        std::thread::sleep(Duration::from_millis(2));
        Ok(LocalTtsInferenceOutput {
            samples: vec![0.0, 0.25, -0.25],
            sample_rate_hz: 24_000,
        })
    }

    fn unload(&mut self) {}
}

struct FakeFactory {
    counters: Arc<Counters>,
}

impl LocalTtsRuntimeEngineFactory for FakeFactory {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError> {
        Ok(Box::new(FakeEngine {
            counters: self.counters.clone(),
        }))
    }
}

fn manager() -> (LocalTtsRuntimeManager, Arc<Counters>) {
    let counters = Arc::new(Counters::default());
    (
        LocalTtsRuntimeManager::with_dependencies(
            Arc::new(FakeVerifier),
            Arc::new(FakeFactory {
                counters: counters.clone(),
            }),
        ),
        counters,
    )
}

#[tokio::test]
async fn diagnostics_record_safe_runtime_metrics_without_request_or_audio_payload() {
    let (manager, counters) = manager();
    let sentinel = "KTT305_DIAGNOSTIC_SENTINEL_DO_NOT_EXPOSE";
    let output = manager
        .synthesize_f32(
            DEFAULT_LOCAL_TTS_MODEL_ID,
            LocalTtsInferenceRequest {
                text: sentinel.to_string(),
                voice_id: "Jasper".to_string(),
                speaking_rate: 1.0,
                pitch: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(output.sample_rate_hz, 24_000);
    assert_eq!(counters.loads.load(Ordering::SeqCst), 1);

    let status = manager.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string());
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    assert_eq!(status.phase, LocalTtsRuntimePhase::Ready);
    assert_eq!(status.sample_rate_hz, Some(manifest.sample_rate_hz));
    assert_eq!(
        status.inference_thread_count,
        Some(u32::from(manifest.runtime.inference_threads))
    );
    assert!(status.last_model_load_duration_ms.is_some());
    assert!(status.last_synthesis_duration_ms.is_some());
    let audio_ms = status.last_generated_audio_duration_ms.unwrap();
    assert!((audio_ms - 0.125).abs() < f64::EPSILON);
    let rtf = status.last_real_time_factor.unwrap();
    assert!(rtf.is_finite());
    assert!(rtf > 0.0);
    assert_eq!(status.last_error_category, None);

    let serialized = serde_json::to_string(&status).unwrap();
    assert!(!serialized.contains(sentinel));
    assert!(!serialized.contains("samples"));
    assert!(!serialized.contains("pcm_bytes"));
}

#[test]
fn failed_synthesis_clears_audio_metrics_but_preserves_safe_error_only() {
    let mut telemetry = RuntimeTelemetry::default();
    telemetry.record_synthesis(
        &Ok(LocalTtsInferenceOutput {
            samples: vec![0.25; 24_000],
            sample_rate_hz: 24_000,
        }),
        Duration::from_millis(250),
    );
    assert_eq!(telemetry.last_generated_audio_duration_ms, Some(1_000.0));
    assert_eq!(telemetry.last_real_time_factor, Some(0.25));

    telemetry.record_synthesis(
        &Err(LocalTtsRuntimeError::inference()),
        Duration::from_millis(10),
    );
    assert_eq!(telemetry.last_generated_audio_duration_ms, None);
    assert_eq!(telemetry.last_real_time_factor, None);
    assert_eq!(telemetry.last_synthesis_duration_ms, Some(10));
}
