use super::*;
use crate::ai::local_tts::manifest::{
    local_tts_model_manifest, local_tts_platform_artifact, LocalTtsPlatform,
};
use crate::ai::local_tts::storage;
use crate::ai::local_tts::{
    LocalSpeechSynthesizer, LocalTtsRuntimeManager, DEFAULT_LOCAL_TTS_MODEL_ID, LOCAL_TTS_VOICE_IDS,
};
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::TtsRequest;
use crate::asr::moonshine::{
    model_manifest_info, MoonshineModelInstallCancellation, MoonshineTinyEngine,
};
use serde::Serialize;
use std::fs;
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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn roundtrip_tts_platform() -> LocalTtsPlatform {
    LocalTtsPlatform::MacosArm64
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn roundtrip_tts_platform() -> LocalTtsPlatform {
    LocalTtsPlatform::MacosX86_64
}

fn roundtrip_platform_label(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "linux-x86_64",
        LocalTtsPlatform::MacosArm64 => "macos-arm64",
        LocalTtsPlatform::MacosX86_64 => "macos-x86_64",
    }
}

fn roundtrip_ort_filename(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::MacosArm64 => "onnxruntime-osx-arm64-1.23.2.tgz",
        LocalTtsPlatform::MacosX86_64 => "onnxruntime-osx-x86_64-1.23.2.tgz",
        LocalTtsPlatform::LinuxX86_64 => {
            panic!("KittenTTS-to-Moonshine round-trip is a macOS acceptance test")
        }
    }
}

