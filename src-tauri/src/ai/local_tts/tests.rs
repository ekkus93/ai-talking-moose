use super::*;
use crate::ai::local_tts::manifest::{local_tts_model_manifest, LocalTtsPlatform};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn local_tts_defaults_are_valid_catalog_values() {
    assert!(validate_local_tts_model(DEFAULT_LOCAL_TTS_MODEL_ID).is_ok());
    assert!(validate_local_tts_voice(DEFAULT_LOCAL_TTS_VOICE).is_ok());
}

#[test]
fn local_tts_catalog_rejects_unknown_values() {
    assert!(validate_local_tts_model("latest").is_err());
    assert!(validate_local_tts_voice("Fenrir").is_err());
}

#[test]
fn runtime_provider_errors_preserve_safe_local_tts_guidance_and_retry_policy() {
    let not_installed = provider_error_for_runtime(LocalTtsRuntimeError {
        kind: LocalTtsRuntimeErrorKind::ModelNotInstalled,
        message: "The selected Local TTS model is not installed and verified.",
    });
    assert_eq!(not_installed.kind, ProviderErrorKind::Setup);
    assert!(!not_installed.retryable);
    assert_eq!(
        not_installed.message,
        "The selected Local TTS model is not installed and verified."
    );
    assert!(!not_installed.message.contains("conversation"));

    let shutting_down = provider_error_for_runtime(LocalTtsRuntimeError {
        kind: LocalTtsRuntimeErrorKind::ShuttingDown,
        message: "The Local TTS runtime is shutting down.",
    });
    assert_eq!(shutting_down.kind, ProviderErrorKind::Closed);
    assert!(shutting_down.retryable);
    assert_eq!(shutting_down.message, "The Local TTS runtime is shutting down.");

    let inference = provider_error_for_runtime(LocalTtsRuntimeError {
        kind: LocalTtsRuntimeErrorKind::Inference,
        message: "Local TTS inference failed.",
    });
    assert_eq!(inference.kind, ProviderErrorKind::Internal);
    assert!(!inference.retryable);
    assert_eq!(inference.message, "Local TTS inference failed.");
}

#[tokio::test]
async fn production_local_provider_rejects_unknown_model_before_runtime_use() {
    let synthesizer = LocalSpeechSynthesizer::new(
        Arc::new(LocalTtsRuntimeManager::new()),
        "missing-local-model".to_string(),
        DEFAULT_LOCAL_TTS_VOICE.to_string(),
    );
    let error = synthesizer
        .synthesize(TtsRequest {
            text: "private local text".to_string(),
            voice_name: None,
            speaking_rate: Some(1.0),
            pitch: None,
        })
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Model);
    assert!(!error.retryable);
    assert_eq!(
        error.message,
        "The selected Local TTS model is not in the supported catalog."
    );
    assert!(!error.message.contains("private local text"));
    assert!(!error.message.contains("conversation"));
}

#[tokio::test]
async fn production_local_provider_rejects_non_kitten_voice_before_runtime_use() {
    let synthesizer = LocalSpeechSynthesizer::new(
        Arc::new(LocalTtsRuntimeManager::new()),
        DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),
        DEFAULT_LOCAL_TTS_VOICE.to_string(),
    );
    let error = synthesizer
        .synthesize(TtsRequest {
            text: "private local text".to_string(),
            voice_name: Some("Puck".to_string()),
            speaking_rate: Some(1.0),
            pitch: None,
        })
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Setup);
    assert!(!error.retryable);
    assert_eq!(error.message, "The selected Local TTS voice is unavailable.");
    assert!(!error.message.contains("private local text"));
    assert!(!error.message.contains("conversation"));
}

