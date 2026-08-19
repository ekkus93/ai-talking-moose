use crate::audio::levels::LevelMeter;
use crate::audio::resample::AudioResampler;
use crate::character::state::MouthShape;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub const MAX_QUEUED_PLAYBACK_SECONDS: usize = 10;

pub struct SafeStream(pub cpal::Stream);

// SAFETY: CPAL intentionally makes its cross-platform `Stream` wrapper !Send because
// Android's AAudio stream API is not thread-safe. Talking Moose V1 supports macOS and
// Linux desktop builds only; CPAL's CoreAudio/ALSA stream handles may be moved between
// threads, and this wrapper is only accessed through exclusive ownership or a mutex.
#[cfg(any(target_os = "macos", target_os = "linux"))]
unsafe impl Send for SafeStream {}

#[derive(Debug, Error)]
pub enum AudioPlaybackError {
    #[error("no audio output device is available")]
    NoOutputDevice,
    #[error("failed to enumerate audio output devices: {0}")]
    DeviceEnumeration(String),
    #[error("requested audio output device was not found: {0}")]
    RequestedDeviceNotFound(String),
    #[error("failed to read output-device configuration: {0}")]
    OutputConfiguration(String),
    #[error("unsupported output sample format: {0:?}")]
    UnsupportedSampleFormat(cpal::SampleFormat),
    #[error("failed to build output stream: {0}")]
    BuildStream(String),
    #[error("failed to start output stream: {0}")]
    StartStream(String),
    #[error("audio playback is not initialized")]
    NotStarted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackEnqueueReport {
    pub queued_samples: usize,
    pub dropped_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioPlaybackDiagnostics {
    pub selected_device: Option<String>,
    pub sample_rate_hz: Option<u32>,
    pub sample_format: Option<String>,
    pub channels: Option<u16>,
    pub playing: bool,
    pub output_level: f32,
    pub queue_depth_samples: usize,
    pub queue_limit_samples: usize,
    pub dropped_samples: u64,
    pub last_error: Option<String>,
}

#[derive(Clone)]
struct PlaybackCallbackState {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    is_playing: Arc<AtomicBool>,
    output_level: Arc<AtomicU32>,
    level_meter: Arc<Mutex<LevelMeter>>,
    mouth_sender: Arc<Mutex<Option<mpsc::Sender<MouthShape>>>>,
    output_level_sender: Arc<Mutex<Option<mpsc::Sender<f32>>>>,
    channels: usize,
}

impl PlaybackCallbackState {
    fn take_mono_frames(&self, frame_count: usize) -> Vec<f32> {
        let mut buffer = self.buffer.lock();
        let mut mono = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            mono.push(buffer.pop_front().unwrap_or(0.0));
        }
        let has_non_silent_sample = mono.iter().any(|sample| sample.abs() > 0.001);
        self.is_playing.store(
            has_non_silent_sample || !buffer.is_empty(),
            Ordering::SeqCst,
        );
        mono
    }

    fn publish_level_and_mouth(&self, mono: &[f32]) {
        let (rms, mouth) = self.level_meter.lock().feed_samples(mono);
        self.output_level.store(rms.to_bits(), Ordering::Relaxed);
        if let Some(ref sender) = *self.mouth_sender.lock() {
            let _ = sender.try_send(mouth);
        }
        if let Some(ref sender) = *self.output_level_sender.lock() {
            let _ = sender.try_send(rms);
        }
    }

    fn render_f32(&self, data: &mut [f32]) {
        let mono = self.take_mono_frames(data.len() / self.channels);
        for (frame, sample) in data.chunks_mut(self.channels).zip(mono.iter().copied()) {
            frame.fill(sample);
        }
        self.publish_level_and_mouth(&mono);
    }

    fn render_i16(&self, data: &mut [i16]) {
        let mono = self.take_mono_frames(data.len() / self.channels);
        for (frame, sample) in data.chunks_mut(self.channels).zip(mono.iter().copied()) {
            frame.fill(f32_to_i16_sample(sample));
        }
        self.publish_level_and_mouth(&mono);
    }

    fn render_u16(&self, data: &mut [u16]) {
        let mono = self.take_mono_frames(data.len() / self.channels);
        for (frame, sample) in data.chunks_mut(self.channels).zip(mono.iter().copied()) {
            frame.fill(f32_to_u16_sample(sample));
        }
        self.publish_level_and_mouth(&mono);
    }
}

fn f32_to_i16_sample(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
}

fn f32_to_u16_sample(sample: f32) -> u16 {
    (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5) * u16::MAX as f32).round() as u16
}

