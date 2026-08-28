use crate::audio::levels::LevelMeter;
#[cfg(target_os = "macos")]
use crate::audio::permissions::microphone_permission_state;
#[cfg(any(target_os = "macos", test))]
use crate::audio::permissions::MicrophonePermissionState;
use crate::audio::resample::AudioResampler;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{error, info, warn};

const OVERLOAD_WARNING_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCaptureMode {
    Real,
    Mock,
}

#[derive(Debug, Error)]
pub enum AudioCaptureError {
    #[error("failed to enumerate audio input devices: {0}")]
    DeviceEnumeration(String),
    #[error("requested audio input device was not found: {0}")]
    RequestedDeviceNotFound(String),
    #[error("no audio input device is available")]
    NoInputDevice,
    #[error("microphone permission has not been requested; request access from Settings first")]
    PermissionNotRequested,
    #[error("microphone permission was denied or restricted: {0}")]
    PermissionDenied(String),
    #[error("microphone permission state is unavailable")]
    PermissionUnavailable,
    #[error("failed to read input-device configuration: {0}")]
    InputConfiguration(String),
    #[error("unsupported input sample format: {0:?}")]
    UnsupportedSampleFormat(cpal::SampleFormat),
    #[error("failed to build input stream: {0}")]
    BuildStream(String),
    #[error("failed to start input stream: {0}")]
    StartStream(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioCaptureDiagnostics {
    pub selected_device: Option<String>,
    pub sample_rate_hz: Option<u32>,
    pub sample_format: Option<String>,
    pub channels: Option<u16>,
    pub active: bool,
    pub input_level: f32,
    pub dropped_chunks: u64,
    pub last_error: Option<String>,
}

fn looks_like_permission_denied(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("permission")
        || lower.contains("not permitted")
        || lower.contains("access denied")
        || lower.contains("not authorized")
}

fn require_default_input_device<T>(device: Option<T>) -> Result<T, AudioCaptureError> {
    device.ok_or(AudioCaptureError::NoInputDevice)
}

fn configuration_error(message: String) -> AudioCaptureError {
    if looks_like_permission_denied(&message) {
        AudioCaptureError::PermissionDenied(message)
    } else {
        AudioCaptureError::InputConfiguration(message)
    }
}

fn build_error(message: String) -> AudioCaptureError {
    if looks_like_permission_denied(&message) {
        AudioCaptureError::PermissionDenied(message)
    } else {
        AudioCaptureError::BuildStream(message)
    }
}

fn start_error(message: String) -> AudioCaptureError {
    if looks_like_permission_denied(&message) {
        AudioCaptureError::PermissionDenied(message)
    } else {
        AudioCaptureError::StartStream(message)
    }
}

fn start_capture_stream<F>(
    is_running: &AtomicBool,
    start_stream: F,
) -> Result<(), AudioCaptureError>
where
    F: FnOnce() -> Result<(), String>,
{
    // CPAL may begin delivering input callbacks as soon as `play()` starts. Arm the
    // processor first so a callback that races with `play()` cannot be discarded as
    // though capture were still stopped. A failed start rolls the state back.
    is_running.store(true, Ordering::SeqCst);
    if let Err(message) = start_stream() {
        is_running.store(false, Ordering::SeqCst);
        return Err(start_error(message));
    }
    Ok(())
}

fn mark_runtime_stream_failure(
    message: &str,
    is_running: &AtomicBool,
    last_error: &Mutex<Option<String>>,
) {
    is_running.store(false, Ordering::SeqCst);
    *last_error.lock() = Some(format!("runtime microphone stream error: {message}"));
}

fn stream_error_handler(
    is_running: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<String>>>,
) -> impl FnMut(cpal::StreamError) + Send + 'static {
    move |error_value| {
        mark_runtime_stream_failure(&error_value.to_string(), &is_running, &last_error);
        error!(error = %error_value, "Microphone capture stream failed");
    }
}

#[cfg(any(target_os = "macos", test))]
fn microphone_permission_error(
    permission_state: MicrophonePermissionState,
) -> Option<AudioCaptureError> {
    match permission_state {
        MicrophonePermissionState::Granted => None,
        MicrophonePermissionState::NotRequested => Some(AudioCaptureError::PermissionNotRequested),
        MicrophonePermissionState::Denied => Some(AudioCaptureError::PermissionDenied(
            "grant microphone access in macOS System Settings > Privacy & Security > Microphone"
                .to_string(),
        )),
        MicrophonePermissionState::Unavailable => Some(AudioCaptureError::PermissionUnavailable),
    }
}

#[derive(Debug, Clone, Copy)]
struct CaptureProcessorConfig {
    channels: usize,
    source_sample_rate: u32,
    target_sample_rate: u32,
}

struct CaptureProcessor {
    config: CaptureProcessorConfig,
    chunk_samples: usize,
    pcm_sender: mpsc::Sender<Vec<u8>>,
    level_sender: Option<mpsc::Sender<f32>>,
    is_running: Arc<AtomicBool>,
    input_level: Arc<AtomicU32>,
    dropped_chunks: Arc<AtomicU64>,
    level_meter: LevelMeter,
    accumulated_samples: Vec<f32>,
    last_overload_warning: Instant,
}

impl CaptureProcessor {
    fn new(
        config: CaptureProcessorConfig,
        pcm_sender: mpsc::Sender<Vec<u8>>,
        level_sender: Option<mpsc::Sender<f32>>,
        is_running: Arc<AtomicBool>,
        input_level: Arc<AtomicU32>,
        dropped_chunks: Arc<AtomicU64>,
    ) -> Self {
        let chunk_samples = (config.target_sample_rate / 10).max(1) as usize;
        Self {
            config,
            chunk_samples,
            pcm_sender,
            level_sender,
            is_running,
            input_level,
            dropped_chunks,
            level_meter: LevelMeter::new(),
            accumulated_samples: Vec::with_capacity(chunk_samples * 2),
            last_overload_warning: Instant::now() - OVERLOAD_WARNING_INTERVAL,
        }
    }

