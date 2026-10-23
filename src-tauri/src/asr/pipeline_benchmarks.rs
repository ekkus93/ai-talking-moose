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
use crate::asr::whisper::{
    engine::WhisperEngine, installer::WhisperModelInstallCancellation, manifest,
    WhisperModelInstaller,
};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const WHISPER_INPUT_SAMPLE_RATE_HZ: u32 = LOCAL_ASR_INPUT_SAMPLE_RATE_HZ;

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
            panic!("KittenTTS-to-Whisper round-trip is a macOS acceptance test")
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
    assert_eq!(
        status.install_state,
        storage::LocalTtsInstallState::Installed
    );
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

fn resample_kitten_audio_for_whisper(audio: &crate::ai::types::AudioStreamData) -> Vec<f32> {
    assert_eq!(audio.sample_rate, 24_000);
    let source_i16 = crate::audio::resample::AudioResampler::bytes_to_i16(&audio.pcm_bytes);
    let source_f32 = crate::audio::resample::AudioResampler::i16_to_f32(&source_i16);
    crate::audio::resample::AudioResampler::resample_linear(
        audio.sample_rate,
        WHISPER_INPUT_SAMPLE_RATE_HZ,
        &source_f32,
    )
}

fn whisper_transcribe_generated_audio(engine: &mut WhisperEngine, pcm: &[f32]) -> String {
    let chunk_samples = usize::try_from(WHISPER_INPUT_SAMPLE_RATE_HZ / 10).unwrap();
    let mut updates = Vec::new();
    for chunk in pcm.chunks(chunk_samples) {
        updates.extend(
            engine
                .push_pcm(chunk)
                .expect("Whisper failed while consuming generated KittenTTS audio"),
        );
    }

    // Give the batch decoder a clean utterance boundary before stopping its
    // current transcript state. This is synthetic silence generated in-memory;
    // no microphone or speaker loopback is involved.
    let silence = vec![0.0_f32; chunk_samples];
    for _ in 0..8 {
        updates.extend(
            engine
                .push_pcm(&silence)
                .expect("Whisper failed while consuming round-trip trailing silence"),
        );
    }
    assert!(
        updates
            .iter()
            .any(|update| matches!(update, StreamingTranscriptUpdate::Final { .. })),
        "Whisper silence endpoint must finalize each generated utterance"
    );

    engine.updates_delivered(&updates);
    updates
        .iter()
        .filter_map(|update| match update {
            StreamingTranscriptUpdate::Final { text, .. } if !text.trim().is_empty() => {
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

/// Automated intelligibility smoke only. This test does not measure subjective
/// naturalness, character fit, timbre, or fatigue. A later owner-approved ASR
/// proxy used this objective evidence as part of the V1 default-voice decision;
/// that policy decision does not turn this smoke test into human-listening evidence.
#[tokio::test]
#[ignore = "requires verified KittenTTS artifacts, pinned Whisper Small, and native macOS runtimes"]
async fn kittentts_all_voices_round_trip_through_whisper_small() {
    const PHRASE: &str = "The talking moose reads seven blue books beside the quiet river.";
    const CONTENT_WORDS: [&str; 7] = [
        "talking", "moose", "seven", "blue", "books", "quiet", "river",
    ];
    const WER_LIMIT: f64 = 0.40;
    const CONTENT_RECALL_FLOOR: f64 = 0.70;

    let (_tts_install, platform) = stage_roundtrip_tts_install();
    let asr_model_root = PathBuf::from(
        std::env::var("KITTENTTS_ASR_MODEL_ROOT")
            .expect("KITTENTTS_ASR_MODEL_ROOT must point at the Whisper installer cache"),
    );
    let asr_installer = Arc::new(
        WhisperModelInstaller::new(asr_model_root)
            .expect("failed to initialize Whisper installer for round-trip smoke"),
    );
    let mut install_cancellation = WhisperModelInstallCancellation::default();
    asr_installer
        .install(&mut install_cancellation, &None)
        .await
        .expect("failed to install or verify pinned Whisper Small for round-trip smoke");

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
        tokio::task::spawn_blocking(move || crate::asr::whisper::engine::open(engine_installer))
            .await
            .expect("Whisper Small open worker panicked")
            .expect("failed to open pinned Whisper Small");
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

        let resampled = resample_kitten_audio_for_whisper(&generated);
        assert!(!resampled.is_empty());
        let transcript = whisper_transcribe_generated_audio(&mut asr_engine, &resampled);
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

    asr_engine.stop().expect("failed to stop Whisper Small");
    let all_passed = voice_results.iter().all(|voice| voice.passed);
    let evidence = RoundTripEvidence {
        platform: roundtrip_platform_label(platform),
        tts_model_id: DEFAULT_LOCAL_TTS_MODEL_ID,
        asr_model_id: manifest::WHISPER_SMALL_ID,
        asr_model_revision: manifest::WHISPER_MODEL_REVISION,
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
        "one or more KittenTTS voices failed the Whisper intelligibility smoke"
    );
}