#[tokio::test]
async fn production_local_provider_reports_provider_neutral_cancellation_before_runtime_use() {
    let synthesizer = LocalSpeechSynthesizer::new(
        Arc::new(LocalTtsRuntimeManager::new()),
        DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),
        DEFAULT_LOCAL_TTS_VOICE.to_string(),
    );
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let error = synthesizer
        .synthesize_cancellable(
            TtsRequest {
                text: "cancelled local text".to_string(),
                voice_name: None,
                speaking_rate: Some(1.0),
                pitch: None,
            },
            &cancellation,
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Cancelled);
    assert!(!error.retryable);
    assert!(!error.message.contains("cancelled local text"));
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn real_acceptance_platform() -> LocalTtsPlatform {
    LocalTtsPlatform::LinuxX86_64
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn real_acceptance_platform() -> LocalTtsPlatform {
    LocalTtsPlatform::MacosArm64
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn real_acceptance_platform() -> LocalTtsPlatform {
    LocalTtsPlatform::MacosX86_64
}

#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64")
)))]
fn real_acceptance_platform() -> LocalTtsPlatform {
    panic!("unsupported real Local TTS acceptance platform")
}

fn real_acceptance_platform_label(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "linux-x86_64",
        LocalTtsPlatform::MacosArm64 => "macos-arm64",
        LocalTtsPlatform::MacosX86_64 => "macos-x86_64",
    }
}

fn real_acceptance_ort_filename(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "onnxruntime-linux-x64-1.23.2.tgz",
        LocalTtsPlatform::MacosArm64 => "onnxruntime-osx-arm64-1.23.2.tgz",
        LocalTtsPlatform::MacosX86_64 => "onnxruntime-osx-x86_64-1.23.2.tgz",
    }
}

fn stage_real_acceptance_install() -> (tempfile::TempDir, LocalTtsPlatform) {
    let temp = tempfile::tempdir().unwrap();
    let storage_root = temp.path().join("models").join("tts");
    let storage = storage::initialize_global_local_tts_storage(storage_root).unwrap();
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let platform = real_acceptance_platform();
    let artifacts = storage::expected_artifacts(manifest, platform).unwrap();
    let revision_dir = storage
        .model_revision_dir(DEFAULT_LOCAL_TTS_MODEL_ID)
        .unwrap();
    fs::create_dir_all(&revision_dir).unwrap();

    let source_paths = [
        (
            "kitten_tts_mini_v0_8.onnx",
            PathBuf::from(std::env::var("KTT301_MODEL_PATH").unwrap()),
        ),
        (
            "voices.npz",
            PathBuf::from(std::env::var("KTT301_VOICES_PATH").unwrap()),
        ),
        (
            "cmudict_data.json",
            PathBuf::from(std::env::var("KTT301_G2P_PATH").unwrap()),
        ),
        (
            real_acceptance_ort_filename(platform),
            PathBuf::from(std::env::var("KTT301_ORT_ARCHIVE_PATH").unwrap()),
        ),
    ];

    for artifact in &artifacts {
        let source = source_paths
            .iter()
            .find_map(|(filename, path)| (*filename == artifact.filename).then_some(path))
            .unwrap();
        let destination = revision_dir.join(artifact.filename);
        fs::copy(source, &destination).unwrap();
        assert_eq!(fs::metadata(destination).unwrap().len(), artifact.expected_bytes);
    }

    let marker = storage::install_marker(manifest, platform, &artifacts);
    fs::write(
        revision_dir.join(storage::INSTALL_MARKER),
        serde_json::to_vec_pretty(&marker).unwrap(),
    )
    .unwrap();
    assert!(storage.marker_shape_is_valid(manifest, platform));
    (temp, platform)
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn p95(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[index]
}

fn max_rss_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    let raw = u64::try_from(usage.ru_maxrss).ok()?;
    #[cfg(target_os = "linux")]
    {
        return raw.checked_mul(1024);
    }
    #[cfg(target_os = "macos")]
    {
        return Some(raw);
    }
    #[allow(unreachable_code)]
    Some(raw)
}

#[derive(Debug, Serialize)]
struct RealAcceptanceEvidence {
    platform: &'static str,
    model_id: &'static str,
    sample_rate_hz: u32,
    inference_threads: u32,
    model_load_duration_ms: u64,
    cold_end_to_end_ms: u64,
    warm_synthesis_duration_ms: Vec<u64>,
    warm_audio_duration_ms: Vec<f64>,
    warm_real_time_factors: Vec<f64>,
    median_warm_rtf: f64,
    p95_warm_rtf: f64,
    max_rss_bytes: Option<u64>,
    voices_exercised: Vec<&'static str>,
    network_denied: bool,
    cancellation_observed: bool,
}