    fn process_f32(&mut self, interleaved_samples: &[f32]) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        let mono = AudioResampler::downmix_to_mono(self.config.channels, interleaved_samples);
        let (rms, _) = self.level_meter.feed_samples(&mono);
        self.input_level.store(rms.to_bits(), Ordering::Relaxed);
        if let Some(ref sender) = self.level_sender {
            let _ = sender.try_send(rms);
        }

        let resampled = AudioResampler::resample_linear(
            self.config.source_sample_rate,
            self.config.target_sample_rate,
            &mono,
        );
        self.accumulated_samples.extend(resampled);

        while self.accumulated_samples.len() >= self.chunk_samples {
            let i16_samples =
                AudioResampler::f32_to_i16(&self.accumulated_samples[..self.chunk_samples]);
            let bytes = AudioResampler::i16_to_bytes(&i16_samples);
            self.accumulated_samples.drain(..self.chunk_samples);

            match self.pcm_sender.try_send(bytes) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => {
                    let dropped = self.dropped_chunks.fetch_add(1, Ordering::SeqCst) + 1;
                    if self.last_overload_warning.elapsed() >= OVERLOAD_WARNING_INTERVAL {
                        warn!(
                            dropped_chunks = dropped,
                            "Microphone queue full; dropping newest input chunk"
                        );
                        self.last_overload_warning = Instant::now();
                    }
                }
                Err(TrySendError::Closed(_)) => {
                    self.is_running.store(false, Ordering::SeqCst);
                    return;
                }
            }
        }
    }
}

