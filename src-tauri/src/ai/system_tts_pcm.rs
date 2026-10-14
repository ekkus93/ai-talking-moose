//! Bounded, format-honest PCM assembly for the future macOS AVSpeechSynthesizer
//! buffer callback. This module never starts an OS voice or plays audio itself.
use crate::ai::types::{AudioStreamData, ProviderError, ProviderErrorKind};

const MAX_SAMPLE_RATE_HZ: u32 = 96_000;
const MIN_SAMPLE_RATE_HZ: u32 = 8_000;
const MAX_DURATION_SECONDS: usize = crate::audio::playback::MAX_QUEUED_PLAYBACK_SECONDS;
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

    /// Append native planar double-precision float PCM. Scale each channel
    /// before summing so even finite f64::MAX inputs cannot overflow the mix.
    /// As with other formats, reject the entire chunk before writing samples.
    pub fn append_planar_f64(&mut self, channels: &[&[f64]]) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channels.is_empty() || channels.len() > MAX_CHANNELS {
            return Err(invalid());
        }
        let frames = channels[0].len();
        let remaining = (self.sample_rate_hz as usize)
            .saturating_mul(MAX_DURATION_SECONDS)
            .saturating_sub(self.pcm_bytes.len() / 2);
        if frames > remaining
            || channels.iter().any(|channel| {
                channel.len() != frames || channel.iter().any(|sample| !sample.is_finite())
            })
        {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for frame in 0..frames {
            let mono = channels
                .iter()
                .map(|channel| channel[frame] / channels.len() as f64)
                .sum::<f64>();
            let sample = (mono.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round() as i16;
            self.pcm_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(())
    }

    /// Append native planar signed 16-bit PCM without allocating float samples.
    pub fn append_planar_i16(&mut self, channels: &[&[i16]]) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channels.is_empty() || channels.len() > MAX_CHANNELS {
            return Err(invalid());
        }
        let frames = channels[0].len();
        let remaining = (self.sample_rate_hz as usize)
            .saturating_mul(MAX_DURATION_SECONDS)
            .saturating_sub(self.pcm_bytes.len() / 2);
        if frames > remaining || channels.iter().any(|channel| channel.len() != frames) {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for frame in 0..frames {
            let sum: i64 = channels
                .iter()
                .map(|channel| i64::from(channel[frame]))
                .sum();
            let sample = (sum as f64 / channels.len() as f64).round() as i16;
            self.pcm_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(())
    }

    /// Append native planar signed 32-bit PCM without unbounded intermediate
    /// buffers. Downmix in i64 to avoid overflow at extreme sample values.
    pub fn append_planar_i32(&mut self, channels: &[&[i32]]) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channels.is_empty() || channels.len() > MAX_CHANNELS {
            return Err(invalid());
        }
        let frames = channels[0].len();
        let remaining = (self.sample_rate_hz as usize)
            .saturating_mul(MAX_DURATION_SECONDS)
            .saturating_sub(self.pcm_bytes.len() / 2);
        if frames > remaining || channels.iter().any(|channel| channel.len() != frames) {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for frame in 0..frames {
            let sum: i64 = channels
                .iter()
                .map(|channel| i64::from(channel[frame]))
                .sum();
            let mean = sum as f64 / channels.len() as f64;
            let sample = (mean / 2_147_483_648.0 * 32_768.0)
                .round()
                .clamp(-32_768.0, 32_767.0) as i16;
            self.pcm_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(())
    }

    /// Append native interleaved signed 16-bit PCM with bounded duration.
    /// Reject malformed chunks before changing the accumulated audio.
    pub fn append_interleaved_i16(
        &mut self,
        samples: &[i16],
        channels: usize,
    ) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channels == 0 || channels > MAX_CHANNELS || !samples.len().is_multiple_of(channels) {
            return Err(invalid());
        }
        let frames = samples.len() / channels;
        let remaining = (self.sample_rate_hz as usize)
            .saturating_mul(MAX_DURATION_SECONDS)
            .saturating_sub(self.pcm_bytes.len() / 2);
        if frames > remaining {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for frame in samples.chunks_exact(channels) {
            let sum: i64 = frame.iter().map(|sample| i64::from(*sample)).sum();
            let mono = (sum as f64 / channels as f64).round() as i16;
            self.pcm_bytes.extend_from_slice(&mono.to_le_bytes());
        }
        Ok(())
    }

    /// Append native interleaved signed 32-bit PCM without allocating a
    /// planar copy. Validate the entire chunk before changing queued audio.
    pub fn append_interleaved_i32(
        &mut self,
        samples: &[i32],
        channels: usize,
    ) -> Result<(), ProviderError> {
        let invalid = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if channels == 0 || channels > MAX_CHANNELS || !samples.len().is_multiple_of(channels) {
            return Err(invalid());
        }
        let frames = samples.len() / channels;
        let remaining = (self.sample_rate_hz as usize)
            .saturating_mul(MAX_DURATION_SECONDS)
            .saturating_sub(self.pcm_bytes.len() / 2);
        if frames > remaining {
            return Err(invalid());
        }
        self.pcm_bytes.reserve(frames * 2);
        for frame in samples.chunks_exact(channels) {
            let sum: i64 = frame.iter().map(|sample| i64::from(*sample)).sum();
            let mean = sum as f64 / channels as f64;
            let sample = (mean / 2_147_483_648.0 * 32_768.0)
                .round()
                .clamp(-32_768.0, 32_767.0) as i16;
            self.pcm_bytes.extend_from_slice(&sample.to_le_bytes());
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
        pcm.append_interleaved_f32(&vec![0.0; 8_000 * 4], 1)
            .unwrap();
        pcm.append_interleaved_f32(&vec![0.0; 8_000 * 6], 1)
            .unwrap();
        assert_eq!(
            pcm.append_interleaved_f32(&[0.0], 1).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        assert_eq!(
            pcm.finish().unwrap().pcm_bytes.len(),
            8_000 * MAX_DURATION_SECONDS * 2
        );
    }

    #[test]
    fn planar_native_channels_downmix_without_interleaving_or_rate_changes() {
        let mut pcm = SystemPcmAccumulator::new(48_000).unwrap();
        pcm.append_planar_f32(&[&[1.0, 0.25], &[-1.0, 0.75]])
            .unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 48_000);
        let first = i16::from_le_bytes([output.pcm_bytes[0], output.pcm_bytes[1]]);
        let second = i16::from_le_bytes([output.pcm_bytes[2], output.pcm_bytes[3]]);
        assert_eq!((first, second), (0, 16384));
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
        let channel = vec![0.0_f32; 8_000 * MAX_DURATION_SECONDS];
        pcm.append_planar_f32(&[&channel]).unwrap();
        assert_eq!(
            pcm.append_planar_f32(&[&[0.0]]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        assert_eq!(
            pcm.finish().unwrap().pcm_bytes.len(),
            8_000 * MAX_DURATION_SECONDS * 2
        );
    }

    #[test]
    fn planar_oversized_chunk_is_rejected_without_partial_audio() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        let oversized = vec![0.0_f32; 8_000 * (MAX_DURATION_SECONDS + 1)];
        assert_eq!(
            pcm.append_planar_f32(&[&oversized]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        pcm.append_planar_f32(&[&[0.25]]).unwrap();
        assert_eq!(pcm.finish().unwrap().pcm_bytes.len(), 2);
    }

    #[test]
    fn planar_f64_preserves_rate_and_handles_extreme_finite_channels() {
        let mut pcm = SystemPcmAccumulator::new(48_000).unwrap();
        pcm.append_planar_f64(&[
            &[f64::MAX, 0.5, -1.0],
            &[-f64::MAX, 0.5, 1.0],
        ])
        .unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 48_000);
        assert_eq!(
            output.pcm_bytes,
            [0_i16, 16384_i16, 0_i16]
                .map(i16::to_le_bytes)
                .concat()
        );
    }

    #[test]
    fn planar_f64_invalid_chunks_are_atomic() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        pcm.append_planar_f64(&[&[0.25]]).unwrap();
        let oversized = vec![0.0_f64; 8_000 * (MAX_DURATION_SECONDS + 1)];
        for channels in [
            vec![],
            vec![&[1.0][..], &[0.0, 0.0][..]],
            vec![&[f64::NAN][..]],
            vec![&[f64::INFINITY][..]],
            vec![&oversized[..]],
        ] {
            assert_eq!(
                pcm.append_planar_f64(&channels).unwrap_err().kind,
                ProviderErrorKind::Protocol
            );
        }
        assert_eq!(pcm.finish().unwrap().pcm_bytes.len(), 2);
    }

    #[test]
    fn planar_i16_input_downmixes_and_preserves_sample_rate() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        pcm.append_planar_i16(&[&[i16::MAX, i16::MIN], &[i16::MIN, i16::MIN]])
            .unwrap();
        assert_eq!(
            pcm.append_planar_i16(&[&[1], &[1, 2]]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        let oversized = vec![0_i16; 8_000 * (MAX_DURATION_SECONDS + 1)];
        assert_eq!(
            pcm.append_planar_i16(&[&oversized]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 8_000);
        assert_eq!(output.pcm_bytes.len(), 4);
        assert_eq!(
            i16::from_le_bytes([output.pcm_bytes[0], output.pcm_bytes[1]]),
            -1
        );
        assert_eq!(
            i16::from_le_bytes([output.pcm_bytes[2], output.pcm_bytes[3]]),
            i16::MIN
        );
    }

    #[test]
    fn interleaved_i16_downmixes_and_preserves_sample_rate() {
        let mut pcm = SystemPcmAccumulator::new(44_100).unwrap();
        pcm.append_interleaved_i16(&[i16::MAX, i16::MIN, 10, 20], 2)
            .unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 44_100);
        assert_eq!(
            output.pcm_bytes,
            [-1_i16, 15_i16].map(i16::to_le_bytes).concat()
        );
    }

    #[test]
    fn interleaved_i16_invalid_chunks_leave_accumulated_audio_intact() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        pcm.append_interleaved_i16(&[100], 1).unwrap();
        for (samples, channels) in [
            (vec![1], 0),
            (vec![1], 2),
            (vec![1; MAX_CHANNELS + 1], MAX_CHANNELS + 1),
            (vec![0; 8_000 * (MAX_DURATION_SECONDS + 1)], 1),
        ] {
            assert_eq!(
                pcm.append_interleaved_i16(&samples, channels)
                    .unwrap_err()
                    .kind,
                ProviderErrorKind::Protocol
            );
        }
        assert_eq!(pcm.finish().unwrap().pcm_bytes, 100_i16.to_le_bytes());
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
    #[test]
    fn planar_i32_downmix_is_bounded_and_preserves_signed_extremes() {
        let mut pcm = SystemPcmAccumulator::new(44_100).unwrap();
        pcm.append_planar_i32(&[
            &[i32::MAX, i32::MIN, i32::MAX],
            &[i32::MAX, i32::MIN, i32::MIN],
        ])
        .unwrap();
        assert_eq!(
            pcm.append_planar_i32(&[&[0], &[0, 1]]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        pcm.append_planar_i32(&[&[0]]).unwrap();
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 44_100);
        let samples: Vec<i16> = output
            .pcm_bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| i16::from_le_bytes(*pair))
            .collect();
        assert_eq!(samples, vec![i16::MAX, i16::MIN, 0, 0]);
    }

    #[test]
    fn interleaved_i32_downmixes_and_rejects_invalid_chunks_atomically() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        pcm.append_interleaved_i32(&[i32::MAX, i32::MAX, i32::MIN, i32::MIN], 2)
            .unwrap();
        for (samples, channels) in [
            (vec![1], 0),
            (vec![1], 2),
            (vec![0; MAX_CHANNELS + 1], MAX_CHANNELS + 1),
            (vec![0; 8_000 * (MAX_DURATION_SECONDS + 1)], 1),
        ] {
            assert_eq!(
                pcm.append_interleaved_i32(&samples, channels)
                    .unwrap_err()
                    .kind,
                ProviderErrorKind::Protocol
            );
        }
        let output = pcm.finish().unwrap();
        assert_eq!(output.sample_rate, 8_000);
        assert_eq!(
            output.pcm_bytes,
            [i16::MAX, i16::MIN].map(i16::to_le_bytes).concat()
        );
    }

    #[test]
    fn planar_i32_rejects_oversized_chunks_without_partial_audio() {
        let mut pcm = SystemPcmAccumulator::new(8_000).unwrap();
        let oversized = vec![0_i32; 8_000 * (MAX_DURATION_SECONDS + 1)];
        assert_eq!(
            pcm.append_planar_i32(&[&oversized]).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
        pcm.append_planar_i32(&[&[i32::MIN]]).unwrap();
        assert_eq!(pcm.finish().unwrap().pcm_bytes, i16::MIN.to_le_bytes());
    }
}
