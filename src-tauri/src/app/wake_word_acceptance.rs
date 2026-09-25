use super::state::AppSettings;
use super::wake_word::engine::{
    NativeKwsSession, NativeKwsSessionPaths, SherpaKwsEngine, V1_KWS_SAMPLE_RATE_HZ,
};
use super::wake_word_command_activation::{
    activate_wake_command_and_measure_start_normal_asr_once, WakeCommandStarter,
};
use super::wake_word_command_asr_ingress::{WakeCommandAsrHandoff, WakeCommandAsrIngress};
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_composition::WakeWordApplicationRuntime;
use crate::asr::moonshine::{
    MoonshineModelArchitecture, MoonshineModelInstallCancellation, MoonshineModelInstaller,
};
use crate::asr::pipeline::{LocalAsrPipeline, LocalAsrPipelineEventCallback};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const FRAME_SAMPLES: usize = 1_600;
const HANDOFF_MEASUREMENT_SAMPLES: usize = V1_KWS_SAMPLE_RATE_HZ as usize * 2;
const CONTINUOUS_ASR_BASELINE_AUDIO_MS: u64 = 2_000;
const CONTINUOUS_ASR_CHUNK_SAMPLES: usize = 1_600;
const CONTINUOUS_ASR_CHUNK_AUDIO_MS: u64 = 100;