pub struct AudioCapture {
    is_running: Arc<AtomicBool>,
    input_level: Arc<AtomicU32>,
    dropped_chunks: Arc<AtomicU64>,
    last_error: Arc<Mutex<Option<String>>>,
    mode: AudioCaptureMode,
    selected_device: Option<String>,
    sample_rate_hz: Option<u32>,
    sample_format: Option<String>,
    channels: Option<u16>,
    _stream: Option<cpal::Stream>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self::with_mode(AudioCaptureMode::Real)
    }

    pub fn new_mock() -> Self {
        Self::with_mode(AudioCaptureMode::Mock)
    }

    fn with_mode(mode: AudioCaptureMode) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            input_level: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            dropped_chunks: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            mode,
            selected_device: None,
            sample_rate_hz: None,
            sample_format: None,
            channels: None,
            _stream: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn dropped_chunks(&self) -> u64 {
        self.dropped_chunks.load(Ordering::SeqCst)
    }

    pub fn mode(&self) -> AudioCaptureMode {
        self.mode
    }

    pub fn diagnostics(&self) -> AudioCaptureDiagnostics {
        AudioCaptureDiagnostics {
            selected_device: self.selected_device.clone(),
            sample_rate_hz: self.sample_rate_hz,
            sample_format: self.sample_format.clone(),
            channels: self.channels,
            active: self.is_active(),
            input_level: f32::from_bits(self.input_level.load(Ordering::Relaxed)),
            dropped_chunks: self.dropped_chunks(),
            last_error: self.last_error.lock().clone(),
        }
    }

    /// Start capturing audio. Streams resampled 16-bit mono PCM frames over the
    /// bounded channel supplied by the caller.
    pub fn start(
        &mut self,
        device_name: Option<String>,
        target_sample_rate: u32,
        pcm_sender: mpsc::Sender<Vec<u8>>,
        level_sender: Option<mpsc::Sender<f32>>,
    ) -> Result<(), AudioCaptureError> {
        self.stop();
        self.dropped_chunks.store(0, Ordering::SeqCst);
        self.input_level.store(0.0_f32.to_bits(), Ordering::Relaxed);
        *self.last_error.lock() = None;
        self.selected_device = None;
        self.sample_rate_hz = None;
        self.sample_format = None;
        self.channels = None;

        let result = self.start_inner(device_name, target_sample_rate, pcm_sender, level_sender);
        if let Err(ref error_value) = result {
            *self.last_error.lock() = Some(error_value.to_string());
        }
        result
    }

    fn start_inner(
        &mut self,
        device_name: Option<String>,
        target_sample_rate: u32,
        pcm_sender: mpsc::Sender<Vec<u8>>,
        level_sender: Option<mpsc::Sender<f32>>,
    ) -> Result<(), AudioCaptureError> {
        if self.mode == AudioCaptureMode::Mock {
            self.selected_device = Some("Explicit mock microphone".to_string());
            self.sample_rate_hz = Some(target_sample_rate);
            self.sample_format = Some("I16".to_string());
            self.channels = Some(1);
            self.is_running.store(true, Ordering::SeqCst);
            info!("Explicit mock microphone capture started");
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        if let Some(permission_error) = microphone_permission_error(microphone_permission_state()) {
            return Err(permission_error);
        }

        let host = cpal::default_host();
        let device = if let Some(ref requested_name) = device_name {
            host.input_devices()
                .map_err(|error_value| {
                    AudioCaptureError::DeviceEnumeration(error_value.to_string())
                })?
                .find(|device| {
                    device
                        .name()
                        .map(|candidate| candidate == *requested_name)
                        .unwrap_or(false)
                })
                .ok_or_else(|| AudioCaptureError::RequestedDeviceNotFound(requested_name.clone()))?
        } else {
            require_default_input_device(host.default_input_device())?
        };

        let selected_device = device.name().ok().or(device_name);
        let supported_config = device
            .default_input_config()
            .map_err(|error_value| configuration_error(error_value.to_string()))?;
        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();
        let sample_rate = stream_config.sample_rate;
        let channels = stream_config.channels;
        let processor_config = CaptureProcessorConfig {
            channels: usize::from(channels),
            source_sample_rate: sample_rate,
            target_sample_rate,
        };

        let stream_result = match sample_format {
            cpal::SampleFormat::F32 => {
                let mut processor = CaptureProcessor::new(
                    processor_config,
                    pcm_sender.clone(),
                    level_sender.clone(),
                    self.is_running.clone(),
                    self.input_level.clone(),
                    self.dropped_chunks.clone(),
                );
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        processor.process_f32(data);
                    },
                    stream_error_handler(self.is_running.clone(), self.last_error.clone()),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut processor = CaptureProcessor::new(
                    processor_config,
                    pcm_sender.clone(),
                    level_sender.clone(),
                    self.is_running.clone(),
                    self.input_level.clone(),
                    self.dropped_chunks.clone(),
                );
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let converted = AudioResampler::i16_to_f32(data);
                        processor.process_f32(&converted);
                    },
                    stream_error_handler(self.is_running.clone(), self.last_error.clone()),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut processor = CaptureProcessor::new(
                    processor_config,
                    pcm_sender,
                    level_sender,
                    self.is_running.clone(),
                    self.input_level.clone(),
                    self.dropped_chunks.clone(),
                );
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let converted = AudioResampler::u16_to_f32(data);
                        processor.process_f32(&converted);
                    },
                    stream_error_handler(self.is_running.clone(), self.last_error.clone()),
                    None,
                )
            }
            unsupported => return Err(AudioCaptureError::UnsupportedSampleFormat(unsupported)),
        };

        let stream = stream_result.map_err(|error_value| build_error(error_value.to_string()))?;
        start_capture_stream(&self.is_running, || {
            stream.play().map_err(|error_value| error_value.to_string())
        })?;

        self.selected_device = selected_device;
        self.sample_rate_hz = Some(sample_rate);
        self.sample_format = Some(format!("{sample_format:?}"));
        self.channels = Some(channels);
        self._stream = Some(stream);
        info!(
            sample_rate,
            ?sample_format,
            "Microphone audio capture started"
        );
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.input_level.store(0.0_f32.to_bits(), Ordering::Relaxed);
        self._stream = None;
    }
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_capture_is_real_and_not_running_before_start() {
        let capture = AudioCapture::new();
        assert_eq!(capture.mode(), AudioCaptureMode::Real);
        assert!(!capture.is_active());
        assert!(!capture.diagnostics().active);
    }

    #[test]
    fn mock_capture_requires_explicit_construction() {
        let mut capture = AudioCapture::new_mock();
        let (tx, _rx) = mpsc::channel(1);
        capture.start(None, 16_000, tx, None).unwrap();
        assert_eq!(capture.mode(), AudioCaptureMode::Mock);
        assert!(capture.is_active());

        let diagnostics = capture.diagnostics();
        assert_eq!(
            diagnostics.selected_device.as_deref(),
            Some("Explicit mock microphone")
        );
        assert_eq!(diagnostics.sample_rate_hz, Some(16_000));
        assert_eq!(diagnostics.sample_format.as_deref(), Some("I16"));
        assert_eq!(diagnostics.channels, Some(1));
        assert!(diagnostics.active);

        capture.stop();
        assert!(!capture.is_active());
        assert_eq!(capture.diagnostics().input_level, 0.0);
    }

    #[test]
    fn missing_default_microphone_fails_closed_without_hardware() {
        let result = require_default_input_device::<()>(None);
        assert!(matches!(result, Err(AudioCaptureError::NoInputDevice)));
    }

    #[test]
    fn permission_detection_is_conservative() {
        assert!(looks_like_permission_denied("microphone permission denied"));
        assert!(looks_like_permission_denied("not authorized to use input"));
        assert!(!looks_like_permission_denied("device disconnected"));
    }

    #[test]
    fn microphone_permission_states_fail_closed_without_touching_hardware() {
        assert!(microphone_permission_error(MicrophonePermissionState::Granted).is_none());
        assert!(matches!(
            microphone_permission_error(MicrophonePermissionState::NotRequested),
            Some(AudioCaptureError::PermissionNotRequested)
        ));
        assert!(matches!(
            microphone_permission_error(MicrophonePermissionState::Denied),
            Some(AudioCaptureError::PermissionDenied(_))
        ));
        assert!(matches!(
            microphone_permission_error(MicrophonePermissionState::Unavailable),
            Some(AudioCaptureError::PermissionUnavailable)
        ));
    }

    #[test]
    fn runtime_input_failure_stops_capture_and_records_diagnostics() {
        let mut capture = AudioCapture::new_mock();
        let (tx, _rx) = mpsc::channel(1);
        capture.start(None, 16_000, tx, None).unwrap();
        assert!(capture.is_active());

        mark_runtime_stream_failure(
            "device disconnected",
            &capture.is_running,
            &capture.last_error,
        );

        let diagnostics = capture.diagnostics();
        assert!(!diagnostics.active);
        assert_eq!(
            diagnostics.last_error.as_deref(),
            Some("runtime microphone stream error: device disconnected")
        );
    }

    #[test]
    fn capture_is_armed_before_stream_start_can_deliver_audio() {
        let is_running = Arc::new(AtomicBool::new(false));
        let input_level = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
        let dropped_chunks = Arc::new(AtomicU64::new(0));
        let (pcm_sender, mut pcm_receiver) = mpsc::channel(1);
        let mut processor = CaptureProcessor::new(
            CaptureProcessorConfig {
                channels: 1,
                source_sample_rate: 100,
                target_sample_rate: 100,
            },
            pcm_sender,
            None,
            is_running.clone(),
            input_level,
            dropped_chunks,
        );

        start_capture_stream(&is_running, || {
            // Model the strongest startup race: CPAL delivers the first callback
            // synchronously from inside `play()` before it returns to start_inner.
            processor.process_f32(&[0.25; 10]);
            Ok(())
        })
        .unwrap();

        assert!(is_running.load(Ordering::SeqCst));
        let first_chunk = pcm_receiver
            .try_recv()
            .expect("the first startup callback must not be discarded");
        assert!(!first_chunk.is_empty());
    }

    #[test]
    fn failed_stream_start_rolls_back_armed_capture_state() {
        let is_running = AtomicBool::new(false);
        let error_value = start_capture_stream(&is_running, || Err("start failed".to_string()))
            .expect_err("start failure must be reported");

        assert!(matches!(error_value, AudioCaptureError::StartStream(_)));
        assert!(!is_running.load(Ordering::SeqCst));
    }

    #[test]
    fn microphone_queue_overload_drops_newest_and_counts_drop() {
        let is_running = Arc::new(AtomicBool::new(true));
        let input_level = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
        let dropped_chunks = Arc::new(AtomicU64::new(0));
        let (pcm_sender, mut pcm_receiver) = mpsc::channel(1);
        let mut processor = CaptureProcessor::new(
            CaptureProcessorConfig {
                channels: 1,
                source_sample_rate: 100,
                target_sample_rate: 100,
            },
            pcm_sender,
            None,
            is_running,
            input_level,
            dropped_chunks.clone(),
        );

        processor.process_f32(&[0.25; 10]);
        processor.process_f32(&[0.5; 10]);

        assert_eq!(dropped_chunks.load(Ordering::SeqCst), 1);
        assert!(pcm_receiver.try_recv().is_ok());
        assert!(matches!(
            pcm_receiver.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn dropped_chunk_counter_starts_at_zero() {
        let capture = AudioCapture::new();
        assert_eq!(capture.dropped_chunks(), 0);
    }
}
