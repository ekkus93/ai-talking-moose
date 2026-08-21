use crate::asr::moonshine::MoonshineTinyTranscriptUpdate;
use crate::asr::types::LocalAsrRuntimeDiagnostics;
use crate::asr::{AsrError, AsrEvent};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct RuntimeMetrics {
    started_at: Instant,
    process_cpu_baseline_micros: Option<u64>,
    baseline_resident_memory_bytes: Option<u64>,
    peak_resident_memory_bytes: Option<u64>,
    first_audio_at: Option<Instant>,
    first_partial_latency_ms: Option<u64>,
    first_final_latency_ms: Option<u64>,
    last_transcription_latency_ms: Option<u32>,
    processed_samples: u64,
    inference_wall_time_micros: u64,
    last_error: Option<AsrError>,
}

impl RuntimeMetrics {
    pub(crate) fn new() -> Self {
        let resident = current_resident_memory_bytes();
        Self {
            started_at: Instant::now(),
            process_cpu_baseline_micros: process_cpu_time_micros(),
            baseline_resident_memory_bytes: resident,
            peak_resident_memory_bytes: resident,
            first_audio_at: None,
            first_partial_latency_ms: None,
            first_final_latency_ms: None,
            last_transcription_latency_ms: None,
            processed_samples: 0,
            inference_wall_time_micros: 0,
            last_error: None,
        }
    }

    pub(crate) fn mark_engine_ready(&mut self) {
        self.started_at = Instant::now();
        self.process_cpu_baseline_micros = process_cpu_time_micros();
        self.record_resident_sample();
    }

    pub(crate) fn record_audio_start_if_needed(&mut self) {
        self.first_audio_at.get_or_insert_with(Instant::now);
    }

    pub(crate) fn record_inference(&mut self, samples: usize, elapsed: Duration) {
        self.processed_samples = self
            .processed_samples
            .saturating_add(u64::try_from(samples).unwrap_or(u64::MAX));
        self.inference_wall_time_micros = self
            .inference_wall_time_micros
            .saturating_add(duration_micros_u64(elapsed));
        self.record_resident_sample();
    }

    pub(crate) fn record_transcript_events(
        &mut self,
        native_update: &MoonshineTinyTranscriptUpdate,
        emitted_events: &[AsrEvent],
    ) {
        let (native_latency_ms, emitted_useful_transcript) = match native_update {
            MoonshineTinyTranscriptUpdate::Partial { latency_ms, .. } => (
                *latency_ms,
                emitted_events
                    .iter()
                    .any(|event| matches!(event, AsrEvent::PartialTranscript { .. })),
            ),
            MoonshineTinyTranscriptUpdate::Final { latency_ms, .. } => (
                *latency_ms,
                emitted_events
                    .iter()
                    .any(|event| matches!(event, AsrEvent::FinalTranscript { .. })),
            ),
        };
        self.last_transcription_latency_ms = Some(native_latency_ms);

        if !emitted_useful_transcript {
            return;
        }
        let Some(first_audio_at) = self.first_audio_at else {
            return;
        };
        let elapsed_ms = duration_millis_u64(first_audio_at.elapsed());
        match native_update {
            MoonshineTinyTranscriptUpdate::Partial { .. } => {
                self.first_partial_latency_ms.get_or_insert(elapsed_ms);
            }
            MoonshineTinyTranscriptUpdate::Final { .. } => {
                self.first_final_latency_ms.get_or_insert(elapsed_ms);
            }
        }
    }

    pub(crate) fn record_error(&mut self, error: &AsrError) {
        self.last_error = Some(error.clone());
    }

    pub(crate) fn runtime_diagnostics(
        &self,
        input_sample_rate_hz: u32,
        queue_depth: usize,
        queue_capacity: usize,
        streaming: bool,
    ) -> LocalAsrRuntimeDiagnostics {
        let processed_audio_ms = samples_to_millis(self.processed_samples, input_sample_rate_hz);
        let inference_wall_time_ms = self.inference_wall_time_micros / 1_000;
        let real_time_factor = if self.processed_samples == 0 || input_sample_rate_hz == 0 {
            None
        } else {
            let audio_seconds = self.processed_samples as f64 / f64::from(input_sample_rate_hz);
            Some((self.inference_wall_time_micros as f64 / 1_000_000.0 / audio_seconds) as f32)
        };

        let process_cpu_delta_micros = process_cpu_time_micros()
            .zip(self.process_cpu_baseline_micros)
            .map(|(current, baseline)| current.saturating_sub(baseline));
        let process_cpu_time_ms = process_cpu_delta_micros.map(|micros| micros / 1_000);
        let average_cpu_utilization_percent = process_cpu_delta_micros.and_then(|cpu_micros| {
            let elapsed = duration_micros_u64(self.started_at.elapsed());
            if elapsed > 0 {
                Some((cpu_micros as f64 / elapsed as f64 * 100.0) as f32)
            } else {
                None
            }
        });
        let resident_memory_bytes = current_resident_memory_bytes();
        let peak_resident_memory_bytes =
            match (self.peak_resident_memory_bytes, resident_memory_bytes) {
                (Some(peak), Some(current)) => Some(peak.max(current)),
                (peak, current) => peak.or(current),
            };

        LocalAsrRuntimeDiagnostics {
            input_sample_rate_hz,
            streaming,
            metrics_snapshot: false,
            queue_depth,
            queue_capacity,
            last_error: self.last_error.clone(),
            first_partial_latency_ms: self.first_partial_latency_ms,
            first_final_latency_ms: self.first_final_latency_ms,
            last_transcription_latency_ms: self.last_transcription_latency_ms,
            processed_audio_ms,
            inference_wall_time_ms,
            real_time_factor,
            process_cpu_time_ms,
            average_cpu_utilization_percent,
            baseline_resident_memory_bytes: self.baseline_resident_memory_bytes,
            resident_memory_bytes,
            peak_resident_memory_bytes,
        }
    }

