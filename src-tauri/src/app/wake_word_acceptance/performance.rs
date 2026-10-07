use super::super::state::AppSettings;
use super::super::wake_word::engine::{NativeKwsSession, NativeKwsSessionPaths, WakeWordError};
use super::super::wake_word_command_activation::{
    activate_wake_command_and_measure_start_normal_asr_once, WakeCommandActivationTiming,
    WakeCommandStarter,
};
use super::super::wake_word_command_asr_ingress::{WakeCommandAsrHandoff, WakeCommandAsrIngress};
use super::super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::super::wake_word_composition::WakeWordApplicationRuntime;
use super::super::wake_word_local_listener_thread::{
    spawn_wake_local_listener_thread, WakeLocalListenerEvent, WakeLocalListenerHandle,
};
use super::*;
use crate::audio::capture::AudioCapture;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const PRODUCTION_LISTENER_IDLE_FRAMES: usize = 20;
const PRODUCTION_LISTENER_REPEATED_WAKE_CYCLES: u32 = 3;
const PRODUCTION_LISTENER_REPEATED_ENABLE_CYCLES: u32 = 3;

type ProductionListenerSpawn = (
    WakeLocalListenerHandle,
    mpsc::UnboundedReceiver<WakeLocalListenerEvent>,
    mpsc::Sender<Vec<u8>>,
    u64,
);

#[derive(Debug, Serialize)]
pub struct ProductionListenerPerformanceReport {
    pub schema_version: u32,
    pub measurement_path: &'static str,
    pub capture_transport: &'static str,
    pub platform: String,
    pub architecture: String,
    pub startup_duration_ms: u64,
    pub idle_cpu_percent: f64,
    pub idle_observation_ms: u64,
    pub peak_resident_memory_before_bytes: Option<u64>,
    pub peak_resident_memory_after_start_bytes: Option<u64>,
    pub startup_peak_memory_delta_bytes: Option<u64>,
    pub mean_inference_latency_ms: f64,
    pub p95_inference_latency_ms: f64,
    pub inference_frames: usize,
    pub wake_to_command_asr_ms: u64,
    pub pre_roll_startup_ms: u64,
    pub command_start_ms: u64,
    pub total_activation_ms: u64,
    pub handoff_samples: usize,
    pub repeated_wake_command_resume_cycles: u32,
    pub repeated_enable_disable_cycles: u32,
    pub peak_active_native_sessions: usize,
    pub final_active_native_sessions: usize,
    pub final_capture_active: bool,
    pub final_runtime_phase: String,
    pub ring_buffer_delta_samples: i64,
    pub handoff_pre_roll_delta_samples: i64,
    pub inference_threads: u16,
    pub passed: bool,
}

#[derive(Default)]
struct ProductionListenerMeasurements {
    inference_durations_us: Vec<u64>,
    active_native_sessions: usize,
    peak_active_native_sessions: usize,
}

struct MeasuredNativeKwsSession {
    inner: NativeKwsSession,
    measurements: Arc<Mutex<ProductionListenerMeasurements>>,
}

impl MeasuredNativeKwsSession {
    fn new(
        paths: NativeKwsSessionPaths,
        measurements: Arc<Mutex<ProductionListenerMeasurements>>,
    ) -> Result<Self, WakeWordError> {
        let inner = NativeKwsSession::new(paths)?;
        {
            let mut state = measurements.lock();
            state.active_native_sessions += 1;
            state.peak_active_native_sessions = state
                .peak_active_native_sessions
                .max(state.active_native_sessions);
        }
        Ok(Self {
            inner,
            measurements,
        })
    }
}

impl Drop for MeasuredNativeKwsSession {
    fn drop(&mut self) {
        let mut state = self.measurements.lock();
        state.active_native_sessions = state.active_native_sessions.saturating_sub(1);
    }
}

impl SherpaKwsEngine for MeasuredNativeKwsSession {
    fn config(&self) -> &SherpaKwsConfig {
        self.inner.config()
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        let started = Instant::now();
        let result = self.inner.accept_pcm16_mono(sample_rate_hz, samples);
        let elapsed_us = started.elapsed().as_micros().try_into().unwrap_or(u64::MAX);
        self.measurements
            .lock()
            .inference_durations_us
            .push(elapsed_us);
        result
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        self.inner.reset_stream()
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        self.inner.shutdown()
    }
}

#[derive(Default)]
struct PerformanceIngress {
    handoff_samples: usize,
}

impl WakeCommandAsrIngress for PerformanceIngress {
    fn accept_wake_handoff(&mut self, audio: WakeCommandHandoffAudio) -> Result<(), String> {
        self.handoff_samples = audio.samples_i16().len();
        Ok(())
    }
}

