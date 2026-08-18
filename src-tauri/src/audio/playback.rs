use crate::audio::levels::LevelMeter;
use crate::audio::resample::AudioResampler;
use crate::character::state::MouthShape;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct SafeStream(pub cpal::Stream);
unsafe impl Send for SafeStream {}
unsafe impl Sync for SafeStream {}

pub struct AudioPlayback {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    is_playing: Arc<AtomicBool>,
    level_meter: Arc<Mutex<LevelMeter>>,
    _stream: Mutex<Option<SafeStream>>,
    mouth_sender: Arc<Mutex<Option<mpsc::Sender<MouthShape>>>>,
    output_level_sender: Arc<Mutex<Option<mpsc::Sender<f32>>>>,
}

impl AudioPlayback {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(48000))),
            is_playing: Arc::new(AtomicBool::new(false)),
            level_meter: Arc::new(Mutex::new(LevelMeter::new())),
            _stream: Mutex::new(None),
            mouth_sender: Arc::new(Mutex::new(None)),
            output_level_sender: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_mouth_sender(&self, tx: mpsc::Sender<MouthShape>) {
        *self.mouth_sender.lock() = Some(tx);
    }

    pub fn set_output_level_sender(&self, tx: mpsc::Sender<f32>) {
        *self.output_level_sender.lock() = Some(tx);
    }

    pub fn start(&self, device_name: Option<String>) -> Result<(), String> {
        let host = cpal::default_host();
        let device = if let Some(ref name) = device_name {
            host.output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .or_else(|| host.default_output_device())
        } else {
            host.default_output_device()
        };

        let device = match device {
            Some(d) => d,
            None => {
                warn!("No audio output device available. Mock fallback.");
                return Ok(());
            }
        };

        let default_config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get default output config: {}. Mock fallback.", e);
                return Ok(());
            }
        };

        let channels = default_config.channels() as usize;
        let sample_rate = default_config.sample_rate().0;

        let buffer_clone = self.buffer.clone();
        let is_playing_clone = self.is_playing.clone();
        let level_meter_clone = self.level_meter.clone();
        let mouth_tx = self.mouth_sender.clone();
        let level_tx = self.output_level_sender.clone();

        let err_fn = |err| {
            error!("Audio playback error: {}", err);
        };

        let stream_res = device.build_output_stream(
            &default_config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer_clone.lock();
                let mut chunk_samples = Vec::with_capacity(data.len() / channels);

                for frame in data.chunks_mut(channels) {
                    let sample = buf.pop_front().unwrap_or(0.0);
                    chunk_samples.push(sample);
                    for ch in frame.iter_mut() {
                        *ch = sample;
                    }
                }

                let has_audio =
                    !chunk_samples.is_empty() && chunk_samples.iter().any(|&s| s.abs() > 0.001);
                is_playing_clone.store(!buf.is_empty(), Ordering::SeqCst);

                let mut meter = level_meter_clone.lock();
                let (rms, mouth) = if has_audio {
                    meter.feed_samples(&chunk_samples)
                } else {
                    meter.feed_samples(&vec![0.0; chunk_samples.len()])
                };

                if let Some(ref mtx) = *mouth_tx.lock() {
                    let _ = mtx.try_send(mouth);
                }
                if let Some(ref ltx) = *level_tx.lock() {
                    let _ = ltx.try_send(rms);
                }
            },
            err_fn,
            None,
        );

        match stream_res {
            Ok(stream) => {
                stream.play().map_err(|e| e.to_string())?;
                *self._stream.lock() = Some(SafeStream(stream));
                info!("Audio playback initialized at {}Hz", sample_rate);
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Could not initialize audio output stream: {}. Mock fallback.",
                    e
                );
                Ok(())
            }
        }
    }

    /// Enqueue incoming raw PCM i16 samples (e.g. 24kHz)
    pub fn enqueue_pcm_i16(
        &self,
        samples: &[i16],
        source_sample_rate: u32,
        target_sample_rate: u32,
    ) {
        let f32_samples = AudioResampler::i16_to_f32(samples);
        let resampled =
            AudioResampler::resample_linear(source_sample_rate, target_sample_rate, &f32_samples);
        let mut buf = self.buffer.lock();
        buf.extend(resampled);
    }

    /// Enqueue incoming raw PCM bytes
    pub fn enqueue_pcm_bytes(
        &self,
        bytes: &[u8],
        source_sample_rate: u32,
        target_sample_rate: u32,
    ) {
        let i16_samples = AudioResampler::bytes_to_i16(bytes);
        self.enqueue_pcm_i16(&i16_samples, source_sample_rate, target_sample_rate);
    }

    /// Immediate barge-in / cancellation: discard all queued audio samples and reset mouth shape
    pub fn flush(&self) {
        let mut buf = self.buffer.lock();
        buf.clear();
        self.is_playing.store(false, Ordering::SeqCst);
        let mut meter = self.level_meter.lock();
        meter.reset();
        if let Some(ref mtx) = *self.mouth_sender.lock() {
            let _ = mtx.try_send(MouthShape::Closed);
        }
        if let Some(ref ltx) = *self.output_level_sender.lock() {
            let _ = ltx.try_send(0.0);
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }

    pub fn queue_length(&self) -> usize {
        self.buffer.lock().len()
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

    #[test]
    fn test_playback_queue_and_flush() {
        let playback = AudioPlayback::new();
        let samples = vec![1000i16, 2000, 3000, 4000];
        playback.enqueue_pcm_i16(&samples, 24000, 24000);
        assert_eq!(playback.queue_length(), 4);

        playback.flush();
        assert_eq!(playback.queue_length(), 0);
        assert!(!playback.is_playing());
    }
}
