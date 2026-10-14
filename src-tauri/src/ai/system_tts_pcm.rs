//! Bounded, format-honest PCM assembly for the future macOS AVSpeechSynthesizer
//! buffer callback. This module never starts an OS voice or plays audio itself.
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind};

const MAX_SAMPLE_RATE_HZ: u32 = 96_000;
const MIN_SAMPLE_RATE_HZ: u32 = 8_000;
const MAX_DURATION_SECONDS: usize = 45;
const MAX_CHANNELS: usize = 8;

/// Collects native float PCM chunks while keeping the true source sample rate.
/// Output is signed 16-bit little-endian mono, as required by Rust playback.
pub struct SystemPcmAccumulator {
    sample_rate_hz: u32,
    pcm_bytes: Vec<u8>,
}

impl SystemPcmAccumulator {
    pub fn new(sample_rate_hz: u32) -> Result<Self, ProviderError> {
        if !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
        }
        Ok(Self {
            sample_rate_hz,
            pcm_bytes: Vec::new(),
        })
    }

    /// Chunk samples must be frame-major/interleaved, with the same channel
    /// count throughout each chunk. Native planar buffers should be interleaved
    /// by the platform adapter before passing them here.
    pub fn append_interleaved_f32(
        &mut self,
        samples: &[f32],
        channels: usize,
    ) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channels == 0 || channels > MAX_CHANNELS || !samples.len().is_multiple_of(channels) {
            return Err(invalid());
        }
        let frames = samples.len() / channels;
        let used_frames = self.pcm_bytes.len() / 2;
        let max_frames = (self.sample_rate_hz as usize) * MAX_DURATION_SECONDS;
        if frames > max_frames.saturating_sub(used_frames)
            || samples.iter().any(|sample| !sample.is_finite())
        {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for frame in samples.chunks_exact(channels) {
            // Accumulate in f64: finite but extreme f32 channels can overflow
            // a f32 sum before opposing channels cancel each other out.
            let mono = frame.iter().map(|sample| f64::from(*sample)).sum::<f64>() / channels as f64;
            let i16_sample = (mono.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round() as i16;
            self.pcm_bytes.extend_from_slice(&i16_sample.to_le_bytes());
        }
        Ok(())
    }

    /// Append native AVAudioPCMBuffer-style planar float channels without an
    /// intermediate interleaved allocation. All channels must have the same
    /// frame count. Validate the entire chunk before mutating accumulated PCM.
    pub fn append_planar_f32(&mut self, channel_samples: &[&[f32]]) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channel_samples.is_empty() || channel_samples.len() > MAX_CHANNELS {
            return Err(invalid());
        }
        let frames = channel_samples[0].len();
        if channel_samples.iter().any(|channel| {
            channel.len() != frames || channel.iter().any(|sample| !sample.is_finite())
        }) {
            return Err(invalid());
        }
        let used_frames = self.pcm_bytes.len() / 2;
        let max_frames = (self.sample_rate_hz as usize) * MAX_DURATION_SECONDS;
        if frames > max_frames.saturating_sub(used_frames) {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for index in 0..frames {
            let mono = channel_samples
                .iter()
                .map(|channel| f64::from(channel[index]))
                .sum::<f64>()
                / channel_samples.len() as f64;
            let i16_sample = (mono.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round() as i16;
            self.pcm_bytes.extend_from_slice(&i16_sample.to_le_bytes());
        }
        Ok(())
    }

    pub fn finish(self) -> Result<AudioStreamData, ProviderError> {
        if self.pcm_bytes.is_empty() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
        }
        Ok(AudioStreamData {
            pcm_bytes: self.pcm_bytes,
            sample_rate: self.sample_rate_hz,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_samples_become_mono_without_falsifying_sample_rate() {
        let mut pcm = SystemPcmAccumulator::new(44_100).unwrap();
        pcm.append_interleaved_f32(&[1.0, -1.0, 0.5, 0.5], 2)
            .unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 44_100);
        assert_eq!(output.pcm_bytes.len(), 4);
        assert_eq!(
            i16::from_le_bytes([output.pcm_bytes[0], output.pcm_bytes[1]]),
            0
        );
        assert_eq!(
            i16::from_le_bytes([output.pcm_bytes[2], output.pcm_bytes[3]]),
            16384
        );
    }

    #[test]
    fn rejects_invalid_format_non_finite_audio_and_empty_synthesis() {
        for rate in [0, 7_999, 96_001] {
            assert!(SystemPcmAccumulator::new(rate).is_err());
        }
        assert!(SystemPcmAccumulator::new(24_000).unwrap().finish().is_err());
        let mut pcm = SystemPcmAccumulator::new(24_000).unwrap();
        for (samples, channels) in [
            (vec![0.1], 0),
            (vec![0.1], 2),
            (vec![f32::NAN], 1),
            (vec![f32::INFINITY], 1),
        ] {
            assert_eq!(
                pcm.append_interleaved_f32(&samples, channels)
                    .unwrap_err()
                    .kind,
                ProviderErrorKind::Protocol
            );
        }
        pcm.append_interleaved_f32(&[-2.0, 2.0], 1).unwrap();
        let audio = pcm.finish().unwrap();
        assert_eq!(
            i16::from_le_bytes([audio.pcm_bytes[0], audio.pcm_bytes[1]]),
            -32767
        );
        assert_eq!(
            i16::from_le_bytes([audio.pcm_bytes[2], audio.pcm_bytes[3]]),
            32767
        );
    }

    #[test]
    fn extreme_finite_channels_cancel_without_float_overflow() {
        let mut pcm = SystemPcmAccumulator::new(24_000).unwrap();
        pcm.append_interleaved_f32(&[f32::MAX, f32::MAX, -f32::MAX, -f32::MAX], 4)
            .unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 24_000);
        assert_eq!(output.pcm_bytes, 0_i16.to_le_bytes());
    }

    #[test]
    fn cumulative_duration_limit_rejects_extra_frames_without_corrupting_audio() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        pcm.append_interleaved_f32(&vec![0.0; 8_000 * 20], 1)
            .unwrap();
        pcm.append_interleaved_f32(&vec![0.0; 8_000 * 25], 1)
            .unwrap();
        assert_eq!(
            pcm.append_interleaved_f32(&[0.0], 1).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        assert_eq!(pcm.finish().unwrap().pcm_bytes.len(), 8_000 * 45 * 2);
    }

    #[test]
    fn planar_native_channels_downmix_without_interleaving_or_rate_changes() {
        let mut pcm = SystemPcmAccumulator::new(48_000).unwrap();
        pcm.append_planar_f32(&[&[1.0, 0.25], &[-1.0, 0.75]])
            .unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 48_000);
        let samples: Vec<i16> = output
            .pcm_bytes
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        assert_eq!(samples, [0, 16384]);
    }

    #[test]
    fn malformed_planar_chunk_is_rejected_atomically() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        pcm.append_planar_f32(&[&[0.25]]).unwrap();
        let invalid_channels: Vec<&[f32]> = vec![&[0.0]; MAX_CHANNELS + 1];
        for chunk in [
            vec![],
            vec![&[0.0][..], &[0.0, 0.0][..]],
            vec![&[f32::NAN][..]],
            vec![&[f32::INFINITY][..]],
            invalid_channels,
        ] {
            assert_eq!(
                pcm.append_planar_f32(&chunk).unwrap_err().kind,
                ProviderErrorKind::Protocol
            );
        }
        pcm.append_planar_f32(&[&[0.5]]).unwrap();
        assert_eq!(pcm.finish().unwrap().pcm_bytes.len(), 4);
    }

    #[test]
    fn planar_frames_share_the_same_cumulative_duration_budget() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        let channel = vec![0.0_f32; 8_000 * 45];
        pcm.append_planar_f32(&[&channel]).unwrap();
        assert_eq!(
            pcm.append_planar_f32(&[&[0.0]]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        assert_eq!(pcm.finish().unwrap().pcm_bytes.len(), 8_000 * 45 * 2);
    }

    #[test]
    fn bounded_duration_does_not_accept_an_oversized_chunk() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        let too_long = vec![0.0_f32; 8_000 * (MAX_DURATION_SECONDS + 1)];
        assert_eq!(
            pcm.append_interleaved_f32(&too_long, 1).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
    }
}
