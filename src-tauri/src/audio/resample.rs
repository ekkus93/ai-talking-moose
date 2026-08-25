/// Resampling and audio format conversion utilities
pub struct AudioResampler;

fn resample_output_len(input_len: usize, ratio: f64) -> usize {
    (input_len as f64 / ratio).round() as usize
}

fn resample_source_position(
    output_index: usize,
    input_len: usize,
    ratio: f64,
) -> (usize, usize, f32) {
    let src_idx = output_index as f64 * ratio;
    let raw_idx0 = src_idx.floor() as usize;
    let idx0 = raw_idx0.min(input_len - 1);
    let idx1 = (idx0 + 1).min(input_len - 1);
    let frac = (src_idx - raw_idx0 as f64) as f32;
    (idx0, idx1, frac)
}

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
        let output_len = resample_output_len(input.len(), ratio);
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let (idx0, idx1, frac) = resample_source_position(i, input.len(), ratio);
            let sample = input[idx0] * (1.0 - frac) + input[idx1] * frac;
            output.push(sample);
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

    /// Convert unsigned 16-bit PCM samples to f32 (-1.0 to 1.0).
    pub fn u16_to_f32(samples: &[u16]) -> Vec<f32> {
        samples
            .iter()
            .map(|&sample| (sample as f32 / 65535.0) * 2.0 - 1.0)
            .collect()
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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| i16::from_le_bytes(*chunk))
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
    fn resample_linear_always_honors_rounded_output_length() {
        for input_len in 1..=64 {
            let input = vec![0.25; input_len];
            for (from_rate, to_rate) in [
                (8_000, 44_100),
                (11_025, 48_000),
                (16_000, 44_100),
                (44_100, 16_000),
                (44_100, 48_000),
                (48_000, 44_100),
            ] {
                let ratio = from_rate as f64 / to_rate as f64;
                let expected_len = (input_len as f64 / ratio).round() as usize;
                let output = AudioResampler::resample_linear(from_rate, to_rate, &input);
                assert_eq!(
                    output.len(),
                    expected_len,
                    "input_len={input_len}, from={from_rate}, to={to_rate}"
                );
            }
        }
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn rounding_edge_clamps_the_last_source_index() {
        // This is a concrete f64 boundary where the rounded output length makes
        // the final source position round to exactly `input_len`. Allocating a
        // slice this large is neither practical nor necessary: this helper is
        // the production index calculation used by `resample_linear`.
        let input_len = 2_430_022_687_152_955usize;
        let from_rate = 1_976_296_948u32;
        let to_rate = 2_333_858_686u32;
        let ratio = from_rate as f64 / to_rate as f64;
        let output_len = resample_output_len(input_len, ratio);
        let (idx0, idx1, _frac) = resample_source_position(output_len - 1, input_len, ratio);

        assert_eq!(
            ((output_len - 1) as f64 * ratio).floor() as usize,
            input_len,
            "fixture must exercise the historical out-of-range rounding edge"
        );
        assert_eq!(idx0, input_len - 1);
        assert_eq!(idx1, input_len - 1);
    }

    #[test]
    fn test_u16_to_f32_endpoints_and_midpoint() {
        let converted = AudioResampler::u16_to_f32(&[u16::MIN, 32768, u16::MAX]);
        assert!((converted[0] + 1.0).abs() < 0.0001);
        assert!(converted[1].abs() < 0.0001);
        assert!((converted[2] - 1.0).abs() < 0.0001);
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
