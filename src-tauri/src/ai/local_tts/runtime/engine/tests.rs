use super::super::LocalTtsRuntimeErrorKind;
use super::*;
use crate::ai::local_tts::{DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE};
use serde::Serialize;
use std::time::{Duration, Instant};

#[test]
fn request_validation_rejects_unsupported_values() {
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    assert_eq!(
        resolve_voice_key(manifest, DEFAULT_LOCAL_TTS_VOICE).unwrap(),
        "expr-voice-3-f"
    );
    assert_eq!(
        resolve_voice_key(manifest, "not-a-voice").unwrap_err().kind,
        LocalTtsRuntimeErrorKind::InvalidVoice
    );
    assert_eq!(
        validate_speed(f32::NAN).unwrap_err().kind,
        LocalTtsRuntimeErrorKind::UnsupportedConfig
    );
    assert_eq!(
        validate_speed(0.0).unwrap_err().kind,
        LocalTtsRuntimeErrorKind::UnsupportedConfig
    );
    assert_eq!(
        validate_pitch(Some(1.0)).unwrap_err().kind,
        LocalTtsRuntimeErrorKind::UnsupportedConfig
    );
    assert!(validate_pitch(None).is_ok());
    assert!(validate_pitch(Some(0.0)).is_ok());
}

#[test]
fn model_output_contract_requires_exact_named_outputs_and_presence_only_duration() {
    assert!(model_output_contract_is_valid(2, true, true));
    assert!(!model_output_contract_is_valid(0, false, false));
    assert!(!model_output_contract_is_valid(1, true, false));
    assert!(!model_output_contract_is_valid(2, false, true));
    assert!(!model_output_contract_is_valid(2, true, false));
    assert!(!model_output_contract_is_valid(3, true, true));
}

#[test]
fn waveform_shape_requires_mono_vector_or_single_batch_vector() {
    assert!(waveform_shape_is_valid(&[24_000], 24_000));
    assert!(waveform_shape_is_valid(&[1, 24_000], 24_000));
    assert!(!waveform_shape_is_valid(&[], 0));
    assert!(!waveform_shape_is_valid(&[0], 0));
    assert!(!waveform_shape_is_valid(&[2, 12_000], 24_000));
    assert!(!waveform_shape_is_valid(&[1, 1, 24_000], 24_000));
    assert!(!waveform_shape_is_valid(&[1, 23_999], 24_000));
}

#[test]
fn unknown_runtime_identity_fails_closed_before_artifact_use() {
    let mut engine = KittenTtsRuntimeEngine::new();
    let identity = LocalTtsRuntimeIdentity {
        model_id: "unsupported/model".to_string(),
        model_revision: "invalid".to_string(),
        runtime_compatibility_version: 0,
        adapter_contract: "invalid".to_string(),
        onnx_runtime_version: "invalid".to_string(),
        g2p_source_revision: "invalid".to_string(),
        platform: LocalTtsPlatform::LinuxX86_64,
    };
    let paths = vec![PathBuf::from("missing"); VERIFIED_ARTIFACT_COUNT];
    assert_eq!(
        engine.load(&identity, &paths).unwrap_err().kind,
        LocalTtsRuntimeErrorKind::UnknownModel
    );
}

fn thread_sweep_platform_label(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "linux-x86_64",
        LocalTtsPlatform::MacosArm64 => "macos-arm64",
        LocalTtsPlatform::MacosX86_64 => "macos-x86_64",
    }
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

