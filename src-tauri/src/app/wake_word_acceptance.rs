use super::wake_word::engine::{
    NativeKwsSession, NativeKwsSessionPaths, SherpaKwsEngine, V1_KWS_SAMPLE_RATE_HZ,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const FRAME_SAMPLES: usize = 1_600;

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
    pub peak_resident_memory_bytes: Option<u64>,
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
        let samples: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
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

    let positive: Vec<_> = results.iter().filter(|item| item.expected_detection).collect();
    let negative: Vec<_> = results.iter().filter(|item| !item.expected_detection).collect();
    if positive.is_empty() || negative.is_empty() {
        return Err("acceptance corpus must contain positive and negative fixtures".to_string());
    }
    let positive_detected = positive.iter().filter(|item| item.detected).count() as u64;
    let negative_false_accepts = negative.iter().filter(|item| item.detected).count() as u64;
    let positive_recall = positive_detected as f64 / positive.len() as f64;
    let passed = positive_recall >= index.acceptance_criteria.positive_recall_minimum
        && negative_false_accepts <= index.acceptance_criteria.negative_false_accepts_maximum;

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
        peak_resident_memory_bytes: peak_resident_memory_bytes(),
        criteria_version: index.acceptance_criteria.criteria_version,
        positive_recall_minimum: index.acceptance_criteria.positive_recall_minimum,
        negative_false_accepts_maximum: index.acceptance_criteria.negative_false_accepts_maximum,
        passed,
        fixtures: results,
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
