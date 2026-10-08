// Manual, feature-gated, real-CPU acceptance support for the Whisper.cpp
// `ggml-small` local ASR path.
//
// This mirrors `ai::local::acceptance` (the Local LLM P12/P13 real-CPU
// acceptance). It is the contract that a Linux CPU-only whisper.cpp runtime can
// install the exact pinned model weight through the production installer and run
// a full transcription of a pinned WAV corpus entirely over CPU, with the
// transcribe phase running under an OS-level denied network boundary.
//
// No model weights are committed. No Google/Fake provider is ever used. The
// install phase requires a network (it downloads the 487 MiB weight); the
// transcribe phase requires a denied network (proves the transcription is fully
// local).

use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use super::super::runtime_metrics::{
    current_resident_memory_bytes, peak_resident_memory_bytes, process_cpu_time_micros,
};
use super::engine::{
    WHISPER_ENDPOINT_SILENCE_SAMPLES, WHISPER_MAX_UTTERANCE_SAMPLES,
    WHISPER_PARTIAL_INTERVAL_SAMPLES,
};
use super::ffi::{path_to_cstring, NativeWhisperApi, WhisperApi};
use super::installer::{
    WhisperModelInstallCancellation, WhisperModelInstallDisposition,
    WhisperModelInstallProgressCallback, WhisperModelInstaller,
};
use super::manifest::{
    WHISPER_MODEL_BYTES, WHISPER_MODEL_MAGIC, WHISPER_MODEL_REVISION, WHISPER_MODEL_SHA256,
    WHISPER_SMALL_ID, WHISPER_SOURCE_COMMIT,
};
use crate::asr::pipeline::{LocalAsrPipeline, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS};
use crate::asr::AsrEvent;

const REPORT_SCHEMA_VERSION: u32 = 2;
const MODEL_FILENAME: &str = "ggml-small.bin";
const QUANTIZATION: &str = "f16";
const LICENSE_STATE: &str = "MIT";