#[derive(Default)]
struct PerformanceStarter {
    starts: u32,
}

#[async_trait]
impl WakeCommandStarter for PerformanceStarter {
    async fn start_normal_command_interaction(&mut self) -> Result<(), String> {
        self.starts += 1;
        Ok(())
    }
}

fn listener_event_with_timeout(
    receiver: &mut mpsc::UnboundedReceiver<WakeLocalListenerEvent>,
    timeout: Duration,
) -> Result<WakeLocalListenerEvent, String> {
    let deadline = Instant::now() + timeout;
    loop {
        match receiver.try_recv() {
            Ok(event) => return Ok(event),
            Err(mpsc::error::TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    return Err("timed out waiting for Wake listener event".to_string());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                return Err("Wake listener event channel disconnected".to_string());
            }
        }
    }
}

fn spawn_measured_production_listener(
    paths: &NativeKwsSessionPaths,
    capture: Arc<Mutex<AudioCapture>>,
    runtime: WakeWordApplicationRuntime,
    measurements: Arc<Mutex<ProductionListenerMeasurements>>,
) -> Result<ProductionListenerSpawn, String> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let session_paths = paths.clone();
    let measurement_state = measurements.clone();
    let started = Instant::now();
    let handle = spawn_wake_local_listener_thread::<MeasuredNativeKwsSession, _>(
        capture.clone(),
        runtime,
        None,
        move |wake_runtime| {
            let engine = MeasuredNativeKwsSession::new(session_paths, measurement_state)
                .map_err(|error| error.message)?;
            Ok(wake_runtime.capture_consumer(engine))
        },
        event_tx,
    )
    .map_err(|error| error.to_string())?;

    match listener_event_with_timeout(&mut event_rx, Duration::from_secs(30))? {
        WakeLocalListenerEvent::Started => {}
        WakeLocalListenerEvent::StartupFailed(error)
        | WakeLocalListenerEvent::CaptureFailed(error) => return Err(error),
        WakeLocalListenerEvent::Triggered(_) => {
            return Err("Wake listener triggered before startup completed".to_string())
        }
        WakeLocalListenerEvent::Stopped => {
            return Err("Wake listener stopped before startup completed".to_string())
        }
    }

    let startup_duration_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    let sender = capture
        .lock()
        .mock_pcm_sender()
        .ok_or_else(|| "mock Wake capture sender is unavailable after startup".to_string())?;
    Ok((handle, event_rx, sender, startup_duration_ms))
}

fn pcm16_bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

fn send_pcm_frame(sender: &mpsc::Sender<Vec<u8>>, samples: &[i16]) -> Result<(), String> {
    sender
        .blocking_send(pcm16_bytes(samples))
        .map_err(|_| "Wake listener PCM queue closed".to_string())
}

fn triggered_handoff_from_event(
    event: WakeLocalListenerEvent,
) -> Result<WakeCommandHandoffAudio, String> {
    match event {
        WakeLocalListenerEvent::Triggered(audio) => Ok(audio),
        WakeLocalListenerEvent::StartupFailed(error)
        | WakeLocalListenerEvent::CaptureFailed(error) => Err(error),
        WakeLocalListenerEvent::Stopped => Err("Wake listener stopped before trigger".to_string()),
        WakeLocalListenerEvent::Started => {
            Err("unexpected duplicate Wake listener start event".to_string())
        }
    }
}

fn feed_until_trigger(
    sender: &mpsc::Sender<Vec<u8>>,
    receiver: &mut mpsc::UnboundedReceiver<WakeLocalListenerEvent>,
    samples: &[i16],
    frame_pause: Duration,
) -> Result<WakeCommandHandoffAudio, String> {
    for frame in samples.chunks(FRAME_SAMPLES) {
        if let Ok(event) = receiver.try_recv() {
            return triggered_handoff_from_event(event);
        }
        if send_pcm_frame(sender, frame).is_err() {
            break;
        }
        std::thread::sleep(frame_pause);
        if let Ok(event) = receiver.try_recv() {
            return triggered_handoff_from_event(event);
        }
    }
    triggered_handoff_from_event(listener_event_with_timeout(
        receiver,
        Duration::from_secs(10),
    )?)
}

