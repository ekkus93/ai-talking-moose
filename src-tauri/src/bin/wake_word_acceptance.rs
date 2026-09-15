use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use talking_moose_lib::audio::wake_word::{
    SherpaKwsEngine, WakeWordEngine, DEFAULT_KEYWORDS_SCORE, DEFAULT_KEYWORDS_THRESHOLD,
    WAKE_INFERENCE_THREADS, WAKE_SAMPLE_RATE_HZ,
};

#[derive(Debug, Deserialize)]
struct CorpusManifest {
    cases: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
struct CorpusCase {
    path: String,
    expect_trigger: bool,
    category: String,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    path: String,
    category: String,
    expected_trigger: bool,
    observed_trigger: bool,
    samples: usize,
    audio_duration_ms: f64,
    inference_duration_ms: f64,
    realtime_factor: f64,
}

#[derive(Debug, Serialize)]
struct AcceptanceReport {
    engine: &'static str,
    sample_rate_hz: u32,
    inference_threads: i32,
    keywords_score: f32,
    keywords_threshold: f32,
    initialization_duration_ms: f64,
    max_rss_kib: u64,
    positive_total: usize,
    positive_detected: usize,
    negative_total: usize,
    negative_false_accepts: usize,
    maximum_realtime_factor: f64,
    cases: Vec<CaseResult>,
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
    bytes
        .get(offset..offset + 2)
        .and_then(|slice| slice.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| "WAV header is truncated.".to_string())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| "WAV header is truncated.".to_string())
}

fn read_pcm_wav(path: &Path) -> Result<Vec<i16>, String> {
    let bytes = fs::read(path).map_err(|_| "Acceptance WAV could not be read.".to_string())?;
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err("Acceptance fixture is not a RIFF/WAVE file.".to_string());
    }
    let mut cursor = 12usize;
    let mut format: Option<(u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = read_u32_le(&bytes, cursor + 4)? as usize;
        let start = cursor + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| "Acceptance WAV chunk is truncated.".to_string())?;
        if id == b"fmt " {
            if size < 16 {
                return Err("Acceptance WAV fmt chunk is invalid.".to_string());
            }
            let encoding = read_u16_le(&bytes, start)?;
            let channels = read_u16_le(&bytes, start + 2)?;
            let sample_rate = read_u32_le(&bytes, start + 4)?;
            let bits = read_u16_le(&bytes, start + 14)?;
            if encoding != 1 {
                return Err("Acceptance WAV must use integer PCM.".to_string());
            }
            format = Some((channels, sample_rate, bits));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        cursor = end + (size % 2);
    }
    let (channels, sample_rate, bits) =
        format.ok_or_else(|| "Acceptance WAV has no format chunk.".to_string())?;
    if channels != 1 || sample_rate != WAKE_SAMPLE_RATE_HZ || bits != 16 {
        return Err(format!(
            "Acceptance WAV must be mono 16-bit PCM at {WAKE_SAMPLE_RATE_HZ} Hz."
        ));
    }
    let data = data.ok_or_else(|| "Acceptance WAV has no data chunk.".to_string())?;
    if data.len() % 2 != 0 {
        return Err("Acceptance WAV has an odd PCM byte count.".to_string());
    }
    Ok(data
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn max_rss_kib() -> u64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return 0;
        }
        #[cfg(target_os = "linux")]
        {
            usage.ru_maxrss.max(0) as u64
        }
        #[cfg(target_os = "macos")]
        {
            (usage.ru_maxrss.max(0) as u64) / 1024
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn max_rss_kib() -> u64 {
    0
}

fn parse_args() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let mut model_root = None;
    let mut manifest = None;
    let mut report = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value for {arg}."))?;
        match arg.as_str() {
            "--model-root" => model_root = Some(PathBuf::from(value)),
            "--manifest" => manifest = Some(PathBuf::from(value)),
            "--report" => report = Some(PathBuf::from(value)),
            _ => return Err(format!("Unsupported acceptance argument: {arg}.")),
        }
    }
    Ok((
        model_root.ok_or_else(|| "--model-root is required.".to_string())?,
        manifest.ok_or_else(|| "--manifest is required.".to_string())?,
        report.ok_or_else(|| "--report is required.".to_string())?,
    ))
}

