use crate::audio::levels::LevelMeter;
use crate::audio::resample::AudioResampler;
use crate::character::state::MouthShape;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
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

#[derive(Clone)]
struct PlaybackCallbackState {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    is_playing: Arc<AtomicBool>,
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

fn log_stream_error(error_value: cpal::StreamError) {
    error!("Audio playback error: {}", error_value);
}

pub struct AudioPlayback {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    is_playing: Arc<AtomicBool>,
    level_meter: Arc<Mutex<LevelMeter>>,
    _stream: Mutex<Option<SafeStream>>,
    mouth_sender: Arc<Mutex<Option<mpsc::Sender<MouthShape>>>>,
    output_level_sender: Arc<Mutex<Option<mpsc::Sender<f32>>>>,
    output_sample_rate_hz: AtomicU32,
    dropped_samples: AtomicU64,
}

impl AudioPlayback {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            is_playing: Arc::new(AtomicBool::new(false)),
            level_meter: Arc::new(Mutex::new(LevelMeter::new())),
            _stream: Mutex::new(None),
            mouth_sender: Arc::new(Mutex::new(None)),
            output_level_sender: Arc::new(Mutex::new(None)),
            output_sample_rate_hz: AtomicU32::new(0),
            dropped_samples: AtomicU64::new(0),
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

        let host = cpal::default_host();
        let device = if let Some(ref name) = device_name {
            host.output_devices()
                .map_err(|error_value| {
                    AudioPlaybackError::DeviceEnumeration(error_value.to_string())
                })?
                .find(|device| {
                    device
                        .name()
                        .map(|candidate| candidate == *name)
                        .unwrap_or(false)
                })
                .or_else(|| host.default_output_device())
        } else {
            host.default_output_device()
        }
        .ok_or(AudioPlaybackError::NoOutputDevice)?;

        let supported_config = device.default_output_config().map_err(|error_value| {
            AudioPlaybackError::OutputConfiguration(error_value.to_string())
        })?;
        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();
        let sample_rate_hz = stream_config.sample_rate.0;
        let channels = usize::from(stream_config.channels);

        let callback_state = PlaybackCallbackState {
            buffer: self.buffer.clone(),
            is_playing: self.is_playing.clone(),
            level_meter: self.level_meter.clone(),
            mouth_sender: self.mouth_sender.clone(),
            output_level_sender: self.output_level_sender.clone(),
            channels,
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let state = callback_state.clone();
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| state.render_f32(data),
                    log_stream_error,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let state = callback_state.clone();
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| state.render_i16(data),
                    log_stream_error,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let state = callback_state;
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| state.render_u16(data),
                    log_stream_error,
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
        self.level_meter.lock().reset();
        if let Some(ref sender) = *self.mouth_sender.lock() {
            let _ = sender.try_send(MouthShape::Closed);
        }
        if let Some(ref sender) = *self.output_level_sender.lock() {
            let _ = sender.try_send(0.0);
        }
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