fn git_sha() -> Option<String> {
    std::env::var("GITHUB_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn host_cpu() -> Option<String> {
    std::env::var("TALKING_MOOSE_ACCEPTANCE_HOST_CPU")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Returns true when a default IP route is present, meaning network egress is
/// reachable. Reads the kernel's routing table on Linux.
#[cfg(target_os = "linux")]
fn linux_default_route_present() -> bool {
    let Ok(routes) = fs::read_to_string("/proc/net/route") else {
        return true;
    };
    routes.lines().skip(1).any(|line| {
        let mut fields = line.split_whitespace();
        let _interface = fields.next();
        matches!(fields.next(), Some("00000000"))
    })
}

#[cfg(not(target_os = "linux"))]
fn linux_default_route_present() -> bool {
    false
}

/// Probes for a denied network boundary. Returns true only when the default
/// route is absent AND a short TCP connect to an unreachable address fails.
fn network_denial_probe() -> bool {
    if linux_default_route_present() {
        return false;
    }
    let address = SocketAddr::from(([1, 1, 1, 1], 443));
    TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_err()
}

fn write_report(path: &Path, report: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "acceptance report path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|_| "could not create acceptance report directory".to_string())?;
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|_| "could not serialize acceptance report".to_string())?;
    fs::write(path, bytes).map_err(|_| "could not write acceptance report".to_string())
}

fn disposition_name(disposition: WhisperModelInstallDisposition) -> &'static str {
    match disposition {
        WhisperModelInstallDisposition::Installed => "installed",
        WhisperModelInstallDisposition::AlreadyInstalled => "already-installed",
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperInstallAcceptanceReport {
    pub schema_version: u32,
    pub phase: &'static str,
    pub generated_at_utc: String,
    pub git_sha: Option<String>,
    pub model_id: String,
    /// Immutable model-artifact repository revision.
    pub revision: String,
    /// Exact whisper.cpp native source revision compiled into the runtime.
    pub source_commit: String,
    pub artifact_filename: String,
    pub sha256: String,
    pub expected_bytes: u64,
    pub installed_bytes: u64,
    pub disposition: String,
    pub magic: String,
    pub quantization: String,
    pub license: String,
    pub production_installer_verified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperDeleteAcceptanceReport {
    pub schema_version: u32,
    pub phase: String,
    pub removed: bool,
    pub model_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperTranscriptReport {
    pub duration_ms: u64,
    pub no_speech_prob: f32,
    pub num_segments: u32,
    pub text: String,
    pub transcription_ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperTranscribeRuntime {
    pub whisper_native_linked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperPipelineAcceptanceMetrics {
    pub partial_interval_samples: usize,
    pub endpoint_silence_samples: usize,
    pub maximum_utterance_samples: usize,
    pub queue_capacity_chunks: usize,
    pub partial_event_count: u64,
    pub final_event_count: u64,
    pub first_partial_latency_ms: Option<u64>,
    pub first_final_latency_ms: Option<u64>,
    pub processed_audio_ms: u64,
    pub inference_wall_time_ms: u64,
    pub real_time_factor: Option<f32>,
    pub process_cpu_time_ms: Option<u64>,
    pub average_cpu_utilization_percent: Option<f32>,
    pub peak_resident_memory_bytes: Option<u64>,
    pub nominal_dropped_chunks: u64,
    pub overload_attempted_chunks: u64,
    pub overload_accepted_chunks: u64,
    pub overload_dropped_chunks: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperTranscribeAcceptanceReport {
    pub schema_version: u32,
    pub phase: &'static str,
    pub generated_at_utc: String,
    pub git_sha: Option<String>,
    pub model_id: String,
    /// Immutable model-artifact repository revision.
    pub revision: String,
    /// Exact whisper.cpp native source revision compiled into the runtime.
    pub source_commit: String,
    pub artifact_filename: String,
    pub sha256: String,
    pub expected_bytes: u64,
    pub installed_bytes: u64,
    pub corpus_path: String,
    pub corpus_bytes: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_count: u64,
    pub duration_ms: u64,
    pub host_os: String,
    pub host_arch: String,
    pub host_cpu: Option<String>,
    pub available_parallelism: usize,
    pub network_denied_required: bool,
    pub network_denial_probe_passed: bool,
    pub status: String,
    pub runtime: WhisperTranscribeRuntime,
    pub pipeline: WhisperPipelineAcceptanceMetrics,
    pub transcript: WhisperTranscriptReport,
    pub transcribe_wall_ms: u64,
    pub phase_wall_ms: u64,
    pub process_cpu_time_ms: u64,
    pub process_cpu_usage_percent: Option<f64>,
    pub baseline_resident_memory_bytes: Option<u64>,
    pub resident_memory_bytes: Option<u64>,
    pub peak_resident_memory_bytes: Option<u64>,
}

#[derive(Debug)]
struct WavSamples {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
}

/// Parses a 16-bit PCM WAV file into 32-bit float samples in the range
/// [-1.0, 1.0). The acceptance binary needs a small, dependency-free parser
/// because the model weights are not bundled and the project has no WAV crate.
/// It fails closed for any non-PCM, non-16-bit, or malformed container.
fn read_wav_f32(path: &Path) -> Result<WavSamples, String> {
    let raw = fs::read(path).map_err(|error| format!("read wav file: {error}"))?;
    if raw.len() < 44 {
        return Err("wav file is too small to contain a valid RIFF/WAVE header".to_string());
    }

    if raw[0..4].to_vec() != b"RIFF".to_vec() {
        return Err("wav file is not a RIFF file".to_string());
    }
    if raw[8..12].to_vec() != b"WAVE".to_vec() {
        return Err("wav file is not a WAVE file".to_string());
    }

    // The fmt chunk must immediately follow the WAVE header, at offset 12.
    if raw[12..16].to_vec() != b"fmt ".to_vec() {
        return Err("wav file has no fmt chunk immediately after the WAVE header".to_string());
    }
    let fmt_size = le32(&raw, 16);
    if fmt_size != 16 {
        return Err(format!(
            "wav fmt chunk size {fmt_size} is not the standard PCM size of 16"
        ));
    }

    let audio_format = le16(&raw, 20);
    if audio_format != 1 {
        return Err(format!("wav file is not PCM (format tag {audio_format})"));
    }
    let channels = le16(&raw, 22);
    let sample_rate = le32(&raw, 24);
    let block_align = le16(&raw, 32);
    let bits_per_sample = le16(&raw, 34);

    if bits_per_sample != 16 {
        return Err(format!(
            "wav file is {bits_per_sample}-bit; whisper acceptance requires 16-bit PCM"
        ));
    }
    if sample_rate == 0 {
        return Err("wav fmt chunk does not declare a sample rate".to_string());
    }
    let expected_block_align = (channels as u32) * (bits_per_sample as u32) / 8;
    if (block_align as u32) != expected_block_align {
        return Err(format!(
            "wav block align {block_align} does not match expected {expected_block_align} for {channels}-channel {bits_per_sample}-bit PCM"
        ));
    }

    // Scan the remaining chunks for the data chunk.
    let file_len = raw.len();
    let mut offset: usize = 40;
    let mut data_start: Option<usize> = None;
    let mut data_size: Option<u32> = None;
    while offset + 8 <= file_len {
        if raw[offset..offset + 4].to_vec() != b"data".to_vec() {
            let chunk_size = le32(&raw, offset + 4) as usize;
            if chunk_size == 0 {
                offset = file_len;
            } else {
                let next = offset + 8 + chunk_size + (chunk_size % 2);
                if next > file_len {
                    return Err("wav chunk extends beyond the file".to_string());
                }
                offset = next;
            }
        } else {
            data_size = Some(le32(&raw, offset + 4));
            data_start = Some(offset + 8);
            break;
        }
    }

    let (data_start, data_size) = match (data_start, data_size) {
        (Some(start), Some(size)) => (start, size),
        _ => return Err("wav file has no data chunk".to_string()),
    };
    if data_start + data_size as usize > file_len {
        return Err("wav data chunk extends beyond the file".to_string());
    }

    let data = &raw[data_start..data_start + data_size as usize];
    if data.len() % 2 != 0 {
        return Err("wav data has an odd number of bytes for 16-bit samples".to_string());
    }

    let samples: Vec<f32> = data
        .chunks(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32_768.0)
        .collect();

    Ok(WavSamples {
        samples,
        sample_rate,
        channels,
        bits_per_sample,
    })
}

fn le16(raw: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([raw[offset], raw[offset + 1]])
}

fn le32(raw: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ])
}

pub async fn install_for_acceptance(
    model_root: &Path,
    report_path: &Path,
) -> Result<WhisperInstallAcceptanceReport, String> {
    let installer = WhisperModelInstaller::new(model_root.to_path_buf())
        .map_err(|error| error.message.to_string())?;

    let mut cancellation = WhisperModelInstallCancellation::default();
    let progress: Option<WhisperModelInstallProgressCallback> = None;
    let outcome = installer
        .install(&mut cancellation, &progress)
        .await
        .map_err(|error| error.message.to_string())?;

    if outcome.installed_bytes != WHISPER_MODEL_BYTES {
        return Err(format!(
            "installed model byte count {} does not match pinned bytes {}",
            outcome.installed_bytes, WHISPER_MODEL_BYTES
        ));
    }

    let report = WhisperInstallAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "install",
        generated_at_utc: Utc::now().to_rfc3339(),
        git_sha: git_sha(),
        model_id: WHISPER_SMALL_ID.to_string(),
        revision: WHISPER_MODEL_REVISION.to_string(),
        source_commit: WHISPER_SOURCE_COMMIT.to_string(),
        artifact_filename: MODEL_FILENAME.to_string(),
        sha256: WHISPER_MODEL_SHA256.to_string(),
        expected_bytes: WHISPER_MODEL_BYTES,
        installed_bytes: outcome.installed_bytes,
        disposition: disposition_name(outcome.disposition).to_string(),
        magic: String::from_utf8(WHISPER_MODEL_MAGIC.to_vec())
            .unwrap_or_else(|error| format!("invalid model magic: {error}")),
        quantization: QUANTIZATION.to_string(),
        license: LICENSE_STATE.to_string(),
        production_installer_verified: true,
    };

    write_report(report_path, &report)?;
    Ok(report)
}

pub async fn delete_for_acceptance(
    model_root: &Path,
) -> Result<WhisperDeleteAcceptanceReport, String> {
    let installer = WhisperModelInstaller::new(model_root.to_path_buf())
        .map_err(|error| error.message.to_string())?;
    if !installer.model_path().is_file() {
        return Err(
            "production installer delete acceptance requires an installed model".to_string(),
        );
    }
    installer
        .delete()
        .await
        .map_err(|error| error.message.to_string())?;
    if installer.model_path().exists() {
        return Err("production installer left the Whisper model after delete".to_string());
    }
    Ok(WhisperDeleteAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "delete".to_string(),
        removed: true,
        model_path: installer.model_path().display().to_string(),
    })
}

fn pcm_f32_to_i16_le(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len().saturating_mul(2));
    for sample in samples {
        let scaled = (sample.clamp(-1.0, 32767.0 / 32768.0) * 32768.0).round() as i16;
        bytes.extend_from_slice(&scaled.to_le_bytes());
    }
    bytes
}

async fn production_pipeline_metrics(
    installer: std::sync::Arc<WhisperModelInstaller>,
    samples: &[f32],
) -> Result<WhisperPipelineAcceptanceMetrics, String> {
    let events = Arc::new(StdMutex::new(Vec::<AsrEvent>::new()));
    let callback_events = events.clone();
    let callback = Arc::new(move |event: AsrEvent| {
        callback_events
            .lock()
            .expect("acceptance event lock")
            .push(event);
    });

    let mut pipeline = LocalAsrPipeline::start_whisper(installer.clone(), callback)
        .await
        .map_err(|error| error.message)?;

    let mut nominal_dropped_chunks = 0_u64;
    for chunk in samples.chunks(1_600) {
        let accepted = pipeline
            .try_send_pcm_for_acceptance(pcm_f32_to_i16_le(chunk))
            .map_err(|error| error.message)?;
        if !accepted {
            nominal_dropped_chunks = nominal_dropped_chunks.saturating_add(1);
        }
        // Feed at the production 100 ms cadence so nominal drop evidence is
        // meaningful rather than an artificial producer burst.
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    pipeline
        .stop_and_join()
        .await
        .map_err(|error| error.message)?;
    let diagnostics = pipeline.diagnostics();
    let (partial_event_count, final_event_count) = {
        let nominal_events = events.lock().expect("acceptance event lock");
        (
            nominal_events
                .iter()
                .filter(|event| matches!(event, AsrEvent::PartialTranscript { .. }))
                .count() as u64,
            nominal_events
                .iter()
                .filter(|event| matches!(event, AsrEvent::FinalTranscript { .. }))
                .count() as u64,
        )
    };

    // Deliberately outrun a second production worker. The bounded sender must
    // exercise drop-newest rather than blocking or growing without bound.
    let overload_events = Arc::new(StdMutex::new(Vec::<AsrEvent>::new()));
    let overload_events_for_callback = overload_events.clone();
    let overload_callback = Arc::new(move |event: AsrEvent| {
        overload_events_for_callback
            .lock()
            .expect("overload event lock")
            .push(event);
    });
    let mut overload = LocalAsrPipeline::start_whisper(installer, overload_callback)
        .await
        .map_err(|error| error.message)?;
    let representative = samples
        .chunks(1_600)
        .find(|chunk| !chunk.is_empty())
        .ok_or_else(|| "Whisper acceptance corpus contains no PCM samples".to_string())?;
    let overload_bytes = pcm_f32_to_i16_le(representative);
    let overload_attempted_chunks = 64_u64;
    let mut overload_accepted_chunks = 0_u64;
    let mut overload_dropped_chunks = 0_u64;
    for _ in 0..overload_attempted_chunks {
        if overload
            .try_send_pcm_for_acceptance(overload_bytes.clone())
            .map_err(|error| error.message)?
        {
            overload_accepted_chunks = overload_accepted_chunks.saturating_add(1);
        } else {
            overload_dropped_chunks = overload_dropped_chunks.saturating_add(1);
        }
    }
    overload
        .stop_and_join()
        .await
        .map_err(|error| error.message)?;

    Ok(WhisperPipelineAcceptanceMetrics {
        partial_interval_samples: WHISPER_PARTIAL_INTERVAL_SAMPLES,
        endpoint_silence_samples: WHISPER_ENDPOINT_SILENCE_SAMPLES,
        maximum_utterance_samples: WHISPER_MAX_UTTERANCE_SAMPLES,
        queue_capacity_chunks: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        partial_event_count,
        final_event_count,
        first_partial_latency_ms: diagnostics.first_partial_latency_ms,
        first_final_latency_ms: diagnostics.first_final_latency_ms,
        processed_audio_ms: diagnostics.processed_audio_ms,
        inference_wall_time_ms: diagnostics.inference_wall_time_ms,
        real_time_factor: diagnostics.real_time_factor,
        process_cpu_time_ms: diagnostics.process_cpu_time_ms,
        average_cpu_utilization_percent: diagnostics.average_cpu_utilization_percent,
        peak_resident_memory_bytes: diagnostics.peak_resident_memory_bytes,
        nominal_dropped_chunks,
        overload_attempted_chunks,
        overload_accepted_chunks,
        overload_dropped_chunks,
    })
}

pub async fn transcribe_for_acceptance(
    model_root: &Path,
    corpus_wav: &Path,
    report_path: &Path,
    require_network_denied: bool,
) -> Result<WhisperTranscribeAcceptanceReport, String> {
    let network_denial_probe_passed = network_denial_probe();
    if require_network_denied && !network_denial_probe_passed {
        return Err(
            "real-corpus transcription acceptance requires an OS-level denied network boundary"
                .to_string(),
        );
    }

    let installer = WhisperModelInstaller::new(model_root.to_path_buf())
        .map_err(|error| error.message.to_string())?;
    // `acquire_verified_model_lease` uses a blocking lock owned by a
    // tokio runtime mutex, which panics on tokio runtime threads. The
    // acceptance binary runs on a tokio runtime, so the lease is acquired
    // on a dedicated std thread — matching the production pipeline, where
    // engine load happens on std threads.
    let lease = {
        let joined = std::thread::scope(|scope| {
            scope
                .spawn(|| installer.acquire_verified_model_lease())
                .join()
        });
        match joined {
            Ok(inner) => match inner {
                Ok(Some(lease)) => lease,
                Ok(None) => {
                    return Err("selected Whisper model is not installed and verified".to_string());
                }
                Err(error) => return Err(error.message.to_string()),
            },
            Err(join_error) => {
                return Err(format!(
                    "whisper model lease acquisition thread failed: {join_error:?}"
                ));
            }
        }
    };

    let installed_bytes = lease
        .model_path()
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if installed_bytes != WHISPER_MODEL_BYTES {
        return Err(
            "installed Whisper model byte count does not match the pinned manifest".to_string(),
        );
    }

    let corpus_bytes = corpus_wav
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    let wav = read_wav_f32(corpus_wav)?;
    if wav.sample_rate != 16_000 {
        return Err(format!(
            "corpus sample rate {} is not 16000 Hz",
            wav.sample_rate
        ));
    }
    if wav.channels != 1 {
        return Err(format!("corpus channel count {} is not mono", wav.channels));
    }
    if wav.bits_per_sample != 16 {
        return Err(format!(
            "corpus bits per sample {} is not 16-bit",
            wav.bits_per_sample
        ));
    }

    let cpu_before = process_cpu_time_micros();
    let baseline_resident_memory_bytes = current_resident_memory_bytes();
    let phase_started = Instant::now();

    let api: Box<dyn WhisperApi> = Box::new(NativeWhisperApi);
    let model_path_c =
        path_to_cstring(lease.model_path()).map_err(|error| error.message.to_string())?;
    let model = api
        .load_model(&model_path_c)
        .map_err(|error| error.message.to_string())?;

    let started = Instant::now();
    let transcript = api
        .transcribe(&model, &wav.samples)
        .map_err(|error| error.message.to_string())?;
    let transcribe_wall_ms = elapsed_ms(started);
    let phase_wall_ms = elapsed_ms(phase_started);

    let cpu_after = process_cpu_time_micros();
    let resident_memory_bytes = current_resident_memory_bytes();

    let process_cpu_time_ms = match (cpu_before, cpu_after) {
        (Some(before), Some(after)) => {
            let delta_micros = after.saturating_sub(before);
            delta_micros / 1_000
        }
        _ => 0,
    };

    let process_cpu_usage_percent = if phase_wall_ms == 0 {
        None
    } else {
        let usage_percent = (process_cpu_time_ms as f64) / (phase_wall_ms as f64) * 100.0;
        if usage_percent.is_finite() && usage_percent >= 0.0 {
            Some(usage_percent)
        } else {
            None
        }
    };

    let sampled_peak_resident_memory_bytes =
        match (baseline_resident_memory_bytes, resident_memory_bytes) {
            (Some(baseline), Some(resident)) => Some(baseline.max(resident)),
            (None, resident) => resident,
            _ => None,
        };
    let peak_resident_memory_bytes = match (
        peak_resident_memory_bytes(),
        sampled_peak_resident_memory_bytes,
    ) {
        (Some(high_water), Some(sampled)) => Some(high_water.max(sampled)),
        (high_water, sampled) => high_water.or(sampled),
    };

    let mut combined = String::new();
    let mut segment_count = 0;
    for segment in &transcript.segments {
        let text = segment.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        segment_count += 1;
        if !combined.is_empty() {
            combined.push(' ');
        }
        combined.push_str(&text);
    }
    let no_speech_prob = transcript.no_speech_prob;
    let success = !combined.trim().is_empty();

    let duration_ms = sample_count_to_duration_ms(wav.samples.len(), wav.sample_rate);

    // Release the direct FFI model lease before starting the production worker;
    // both paths intentionally serialize model-owned operations through the
    // install/runtime lease.
    drop(model);
    drop(lease);
    let pipeline =
        production_pipeline_metrics(std::sync::Arc::new(installer), &wav.samples).await?;

    let report = WhisperTranscribeAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "transcribe",
        generated_at_utc: Utc::now().to_rfc3339(),
        git_sha: git_sha(),
        model_id: WHISPER_SMALL_ID.to_string(),
        revision: WHISPER_MODEL_REVISION.to_string(),
        source_commit: WHISPER_SOURCE_COMMIT.to_string(),
        artifact_filename: MODEL_FILENAME.to_string(),
        sha256: WHISPER_MODEL_SHA256.to_string(),
        expected_bytes: WHISPER_MODEL_BYTES,
        installed_bytes,
        corpus_path: corpus_wav.to_string_lossy().to_string(),
        corpus_bytes,
        sample_rate: wav.sample_rate,
        channels: wav.channels,
        bits_per_sample: wav.bits_per_sample,
        sample_count: wav.samples.len() as u64,
        duration_ms,
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        host_cpu: host_cpu(),
        available_parallelism: std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1),
        network_denied_required: require_network_denied,
        network_denial_probe_passed,
        status: if success {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        runtime: WhisperTranscribeRuntime {
            whisper_native_linked: true,
        },
        pipeline,
        transcript: WhisperTranscriptReport {
            duration_ms,
            no_speech_prob,
            num_segments: segment_count as u32,
            text: combined,
            transcription_ok: success,
        },
        transcribe_wall_ms,
        phase_wall_ms,
        process_cpu_time_ms,
        process_cpu_usage_percent,
        baseline_resident_memory_bytes,
        resident_memory_bytes,
        peak_resident_memory_bytes,
    };

    write_report(report_path, &report)?;
    if !success {
        return Err("Whisper transcription returned empty output".to_string());
    }

    Ok(report)
}

fn sample_count_to_duration_ms(sample_count: usize, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        0
    } else {
        (sample_count as f64 / sample_rate as f64 * 1000.0).ceil() as u64
    }
}