#[derive(Debug, Deserialize)]
pub struct GeneratedCorpusIndex {
    pub corpus_id: String,
    pub generator: String,
    pub generator_version: String,
    pub acceptance_criteria: AcceptanceCriteria,
    pub fixtures: Vec<GeneratedFixture>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptanceCriteria {
    pub criteria_version: u32,
    pub positive_recall_minimum: f64,
    pub negative_false_accepts_maximum: u64,
}

#[derive(Debug, Deserialize)]
pub struct GeneratedFixture {
    pub id: String,
    pub label: String,
    pub path: String,
    pub expected_detection: bool,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct FixtureAcceptanceResult {
    pub id: String,
    pub label: String,
    pub expected_detection: bool,
    pub detected: bool,
    pub detected_score: Option<f32>,
    pub audio_ms: u64,
    pub inference_wall_time_ms: u64,
    pub real_time_factor: f64,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct WakeWordAcceptanceReport {
    pub schema_version: u32,
    pub corpus_id: String,
    pub generator: String,
    pub generator_version: String,
    pub platform: String,
    pub architecture: String,
    pub sample_rate_hz: u32,
    pub inference_threads: u16,
    pub score: f32,
    pub threshold: f32,
    pub positive_total: u64,
    pub positive_detected: u64,
    pub positive_recall: f64,
    pub negative_total: u64,
    pub negative_false_accepts: u64,
    pub process_cpu_time_ms: u64,
    pub idle_cpu_percent: f64,
    pub idle_observation_ms: u64,
    pub peak_resident_memory_bytes: Option<u64>,
    pub wake_to_command_asr_ms: u64,
    pub command_start_ms: u64,
    pub total_activation_ms: u64,
    pub pre_roll_startup_ms: u64,
    pub pre_roll_samples: u64,
    pub continuous_asr_idle_cpu_percent: Option<f64>,
    pub continuous_asr_observation_ms: Option<u64>,
    pub continuous_asr_audio_ms: Option<u64>,
    pub continuous_asr_processed_audio_ms: Option<u64>,
    pub criteria_version: u32,
    pub positive_recall_minimum: f64,
    pub negative_false_accepts_maximum: u64,
    pub passed: bool,
    pub fixtures: Vec<FixtureAcceptanceResult>,
}

pub fn run_real_kws_acceptance(
    model_dir: &Path,
    runtime_dir: &Path,
    corpus_dir: &Path,
    index_path: &Path,
) -> Result<WakeWordAcceptanceReport, String> {
    let index: GeneratedCorpusIndex = serde_json::from_slice(
        &fs::read(index_path).map_err(|_| "failed to read generated corpus index".to_string())?,
    )
    .map_err(|_| "generated corpus index is invalid".to_string())?;
    if index.fixtures.is_empty() {
        return Err("generated corpus index contains no fixtures".to_string());
    }

    let mut session = NativeKwsSession::new(NativeKwsSessionPaths {
        model_dir: model_dir.to_path_buf(),
        runtime_dir: runtime_dir.to_path_buf(),
    })
    .map_err(|error| error.message)?;
    let config = session.config().clone();
    // Observe the initialized KWS session while it is idle. This deliberately
    // happens before any corpus PCM is fed so WWR-630 does not confuse active
    // inference CPU with idle wake-listening cost.
    let idle_cpu_before = process_cpu_time_ms();
    let idle_started = Instant::now();
    std::thread::sleep(std::time::Duration::from_secs(2));
    let idle_wall_ms: u64 = idle_started
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let idle_cpu_ms = process_cpu_time_ms().saturating_sub(idle_cpu_before);
    let idle_cpu_percent = if idle_wall_ms == 0 {
        0.0
    } else {
        (idle_cpu_ms as f64 * 100.0) / idle_wall_ms as f64
    };
    let cpu_before = process_cpu_time_ms();
    let mut results = Vec::with_capacity(index.fixtures.len());

    for fixture in &index.fixtures {
        let fixture_path = safe_fixture_path(corpus_dir, &fixture.path)?;
        let bytes = fs::read(&fixture_path)
            .map_err(|_| format!("failed to read generated fixture {}", fixture.id))?;
        if bytes.len() as u64 != fixture.bytes {
            return Err(format!(
                "generated fixture {} byte size mismatch",
                fixture.id
            ));
        }
        if bytes.len() % 2 != 0 {
            return Err(format!(
                "generated fixture {} is not PCM16 aligned",
                fixture.id
            ));
        }
        if sha256_hex(&bytes) != fixture.sha256 {
            return Err(format!(
                "generated fixture {} identity mismatch",
                fixture.id
            ));
        }
        let (pairs, remainder) = bytes.as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(format!(
                "generated fixture {} is not PCM16 aligned",
                fixture.id
            ));
        }
        let samples: Vec<i16> = pairs.iter().map(|pair| i16::from_le_bytes(*pair)).collect();
        if samples.is_empty() {
            return Err(format!("generated fixture {} contains no PCM", fixture.id));
        }

        session.reset_stream().map_err(|error| error.message)?;
        let started = Instant::now();
        let mut detected_score = None;
        for frame in samples.chunks(FRAME_SAMPLES) {
            if let Some(detection) = session
                .accept_pcm16_mono(V1_KWS_SAMPLE_RATE_HZ, frame)
                .map_err(|error| error.message)?
            {
                detected_score = Some(detection.score);
                break;
            }
        }
        let wall_ms: u64 = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        let audio_ms = (samples.len() as u64 * 1_000) / u64::from(V1_KWS_SAMPLE_RATE_HZ);
        results.push(FixtureAcceptanceResult {
            id: fixture.id.clone(),
            label: fixture.label.clone(),
            expected_detection: fixture.expected_detection,
            detected: detected_score.is_some(),
            detected_score,
            audio_ms,
            inference_wall_time_ms: wall_ms,
            real_time_factor: if audio_ms == 0 {
                0.0
            } else {
                wall_ms as f64 / audio_ms as f64
            },
            bytes: fixture.bytes,
            sha256: fixture.sha256.clone(),
        });
    }
    session.shutdown().map_err(|error| error.message)?;
    let kws_peak_resident_memory_bytes = peak_resident_memory_bytes();

    let activation_measurement = measure_wake_command_activation_timing()?;
    let continuous_asr_measurement = measure_continuous_asr_idle_baseline()?;

    let positive: Vec<_> = results
        .iter()
        .filter(|item| item.expected_detection)
        .collect();
    let negative: Vec<_> = results
        .iter()
        .filter(|item| !item.expected_detection)
        .collect();
    if positive.is_empty() || negative.is_empty() {
        return Err("acceptance corpus must contain positive and negative fixtures".to_string());
    }
    let positive_detected = positive.iter().filter(|item| item.detected).count() as u64;
    let negative_false_accepts = negative.iter().filter(|item| item.detected).count() as u64;
    let positive_recall = positive_detected as f64 / positive.len() as f64;
    let continuous_asr_comparison_passed = continuous_asr_measurement
        .map(|measurement| idle_cpu_percent < measurement.idle_cpu_percent)
        .unwrap_or(true);
    let passed = positive_recall >= index.acceptance_criteria.positive_recall_minimum
        && negative_false_accepts <= index.acceptance_criteria.negative_false_accepts_maximum
        && continuous_asr_comparison_passed;

    Ok(WakeWordAcceptanceReport {
        schema_version: 1,
        corpus_id: index.corpus_id,
        generator: index.generator,
        generator_version: index.generator_version,
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        sample_rate_hz: config.sample_rate_hz,
        inference_threads: config.threads,
        score: config.score,
        threshold: config.threshold,
        positive_total: positive.len() as u64,
        positive_detected,
        positive_recall,
        negative_total: negative.len() as u64,
        negative_false_accepts,
        process_cpu_time_ms: process_cpu_time_ms().saturating_sub(cpu_before),
        idle_cpu_percent,
        idle_observation_ms: idle_wall_ms,
        peak_resident_memory_bytes: kws_peak_resident_memory_bytes,
        wake_to_command_asr_ms: activation_measurement.wake_to_command_asr_ms,
        command_start_ms: activation_measurement.command_start_ms,
        total_activation_ms: activation_measurement.total_activation_ms,
        pre_roll_startup_ms: activation_measurement.pre_roll_startup_ms,
        pre_roll_samples: activation_measurement.pre_roll_samples,
        continuous_asr_idle_cpu_percent: continuous_asr_measurement
            .map(|measurement| measurement.idle_cpu_percent),
        continuous_asr_observation_ms: continuous_asr_measurement
            .map(|measurement| measurement.observation_ms),
        continuous_asr_audio_ms: continuous_asr_measurement.map(|measurement| measurement.audio_ms),
        continuous_asr_processed_audio_ms: continuous_asr_measurement
            .map(|measurement| measurement.processed_audio_ms),
        criteria_version: index.acceptance_criteria.criteria_version,
        positive_recall_minimum: index.acceptance_criteria.positive_recall_minimum,
        negative_false_accepts_maximum: index.acceptance_criteria.negative_false_accepts_maximum,
        passed,
        fixtures: results,
    })
}

#[derive(Debug, Clone, Copy)]
struct ActivationMeasurement {
    wake_to_command_asr_ms: u64,
    command_start_ms: u64,
    total_activation_ms: u64,
    pre_roll_startup_ms: u64,
    pre_roll_samples: u64,
}

#[derive(Debug, Clone, Copy)]
struct ContinuousAsrMeasurement {
    idle_cpu_percent: f64,
    observation_ms: u64,
    audio_ms: u64,
    processed_audio_ms: u64,
}

#[derive(Default)]
struct MeasurementCommandIngress {
    accepted_samples: usize,
    accepted_bytes: usize,
}

impl WakeCommandAsrIngress for MeasurementCommandIngress {
    fn accept_wake_handoff(&mut self, audio: WakeCommandHandoffAudio) -> Result<(), String> {
        self.accepted_samples = audio.samples_i16().len();
        self.accepted_bytes = audio.to_pcm16_le_bytes().len();
        Ok(())
    }
}

struct ImmediateCommandStarter;

#[async_trait]
impl WakeCommandStarter for ImmediateCommandStarter {
    async fn start_normal_command_interaction(&mut self) -> Result<(), String> {
        Ok(())
    }
}

fn measure_wake_command_activation_timing() -> Result<ActivationMeasurement, String> {
    let settings = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };
    let runtime = WakeWordApplicationRuntime::from_settings(&settings)
        .map_err(|error| format!("failed to create Wake Word runtime: {error}"))?;
    runtime
        .mark_loaded()
        .map_err(|error| format!("failed to load Wake Word runtime: {error}"))?;
    let samples = vec![0_i16; HANDOFF_MEASUREMENT_SAMPLES];
    let audio = WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, samples)?;
    let mut handoff = WakeCommandAsrHandoff::new(audio);
    let mut ingress = MeasurementCommandIngress::default();
    let mut starter = ImmediateCommandStarter;
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "failed to create Wake Word timing runtime".to_string())?;
    let (delivered, timing) = tokio_runtime.block_on(async {
        activate_wake_command_and_measure_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
    })?;
    if !delivered {
        return Err("Wake command timing measurement did not deliver handoff".to_string());
    }
    if ingress.accepted_samples != HANDOFF_MEASUREMENT_SAMPLES {
        return Err("Wake command timing measurement delivered an unexpected sample count".to_string());
    }
    if ingress.accepted_bytes != HANDOFF_MEASUREMENT_SAMPLES.saturating_mul(2) {
        return Err("Wake command timing measurement delivered an unexpected byte count".to_string());
    }
    Ok(ActivationMeasurement {
        wake_to_command_asr_ms: timing.wake_to_command_asr_ms,
        command_start_ms: timing.command_start_ms,
        total_activation_ms: timing.total_activation_ms,
        pre_roll_startup_ms: timing.wake_to_command_asr_ms,
        pre_roll_samples: HANDOFF_MEASUREMENT_SAMPLES as u64,
    })
}

