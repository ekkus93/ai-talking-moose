use crate::audio::levels::LevelMeter;
use crate::audio::resample::AudioResampler;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{error, info, warn};

const OVERLOAD_WARNING_INTERVAL: Duration = Duration::from_secs(5);

pub struct SafeStream(pub cpal::Stream);

// SAFETY: CPAL intentionally makes its cross-platform `Stream` wrapper !Send because
// Android's AAudio stream API is not thread-safe. Talking Moose V1 supports macOS and
// Linux desktop builds only; CPAL's CoreAudio/ALSA stream handles may be moved between
// threads, and this wrapper is only accessed through exclusive ownership or a mutex.
#[cfg(any(target_os = "macos", target_os = "linux"))]
unsafe impl Send for SafeStream {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCaptureMode {
    Real,
    Mock,
}

#[derive(Debug, Error)]
pub enum AudioCaptureError {
    #[error("failed to enumerate audio input devices: {0}")]
    DeviceEnumeration(String),
    #[error("no audio input device is available")]
    NoInputDevice,
    #[error("microphone permission was denied or unavailable: {0}")]
    PermissionDenied(String),
    #[error("failed to read input-device configuration: {0}")]
    InputConfiguration(String),
    #[error("unsupported input sample format: {0:?}")]
    UnsupportedSampleFormat(cpal::SampleFormat),
    #[error("failed to build input stream: {0}")]
    BuildStream(String),
    #[error("failed to start input stream: {0}")]
    StartStream(String),
}

fn looks_like_permission_denied(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("permission")
        || lower.contains("not permitted")
        || lower.contains("access denied")
        || lower.contains("not authorized")
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

fn stream_error_handler(
    is_running: Arc<AtomicBool>,
) -> impl FnMut(cpal::StreamError) + Send + 'static {
    move |error_value| {
        is_running.store(false, Ordering::SeqCst);
        error!(error = %error_value, "Microphone capture stream failed");
    }
}

struct CaptureProcessor {
    channels: usize,
    source_sample_rate: u32,
    target_sample_rate: u32,
    chunk_samples: usize,
    pcm_sender: mpsc::Sender<Vec<u8>>,
    level_sender: Option<mpsc::Sender<f32>>,
    is_running: Arc<AtomicBool>,
    dropped_chunks: Arc<AtomicU64>,
    level_meter: LevelMeter,
    accumulated_samples: Vec<f32>,
    last_overload_warning: Instant,
}

impl CaptureProcessor {
    fn new(
        channels: usize,
        source_sample_rate: u32,
        target_sample_rate: u32,
        pcm_sender: mpsc::Sender<Vec<u8>>,
        level_sender: Option<mpsc::Sender<f32>>,
        is_running: Arc<AtomicBool>,
        dropped_chunks: Arc<AtomicU64>,
    ) -> Self {
        let chunk_samples = (target_sample_rate / 10).max(1) as usize;
        Self {
            channels,
            source_sample_rate,
            target_sample_rate,
            chunk_samples,
            pcm_sender,
            level_sender,
            is_running,
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

        let mono = AudioResampler::downmix_to_mono(self.channels, interleaved_samples);
        let (rms, _) = self.level_meter.feed_samples(&mono);
        if let Some(ref sender) = self.level_sender {
            let _ = sender.try_send(rms);
        }

        let resampled = AudioResampler::resample_linear(
            self.source_sample_rate,
            self.target_sample_rate,
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
    dropped_chunks: Arc<AtomicU64>,
    mode: AudioCaptureMode,
    _stream: Option<SafeStream>,
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
            dropped_chunks: Arc::new(AtomicU64::new(0)),
            mode,
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

        if self.mode == AudioCaptureMode::Mock {
            self.is_running.store(true, Ordering::SeqCst);
            info!("Explicit mock microphone capture started");
            return Ok(());
        }

        let host = cpal::default_host();
        let device = if let Some(ref name) = device_name {
            host.input_devices()
                .map_err(|error_value| {
                    AudioCaptureError::DeviceEnumeration(error_value.to_string())
                })?
                .find(|device| {
                    device
                        .name()
                        .map(|candidate| candidate == *name)
                        .unwrap_or(false)
                })
                .or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        }
        .ok_or(AudioCaptureError::NoInputDevice)?;

        let supported_config = device
            .default_input_config()
            .map_err(|error_value| configuration_error(error_value.to_string()))?;
        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();
        let sample_rate = stream_config.sample_rate.0;
        let channels = usize::from(stream_config.channels);

        let stream_result = match sample_format {
            cpal::SampleFormat::F32 => {
                let mut processor = CaptureProcessor::new(
                    channels,
                    sample_rate,
                    target_sample_rate,
                    pcm_sender.clone(),
                    level_sender.clone(),
                    self.is_running.clone(),
                    self.dropped_chunks.clone(),
                );
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        processor.process_f32(data);
                    },
                    stream_error_handler(self.is_running.clone()),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut processor = CaptureProcessor::new(
                    channels,
                    sample_rate,
                    target_sample_rate,
                    pcm_sender.clone(),
                    level_sender.clone(),
                    self.is_running.clone(),
                    self.dropped_chunks.clone(),
                );
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let converted = AudioResampler::i16_to_f32(data);
                        processor.process_f32(&converted);
                    },
                    stream_error_handler(self.is_running.clone()),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut processor = CaptureProcessor::new(
                    channels,
                    sample_rate,
                    target_sample_rate,
                    pcm_sender,
                    level_sender,
                    self.is_running.clone(),
                    self.dropped_chunks.clone(),
                );
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let converted = AudioResampler::u16_to_f32(data);
                        processor.process_f32(&converted);
                    },
                    stream_error_handler(self.is_running.clone()),
                    None,
                )
            }
            unsupported => return Err(AudioCaptureError::UnsupportedSampleFormat(unsupported)),
        };

        let stream = stream_result.map_err(|error_value| build_error(error_value.to_string()))?;
        stream
            .play()
            .map_err(|error_value| start_error(error_value.to_string()))?;
        self._stream = Some(SafeStream(stream));
        self.is_running.store(true, Ordering::SeqCst);
        info!(
            sample_rate,
            ?sample_format,
            "Microphone audio capture started"
        );
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
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
    }

    #[test]
    fn mock_capture_requires_explicit_construction() {
        let mut capture = AudioCapture::new_mock();
        let (tx, _rx) = mpsc::channel(1);
        capture.start(None, 16_000, tx, None).unwrap();
        assert_eq!(capture.mode(), AudioCaptureMode::Mock);
        assert!(capture.is_active());
        capture.stop();
        assert!(!capture.is_active());
    }

    #[test]
    fn permission_detection_is_conservative() {
        assert!(looks_like_permission_denied("microphone permission denied"));
        assert!(looks_like_permission_denied("not authorized to use input"));
        assert!(!looks_like_permission_denied("device disconnected"));
    }

    #[test]
    fn dropped_chunk_counter_starts_at_zero() {
        let capture = AudioCapture::new();
        assert_eq!(capture.dropped_chunks(), 0);
    }
}
