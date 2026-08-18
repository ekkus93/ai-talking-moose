/// Resampling and audio format conversion utilities
pub struct AudioResampler;

impl AudioResampler {
    /// Downmix multi-channel interleaved float audio to mono
    pub fn downmix_to_mono(channels: usize, interleaved_samples: &[f32]) -> Vec<f32> {
        if channels <= 1 {
            return interleaved_samples.to_vec();
        }
        let frame_count = interleaved_samples.len() / channels;
        let mut mono = Vec::with_capacity(frame_count);
        for frame_idx in 0..frame_count {
            let offset = frame_idx * channels;
            let sum: f32 = interleaved_samples[offset..offset + channels].iter().sum();
            mono.push(sum / channels as f32);
        }
        mono
    }

    /// Resample mono audio from `from_rate` to `to_rate` using linear interpolation
    pub fn resample_linear(from_rate: u32, to_rate: u32, input: &[f32]) -> Vec<f32> {
        if from_rate == to_rate || input.is_empty() {
            return input.to_vec();
        }

        let ratio = from_rate as f64 / to_rate as f64;
        let output_len = (input.len() as f64 / ratio).round() as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_idx = i as f64 * ratio;
            let idx0 = src_idx.floor() as usize;
            let idx1 = (idx0 + 1).min(input.len() - 1);
            let frac = (src_idx - idx0 as f64) as f32;

            if idx0 < input.len() {
                let sample = input[idx0] * (1.0 - frac) + input[idx1] * frac;
                output.push(sample);
            }
        }

        output
    }

    /// Convert f32 samples (-1.0 to 1.0) to 16-bit signed integer PCM samples
    pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
        samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * 32767.0).round() as i16
            })
            .collect()
    }

    /// Convert 16-bit signed integer PCM samples to f32 (-1.0 to 1.0)
    pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / 32768.0).collect()
    }

    /// Convert i16 samples to raw little-endian bytes
    pub fn i16_to_bytes(samples: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for &s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        bytes
    }

    /// Convert raw little-endian bytes to i16 samples
    pub fn bytes_to_i16(bytes: &[u8]) -> Vec<i16> {
        bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downmix_to_mono() {
        let stereo = vec![1.0, 0.0, 0.5, 0.5, -1.0, 1.0];
        let mono = AudioResampler::downmix_to_mono(2, &stereo);
        assert_eq!(mono, vec![0.5, 0.5, 0.0]);
    }

    #[test]
    fn test_resample_linear() {
        let input = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        // Downsample 48kHz -> 24kHz (half length)
        let output = AudioResampler::resample_linear(48000, 24000, &input);
        assert_eq!(output.len(), 3);
    }

    #[test]
    fn test_f32_i16_roundtrip() {
        let original = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let i16_samples = AudioResampler::f32_to_i16(&original);
        let bytes = AudioResampler::i16_to_bytes(&i16_samples);
        let parsed_i16 = AudioResampler::bytes_to_i16(&bytes);
        let reconstructed = AudioResampler::i16_to_f32(&parsed_i16);

        for (a, b) in original.iter().zip(reconstructed.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }
}