#[test]
#[ignore = "requires exact pinned Kitten/CMUdict/ONNX Runtime acceptance artifacts"]
fn real_kitten_cpu_acceptance_production_provider_offline_privacy() {
    const SUCCESS_SENTINEL: &str = "KTT404_SUCCESS_SENTINEL_9F6E3C2D";
    const FAILURE_SENTINEL: &str = "KTT404_FAILURE_SENTINEL_6A17B8E4";
    const CANCEL_SENTINEL: &str = "KTT404_CANCEL_SENTINEL_D52C1A90";
    const WARM_CASES: [(&str, &str); 8] = [
        ("Bella", "Well, this is awkward."),
        ("Jasper", "The Talking Moose is still speaking locally."),
        ("Luna", "I had a plan, but apparently the plan had other plans."),
        ("Bruno", "Please remain calm. I am a highly qualified decorative moose."),
        ("Rosie", "Nothing says efficiency like explaining the same thing to a moose twice."),
        ("Hugo", "The future arrived early, forgot the instructions, and parked on the lawn."),
        ("Kiki", "I would offer useful advice, but then we would both be disappointed."),
        (
            "Leo",
            "For this longer acceptance sentence, the Talking Moose is measuring real CPU speech synthesis while the network boundary remains denied and the production runtime stays warm.",
        ),
    ];

    let (_install, platform) = stage_real_acceptance_install();
    let runtime = Arc::new(LocalTtsRuntimeManager::new());
    let synthesizer = LocalSpeechSynthesizer::new(
        runtime.clone(),
        DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),
        "Jasper".to_string(),
    );

    let ((audio, failure_kind, cancellation_kind, status_json, evidence), logs) =
        crate::test_support::capture_logs(|| {
            let _network_guard = crate::test_support::deny_network_for_scope();
            assert!(crate::test_support::network_denied());
            let async_runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            async_runtime.block_on(async {
                let cold_started = Instant::now();
                let audio = synthesizer
                    .synthesize(TtsRequest {
                        text: SUCCESS_SENTINEL.to_string(),
                        voice_name: Some("Jasper".to_string()),
                        speaking_rate: Some(1.0),
                        pitch: None,
                    })
                    .await
                    .expect("real Local provider must synthesize while network is denied");
                let cold_end_to_end_ms =
                    u64::try_from(cold_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let cold_status = runtime.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string());
                let model_load_duration_ms = cold_status
                    .last_model_load_duration_ms
                    .expect("real acceptance must report model-load duration");
                assert_eq!(cold_status.sample_rate_hz, Some(24_000));
                assert_eq!(cold_status.inference_thread_count, Some(2));

                let mut warm_synthesis_duration_ms = Vec::with_capacity(WARM_CASES.len());
                let mut warm_audio_duration_ms = Vec::with_capacity(WARM_CASES.len());
                let mut warm_real_time_factors = Vec::with_capacity(WARM_CASES.len());
                let mut voices_exercised = Vec::with_capacity(WARM_CASES.len());
                for (voice, text) in WARM_CASES {
                    let warm_audio = synthesizer
                        .synthesize(TtsRequest {
                            text: text.to_string(),
                            voice_name: Some(voice.to_string()),
                            speaking_rate: Some(1.0),
                            pitch: None,
                        })
                        .await
                        .unwrap_or_else(|error| {
                            panic!("warm real synthesis failed for {voice}: {error}")
                        });
                    assert_eq!(warm_audio.sample_rate, 24_000);
                    assert!(!warm_audio.pcm_bytes.is_empty());
                    let status = runtime.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string());
                    let synthesis_ms = status
                        .last_synthesis_duration_ms
                        .expect("warm synthesis duration must be reported");
                    let audio_ms = status
                        .last_generated_audio_duration_ms
                        .expect("generated audio duration must be reported");
                    let rtf = status
                        .last_real_time_factor
                        .expect("warm RTF must be reported");
                    assert!(audio_ms.is_finite() && audio_ms > 0.0);
                    assert!(rtf.is_finite() && rtf > 0.0);
                    assert!(
                        rtf < 1.0,
                        "warm Local TTS RTF hard gate failed for {voice}: {rtf:.4}"
                    );
                    warm_synthesis_duration_ms.push(synthesis_ms);
                    warm_audio_duration_ms.push(audio_ms);
                    warm_real_time_factors.push(rtf);
                    voices_exercised.push(voice);
                }

                let failure = synthesizer
                    .synthesize(TtsRequest {
                        text: FAILURE_SENTINEL.to_string(),
                        voice_name: Some("not-a-kitten-voice".to_string()),
                        speaking_rate: Some(1.0),
                        pitch: None,
                    })
                    .await
                    .expect_err("invalid Local voice must fail closed");

                let cancellation = CancellationToken::new();
                let canceller = cancellation.clone();
                let cancel_task = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    canceller.cancel();
                });
                let cancelled = synthesizer
                    .synthesize_cancellable(
                        TtsRequest {
                            text: format!(
                                "{CANCEL_SENTINEL} Cancel this production Local TTS request while real ONNX CPU inference is executing. The deliberately longer utterance keeps inference active long enough for cancellation to preempt it."
                            ),
                            voice_name: Some("Jasper".to_string()),
                            speaking_rate: Some(1.0),
                            pitch: None,
                        },
                        &cancellation,
                    )
                    .await
                    .expect_err("real in-flight Local provider request must be cancellable");
                cancel_task.await.unwrap();

                let status = runtime.status(DEFAULT_LOCAL_TTS_MODEL_ID.to_string());
                let status_json = serde_json::to_string(&status).unwrap();
                let evidence = RealAcceptanceEvidence {
                    platform: real_acceptance_platform_label(platform),
                    model_id: DEFAULT_LOCAL_TTS_MODEL_ID,
                    sample_rate_hz: 24_000,
                    inference_threads: 2,
                    model_load_duration_ms,
                    cold_end_to_end_ms,
                    median_warm_rtf: median(&warm_real_time_factors),
                    p95_warm_rtf: p95(&warm_real_time_factors),
                    max_rss_bytes: max_rss_bytes(),
                    warm_synthesis_duration_ms,
                    warm_audio_duration_ms,
                    warm_real_time_factors,
                    voices_exercised,
                    network_denied: crate::test_support::network_denied(),
                    cancellation_observed: cancelled.kind == ProviderErrorKind::Cancelled,
                };
                (audio, failure.kind, cancelled.kind, status_json, evidence)
            })
        });

    assert_eq!(audio.sample_rate, 24_000);
    assert!(!audio.pcm_bytes.is_empty());
    assert_eq!(failure_kind, ProviderErrorKind::Setup);
    assert_eq!(cancellation_kind, ProviderErrorKind::Cancelled);
    assert!(status_json.contains(DEFAULT_LOCAL_TTS_MODEL_ID));
    assert!(status_json.contains("\"phase\":\"ready\""));
    assert!(!status_json.contains("pcm_bytes"));
    assert!(!status_json.contains("audio_bytes"));
    assert!(evidence.network_denied);
    assert!(evidence.cancellation_observed);
    assert_eq!(evidence.voices_exercised.len(), LOCAL_TTS_VOICE_IDS.len());
    assert!(evidence.warm_real_time_factors.iter().all(|rtf| *rtf < 1.0));

    for sentinel in [SUCCESS_SENTINEL, FAILURE_SENTINEL, CANCEL_SENTINEL] {
        assert!(!logs.contains(sentinel));
        assert!(!status_json.contains(sentinel));
    }

    println!(
        "KITTENTTS_ACCEPTANCE_JSON={}",
        serde_json::to_string(&evidence).unwrap()
    );
}