    fn record_resident_sample(&mut self) {
        let Some(resident) = current_resident_memory_bytes() else {
            return;
        };
        self.peak_resident_memory_bytes = Some(
            self.peak_resident_memory_bytes
                .map_or(resident, |peak| peak.max(resident)),
        );
    }
}

fn duration_micros_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn samples_to_millis(samples: u64, sample_rate_hz: u32) -> u64 {
    if sample_rate_hz == 0 {
        return 0;
    }
    samples
        .saturating_mul(1_000)
        .checked_div(u64::from(sample_rate_hz))
        .unwrap_or(0)
}

#[cfg(unix)]
fn process_cpu_time_micros() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `usage` points to writable storage for exactly one `libc::rusage`.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: a successful `getrusage` initialized the entire structure.
    let usage = unsafe { usage.assume_init() };
    timeval_micros(usage.ru_utime)?.checked_add(timeval_micros(usage.ru_stime)?)
}

#[cfg(unix)]
fn timeval_micros(value: libc::timeval) -> Option<u64> {
    let seconds = u64::try_from(value.tv_sec).ok()?;
    let micros = u64::try_from(value.tv_usec).ok()?;
    seconds.checked_mul(1_000_000)?.checked_add(micros)
}

#[cfg(not(unix))]
fn process_cpu_time_micros() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn current_resident_memory_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    // SAFETY: `sysconf` has no pointer arguments and `_SC_PAGESIZE` is a valid query.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page_size = u64::try_from(page_size).ok()?;
    resident_pages.checked_mul(page_size)
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct MachTimeValue {
    seconds: i32,
    microseconds: i32,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct MachTaskBasicInfo {
    virtual_size: u64,
    resident_size: u64,
    resident_size_max: u64,
    user_time: MachTimeValue,
    system_time: MachTimeValue,
    policy: i32,
    suspend_count: i32,
}

#[cfg(target_os = "macos")]
const MACH_TASK_BASIC_INFO: i32 = 20;

#[cfg(target_os = "macos")]
extern "C" {
    static mach_task_self_: u32;
    fn task_info(
        target_task: u32,
        flavor: i32,
        task_info_out: *mut i32,
        task_info_out_count: *mut u32,
    ) -> i32;
}

#[cfg(target_os = "macos")]
fn current_resident_memory_bytes() -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<MachTaskBasicInfo>::zeroed();
    let mut count =
        u32::try_from(std::mem::size_of::<MachTaskBasicInfo>() / std::mem::size_of::<u32>())
            .ok()?;
    // SAFETY: `mach_task_self_` is the process task port exported by libSystem;
    // `info` and `count` provide writable buffers of the size required by
    // MACH_TASK_BASIC_INFO. No pointer escapes this call.
    let result = unsafe {
        task_info(
            mach_task_self_,
            MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast::<i32>(),
            &mut count,
        )
    };
    if result != 0 {
        return None;
    }
    // SAFETY: successful `task_info` initialized the requested structure.
    Some(unsafe { info.assume_init() }.resident_size)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn current_resident_memory_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_to_millis_handles_zero_rate_and_whole_seconds() {
        assert_eq!(samples_to_millis(16_000, 16_000), 1_000);
        assert_eq!(samples_to_millis(1, 0), 0);
    }

    #[test]
    fn blank_transcript_event_does_not_set_useful_latency() {
        let mut metrics = RuntimeMetrics::new();
        metrics.record_audio_start_if_needed();
        metrics.record_transcript_events(
            &MoonshineTinyTranscriptUpdate::Partial {
                line_id: 1,
                text: " ".to_string(),
                latency_ms: 7,
            },
            &[],
        );
        let diagnostics = metrics.runtime_diagnostics(16_000, 0, 8, true);
        assert_eq!(diagnostics.first_partial_latency_ms, None);
        assert_eq!(diagnostics.last_transcription_latency_ms, Some(7));
    }
}