fn main() -> Result<(), String> {
    let (model_root, manifest_path, report_path) = parse_args()?;
    let corpus: CorpusManifest = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|_| "Corpus manifest could not be read.".to_string())?,
    )
    .map_err(|_| "Corpus manifest is invalid.".to_string())?;
    if corpus.cases.is_empty() {
        return Err("Corpus manifest contains no cases.".to_string());
    }

    let initialization_started = Instant::now();
    let mut engine = SherpaKwsEngine::open(&model_root).map_err(|error| error.message)?;
    let initialization_duration_ms = initialization_started.elapsed().as_secs_f64() * 1000.0;
    if engine.inference_threads() != 1 || WAKE_INFERENCE_THREADS != 1 {
        return Err("Wake Word production inference must use exactly one thread.".to_string());
    }

    let corpus_root = manifest_path
        .parent()
        .ok_or_else(|| "Corpus manifest has no parent directory.".to_string())?;
    let mut results = Vec::with_capacity(corpus.cases.len());
    let mut positive_total = 0usize;
    let mut positive_detected = 0usize;
    let mut negative_total = 0usize;
    let mut negative_false_accepts = 0usize;
    let mut maximum_realtime_factor = 0.0f64;

    for case in corpus.cases {
        engine.reset().map_err(|error| error.message)?;
        let samples = read_pcm_wav(&corpus_root.join(&case.path))?;
        let audio_duration_ms = samples.len() as f64 / WAKE_SAMPLE_RATE_HZ as f64 * 1000.0;
        let inference_started = Instant::now();
        let mut observed_trigger = false;
        for chunk in samples.chunks(1600) {
            if engine
                .process_pcm_i16(chunk)
                .map_err(|error| error.message)?
            {
                observed_trigger = true;
                break;
            }
        }
        if !observed_trigger {
            let silence = vec![0_i16; WAKE_SAMPLE_RATE_HZ as usize];
            for chunk in silence.chunks(1600) {
                if engine
                    .process_pcm_i16(chunk)
                    .map_err(|error| error.message)?
                {
                    observed_trigger = true;
                    break;
                }
            }
        }
        let inference_duration_ms = inference_started.elapsed().as_secs_f64() * 1000.0;
        let realtime_factor = if audio_duration_ms > 0.0 {
            inference_duration_ms / audio_duration_ms
        } else {
            0.0
        };
        maximum_realtime_factor = maximum_realtime_factor.max(realtime_factor);

        if case.expect_trigger {
            positive_total += 1;
            if observed_trigger {
                positive_detected += 1;
            }
        } else {
            negative_total += 1;
            if observed_trigger {
                negative_false_accepts += 1;
            }
        }
        results.push(CaseResult {
            path: case.path,
            category: case.category,
            expected_trigger: case.expect_trigger,
            observed_trigger,
            samples: samples.len(),
            audio_duration_ms,
            inference_duration_ms,
            realtime_factor,
        });
    }

    engine.shutdown().map_err(|error| error.message)?;
    let report = AcceptanceReport {
        engine: "sherpa-onnx-kws",
        sample_rate_hz: WAKE_SAMPLE_RATE_HZ,
        inference_threads: WAKE_INFERENCE_THREADS,
        keywords_score: DEFAULT_KEYWORDS_SCORE,
        keywords_threshold: DEFAULT_KEYWORDS_THRESHOLD,
        initialization_duration_ms,
        max_rss_kib: max_rss_kib(),
        positive_total,
        positive_detected,
        negative_total,
        negative_false_accepts,
        maximum_realtime_factor,
        cases: results,
    };
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report)
            .map_err(|_| "Report serialization failed.".to_string())?,
    )
    .map_err(|_| "Acceptance report could not be written.".to_string())?;
    println!(
        "wake-word-acceptance positives={}/{} false_accepts={}/{} max_rtf={:.4} init_ms={:.1} max_rss_kib={}",
        report.positive_detected,
        report.positive_total,
        report.negative_false_accepts,
        report.negative_total,
        report.maximum_realtime_factor,
        report.initialization_duration_ms,
        report.max_rss_kib
    );
    if report.positive_detected != report.positive_total {
        return Err("Wake Word positive corpus contains false rejects.".to_string());
    }
    if report.negative_false_accepts != 0 {
        return Err("Wake Word negative corpus contains false accepts.".to_string());
    }
    if report.maximum_realtime_factor >= 1.0 {
        return Err("Wake Word inference failed the real-time performance gate.".to_string());
    }
    Ok(())
}