fn stream_error_handler(
    is_playing: Arc<AtomicBool>,
    output_level: Arc<AtomicU32>,
    last_error: Arc<Mutex<Option<String>>>,
) -> impl FnMut(cpal::StreamError) + Send + 'static {
    move |error_value| {
        is_playing.store(false, Ordering::SeqCst);
        output_level.store(0.0_f32.to_bits(), Ordering::Relaxed);
        *last_error.lock() = Some(format!("runtime audio output stream error: {error_value}"));
        error!(error = %error_value, "Audio playback stream failed");
    }
}

pub struct AudioPlayback {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    is_playing: Arc<AtomicBool>,
    output_level: Arc<AtomicU32>,
    level_meter: Arc<Mutex<LevelMeter>>,
    _stream: Mutex<Option<SafeStream>>,
    mouth_sender: Arc<Mutex<Option<mpsc::Sender<MouthShape>>>>,
    output_level_sender: Arc<Mutex<Option<mpsc::Sender<f32>>>>,
    output_sample_rate_hz: AtomicU32,
    output_channels: AtomicU32,
    selected_device: Mutex<Option<String>>,
    sample_format: Mutex<Option<String>>,
    dropped_samples: AtomicU64,
    last_error: Arc<Mutex<Option<String>>>,
}

impl AudioPlayback {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            is_playing: Arc::new(AtomicBool::new(false)),
            output_level: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            level_meter: Arc::new(Mutex::new(LevelMeter::new())),
            _stream: Mutex::new(None),
            mouth_sender: Arc::new(Mutex::new(None)),
            output_level_sender: Arc::new(Mutex::new(None)),
            output_sample_rate_hz: AtomicU32::new(0),
            output_channels: AtomicU32::new(0),
            selected_device: Mutex::new(None),
            sample_format: Mutex::new(None),
            dropped_samples: AtomicU64::new(0),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_mouth_sender(&self, tx: mpsc::Sender<MouthShape>) {
        *self.mouth_sender.lock() = Some(tx);
    }

    pub fn set_output_level_sender(&self, tx: mpsc::Sender<f32>) {
        *self.output_level_sender.lock() = Some(tx);
    }

    pub fn start(&self, device_name: Option<String>) -> Result<(), AudioPlaybackError> {
        self.flush();
        *self._stream.lock() = None;
        self.output_sample_rate_hz.store(0, Ordering::SeqCst);
        self.output_channels.store(0, Ordering::SeqCst);
        *self.selected_device.lock() = None;
        *self.sample_format.lock() = None;
        *self.last_error.lock() = None;

        let result = self.start_inner(device_name);
        if let Err(ref error_value) = result {
            *self.last_error.lock() = Some(error_value.to_string());
        }
        result
    }

