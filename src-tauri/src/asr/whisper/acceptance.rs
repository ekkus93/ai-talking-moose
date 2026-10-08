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
    WhisperModelInstallCancellation, WhisperModelInstallDisposition, WhisperModelInstallOutcome,
    WhisperModelInstallProgressCallback, WhisperModelInstaller,
};
use super::manifest::{
    WHISPER_MODEL_BYTES, WHISPER_MODEL_MAGIC, WHISPER_MODEL_REVISION, WHISPER_MODEL_SHA256,
    WHISPER_SMALL_ID, WHISPER_SOURCE_COMMIT,
};
use crate::asr::pipeline::{LocalAsrPipeline, WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS};
use crate::asr::AsrEvent;

const REPORT_SCHEMA_VERSION: u32 = 2;
const DELETE_REPORT_SCHEMA_VERSION: u32 = 1;
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
    network_denial_probe_with(linux_default_route_present(), || {
        let address = SocketAddr::from(([1, 1, 1, 1], 443));
        TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok()
    })
}

fn network_denial_probe_with(
    default_route_present: bool,
    connect_succeeded: impl FnOnce() -> bool,
) -> bool {
    !default_route_present && !connect_succeeded()
}

fn check_network_requirement(required: bool, probe_passed: bool) -> Result<(), String> {
    if required && !probe_passed {
        Err(
            "real-corpus transcription acceptance requires an OS-level denied network boundary"
                .to_string(),
        )
    } else {
        Ok(())
    }
}

fn validate_corpus_format(wav: &wav::WavSamples) -> Result<(), String> {
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
    Ok(())
}

fn summarize_transcript_segments(
    segments: &[super::ffi::WhisperSegment],
) -> WhisperTranscriptReport {
    let mut combined = String::new();
    let mut segment_count = 0_u32;
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        segment_count += 1;
        if !combined.is_empty() {
            combined.push(' ');
        }
        combined.push_str(text);
    }
    let transcription_ok = !combined.is_empty();
    WhisperTranscriptReport {
        duration_ms: 0,
        no_speech_prob: 0.0,
        num_segments: segment_count,
        text: combined,
        transcription_ok,
    }
}

struct TranscribeReportContext {
    generated_at_utc: String,
    git_sha: Option<String>,
    installed_bytes: u64,
    corpus_path: String,
    corpus_bytes: u64,
    host_os: String,
    host_arch: String,
    host_cpu: Option<String>,
    available_parallelism: usize,
    network_denied_required: bool,
    network_denial_probe_passed: bool,
    pipeline: WhisperPipelineAcceptanceMetrics,
    transcript: WhisperTranscriptReport,
    transcribe_wall_ms: u64,
    phase_wall_ms: u64,
    process_cpu_time_ms: u64,
    process_cpu_usage_percent: Option<f64>,
    baseline_resident_memory_bytes: Option<u64>,
    resident_memory_bytes: Option<u64>,
    peak_resident_memory_bytes: Option<u64>,
}

fn build_transcribe_acceptance_report(
    wav: &wav::WavSamples,
    context: TranscribeReportContext,
) -> WhisperTranscribeAcceptanceReport {
    let status = if context.transcript.transcription_ok {
        "pass"
    } else {
        "fail"
    };
    WhisperTranscribeAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "transcribe",
        generated_at_utc: context.generated_at_utc,
        git_sha: context.git_sha,
        model_id: WHISPER_SMALL_ID.to_string(),
        revision: WHISPER_MODEL_REVISION.to_string(),
        source_commit: WHISPER_SOURCE_COMMIT.to_string(),
        artifact_filename: MODEL_FILENAME.to_string(),
        sha256: WHISPER_MODEL_SHA256.to_string(),
        expected_bytes: WHISPER_MODEL_BYTES,
        installed_bytes: context.installed_bytes,
        corpus_path: context.corpus_path,
        corpus_bytes: context.corpus_bytes,
        sample_rate: wav.sample_rate,
        channels: wav.channels,
        bits_per_sample: wav.bits_per_sample,
        sample_count: wav.samples.len() as u64,
        duration_ms: context.transcript.duration_ms,
        host_os: context.host_os,
        host_arch: context.host_arch,
        host_cpu: context.host_cpu,
        available_parallelism: context.available_parallelism,
        network_denied_required: context.network_denied_required,
        network_denial_probe_passed: context.network_denial_probe_passed,
        status: status.to_string(),
        runtime: WhisperTranscribeRuntime {
            whisper_native_linked: true,
        },
        pipeline: context.pipeline,
        transcript: context.transcript,
        transcribe_wall_ms: context.transcribe_wall_ms,
        phase_wall_ms: context.phase_wall_ms,
        process_cpu_time_ms: context.process_cpu_time_ms,
        process_cpu_usage_percent: context.process_cpu_usage_percent,
        baseline_resident_memory_bytes: context.baseline_resident_memory_bytes,
        resident_memory_bytes: context.resident_memory_bytes,
        peak_resident_memory_bytes: context.peak_resident_memory_bytes,
    }
}