fn fixture_samples(fixture: &GeneratedFixture, corpus_dir: &Path) -> Result<Vec<i16>, String> {
    let fixture_path = safe_fixture_path(corpus_dir, &fixture.path)?;
    let bytes = fs::read(&fixture_path)
        .map_err(|_| format!("failed to read generated fixture {}", fixture.id))?;
    if bytes.len() as u64 != fixture.bytes || sha256_hex(&bytes) != fixture.sha256 {
        return Err(format!(
            "generated fixture {} identity mismatch",
            fixture.id
        ));
    }
    if bytes.len() % 2 != 0 {
        return Err(format!(
            "generated fixture {} is not PCM16 aligned",
            fixture.id
        ));
    }
    let (sample_bytes, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(format!(
            "generated fixture {} is not PCM16 aligned",
            fixture.id
        ));
    }
    Ok(sample_bytes
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect())
}

fn activate_performance_handoff(
    executor: &tokio::runtime::Runtime,
    runtime: &WakeWordApplicationRuntime,
    audio: WakeCommandHandoffAudio,
) -> Result<(WakeCommandActivationTiming, usize), String> {
    let mut handoff = WakeCommandAsrHandoff::new(audio);
    let mut ingress = PerformanceIngress::default();
    let mut starter = PerformanceStarter::default();
    let (delivered, timing) =
        executor.block_on(activate_wake_command_and_measure_start_normal_asr_once(
            runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        ))?;
    if !delivered || starter.starts != 1 || ingress.handoff_samples == 0 {
        return Err(
            "production listener handoff did not activate command ASR exactly once".to_string(),
        );
    }
    Ok((timing, ingress.handoff_samples))
}

fn percentile_95_ms(durations_us: &[u64]) -> f64 {
    if durations_us.is_empty() {
        return 0.0;
    }
    let mut values = durations_us.to_vec();
    values.sort_unstable();
    let index = ((values.len() * 95).saturating_sub(1)) / 100;
    values[index.min(values.len() - 1)] as f64 / 1_000.0
}