    fn start_inner(&self, device_name: Option<String>) -> Result<(), AudioPlaybackError> {
        let host = cpal::default_host();
        let device = if let Some(ref requested_name) = device_name {
            host.output_devices()
                .map_err(|error_value| {
                    AudioPlaybackError::DeviceEnumeration(error_value.to_string())
                })?
                .find(|device| {
                    device
                        .name()
                        .map(|candidate| candidate == *requested_name)
                        .unwrap_or(false)
                })
                .ok_or_else(|| {
                    AudioPlaybackError::RequestedDeviceNotFound(requested_name.clone())
                })?
        } else {
            host.default_output_device()
                .ok_or(AudioPlaybackError::NoOutputDevice)?
        };

        let selected_device = device.name().ok().or(device_name);
        let supported_config = device.default_output_config().map_err(|error_value| {
            AudioPlaybackError::OutputConfiguration(error_value.to_string())
        })?;
        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();
        let sample_rate_hz = stream_config.sample_rate.0;
        let channels = stream_config.channels;

        let callback_state = PlaybackCallbackState {
            buffer: self.buffer.clone(),
            is_playing: self.is_playing.clone(),
            output_level: self.output_level.clone(),
            level_meter: self.level_meter.clone(),
            mouth_sender: self.mouth_sender.clone(),
            output_level_sender: self.output_level_sender.clone(),
            channels: usize::from(channels),
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let state = callback_state.clone();
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| state.render_f32(data),
                    stream_error_handler(
                        self.is_playing.clone(),
                        self.output_level.clone(),
                        self.last_error.clone(),
                    ),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let state = callback_state.clone();
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| state.render_i16(data),
                    stream_error_handler(
                        self.is_playing.clone(),
                        self.output_level.clone(),
                        self.last_error.clone(),
                    ),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let state = callback_state;
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| state.render_u16(data),
                    stream_error_handler(
                        self.is_playing.clone(),
                        self.output_level.clone(),
                        self.last_error.clone(),
                    ),
                    None,
                )
            }
            unsupported => return Err(AudioPlaybackError::UnsupportedSampleFormat(unsupported)),
        }
        .map_err(|error_value| AudioPlaybackError::BuildStream(error_value.to_string()))?;

        stream
            .play()
            .map_err(|error_value| AudioPlaybackError::StartStream(error_value.to_string()))?;
        *self._stream.lock() = Some(SafeStream(stream));
        *self.selected_device.lock() = selected_device;
        *self.sample_format.lock() = Some(format!("{sample_format:?}"));
        self.output_channels
            .store(u32::from(channels), Ordering::SeqCst);
        self.output_sample_rate_hz
            .store(sample_rate_hz, Ordering::SeqCst);
        info!(sample_rate_hz, ?sample_format, "Audio playback initialized");
        Ok(())
    }

    pub fn output_sample_rate_hz(&self) -> Option<u32> {
        let sample_rate = self.output_sample_rate_hz.load(Ordering::SeqCst);
        (sample_rate != 0).then_some(sample_rate)
    }

    pub fn max_queued_samples(&self) -> usize {
        self.output_sample_rate_hz().map_or(0, |sample_rate| {
            sample_rate as usize * MAX_QUEUED_PLAYBACK_SECONDS
        })
    }

    pub fn diagnostics(&self) -> AudioPlaybackDiagnostics {
        let channels = self.output_channels.load(Ordering::SeqCst);
        AudioPlaybackDiagnostics {
            selected_device: self.selected_device.lock().clone(),
            sample_rate_hz: self.output_sample_rate_hz(),
            sample_format: self.sample_format.lock().clone(),
            channels: (channels != 0).then_some(channels as u16),
            playing: self.is_playing(),
            output_level: f32::from_bits(self.output_level.load(Ordering::Relaxed)),
            queue_depth_samples: self.queue_length(),
            queue_limit_samples: self.max_queued_samples(),
            dropped_samples: self.dropped_samples(),
            last_error: self.last_error.lock().clone(),
        }
    }

    /// Enqueue incoming raw PCM i16 mono samples. The source sample rate is the
    /// provider/TTS rate; output is always resampled to the active device rate.
    ///
    /// Overflow policy: retain already-queued speech and drop the newest tail once
    /// the hard queue limit is reached. This preserves playback ordering and bounds
    /// memory/latency rather than allowing an overloaded producer to grow the queue.
    pub fn enqueue_pcm_i16(
        &self,
        samples: &[i16],
        source_sample_rate: u32,
    ) -> Result<PlaybackEnqueueReport, AudioPlaybackError> {
        let target_sample_rate = self
            .output_sample_rate_hz()
            .ok_or(AudioPlaybackError::NotStarted)?;
        let f32_samples = AudioResampler::i16_to_f32(samples);
        let resampled =
            AudioResampler::resample_linear(source_sample_rate, target_sample_rate, &f32_samples);
        let max_samples = target_sample_rate as usize * MAX_QUEUED_PLAYBACK_SECONDS;
        let mut buffer = self.buffer.lock();
        let available = max_samples.saturating_sub(buffer.len());
        let queued_samples = available.min(resampled.len());
        buffer.extend(resampled.iter().take(queued_samples).copied());
        let dropped_samples = resampled.len() - queued_samples;
        if dropped_samples > 0 {
            self.dropped_samples
                .fetch_add(dropped_samples as u64, Ordering::SeqCst);
            warn!(
                dropped_samples,
                queue_limit_samples = max_samples,
                "Playback queue overflow; dropping newest samples"
            );
        }
        Ok(PlaybackEnqueueReport {
            queued_samples,
            dropped_samples,
        })
    }

    /// Enqueue incoming raw little-endian i16 PCM bytes.
    pub fn enqueue_pcm_bytes(
        &self,
        bytes: &[u8],
        source_sample_rate: u32,
    ) -> Result<PlaybackEnqueueReport, AudioPlaybackError> {
        let i16_samples = AudioResampler::bytes_to_i16(bytes);
        self.enqueue_pcm_i16(&i16_samples, source_sample_rate)
    }

    /// Immediate barge-in / cancellation: discard all queued audio samples and reset mouth shape.
    pub fn flush(&self) {
        self.buffer.lock().clear();
        self.is_playing.store(false, Ordering::SeqCst);
        self.output_level.store(0.0_f32.to_bits(), Ordering::Relaxed);
        self.level_meter.lock().reset();
        if let Some(ref sender) = *self.mouth_sender.lock() {
            let _ = sender.try_send(MouthShape::Closed);
        }
        if let Some(ref sender) = *self.output_level_sender.lock() {
            let _ = sender.try_send(0.0);
        }
    }

    /// Fully stop playback ownership, including the negotiated CPAL stream.
    /// This is used for application shutdown; ordinary conversation interruption only flushes.
    pub fn stop(&self) {
        self.flush();
        *self._stream.lock() = None;
        self.output_sample_rate_hz.store(0, Ordering::SeqCst);
        self.output_channels.store(0, Ordering::SeqCst);
        *self.selected_device.lock() = None;
        *self.sample_format.lock() = None;
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }

    pub fn queue_length(&self) -> usize {
        self.buffer.lock().len()
    }

    pub fn dropped_samples(&self) -> u64 {
        self.dropped_samples.load(Ordering::SeqCst)
    }
}

