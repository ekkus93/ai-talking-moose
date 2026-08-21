use super::*;
use crate::asr::moonshine::{model_manifest_info, MoonshineModelInstallCancellation};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
fn benchmark_architecture_name(architecture: MoonshineModelArchitecture) -> &'static str {
    match architecture {
        MoonshineModelArchitecture::TinyStreaming => "tiny_streaming",
        MoonshineModelArchitecture::SmallStreaming => "small_streaming",
    }
}

#[cfg(target_os = "macos")]
fn assert_useful_transcript(text: &str) {
    let normalized = text.to_ascii_lowercase();
    let expected_words = ["fellow", "americans", "ask", "country", "you"];
    let hits = expected_words
        .iter()
        .filter(|word| normalized.contains(*word))
        .count();
    assert!(
        hits >= 3,
        "native benchmark transcript is not recognizably derived from the pinned JFK corpus: {text:?}"
    );
}

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
    if std::env::var("TALKING_MOOSE_ASR_BENCHMARK_INSTALL").as_deref() == Ok("1") {
        let cancellation = MoonshineModelInstallCancellation::default();
        let outcome = installer
            .install(architecture, &cancellation)
            .await
            .expect("failed to install/verify the pinned Moonshine benchmark model");
        println!(
            "ASR015_MODEL_READY architecture={} model_id={} revision={} bytes={} disposition={:?}",
            benchmark_architecture_name(architecture),
            outcome.model_id,
            outcome.revision,
            outcome.installed_bytes,
            outcome.disposition,
        );
    }

    let events = Arc::new(Mutex::new(Vec::<AsrEvent>::new()));
    let callback_events = events.clone();
    let callback: LocalAsrPipelineEventCallback = Arc::new(move |event| {
        callback_events.lock().push(event);
    });
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

    while !events
        .lock()
        .iter()
        .any(|event| matches!(event, AsrEvent::FinalTranscript { .. }))
    {
        assert!(
            pipeline.is_running(),
            "benchmark pipeline stopped before a final transcript"
        );
        assert!(
            Instant::now() < deadline,
            "benchmark final transcript timed out"
        );
        thread::sleep(Duration::from_millis(10));
    }

    let diagnostics = pipeline.diagnostics();
    let final_transcript = events
        .lock()
        .iter()
        .filter_map(|event| match event {
            AsrEvent::FinalTranscript { text } if !text.trim().is_empty() => Some(text.trim()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert_useful_transcript(&final_transcript);

    println!(
        "ASR015_BENCHMARK architecture={:?} corpus={} accepted_chunks={} dropped_chunks={} diagnostics={:#?}",
        architecture,
        pcm_path.display(),
        accepted_chunks,
        dropped_chunks,
        diagnostics,
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

    let model = model_manifest_info(architecture);
    let phase = std::env::var("TALKING_MOOSE_ASR_BENCHMARK_PHASE")
        .unwrap_or_else(|_| "unspecified".to_string());
    let run = std::env::var("TALKING_MOOSE_ASR_BENCHMARK_RUN")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let record = serde_json::json!({
        "architecture": benchmark_architecture_name(architecture),
        "phase": phase,
        "run": run,
        "model_id": model.id,
        "model_revision": model.revision,
        "model_expected_bytes": model.expected_bytes,
        "runtime_release": model.runtime_release,
        "accepted_chunks": accepted_chunks,
        "dropped_chunks": dropped_chunks,
        "first_partial_latency_ms": diagnostics.first_partial_latency_ms,
        "first_final_latency_ms": diagnostics.first_final_latency_ms,
        "last_transcription_latency_ms": diagnostics.last_transcription_latency_ms,
        "processed_audio_ms": diagnostics.processed_audio_ms,
        "inference_wall_time_ms": diagnostics.inference_wall_time_ms,
        "real_time_factor": diagnostics.real_time_factor,
        "process_cpu_time_ms": diagnostics.process_cpu_time_ms,
        "average_cpu_utilization_percent": diagnostics.average_cpu_utilization_percent,
        "baseline_resident_memory_bytes": diagnostics.baseline_resident_memory_bytes,
        "resident_memory_bytes": diagnostics.resident_memory_bytes,
        "peak_resident_memory_bytes": diagnostics.peak_resident_memory_bytes,
        "last_error": diagnostics.last_error,
        "final_transcript": final_transcript,
    });
    println!("ASR015_BENCHMARK_JSON={record}");

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