fn validate_transcription_report(report: &WhisperTranscribeAcceptanceReport) -> Result<(), String> {
    if report.transcript.transcription_ok {
        Ok(())
    } else {
        Err("Whisper transcription returned empty output".to_string())
    }
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

fn build_install_acceptance_report(
    outcome: &WhisperModelInstallOutcome,
    generated_at_utc: String,
    git_sha: Option<String>,
) -> WhisperInstallAcceptanceReport {
    WhisperInstallAcceptanceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        phase: "install",
        generated_at_utc,
        git_sha,
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
    }
}

fn write_delete_acceptance_report(
    report_path: &Path,
    model_path: &Path,
) -> Result<WhisperDeleteAcceptanceReport, String> {
    let report = WhisperDeleteAcceptanceReport {
        schema_version: DELETE_REPORT_SCHEMA_VERSION,
        phase: "delete".to_string(),
        removed: true,
        model_path: model_path.display().to_string(),
    };
    write_report(report_path, &report)?;
    Ok(report)
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

mod wav;
use wav::read_wav_f32;

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

    let report = build_install_acceptance_report(&outcome, Utc::now().to_rfc3339(), git_sha());

    write_report(report_path, &report)?;
    Ok(report)
}

pub async fn delete_for_acceptance(
    model_root: &Path,
    report_path: &Path,
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
    write_delete_acceptance_report(report_path, &installer.model_path())
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
    let callback = Arc::new(move |event: AsrEvent, _ack| {
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
    let overload_callback = Arc::new(move |event: AsrEvent, _ack| {
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
        queue_capacity_chunks: WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
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
    check_network_requirement(require_network_denied, network_denial_probe_passed)?;

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
    validate_corpus_format(&wav)?;

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

    let duration_ms = sample_count_to_duration_ms(wav.samples.len(), wav.sample_rate);
    let mut transcript_report = summarize_transcript_segments(&transcript.segments);
    transcript_report.duration_ms = duration_ms;
    transcript_report.no_speech_prob = transcript.no_speech_prob;
    // Release the direct FFI model lease before starting the production worker;
    // both paths intentionally serialize model-owned operations through the
    // install/runtime lease.
    drop(model);
    drop(lease);
    let pipeline =
        production_pipeline_metrics(std::sync::Arc::new(installer), &wav.samples).await?;

    let report = build_transcribe_acceptance_report(
        &wav,
        TranscribeReportContext {
            generated_at_utc: Utc::now().to_rfc3339(),
            git_sha: git_sha(),
            installed_bytes,
            corpus_path: corpus_wav.to_string_lossy().to_string(),
            corpus_bytes,
            host_os: std::env::consts::OS.to_string(),
            host_arch: std::env::consts::ARCH.to_string(),
            host_cpu: host_cpu(),
            available_parallelism: std::thread::available_parallelism()
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(1),
            network_denied_required: require_network_denied,
            network_denial_probe_passed,
            pipeline,
            transcript: transcript_report,
            transcribe_wall_ms,
            phase_wall_ms,
            process_cpu_time_ms,
            process_cpu_usage_percent,
            baseline_resident_memory_bytes,
            resident_memory_bytes,
            peak_resident_memory_bytes,
        },
    );

    write_report(report_path, &report)?;
    validate_transcription_report(&report)?;

    Ok(report)
}

fn sample_count_to_duration_ms(sample_count: usize, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        0
    } else {
        (sample_count as f64 / sample_rate as f64 * 1000.0).ceil() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::whisper::ffi::WhisperSegment;

    #[test]
    fn delete_acceptance_writes_machine_readable_report() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let report_path = directory.path().join("reports/delete.json");
        let model_path = directory.path().join("model/ggml-small.bin");

        let report = write_delete_acceptance_report(&report_path, &model_path)
            .expect("delete acceptance report");
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report_path).expect("written report"))
                .expect("valid JSON report");

        assert_eq!(report.phase, "delete");
        assert_eq!(written["schema_version"], 1);
        assert_eq!(written["phase"], "delete");
        assert_eq!(written["removed"], true);
        assert_eq!(written["model_path"], model_path.display().to_string());
    }

    #[test]
    fn network_denial_probe_policy_is_injected_and_short_circuits_route() {
        let mut called = false;
        assert!(!network_denial_probe_with(true, || {
            called = true;
            false
        }));
        assert!(!called);
        assert!(network_denial_probe_with(false, || false));
        assert!(!network_denial_probe_with(false, || true));
    }

    #[test]
    fn network_requirement_only_rejects_when_denial_is_required() {
        assert!(check_network_requirement(true, false).is_err());
        assert!(check_network_requirement(true, true).is_ok());
        assert!(check_network_requirement(false, false).is_ok());
    }

    #[test]
    fn transcript_segments_trim_join_and_mark_empty_output_as_failure() {
        let segment = |text: &str| WhisperSegment {
            text: text.to_string(),
            start_ms: 0,
            end_ms: 1,
            no_speech_prob: 0.2,
        };
        let report = summarize_transcript_segments(&[
            segment("  hello "),
            segment(" \n"),
            segment("world  "),
        ]);
        assert_eq!(report.text, "hello world");
        assert_eq!(report.num_segments, 2);
        assert!(report.transcription_ok);
        let empty = summarize_transcript_segments(&[segment("  "), segment("\n")]);
        assert_eq!(empty.text, "");
        assert_eq!(empty.num_segments, 0);
        assert!(!empty.transcription_ok);
    }

    #[test]
    fn install_acceptance_report_serializes_pinned_identity_and_installer_proof() {
        let outcome = WhisperModelInstallOutcome {
            disposition: WhisperModelInstallDisposition::Installed,
            model_id: WHISPER_SMALL_ID.to_string(),
            revision: WHISPER_MODEL_REVISION.to_string(),
            installed_bytes: WHISPER_MODEL_BYTES,
            model_path: Path::new("fixture/ggml-small.bin").to_path_buf(),
        };
        let report = build_install_acceptance_report(
            &outcome,
            "fixed-time".to_string(),
            Some("abc".to_string()),
        );
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["schema_version"], REPORT_SCHEMA_VERSION);
        assert_eq!(json["model_id"], WHISPER_SMALL_ID);
        assert_eq!(json["revision"], WHISPER_MODEL_REVISION);
        assert_eq!(json["source_commit"], WHISPER_SOURCE_COMMIT);
        assert_eq!(json["artifact_filename"], MODEL_FILENAME);
        assert_eq!(json["expected_bytes"], WHISPER_MODEL_BYTES);
        assert_eq!(json["installed_bytes"], WHISPER_MODEL_BYTES);
        assert_eq!(json["disposition"], "installed");
        assert_eq!(json["license"], LICENSE_STATE);
        assert_eq!(json["production_installer_verified"], true);
    }

    #[test]
    fn transcription_report_preserves_metrics_and_nullable_resource_fields() {
        let report = WhisperTranscribeAcceptanceReport {
            schema_version: REPORT_SCHEMA_VERSION,
            phase: "transcribe",
            generated_at_utc: "fixed-time".to_string(),
            git_sha: None,
            model_id: WHISPER_SMALL_ID.to_string(),
            revision: WHISPER_MODEL_REVISION.to_string(),
            source_commit: WHISPER_SOURCE_COMMIT.to_string(),
            artifact_filename: MODEL_FILENAME.to_string(),
            sha256: WHISPER_MODEL_SHA256.to_string(),
            expected_bytes: WHISPER_MODEL_BYTES,
            installed_bytes: WHISPER_MODEL_BYTES,
            corpus_path: "fixture.wav".to_string(),
            corpus_bytes: 48,
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
            sample_count: 8,
            duration_ms: 1,
            host_os: "linux".to_string(),
            host_arch: "x86_64".to_string(),
            host_cpu: None,
            available_parallelism: 1,
            network_denied_required: true,
            network_denial_probe_passed: true,
            status: "pass".to_string(),
            runtime: WhisperTranscribeRuntime {
                whisper_native_linked: true,
            },
            pipeline: WhisperPipelineAcceptanceMetrics {
                partial_interval_samples: 1,
                endpoint_silence_samples: 2,
                maximum_utterance_samples: 3,
                queue_capacity_chunks: 4,
                partial_event_count: 1,
                final_event_count: 1,
                first_partial_latency_ms: Some(5),
                first_final_latency_ms: Some(6),
                processed_audio_ms: 7,
                inference_wall_time_ms: 8,
                real_time_factor: None,
                process_cpu_time_ms: None,
                average_cpu_utilization_percent: None,
                peak_resident_memory_bytes: None,
                nominal_dropped_chunks: 0,
                overload_attempted_chunks: 0,
                overload_accepted_chunks: 0,
                overload_dropped_chunks: 0,
            },
            transcript: WhisperTranscriptReport {
                duration_ms: 1,
                no_speech_prob: 0.0,
                num_segments: 1,
                text: "hello".to_string(),
                transcription_ok: true,
            },
            transcribe_wall_ms: 2,
            phase_wall_ms: 3,
            process_cpu_time_ms: 0,
            process_cpu_usage_percent: None,
            baseline_resident_memory_bytes: None,
            resident_memory_bytes: None,
            peak_resident_memory_bytes: None,
        };
        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["transcript"]["text"], "hello");
        assert_eq!(json["pipeline"]["processed_audio_ms"], 7);
        assert_eq!(
            json["pipeline"]["real_time_factor"],
            serde_json::Value::Null
        );
        assert_eq!(json["host_cpu"], serde_json::Value::Null);
        assert_eq!(json["resident_memory_bytes"], serde_json::Value::Null);
    }

    #[test]
    fn transcription_report_builder_sets_status_from_empty_transcript_and_keeps_wav_metadata() {
        let wav = wav::WavSamples {
            samples: vec![0.0; 16],
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
        };
        let mut transcript = summarize_transcript_segments(&[]);
        transcript.duration_ms = sample_count_to_duration_ms(wav.samples.len(), wav.sample_rate);
        let report = build_transcribe_acceptance_report(
            &wav,
            TranscribeReportContext {
                generated_at_utc: "fixed-time".to_string(),
                git_sha: None,
                installed_bytes: WHISPER_MODEL_BYTES,
                corpus_path: "fixture.wav".to_string(),
                corpus_bytes: 76,
                host_os: "test-os".to_string(),
                host_arch: "test-arch".to_string(),
                host_cpu: None,
                available_parallelism: 1,
                network_denied_required: true,
                network_denial_probe_passed: true,
                pipeline: WhisperPipelineAcceptanceMetrics {
                    partial_interval_samples: 1,
                    endpoint_silence_samples: 2,
                    maximum_utterance_samples: 3,
                    queue_capacity_chunks: 4,
                    partial_event_count: 0,
                    final_event_count: 0,
                    first_partial_latency_ms: None,
                    first_final_latency_ms: None,
                    processed_audio_ms: 1,
                    inference_wall_time_ms: 1,
                    real_time_factor: None,
                    process_cpu_time_ms: None,
                    average_cpu_utilization_percent: None,
                    peak_resident_memory_bytes: None,
                    nominal_dropped_chunks: 0,
                    overload_attempted_chunks: 0,
                    overload_accepted_chunks: 0,
                    overload_dropped_chunks: 0,
                },
                transcript,
                transcribe_wall_ms: 1,
                phase_wall_ms: 2,
                process_cpu_time_ms: 0,
                process_cpu_usage_percent: None,
                baseline_resident_memory_bytes: None,
                resident_memory_bytes: None,
                peak_resident_memory_bytes: None,
            },
        );
        assert_eq!(report.status, "fail");
        assert_eq!(report.sample_count, 16);
        assert_eq!(report.duration_ms, 1);
        assert_eq!(report.corpus_bytes, 76);
        assert!(report.runtime.whisper_native_linked);
        assert_eq!(
            validate_transcription_report(&report).unwrap_err(),
            "Whisper transcription returned empty output"
        );
    }

    #[test]
    fn report_writer_creates_nested_directories_and_sanitizes_write_errors() {
        let directory = tempfile::tempdir().unwrap();
        let nested = directory.path().join("nested/reports/report.json");
        write_report(&nested, &serde_json::json!({"ok": true})).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(nested).unwrap()).unwrap()["ok"],
            true
        );

        let parent_file = directory.path().join("not-a-directory");
        fs::write(&parent_file, "private").unwrap();
        let error = write_report(
            &parent_file.join("report.json"),
            &serde_json::json!({"text": "secret"}),
        )
        .unwrap_err();
        assert_eq!(error, "could not create acceptance report directory");
        assert!(!error.contains("secret"));
    }

    #[test]
    fn report_writer_sanitizes_serialization_errors() {
        struct FailingReport;
        impl Serialize for FailingReport {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom("private transcript sentinel"))
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let error =
            write_report(&directory.path().join("report.json"), &FailingReport).unwrap_err();
        assert_eq!(error, "could not serialize acceptance report");
        assert!(!error.contains("private transcript sentinel"));
    }

    #[test]
    fn caller_rejects_parseable_non_mono_corpus() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&40_u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_000_u32.to_le_bytes());
        wav.extend_from_slice(&64_000_u32.to_le_bytes());
        wav.extend_from_slice(&4_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&4_u32.to_le_bytes());
        wav.extend_from_slice(&[0, 0, 1, 0]);
        let parsed = wav::parse_wav_f32(&wav).unwrap();
        let error = validate_corpus_format(&parsed).unwrap_err();
        assert_eq!(error, "corpus channel count 2 is not mono");
    }
}
