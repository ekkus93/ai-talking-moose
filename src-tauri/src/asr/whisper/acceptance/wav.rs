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

pub(super) fn parse_wav_f32(raw: &[u8]) -> Result<WavSamples, String> {
    if raw.len() < 12 {
        return Err("wav file is too small to contain a valid RIFF/WAVE header".to_string());
    }
    if raw.get(..4) != Some(b"RIFF") {
        return Err("wav file is not a RIFF file".to_string());
    }
    if raw.get(8..12) != Some(b"WAVE") {
        return Err("wav file is not a WAVE file".to_string());
    }

    let container_end = 8_usize
        .checked_add(le32(raw, 4) as usize)
        .ok_or_else(|| "wav RIFF size overflows the address space".to_string())?;
    if container_end != raw.len() {
        return Err("wav RIFF size does not match the file length".to_string());
    }

    // This acceptance parser intentionally supports the canonical 16-byte PCM
    // format chunk at the standard location; ancillary chunks may follow it.
    if raw.get(12..16) != Some(b"fmt ") {
        return Err("wav file has no fmt chunk immediately after the WAVE header".to_string());
    }
    if raw.len() < 20 {
        return Err("wav fmt chunk header is truncated".to_string());
    }
    let fmt_size = le32(raw, 16);
    if fmt_size != 16 {
        return Err(format!(
            "wav fmt chunk size {fmt_size} is not the standard PCM size of 16"
        ));
    }
    let fmt_payload_end = 20_usize
        .checked_add(fmt_size as usize)
        .ok_or_else(|| "wav fmt chunk size overflows the address space".to_string())?;
    if fmt_payload_end > container_end {
        return Err("wav fmt chunk extends beyond the file".to_string());
    }

    let audio_format = le16(raw, 20);
    if audio_format != 1 {
        return Err(format!("wav file is not PCM (format tag {audio_format})"));
    }
    let channels = le16(raw, 22);
    let sample_rate = le32(raw, 24);
    let byte_rate = le32(raw, 28);
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
    if channels == 0 {
        return Err("wav fmt chunk does not declare any channels".to_string());
    }
    let expected_block_align = (channels as u32) * (bits_per_sample as u32) / 8;
    if (block_align as u32) != expected_block_align {
        return Err(format!(
            "wav block align {block_align} does not match expected {expected_block_align} for {channels}-channel {bits_per_sample}-bit PCM"
        ));
    }
    let expected_byte_rate = sample_rate
        .checked_mul(expected_block_align)
        .ok_or_else(|| "wav byte rate overflows the supported PCM format".to_string())?;
    if byte_rate != expected_byte_rate {
        return Err(format!(
            "wav byte rate {byte_rate} does not match expected {expected_byte_rate}"
        ));
    }

    // Validate every chunk boundary against the RIFF container. The first fmt
    // chunk is fixed above; duplicate fmt/data chunks are rejected explicitly.
    let mut offset = fmt_payload_end + (fmt_size as usize % 2);
    let mut data_range: Option<(usize, usize)> = None;
    while offset < container_end {
        if container_end - offset < 8 {
            return Err("wav chunk header is truncated".to_string());
        }
        let chunk_id = &raw[offset..offset + 4];
        let chunk_size = le32(raw, offset + 4) as usize;
        let data_start = offset
            .checked_add(8)
            .ok_or_else(|| "wav chunk offset overflows the address space".to_string())?;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or_else(|| "wav chunk size overflows the address space".to_string())?;
        if data_end > container_end {
            return Err("wav chunk extends beyond the file".to_string());
        }
        let padded_end = data_end
            .checked_add(chunk_size % 2)
            .ok_or_else(|| "wav chunk padding overflows the address space".to_string())?;
        if padded_end > container_end {
            return Err("wav chunk is missing its required padding byte".to_string());
        }
        match chunk_id {
            b"data" => {
                if data_range.is_some() {
                    return Err("wav file contains more than one data chunk".to_string());
                }
                if !chunk_size.is_multiple_of(2) {
                    return Err(
                        "wav data has an odd number of bytes for 16-bit samples".to_string()
                    );
                }
                data_range = Some((data_start, data_end));
            }
            b"fmt " => return Err("wav file contains more than one fmt chunk".to_string()),
            _ => {}
        }
        offset = padded_end;
    }

    let (data_start, data_end) =
        data_range.ok_or_else(|| "wav file has no data chunk".to_string())?;
    let samples = raw[data_start..data_end]
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

    fn wav_with_chunks(channels: u16, sample_rate: u32, chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let block_align = channels * 2;
        let mut wav = Vec::from(&b"RIFF\0\0\0\0WAVEfmt "[..]);
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate.saturating_mul(block_align as u32)).to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        for (id, data) in chunks {
            wav.extend_from_slice(*id);
            wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
            wav.extend_from_slice(data);
            if data.len() % 2 == 1 {
                wav.push(0);
            }
        }
        update_riff_size(&mut wav);
        wav
    }

    fn update_riff_size(wav: &mut [u8]) {
        let size = (wav.len() - 8) as u32;
        wav[4..8].copy_from_slice(&size.to_le_bytes());
    }

    fn valid_mono_wav() -> Vec<u8> {
        wav_with_chunks(1, 16_000, &[(b"data", &[0, 0])])
    }

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

    #[test]
    fn rejects_malformed_headers_and_format_fields() {
        let valid = valid_mono_wav();
        type Mutation = Box<dyn FnOnce(&mut Vec<u8>)>;
        let cases: Vec<(&str, Mutation)> = vec![
            ("RIFF identifier", Box::new(|wav| wav[0] = b'X')),
            ("WAVE identifier", Box::new(|wav| wav[8] = b'X')),
            ("format chunk identifier", Box::new(|wav| wav[12] = b'X')),
            ("format chunk size", Box::new(|wav| wav[16] = 15)),
            ("non-PCM format", Box::new(|wav| wav[20] = 3)),
            ("zero channels", Box::new(|wav| wav[22] = 0)),
            ("zero sample rate", Box::new(|wav| wav[24..28].fill(0))),
            ("incorrect byte rate", Box::new(|wav| wav[28] = 1)),
            ("incorrect block alignment", Box::new(|wav| wav[32] = 1)),
            ("unsupported bit depth", Box::new(|wav| wav[34] = 8)),
        ];
        for (name, mutate) in cases {
            let mut wav = valid.clone();
            mutate(&mut wav);
            assert!(parse_wav_f32(&wav).is_err(), "accepted invalid {name}");
        }

        for truncated_len in [0, 7, 11, 19, 27, 35] {
            let mut truncated = valid.clone();
            truncated.truncate(truncated_len);
            if truncated_len >= 8 {
                update_riff_size(&mut truncated);
            }
            assert!(
                parse_wav_f32(&truncated).is_err(),
                "accepted truncated length {truncated_len}"
            );
        }
    }

    #[test]
    fn rejects_bad_chunk_extents_missing_data_and_duplicate_chunks() {
        let fmt_only = wav_with_chunks(1, 16_000, &[]);
        assert!(parse_wav_f32(&fmt_only)
            .unwrap_err()
            .contains("no data chunk"));

        let mut truncated_header = fmt_only.clone();
        truncated_header.extend_from_slice(b"LIST");
        update_riff_size(&mut truncated_header);
        assert!(parse_wav_f32(&truncated_header)
            .unwrap_err()
            .contains("header is truncated"));

        let mut metadata_overrun = fmt_only.clone();
        metadata_overrun.extend_from_slice(b"LIST");
        metadata_overrun.extend_from_slice(&100_u32.to_le_bytes());
        metadata_overrun.extend_from_slice(b"x");
        update_riff_size(&mut metadata_overrun);
        assert!(parse_wav_f32(&metadata_overrun)
            .unwrap_err()
            .contains("extends beyond"));

        let mut data_overrun = valid_mono_wav();
        let data_size_offset = data_overrun.len() - 6;
        data_overrun[data_size_offset..data_size_offset + 4]
            .copy_from_slice(&100_u32.to_le_bytes());
        assert!(parse_wav_f32(&data_overrun)
            .unwrap_err()
            .contains("extends beyond"));

        let odd_data = wav_with_chunks(1, 16_000, &[(b"data", &[1])]);
        assert!(parse_wav_f32(&odd_data)
            .unwrap_err()
            .contains("odd number of bytes"));

        let missing_pad = {
            let mut wav = fmt_only.clone();
            wav.extend_from_slice(b"LIST");
            wav.extend_from_slice(&3_u32.to_le_bytes());
            wav.extend_from_slice(b"abc");
            update_riff_size(&mut wav);
            wav
        };
        assert!(parse_wav_f32(&missing_pad)
            .unwrap_err()
            .contains("padding byte"));

        let duplicate_fmt = wav_with_chunks(1, 16_000, &[(b"fmt ", &[1; 16]), (b"data", &[0, 0])]);
        assert!(parse_wav_f32(&duplicate_fmt)
            .unwrap_err()
            .contains("more than one fmt"));

        let duplicate_data = wav_with_chunks(1, 16_000, &[(b"data", &[0, 0]), (b"data", &[1, 0])]);
        assert!(parse_wav_f32(&duplicate_data)
            .unwrap_err()
            .contains("more than one data"));
    }

    #[test]
    fn riff_size_must_match_and_well_formed_trailing_chunks_are_allowed() {
        let valid = valid_mono_wav();
        let mut incorrect_declared_size = valid.clone();
        let wrong_size = u32::try_from(valid.len() - 7).unwrap();
        incorrect_declared_size[4..8].copy_from_slice(&wrong_size.to_le_bytes());
        assert!(parse_wav_f32(&incorrect_declared_size)
            .unwrap_err()
            .contains("RIFF size"));

        let trailing_metadata = wav_with_chunks(
            1,
            16_000,
            &[(b"data", &[0, 0]), (b"LIST", b"post-data metadata")],
        );
        let parsed = parse_wav_f32(&trailing_metadata).unwrap();
        assert_eq!(parsed.samples, vec![0.0]);

        let mut unexplained_trailing_bytes = valid;
        unexplained_trailing_bytes.extend_from_slice(b"tail");
        assert!(parse_wav_f32(&unexplained_trailing_bytes)
            .unwrap_err()
            .contains("RIFF size"));
    }

    #[test]
    fn parses_pcm16_boundaries_and_never_panics_on_truncated_prefixes() {
        let boundary_data = [i16::MIN, 0, i16::MAX]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        let wav = wav_with_chunks(1, 16_000, &[(b"data", &boundary_data)]);
        let parsed = parse_wav_f32(&wav).unwrap();
        assert_eq!(parsed.samples[0], -1.0);
        assert_eq!(parsed.samples[1], 0.0);
        assert_eq!(parsed.samples[2], i16::MAX as f32 / 32_768.0);

        for end in 0..wav.len() {
            let result = std::panic::catch_unwind(|| parse_wav_f32(&wav[..end]));
            assert!(result.is_ok(), "parser panicked on {end}-byte prefix");
            assert!(result.unwrap().is_err(), "accepted {end}-byte prefix");
        }
    }

    #[test]
    fn arbitrary_bounded_inputs_never_panic_and_errors_are_stable_and_bounded() {
        for length in 0..256 {
            let input: Vec<u8> = (0..length)
                .map(|index| ((index * 37 + length * 13) % 256) as u8)
                .collect();
            let first = parse_wav_f32(&input).unwrap_err();
            let second = parse_wav_f32(&input).unwrap_err();
            assert_eq!(first, second, "error changed for input length {length}");
            assert!(first.len() <= 160, "error was unexpectedly long: {first}");
        }
    }
}