fn measure_continuous_asr_idle_baseline() -> Result<Option<ContinuousAsrMeasurement>, String> {
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "failed to create continuous ASR timing runtime".to_string())?;
    match tokio_runtime.block_on(measure_continuous_asr_idle_baseline_async()) {
        Ok(measurement) => Ok(Some(measurement)),
        Err(error) if error.contains("Moonshine native runtime is not linked into this build") => {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

async fn measure_continuous_asr_idle_baseline_async() -> Result<ContinuousAsrMeasurement, String> {
    let install_root = std::env::temp_dir().join("talking-moose-moonshine-asr-acceptance");
    let installer = Arc::new(
        MoonshineModelInstaller::new(install_root)
            .map_err(|error| format!("failed to initialize Moonshine installer: {error}"))?,
    );
    let cancellation = MoonshineModelInstallCancellation::default();
    installer
        .install(MoonshineModelArchitecture::TinyStreaming, &cancellation)
        .await
        .map_err(|error| format!("failed to install Moonshine Tiny baseline: {error}"))?;
    let callback: LocalAsrPipelineEventCallback = Arc::new(|_event| {});
    let mut pipeline = LocalAsrPipeline::start_tiny(installer, callback)
        .await
        .map_err(|error| error.message)?;
    let measurement = feed_continuous_asr_silence(&pipeline).await;
    let stop_result = pipeline.stop_and_join().await.map_err(|error| error.message);
    match (measurement, stop_result) {
        (Ok(measurement), Ok(())) => Ok(measurement),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

async fn feed_continuous_asr_silence(
    pipeline: &LocalAsrPipeline,
) -> Result<ContinuousAsrMeasurement, String> {
    let silence = vec![0_i16; CONTINUOUS_ASR_CHUNK_SAMPLES];
    let started = Instant::now();
    let cpu_before = process_cpu_time_ms();
    let mut accepted_audio_ms = 0_u64;

    while accepted_audio_ms < CONTINUOUS_ASR_BASELINE_AUDIO_MS {
        let audio = WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, silence.clone())?;
        match pipeline.prime_wake_handoff(audio) {
            Ok(()) => {
                accepted_audio_ms = accepted_audio_ms.saturating_add(CONTINUOUS_ASR_CHUNK_AUDIO_MS);
                tokio::time::sleep(Duration::from_millis(CONTINUOUS_ASR_CHUNK_AUDIO_MS)).await;
            }
            Err(error) if error.retryable => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => return Err(error.message),
        }
    }

    let deadline = Instant::now() + Duration::from_secs(10);
    while pipeline.diagnostics().processed_audio_ms < accepted_audio_ms && Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let observation_ms: u64 = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    let cpu_ms = process_cpu_time_ms().saturating_sub(cpu_before);
    let idle_cpu_percent = if observation_ms == 0 {
        0.0
    } else {
        (cpu_ms as f64 * 100.0) / observation_ms as f64
    };
    let processed_audio_ms = pipeline.diagnostics().processed_audio_ms;
    if processed_audio_ms < accepted_audio_ms {
        return Err(
            "continuous ASR baseline did not process the requested audio window".to_string(),
        );
    }

    Ok(ContinuousAsrMeasurement {
        idle_cpu_percent,
        observation_ms,
        audio_ms: accepted_audio_ms,
        processed_audio_ms,
    })
}

fn safe_fixture_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("generated fixture path escapes corpus root".to_string());
    }
    Ok(root.join(relative_path))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use ring::digest::{Context, SHA256};
    let mut context = Context::new(&SHA256);
    context.update(bytes);
    context
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn process_cpu_time_ms() -> u64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return 0;
        }
        timeval_ms(usage.ru_utime).saturating_add(timeval_ms(usage.ru_stime))
    }
}

fn timeval_ms(value: libc::timeval) -> u64 {
    let seconds = u64::try_from(value.tv_sec).unwrap_or(0);
    let micros = u64::try_from(value.tv_usec).unwrap_or(0);
    seconds.saturating_mul(1_000).saturating_add(micros / 1_000)
}

fn peak_resident_memory_bytes() -> Option<u64> {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return None;
        }
        let rss = u64::try_from(usage.ru_maxrss).ok()?;
        #[cfg(target_os = "linux")]
        {
            Some(rss.saturating_mul(1_024))
        }
        #[cfg(target_os = "macos")]
        {
            Some(rss)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Some(rss)
        }
    }
}
