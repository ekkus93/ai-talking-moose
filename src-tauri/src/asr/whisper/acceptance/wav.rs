use std::fs;
use std::path::Path;

#[derive(Debug)]
pub(super) struct WavSamples {
    pub(super) samples: Vec<f32>,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) bits_per_sample: u16,
}

/// Parses a 16-bit PCM WAV file into 32-bit float samples in the range
/// [-1.0, 1.0). The acceptance binary needs a small, dependency-free parser
/// because the model weights are not bundled and the project has no WAV crate.
/// It fails closed for any non-PCM, non-16-bit, or malformed container.
pub(super) fn read_wav_f32(path: &Path) -> Result<WavSamples, String> {
    let raw = fs::read(path).map_err(|error| format!("read wav file: {error}"))?;
    parse_wav_f32(&raw)
}

fn parse_wav_f32(raw: &[u8]) -> Result<WavSamples, String> {
    if raw.len() < 44 {
        return Err("wav file is too small to contain a valid RIFF/WAVE header".to_string());
    }

    if raw[0..4].to_vec() != b"RIFF".to_vec() {
        return Err("wav file is not a RIFF file".to_string());
    }
    if raw[8..12].to_vec() != b"WAVE".to_vec() {
        return Err("wav file is not a WAVE file".to_string());
    }

    // The fmt chunk must immediately follow the WAVE header, at offset 12.
    if raw[12..16].to_vec() != b"fmt ".to_vec() {
        return Err("wav file has no fmt chunk immediately after the WAVE header".to_string());
    }
    let fmt_size = le32(raw, 16);
    if fmt_size != 16 {
        return Err(format!(
            "wav fmt chunk size {fmt_size} is not the standard PCM size of 16"
        ));
    }

    let audio_format = le16(raw, 20);
    if audio_format != 1 {
        return Err(format!("wav file is not PCM (format tag {audio_format})"));
    }
    let channels = le16(raw, 22);
    let sample_rate = le32(raw, 24);
    let block_align = le16(raw, 32);
    let bits_per_sample = le16(raw, 34);

    if bits_per_sample != 16 {
        return Err(format!(
            "wav file is {bits_per_sample}-bit; whisper acceptance requires 16-bit PCM"
        ));
    }
    if sample_rate == 0 {
        return Err("wav fmt chunk does not declare a sample rate".to_string());
    }
    let expected_block_align = (channels as u32) * (bits_per_sample as u32) / 8;
    if (block_align as u32) != expected_block_align {
        return Err(format!(
            "wav block align {block_align} does not match expected {expected_block_align} for {channels}-channel {bits_per_sample}-bit PCM"
        ));
    }

    // Scan the remaining chunks for the data chunk.
    let file_len = raw.len();
    let mut offset = 12 + 8 + fmt_size as usize + (fmt_size as usize % 2);
    let mut data_start: Option<usize> = None;
    let mut data_size: Option<u32> = None;
    while offset + 8 <= file_len {
        if raw[offset..offset + 4].to_vec() != b"data".to_vec() {
            let chunk_size = le32(raw, offset + 4) as usize;
            if chunk_size == 0 {
                offset = file_len;
            } else {
                let next = offset + 8 + chunk_size + (chunk_size % 2);
                if next > file_len {
                    return Err("wav chunk extends beyond the file".to_string());
                }
                offset = next;
            }
        } else {
            data_size = Some(le32(raw, offset + 4));
            data_start = Some(offset + 8);
            break;
        }
    }

    let (data_start, data_size) = match (data_start, data_size) {
        (Some(start), Some(size)) => (start, size),
        _ => return Err("wav file has no data chunk".to_string()),
    };
    if data_start + data_size as usize > file_len {
        return Err("wav data chunk extends beyond the file".to_string());
    }

    let data = &raw[data_start..data_start + data_size as usize];
    if !data.len().is_multiple_of(2) {
        return Err("wav data has an odd number of bytes for 16-bit samples".to_string());
    }

    let samples: Vec<f32> = data
        .chunks(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32_768.0)
        .collect();

    Ok(WavSamples {
        samples,
        sample_rate,
        channels,
        bits_per_sample,
    })
}

fn le16(raw: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([raw[offset], raw[offset + 1]])
}

fn le32(raw: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_parser_skips_metadata_chunks_before_audio_data() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&0_u32.to_le_bytes()); // Filled with RIFF payload size below.
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1_u16.to_le_bytes()); // mono
        wav.extend_from_slice(&16_000_u32.to_le_bytes());
        wav.extend_from_slice(&32_000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"LIST");
        wav.extend_from_slice(&3_u32.to_le_bytes());
        wav.extend_from_slice(b"abc\0"); // Odd chunks have one pad byte.
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&4_u32.to_le_bytes());
        wav.extend_from_slice(&i16::MIN.to_le_bytes());
        wav.extend_from_slice(&16_384_i16.to_le_bytes());
        let riff_payload_size = (wav.len() - 8) as u32;
        wav[4..8].copy_from_slice(&riff_payload_size.to_le_bytes());

        let parsed = parse_wav_f32(&wav).expect("valid WAV with a LIST metadata chunk");
        assert_eq!(parsed.sample_rate, 16_000);
        assert_eq!(parsed.channels, 1);
        assert_eq!(parsed.bits_per_sample, 16);
        assert_eq!(parsed.samples, vec![-1.0, 0.5]);
    }
}