fn synthesize_thread_sweep_case(
    engine: &mut KittenTtsRuntimeEngine,
    request: &LocalTtsInferenceRequest,
    inference_threads: usize,
    phase: &str,
) -> LocalTtsInferenceOutput {
    let mut last_error = None;
    for attempt in 1..=3 {
        match engine.synthesize(request) {
            Ok(output) => return output,
            Err(error) => {
                eprintln!(
                        "real KittenTTS thread sweep {phase} synthesis failed for {inference_threads} thread(s) on attempt {attempt}: {error:?}"
                    );
                last_error = Some(error);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    panic!(
            "real KittenTTS thread sweep {phase} synthesis failed for {inference_threads} thread(s) after retries: {:?}",
            last_error.expect("retry loop must record a Local TTS error")
        );
}

#[derive(Debug, Serialize)]
struct ThreadSweepConfigurationEvidence {
    inference_threads: usize,
    model_load_duration_ms: u64,
    first_synthesis_duration_ms: u64,
    first_audio_duration_ms: f64,
    first_real_time_factor: f64,
    warm_synthesis_duration_ms: Vec<u64>,
    warm_audio_duration_ms: Vec<f64>,
    warm_real_time_factors: Vec<f64>,
    median_warm_rtf: f64,
    p95_warm_rtf: f64,
    repeated_sample_counts: Vec<usize>,
    repeated_utterance_stable: bool,
    max_rss_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ThreadSweepEvidence {
    platform: &'static str,
    model_id: &'static str,
    production_default_threads: usize,
    bounded_n_threads: usize,
    network_denied: bool,
    configurations: Vec<ThreadSweepConfigurationEvidence>,
}

#[test]
#[ignore = "requires exact pinned Kitten/CMUdict/ONNX Runtime acceptance artifacts"]
fn real_kittentts_cpu_thread_sweep() {
    const THREAD_CONFIGURATIONS: [usize; 3] = [1, 2, 4];
    const WARM_REPETITIONS: usize = 5;

    let paths = [
        std::env::var("KTT301_MODEL_PATH").unwrap(),
        std::env::var("KTT301_VOICES_PATH").unwrap(),
        std::env::var("KTT301_G2P_PATH").unwrap(),
        std::env::var("KTT301_ORT_ARCHIVE_PATH").unwrap(),
    ]
    .map(PathBuf::from);
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let platform = super::super::current_platform().unwrap();
    let identity = super::super::runtime_identity(manifest, platform);
    let production_default_threads = usize::from(manifest.runtime.inference_threads);
    assert_eq!(production_default_threads, 2);
    let _network_guard = crate::test_support::deny_network_for_scope();
    assert!(crate::test_support::network_denied());

    let request = LocalTtsInferenceRequest {
        text: "The Talking Moose measures local speech performance.".to_string(),
        voice_id: "Jasper".to_string(),
        speaking_rate: 1.0,
        pitch: None,
    };
    let mut configurations = Vec::with_capacity(THREAD_CONFIGURATIONS.len());

    for inference_threads in THREAD_CONFIGURATIONS {
        let mut engine = KittenTtsRuntimeEngine::new_with_inference_threads(inference_threads);
        let load_started = Instant::now();
        engine.load(&identity, &paths).unwrap();
        let model_load_duration_ms = load_started
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);

        let first_started = Instant::now();
        let first = synthesize_thread_sweep_case(&mut engine, &request, inference_threads, "first");
        let first_elapsed = first_started.elapsed();
        assert_eq!(first.sample_rate_hz, manifest.sample_rate_hz);
        assert!(!first.samples.is_empty());
        assert!(first.samples.iter().all(|sample| sample.is_finite()));
        let first_audio_duration_ms =
            first.samples.len() as f64 * 1_000.0 / f64::from(first.sample_rate_hz);
        let first_real_time_factor =
            first_elapsed.as_secs_f64() * 1_000.0 / first_audio_duration_ms;
        assert!(first_real_time_factor.is_finite() && first_real_time_factor > 0.0);

        let mut warm_synthesis_duration_ms = Vec::with_capacity(WARM_REPETITIONS);
        let mut warm_audio_duration_ms = Vec::with_capacity(WARM_REPETITIONS);
        let mut warm_real_time_factors = Vec::with_capacity(WARM_REPETITIONS);
        let mut repeated_sample_counts = vec![first.samples.len()];

        for _ in 0..WARM_REPETITIONS {
            let started = Instant::now();
            let output =
                synthesize_thread_sweep_case(&mut engine, &request, inference_threads, "warm");
            let elapsed = started.elapsed();
            assert_eq!(output.sample_rate_hz, manifest.sample_rate_hz);
            assert!(!output.samples.is_empty());
            assert!(output.samples.iter().all(|sample| sample.is_finite()));
            let audio_duration_ms =
                output.samples.len() as f64 * 1_000.0 / f64::from(output.sample_rate_hz);
            let rtf = elapsed.as_secs_f64() * 1_000.0 / audio_duration_ms;
            assert!(rtf.is_finite() && rtf > 0.0);
            if inference_threads == production_default_threads {
                assert!(
                    rtf < 1.0,
                    "production 2-thread Local TTS policy must remain faster than real time"
                );
            }
            warm_synthesis_duration_ms.push(elapsed.as_millis().try_into().unwrap_or(u64::MAX));
            warm_audio_duration_ms.push(audio_duration_ms);
            warm_real_time_factors.push(rtf);
            repeated_sample_counts.push(output.samples.len());
        }

        let repeated_utterance_stable = repeated_sample_counts
            .windows(2)
            .all(|window| window[0] == window[1]);
        assert!(
            repeated_utterance_stable,
            "repeated identical utterances changed generated sample count"
        );
        let median_warm_rtf = median(&warm_real_time_factors);
        let p95_warm_rtf = p95(&warm_real_time_factors);
        let first_synthesis_duration_ms = first_elapsed.as_millis().try_into().unwrap_or(u64::MAX);
        configurations.push(ThreadSweepConfigurationEvidence {
            inference_threads,
            model_load_duration_ms,
            first_synthesis_duration_ms,
            first_audio_duration_ms,
            first_real_time_factor,
            warm_synthesis_duration_ms,
            warm_audio_duration_ms,
            warm_real_time_factors,
            median_warm_rtf,
            p95_warm_rtf,
            repeated_sample_counts,
            repeated_utterance_stable,
            max_rss_bytes: max_rss_bytes(),
        });
        engine.unload();
    }

    let evidence = ThreadSweepEvidence {
        platform: thread_sweep_platform_label(platform),
        model_id: DEFAULT_LOCAL_TTS_MODEL_ID,
        production_default_threads,
        bounded_n_threads: 4,
        network_denied: crate::test_support::network_denied(),
        configurations,
    };
    println!(
        "KITTENTTS_THREAD_SWEEP_JSON={}",
        serde_json::to_string(&evidence).unwrap()
    );
}

#[test]
#[ignore = "requires exact pinned Kitten/CMUdict/ONNX Runtime acceptance artifacts"]
fn real_kitten_cpu_acceptance() {
    let paths = [
        std::env::var("KTT301_MODEL_PATH").unwrap(),
        std::env::var("KTT301_VOICES_PATH").unwrap(),
        std::env::var("KTT301_G2P_PATH").unwrap(),
        std::env::var("KTT301_ORT_ARCHIVE_PATH").unwrap(),
    ]
    .map(PathBuf::from);
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let platform = super::super::current_platform().unwrap();
    let identity = super::super::runtime_identity(manifest, platform);
    let mut engine = KittenTtsRuntimeEngine::new();
    engine.load(&identity, &paths).unwrap();
    let output = engine
        .synthesize(&LocalTtsInferenceRequest {
            text: "The Talking Moose is speaking locally.".to_string(),
            voice_id: "Jasper".to_string(),
            speaking_rate: 1.0,
            pitch: None,
        })
        .unwrap();
    assert_eq!(output.sample_rate_hz, 24_000);
    assert!(!output.samples.is_empty());
    assert!(output.samples.iter().all(|sample| sample.is_finite()));
    let waveform_shape = engine
        .last_waveform_shape()
        .expect("real KittenTTS acceptance must record the runtime waveform shape");
    assert!(waveform_shape_is_valid(
        waveform_shape,
        output.samples.len()
    ));
    println!("KITTENTTS_WAVEFORM_SHAPE={waveform_shape:?}");

    let faster = engine
        .synthesize(&LocalTtsInferenceRequest {
            text: "Speed is passed directly to Kitten Mini.".to_string(),
            voice_id: "Jasper".to_string(),
            speaking_rate: 1.2,
            pitch: Some(0.0),
        })
        .unwrap();
    assert_eq!(faster.sample_rate_hz, 24_000);
    assert!(!faster.samples.is_empty());
    assert!(faster.samples.iter().all(|sample| sample.is_finite()));

    let invalid_voice = engine
        .synthesize(&LocalTtsInferenceRequest {
            text: "This must fail before inference.".to_string(),
            voice_id: "invalid".to_string(),
            speaking_rate: 1.0,
            pitch: None,
        })
        .unwrap_err();
    assert_eq!(invalid_voice.kind, LocalTtsRuntimeErrorKind::InvalidVoice);

    let invalid_pitch = engine
        .synthesize(&LocalTtsInferenceRequest {
            text: "Pitch is unsupported by Kitten Mini.".to_string(),
            voice_id: "Jasper".to_string(),
            speaking_rate: 1.0,
            pitch: Some(1.0),
        })
        .unwrap_err();
    assert_eq!(
        invalid_pitch.kind,
        LocalTtsRuntimeErrorKind::UnsupportedConfig
    );

    let cancellation = LocalTtsRuntimeCancellation::default();
    let canceller = cancellation.clone();
    let cancel_thread = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !canceller.has_run_options() {
            assert!(
                Instant::now() < deadline,
                "real Kitten inference never registered cancellable ORT run options"
            );
            std::thread::yield_now();
        }
        canceller.cancel();
    });
    let cancelled = engine
        .synthesize_cancellable(
            &LocalTtsInferenceRequest {
                text: "Cancel this real Kitten Mini CPU inference while ONNX Runtime is executing."
                    .to_string(),
                voice_id: "Jasper".to_string(),
                speaking_rate: 1.0,
                pitch: None,
            },
            &cancellation,
        )
        .unwrap_err();
    cancel_thread.join().unwrap();
    assert_eq!(cancelled.kind, LocalTtsRuntimeErrorKind::Cancelled);
}
