use crate::audio::levels::LevelMeter;
use crate::audio::resample::AudioResampler;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct SafeStream(pub cpal::Stream);
unsafe impl Send for SafeStream {}
unsafe impl Sync for SafeStream {}

pub struct AudioCapture {
    is_running: Arc<AtomicBool>,
    _stream: Option<SafeStream>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            _stream: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Start capturing audio. Streams resampled 16-bit 16kHz mono PCM frames over the channel.
    pub fn start(
        &mut self,
        device_name: Option<String>,
        target_sample_rate: u32,
        pcm_sender: mpsc::Sender<Vec<u8>>,
        level_sender: Option<mpsc::Sender<f32>>,
    ) -> Result<(), String> {
        self.stop();

        let host = cpal::default_host();
        let device = if let Some(ref name) = device_name {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        };

        let device = match device {
            Some(d) => d,
            None => {
                warn!("No audio input device available. Falling back to mock capture.");
                self.is_running.store(true, Ordering::SeqCst);
                return Ok(());
            }
        };

        let default_config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get default input config: {}. Mock fallback.", e);
                self.is_running.store(true, Ordering::SeqCst);
                return Ok(());
            }
        };

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels() as usize;

        let is_running = self.is_running.clone();
        is_running.store(true, Ordering::SeqCst);

        let mut level_meter = LevelMeter::new();

        let err_fn = |err| {
            error!("Audio capture error: {}", err);
        };

        let stream_res = match default_config.sample_format() {
            cpal::SampleFormat::F32 => {
                let pcm_tx = pcm_sender.clone();
                let lvl_tx = level_sender.clone();
                let is_running_clone = is_running.clone();
                let mut accumulated_samples = Vec::with_capacity(3200);

                device.build_input_stream(
                    &default_config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !is_running_clone.load(Ordering::SeqCst) {
                            return;
                        }
                        // 1. Downmix to mono
                        let mono = AudioResampler::downmix_to_mono(channels, data);

                        // 2. Measure level
                        let (rms, _) = level_meter.feed_samples(&mono);
                        if let Some(ref ltx) = lvl_tx {
                            let _ = ltx.try_send(rms);
                        }

                        // 3. Resample to target rate (e.g. 16000 or 24000)
                        let resampled =
                            AudioResampler::resample_linear(sample_rate, target_sample_rate, &mono);

                        accumulated_samples.extend(resampled);

                        // Emit in 100ms chunks (1600 samples at 16kHz)
                        if accumulated_samples.len() >= 1600 {
                            let i16_samples = AudioResampler::f32_to_i16(&accumulated_samples);
                            let bytes = AudioResampler::i16_to_bytes(&i16_samples);
                            let _ = pcm_tx.try_send(bytes);
                            accumulated_samples.clear();
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let pcm_tx = pcm_sender.clone();
                let lvl_tx = level_sender.clone();
                let is_running_clone = is_running.clone();
                let mut accumulated_samples = Vec::with_capacity(3200);

                device.build_input_stream(
                    &default_config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if !is_running_clone.load(Ordering::SeqCst) {
                            return;
                        }
                        let f32_samples = AudioResampler::i16_to_f32(data);
                        let mono = AudioResampler::downmix_to_mono(channels, &f32_samples);

                        let (rms, _) = level_meter.feed_samples(&mono);
                        if let Some(ref ltx) = lvl_tx {
                            let _ = ltx.try_send(rms);
                        }

                        let resampled =
                            AudioResampler::resample_linear(sample_rate, target_sample_rate, &mono);

                        accumulated_samples.extend(resampled);

                        // Emit in 100ms chunks (1600 samples at 16kHz)
                        if accumulated_samples.len() >= 1600 {
                            let i16_samples = AudioResampler::f32_to_i16(&accumulated_samples);
                            let bytes = AudioResampler::i16_to_bytes(&i16_samples);
                            let _ = pcm_tx.try_send(bytes);
                            accumulated_samples.clear();
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                return Err("Unsupported sample format".to_string());
            }
        };

        match stream_res {
            Ok(stream) => {
                stream.play().map_err(|e| e.to_string())?;
                self._stream = Some(SafeStream(stream));
                info!("Microphone audio capture started at {}Hz", sample_rate);
                Ok(())
            }
            Err(e) => {
                warn!("Could not build input stream: {}. Mock fallback.", e);
                Ok(())
            }
        }
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