impl Default for AudioPlayback {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playback_with_rate(sample_rate: u32) -> AudioPlayback {
        let playback = AudioPlayback::new();
        playback
            .output_sample_rate_hz
            .store(sample_rate, Ordering::SeqCst);
        playback
    }

    #[test]
    fn stop_flushes_and_resets_negotiated_output_state() {
        let playback = playback_with_rate(48_000);
        playback.output_channels.store(2, Ordering::SeqCst);
        *playback.selected_device.lock() = Some("test-output".to_string());
        *playback.sample_format.lock() = Some("F32".to_string());
        playback.buffer.lock().extend([0.25, -0.25]);
        playback.is_playing.store(true, Ordering::SeqCst);

        playback.stop();

        let diagnostics = playback.diagnostics();
        assert_eq!(diagnostics.sample_rate_hz, None);
        assert_eq!(diagnostics.channels, None);
        assert_eq!(diagnostics.selected_device, None);
        assert_eq!(diagnostics.sample_format, None);
        assert!(!diagnostics.playing);
        assert_eq!(diagnostics.queue_depth_samples, 0);
        assert_eq!(diagnostics.output_level, 0.0);
    }

    #[test]
    fn enqueue_resamples_24khz_to_48khz() {
        let playback = playback_with_rate(48_000);
        let samples = vec![1000_i16; 2_400];
        let report = playback.enqueue_pcm_i16(&samples, 24_000).unwrap();
        assert_eq!(report.queued_samples, 4_800);
        assert_eq!(report.dropped_samples, 0);
        assert_eq!(playback.queue_length(), 4_800);
    }

    #[test]
    fn enqueue_resamples_24khz_to_44_1khz() {
        let playback = playback_with_rate(44_100);
        let samples = vec![1000_i16; 2_400];
        let report = playback.enqueue_pcm_i16(&samples, 24_000).unwrap();
        assert_eq!(report.queued_samples, 4_410);
        assert_eq!(report.dropped_samples, 0);
    }

    #[test]
    fn enqueue_equal_rate_is_noop_length() {
        let playback = playback_with_rate(24_000);
        let samples = vec![1000_i16; 2_400];
        let report = playback.enqueue_pcm_i16(&samples, 24_000).unwrap();
        assert_eq!(report.queued_samples, samples.len());
        assert_eq!(report.dropped_samples, 0);
    }

    #[test]
    fn playback_queue_has_hard_limit_and_drops_newest_tail() {
        let playback = playback_with_rate(1_000);
        let max_samples = 1_000 * MAX_QUEUED_PLAYBACK_SECONDS;
        let samples = vec![1000_i16; max_samples + 250];
        let report = playback.enqueue_pcm_i16(&samples, 1_000).unwrap();

        assert_eq!(playback.queue_length(), max_samples);
        assert_eq!(report.queued_samples, max_samples);
        assert_eq!(report.dropped_samples, 250);
        assert_eq!(playback.dropped_samples(), 250);

        let diagnostics = playback.diagnostics();
        assert_eq!(diagnostics.queue_depth_samples, max_samples);
        assert_eq!(diagnostics.queue_limit_samples, max_samples);
        assert_eq!(diagnostics.dropped_samples, 250);
    }

    #[test]
    fn flush_removes_all_stale_samples_after_overflow() {
        let playback = playback_with_rate(1_000);
        let samples = vec![1000_i16; 2_000];
        playback.enqueue_pcm_i16(&samples, 1_000).unwrap();
        assert_eq!(playback.queue_length(), 2_000);

        playback.flush();
        assert_eq!(playback.queue_length(), 0);
        assert!(!playback.is_playing());
        assert_eq!(playback.diagnostics().output_level, 0.0);
    }

    #[test]
    fn sample_conversion_covers_practical_cpal_formats() {
        assert_eq!(f32_to_i16_sample(-1.0), i16::MIN + 1);
        assert_eq!(f32_to_i16_sample(0.0), 0);
        assert_eq!(f32_to_i16_sample(1.0), i16::MAX);

        assert_eq!(f32_to_u16_sample(-1.0), u16::MIN);
        assert!((i32::from(f32_to_u16_sample(0.0)) - 32_768).abs() <= 1);
        assert_eq!(f32_to_u16_sample(1.0), u16::MAX);
    }
}