pub fn run_production_listener_performance_acceptance(
    model_dir: &Path,
    runtime_dir: &Path,
    corpus_dir: &Path,
    index_path: &Path,
) -> Result<ProductionListenerPerformanceReport, String> {
    let index: GeneratedCorpusIndex = serde_json::from_slice(
        &fs::read(index_path).map_err(|_| "failed to read generated corpus index".to_string())?,
    )
    .map_err(|_| "generated corpus index is invalid".to_string())?;
    let positive = index
        .fixtures
        .iter()
        .find(|fixture| fixture.expected_detection)
        .ok_or_else(|| {
            "production listener acceptance requires a positive Wake fixture".to_string()
        })?;
    let positive_samples = fixture_samples(positive, corpus_dir)?;
    let paths = NativeKwsSessionPaths {
        model_dir: model_dir.to_path_buf(),
        runtime_dir: runtime_dir.to_path_buf(),
    };
    let settings = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };
    let runtime =
        WakeWordApplicationRuntime::from_settings(&settings).map_err(|error| error.to_string())?;
    let capture = Arc::new(Mutex::new(AudioCapture::new_mock()));
    let measurements = Arc::new(Mutex::new(ProductionListenerMeasurements::default()));
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let initial_snapshot = runtime.snapshot(Instant::now());
    let memory_before = peak_resident_memory_bytes();

    let (first_handle, mut first_events, first_sender, startup_duration_ms) =
        spawn_measured_production_listener(
            &paths,
            capture.clone(),
            runtime.clone(),
            measurements.clone(),
        )?;
    let memory_after_start = peak_resident_memory_bytes();

    let idle_cpu_before = process_cpu_time_ms();
    let idle_started = Instant::now();
    let silence = vec![0_i16; FRAME_SAMPLES];
    for _ in 0..PRODUCTION_LISTENER_IDLE_FRAMES {
        send_pcm_frame(&first_sender, &silence)?;
        std::thread::sleep(Duration::from_millis(100));
    }
    let idle_observation_ms: u64 = idle_started
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let idle_cpu_ms = process_cpu_time_ms().saturating_sub(idle_cpu_before);
    let idle_cpu_percent = if idle_observation_ms == 0 {
        0.0
    } else {
        idle_cpu_ms as f64 * 100.0 / idle_observation_ms as f64
    };

    let first_handoff = feed_until_trigger(
        &first_sender,
        &mut first_events,
        &positive_samples,
        Duration::from_millis(100),
    )?;
    first_handle
        .shutdown()
        .map_err(|_| "failed to join production Wake listener thread".to_string())?;
    let (activation_timing, handoff_samples) =
        activate_performance_handoff(&executor, &runtime, first_handoff)?;
    runtime
        .resume_after_interaction(true)
        .map_err(|error| error.to_string())?;

    for _ in 1..PRODUCTION_LISTENER_REPEATED_WAKE_CYCLES {
        let (handle, mut events, sender, _) = spawn_measured_production_listener(
            &paths,
            capture.clone(),
            runtime.clone(),
            measurements.clone(),
        )?;
        let handoff = feed_until_trigger(
            &sender,
            &mut events,
            &positive_samples,
            Duration::from_millis(10),
        )?;
        handle
            .shutdown()
            .map_err(|_| "failed to join repeated Wake listener thread".to_string())?;
        let _ = activate_performance_handoff(&executor, &runtime, handoff)?;
        runtime
            .resume_after_interaction(true)
            .map_err(|error| error.to_string())?;
    }

    runtime
        .apply_enabled_setting(false)
        .map_err(|error| error.to_string())?;
    for _ in 0..PRODUCTION_LISTENER_REPEATED_ENABLE_CYCLES {
        runtime
            .apply_enabled_setting(true)
            .map_err(|error| error.to_string())?;
        let (handle, _events, _sender, _) = spawn_measured_production_listener(
            &paths,
            capture.clone(),
            runtime.clone(),
            measurements.clone(),
        )?;
        handle
            .shutdown()
            .map_err(|_| "failed to join enable/disable Wake listener thread".to_string())?;
        if capture.lock().is_active() {
            return Err("Wake capture remained active after listener shutdown".to_string());
        }
    }

    runtime
        .apply_enabled_setting(false)
        .map_err(|error| error.to_string())?;
    capture.lock().stop();
    let final_snapshot = runtime.snapshot(Instant::now());
    let measurement_state = measurements.lock();
    let inference_frames = measurement_state.inference_durations_us.len();
    let mean_inference_latency_ms = if inference_frames == 0 {
        0.0
    } else {
        measurement_state
            .inference_durations_us
            .iter()
            .copied()
            .sum::<u64>() as f64
            / inference_frames as f64
            / 1_000.0
    };
    let p95_inference_latency_ms = percentile_95_ms(&measurement_state.inference_durations_us);
    let final_capture_active = capture.lock().is_active();
    let startup_peak_memory_delta_bytes = match (memory_before, memory_after_start) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    let ring_buffer_delta_samples =
        final_snapshot.ring_buffer_samples as i64 - initial_snapshot.ring_buffer_samples as i64;
    let handoff_pre_roll_delta_samples = final_snapshot.handoff_pre_roll_samples as i64
        - initial_snapshot.handoff_pre_roll_samples as i64;
    let passed = startup_duration_ms > 0
        && idle_observation_ms >= 1_000
        && inference_frames > 0
        && handoff_samples > 0
        && measurement_state.peak_active_native_sessions == 1
        && measurement_state.active_native_sessions == 0
        && !final_capture_active
        && final_snapshot.phase == super::super::wake_word::runtime::WakeWordRuntimePhase::Disabled
        && ring_buffer_delta_samples == 0
        && handoff_pre_roll_delta_samples == 0;

    Ok(ProductionListenerPerformanceReport {
        schema_version: 1,
        measurement_path: "production_wake_listener_thread",
        capture_transport: "explicit_mock_pcm_injection",
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        startup_duration_ms,
        idle_cpu_percent,
        idle_observation_ms,
        peak_resident_memory_before_bytes: memory_before,
        peak_resident_memory_after_start_bytes: memory_after_start,
        startup_peak_memory_delta_bytes,
        mean_inference_latency_ms,
        p95_inference_latency_ms,
        inference_frames,
        wake_to_command_asr_ms: activation_timing.wake_to_command_asr_ms,
        pre_roll_startup_ms: activation_timing.pre_roll_startup_ms,
        command_start_ms: activation_timing.command_start_ms,
        total_activation_ms: activation_timing.total_activation_ms,
        handoff_samples,
        repeated_wake_command_resume_cycles: PRODUCTION_LISTENER_REPEATED_WAKE_CYCLES,
        repeated_enable_disable_cycles: PRODUCTION_LISTENER_REPEATED_ENABLE_CYCLES,
        peak_active_native_sessions: measurement_state.peak_active_native_sessions,
        final_active_native_sessions: measurement_state.active_native_sessions,
        final_capture_active,
        final_runtime_phase: format!("{:?}", final_snapshot.phase),
        ring_buffer_delta_samples,
        handoff_pre_roll_delta_samples,
        inference_threads: 1,
        passed,
    })
}

pub(super) fn process_cpu_time_ms() -> u64 {
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

pub(super) fn peak_resident_memory_bytes() -> Option<u64> {
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
