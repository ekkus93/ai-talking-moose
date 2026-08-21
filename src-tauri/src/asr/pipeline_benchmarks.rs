use super::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
async fn run_cpu_benchmark(architecture: MoonshineModelArchitecture) {
    assert_eq!(
        std::env::var("TALKING_MOOSE_ASR_BENCHMARK").as_deref(),
        Ok("1"),
        "set TALKING_MOOSE_ASR_BENCHMARK=1 to run the opt-in ASR benchmark"
    );
    let model_root = PathBuf::from(
        std::env::var("TALKING_MOOSE_ASR_BENCHMARK_MODEL_ROOT")
            .expect("TALKING_MOOSE_ASR_BENCHMARK_MODEL_ROOT must point at the installer root"),
    );
    let pcm_path = PathBuf::from(
        std::env::var("TALKING_MOOSE_ASR_BENCHMARK_PCM")
            .expect("TALKING_MOOSE_ASR_BENCHMARK_PCM must be 16 kHz mono signed i16-le PCM"),
    );
    let bytes = std::fs::read(&pcm_path).expect("failed to read benchmark PCM corpus");
    const BYTES_PER_100_MS: usize = 16_000 / 10 * std::mem::size_of::<i16>();
    assert!(!bytes.is_empty(), "benchmark PCM must not be empty");
    assert_eq!(
        bytes.len() % BYTES_PER_100_MS,
        0,
        "benchmark PCM must contain whole 100 ms chunks"
    );

    let installer = Arc::new(
        MoonshineModelInstaller::new(model_root).expect("failed to open benchmark model root"),
    );
    let callback: LocalAsrPipelineEventCallback = Arc::new(|_| {});
    let mut pipeline = match architecture {
        MoonshineModelArchitecture::TinyStreaming => {
            LocalAsrPipeline::start_tiny(installer, callback)
                .await
                .expect("failed to start Tiny benchmark pipeline")
        }
        MoonshineModelArchitecture::SmallStreaming => {
            LocalAsrPipeline::start_small(installer, callback)
                .await
                .expect("failed to start Small benchmark pipeline")
        }
    };
    let sender = pipeline.test_sender();
    let mut accepted_chunks = 0_u64;
    let mut dropped_chunks = 0_u64;

    for chunk in bytes.chunks_exact(BYTES_PER_100_MS) {
        match sender.try_send(chunk.to_vec()) {
            Ok(()) => accepted_chunks += 1,
            Err(mpsc::error::TrySendError::Full(_)) => dropped_chunks += 1,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                panic!("benchmark pipeline closed before the corpus finished")
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    let expected_processed_ms = accepted_chunks.saturating_mul(100);
    let deadline = Instant::now() + Duration::from_secs(60);
    while pipeline.diagnostics().processed_audio_ms < expected_processed_ms {
        assert!(pipeline.is_running(), "benchmark pipeline stopped early");
        assert!(Instant::now() < deadline, "benchmark inference timed out");
        thread::sleep(Duration::from_millis(10));
    }

    let diagnostics = pipeline.diagnostics();
    println!(
        concat!(
            "ASR015_BENCHMARK architecture={architecture:?} corpus={} ",
            "accepted_chunks={} dropped_chunks={} diagnostics={diagnostics:#?}"
        ),
        pcm_path.display(),
        accepted_chunks,
        dropped_chunks,
    );
    assert_eq!(
        dropped_chunks, 0,
        "the representative real-time feed overloaded the bounded ASR queue"
    );
    assert!(diagnostics.first_partial_latency_ms.is_some());
    assert!(diagnostics.first_final_latency_ms.is_some());
    assert!(diagnostics.last_error.is_none());
    assert!(
        diagnostics.real_time_factor.is_some_and(|rtf| rtf < 1.0),
        "candidate supported Mac must sustain local ASR faster than real time"
    );
    assert!(diagnostics.process_cpu_time_ms.is_some());
    assert!(diagnostics.average_cpu_utilization_percent.is_some());
    assert!(diagnostics.baseline_resident_memory_bytes.is_some());
    assert!(diagnostics.resident_memory_bytes.is_some());
    assert!(diagnostics.peak_resident_memory_bytes.is_some());

    pipeline.stop_and_join().await.unwrap();
}

/// Opt-in, hardware-dependent ASR-015 acceptance benchmark. Ordinary tests do
/// not load models or native Moonshine and therefore never execute this test.
#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "requires explicit ASR-015 benchmark environment and native Moonshine"]
async fn asr015_cpu_benchmark_tiny_on_supported_mac() {
    run_cpu_benchmark(MoonshineModelArchitecture::TinyStreaming).await;
}

/// Small-model companion to `asr015_cpu_benchmark_tiny_on_supported_mac`.
#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "requires explicit ASR-015 benchmark environment and native Moonshine"]
async fn asr015_cpu_benchmark_small_on_supported_mac() {
    run_cpu_benchmark(MoonshineModelArchitecture::SmallStreaming).await;
}