fn stage_roundtrip_tts_install() -> (tempfile::TempDir, LocalTtsPlatform) {
    let temp = tempfile::tempdir().expect("failed to create Local TTS round-trip install root");
    let storage_root = temp.path().join("models").join("tts");
    let storage = storage::initialize_global_local_tts_storage(storage_root)
        .expect("failed to initialize Local TTS round-trip storage");
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID)
        .expect("default Local TTS manifest must exist");
    let platform = roundtrip_tts_platform();
    let mut artifacts = manifest.common_artifacts.iter().collect::<Vec<_>>();
    artifacts.push(
        local_tts_platform_artifact(manifest, platform)
            .expect("round-trip platform must have a pinned ONNX Runtime artifact"),
    );
    let revision_dir = storage
        .model_revision_dir(DEFAULT_LOCAL_TTS_MODEL_ID)
        .expect("default Local TTS model ID must be path-safe");
    fs::create_dir_all(&revision_dir).expect("failed to create Local TTS revision directory");

    let source_paths = [
        (
            "kitten_tts_mini_v0_8.onnx",
            PathBuf::from(
                std::env::var("KTT301_MODEL_PATH")
                    .expect("KTT301_MODEL_PATH must point at the verified Kitten model"),
            ),
        ),
        (
            "voices.npz",
            PathBuf::from(
                std::env::var("KTT301_VOICES_PATH")
                    .expect("KTT301_VOICES_PATH must point at the verified voice archive"),
            ),
        ),
        (
            "cmudict_data.json",
            PathBuf::from(
                std::env::var("KTT301_G2P_PATH")
                    .expect("KTT301_G2P_PATH must point at the verified CMUdict data"),
            ),
        ),
        (
            roundtrip_ort_filename(platform),
            PathBuf::from(
                std::env::var("KTT301_ORT_ARCHIVE_PATH")
                    .expect("KTT301_ORT_ARCHIVE_PATH must point at verified ONNX Runtime"),
            ),
        ),
    ];

    for artifact in &artifacts {
        let source = source_paths
            .iter()
            .find_map(|(filename, path)| (*filename == artifact.filename).then_some(path))
            .unwrap_or_else(|| panic!("missing round-trip source for {}", artifact.filename));
        let destination = revision_dir.join(artifact.filename);
        fs::copy(source, &destination)
            .unwrap_or_else(|error| panic!("failed to stage {}: {error}", artifact.filename));
        assert_eq!(
            fs::metadata(&destination).unwrap().len(),
            artifact.expected_bytes,
            "staged Local TTS artifact size drifted for {}",
            artifact.filename
        );
    }

    let marker_artifacts = artifacts
        .iter()
        .map(|artifact| {
            serde_json::json!({
                "filename": artifact.filename,
                "expected_bytes": artifact.expected_bytes,
                "sha256": artifact.sha256,
            })
        })
        .collect::<Vec<_>>();
    let marker = serde_json::json!({
        "schema_version": 1,
        "storage_id": manifest.id,
        "provider_model_id": manifest.provider_model_id,
        "model_source_revision": manifest.model_source_revision,
        "runtime_compatibility_version": manifest.runtime.compatibility_version,
        "platform": roundtrip_platform_label(platform),
        "artifacts": marker_artifacts,
    });
    fs::write(
        revision_dir.join(".talking-moose-local-tts.json"),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .expect("failed to write Local TTS round-trip install marker");
    let status = storage
        .status(DEFAULT_LOCAL_TTS_MODEL_ID, platform)
        .expect("round-trip staged Local TTS status must be readable");
    assert_eq!(status.install_state, storage::LocalTtsInstallState::Installed);
    (temp, platform)
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|character| character.is_ascii_alphanumeric() || *character == '\'')
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn word_edit_distance(expected: &[String], actual: &[String]) -> usize {
    let mut previous = (0..=actual.len()).collect::<Vec<_>>();
    let mut current = vec![0; actual.len() + 1];
    for (expected_index, expected_word) in expected.iter().enumerate() {
        current[0] = expected_index + 1;
        for (actual_index, actual_word) in actual.iter().enumerate() {
            let substitution_cost = usize::from(expected_word != actual_word);
            current[actual_index + 1] = (previous[actual_index + 1] + 1)
                .min(current[actual_index] + 1)
                .min(previous[actual_index] + substitution_cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[actual.len()]
}

fn word_error_rate(expected: &[String], actual: &[String]) -> f64 {
    assert!(!expected.is_empty());
    word_edit_distance(expected, actual) as f64 / expected.len() as f64
}

fn content_word_recall(actual: &[String], content_words: &[&str]) -> f64 {
    let hits = content_words
        .iter()
        .filter(|expected| actual.iter().any(|actual_word| actual_word == **expected))
        .count();
    hits as f64 / content_words.len() as f64
}

fn resample_kitten_audio_for_moonshine(audio: &crate::ai::types::AudioStreamData) -> Vec<f32> {
    assert_eq!(audio.sample_rate, 24_000);
    let source_i16 = crate::audio::resample::AudioResampler::bytes_to_i16(&audio.pcm_bytes);
    let source_f32 = crate::audio::resample::AudioResampler::i16_to_f32(&source_i16);
    crate::audio::resample::AudioResampler::resample_linear(
        audio.sample_rate,
        MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
        &source_f32,
    )
}

fn moonshine_transcribe_generated_audio(engine: &mut MoonshineTinyEngine, pcm: &[f32]) -> String {
    let chunk_samples = usize::try_from(MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ / 10).unwrap();
    let mut updates = Vec::new();
    for chunk in pcm.chunks(chunk_samples) {
        updates.extend(
            engine
                .push_pcm(chunk)
                .expect("Moonshine failed while consuming generated KittenTTS audio"),
        );
    }

    // Give the streaming decoder a clean utterance boundary before forcing its
    // latest transcript state. This is synthetic silence generated in-memory;
    // no microphone or speaker loopback is involved.
    let silence = vec![0.0_f32; chunk_samples];
    for _ in 0..8 {
        updates.extend(
            engine
                .push_pcm(&silence)
                .expect("Moonshine failed while consuming round-trip trailing silence"),
        );
    }
    updates.extend(
        engine
            .flush()
            .expect("Moonshine failed to flush the round-trip transcript"),
    );

    updates
        .iter()
        .filter_map(|update| match update {
            crate::asr::moonshine::MoonshineTinyTranscriptUpdate::Final { text, .. }
                if !text.trim().is_empty() =>
            {
                Some(text.trim())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Serialize)]
struct RoundTripVoiceEvidence {
    voice: &'static str,
    transcript: String,
    word_error_rate: f64,
    content_word_recall: f64,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct RoundTripEvidence {
    platform: &'static str,
    tts_model_id: &'static str,
    asr_model_id: &'static str,
    asr_model_revision: &'static str,
    phrase: &'static str,
    wer_limit: f64,
    content_recall_floor: f64,
    network_denied_during_round_trip: bool,
    voices: Vec<RoundTripVoiceEvidence>,
    all_passed: bool,
}

/// Automated intelligibility smoke only. This deliberately does not replace
/// the deferred human KCR-330 audition for naturalness, character fit, or the
/// final default-voice decision.
#[tokio::test]
#[ignore = "requires verified KittenTTS artifacts, pinned Moonshine Tiny, and native macOS runtimes"]
async fn kittentts_all_voices_round_trip_through_moonshine_tiny() {
    const PHRASE: &str = "The talking moose reads seven blue books beside the quiet river.";
    const CONTENT_WORDS: [&str; 7] = [
        "talking", "moose", "seven", "blue", "books", "quiet", "river",
    ];
    const WER_LIMIT: f64 = 0.40;
    const CONTENT_RECALL_FLOOR: f64 = 0.70;

    let (_tts_install, platform) = stage_roundtrip_tts_install();
    let asr_model_root = PathBuf::from(
        std::env::var("KITTENTTS_ASR_MODEL_ROOT")
            .expect("KITTENTTS_ASR_MODEL_ROOT must point at the Moonshine installer cache"),
    );
    let asr_installer = Arc::new(
        MoonshineModelInstaller::new(asr_model_root)
            .expect("failed to initialize Moonshine installer for round-trip smoke"),
    );
    let install_cancellation = MoonshineModelInstallCancellation::default();
    asr_installer
        .install(
            MoonshineModelArchitecture::TinyStreaming,
            &install_cancellation,
        )
        .await
        .expect("failed to install or verify pinned Moonshine Tiny for round-trip smoke");

    let _network_guard = crate::test_support::deny_network_for_scope();
    assert!(crate::test_support::network_denied());

    let tts_runtime = Arc::new(LocalTtsRuntimeManager::new());
    let synthesizer = LocalSpeechSynthesizer::new(
        tts_runtime,
        DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),
        LOCAL_TTS_VOICE_IDS[0].to_string(),
    );
    let engine_installer = asr_installer.clone();
    let mut asr_engine =
        tokio::task::spawn_blocking(move || MoonshineTinyEngine::open(&engine_installer))
            .await
            .expect("Moonshine Tiny open worker panicked")
            .expect("failed to open pinned Moonshine Tiny");
    let expected_words = normalized_words(PHRASE);
    let mut voice_results = Vec::with_capacity(LOCAL_TTS_VOICE_IDS.len());

    for &voice in LOCAL_TTS_VOICE_IDS {
        let generated = synthesizer
            .synthesize(TtsRequest {
                text: PHRASE.to_string(),
                voice_name: Some(voice.to_string()),
                speaking_rate: Some(1.0),
                pitch: None,
            })
            .await
            .unwrap_or_else(|error| panic!("KittenTTS synthesis failed for {voice}: {error}"));
        assert!(!generated.pcm_bytes.is_empty());

        let resampled = resample_kitten_audio_for_moonshine(&generated);
        assert!(!resampled.is_empty());
        let transcript = moonshine_transcribe_generated_audio(&mut asr_engine, &resampled);
        let actual_words = normalized_words(&transcript);
        let wer = word_error_rate(&expected_words, &actual_words);
        let recall = content_word_recall(&actual_words, &CONTENT_WORDS);
        let passed = !transcript.is_empty() && wer <= WER_LIMIT && recall >= CONTENT_RECALL_FLOOR;

        println!(
            "KITTENTTS_ASR_VOICE voice={voice} wer={wer:.4} content_recall={recall:.4} passed={passed} transcript={transcript:?}"
        );
        voice_results.push(RoundTripVoiceEvidence {
            voice,
            transcript,
            word_error_rate: wer,
            content_word_recall: recall,
            passed,
        });
    }

    asr_engine.stop().expect("failed to stop Moonshine Tiny");
    let asr_manifest = model_manifest_info(MoonshineModelArchitecture::TinyStreaming);
    let all_passed = voice_results.iter().all(|voice| voice.passed);
    let evidence = RoundTripEvidence {
        platform: roundtrip_platform_label(platform),
        tts_model_id: DEFAULT_LOCAL_TTS_MODEL_ID,
        asr_model_id: asr_manifest.id,
        asr_model_revision: asr_manifest.revision,
        phrase: PHRASE,
        wer_limit: WER_LIMIT,
        content_recall_floor: CONTENT_RECALL_FLOOR,
        network_denied_during_round_trip: crate::test_support::network_denied(),
        voices: voice_results,
        all_passed,
    };

    println!(
        "KITTENTTS_ASR_ROUNDTRIP_JSON={}",
        serde_json::to_string(&evidence).unwrap()
    );
    assert!(
        evidence.all_passed,
        "one or more KittenTTS voices failed the Moonshine intelligibility smoke"
    );
}
