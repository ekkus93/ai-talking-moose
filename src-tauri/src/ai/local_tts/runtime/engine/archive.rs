use super::LocalTtsRuntimeError;
use crate::ai::local_tts::manifest::LocalTtsPlatform;
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use tempfile::{Builder as TempFileBuilder, NamedTempFile};

const MAX_RUNTIME_LIBRARY_BYTES: u64 = 256 * 1024 * 1024;
const TAR_BLOCK_BYTES: u64 = 512;

fn runtime_library_suffix(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "/lib/libonnxruntime.so.1.23.2",
        LocalTtsPlatform::MacosArm64 | LocalTtsPlatform::MacosX86_64 => {
            "/lib/libonnxruntime.1.23.2.dylib"
        }
    }
}

pub(super) fn extract_runtime_library(
    archive_path: &Path,
    platform: LocalTtsPlatform,
) -> Result<NamedTempFile, LocalTtsRuntimeError> {
    let file = File::open(archive_path).map_err(|_| LocalTtsRuntimeError::model_load())?;
    let mut archive = GzDecoder::new(file);
    let suffix = runtime_library_suffix(platform);
    let mut header = [0_u8; TAR_BLOCK_BYTES as usize];

    loop {
        read_exact(&mut archive, &mut header)?;
        if header.iter().all(|byte| *byte == 0) {
            return Err(LocalTtsRuntimeError::model_load());
        }
        let name = tar_entry_name(&header)?;
        let size = tar_octal(&header[124..136])?;
        let type_flag = header[156];
        if name.ends_with(suffix) {
            if !matches!(type_flag, 0 | b'0') || size == 0 || size > MAX_RUNTIME_LIBRARY_BYTES {
                return Err(LocalTtsRuntimeError::model_load());
            }
            let suffix = match platform {
                LocalTtsPlatform::LinuxX86_64 => ".so",
                LocalTtsPlatform::MacosArm64 | LocalTtsPlatform::MacosX86_64 => ".dylib",
            };
            let mut output = TempFileBuilder::new()
                .prefix("talking-moose-onnxruntime-")
                .suffix(suffix)
                .tempfile()
                .map_err(|_| LocalTtsRuntimeError::model_load())?;
            copy_exact(&mut archive, &mut output, size)?;
            output
                .flush()
                .map_err(|_| LocalTtsRuntimeError::model_load())?;
            return Ok(output);
        }
        skip_exact(&mut archive, padded_tar_size(size))?;
    }
}

fn tar_entry_name(header: &[u8; 512]) -> Result<String, LocalTtsRuntimeError> {
    let name = tar_string(&header[..100])?;
    let prefix = tar_string(&header[345..500])?;
    if name.is_empty() {
        return Err(LocalTtsRuntimeError::model_load());
    }
    Ok(if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    })
}

fn tar_string(bytes: &[u8]) -> Result<String, LocalTtsRuntimeError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value =
        std::str::from_utf8(&bytes[..end]).map_err(|_| LocalTtsRuntimeError::model_load())?;
    Ok(value.trim().to_string())
}

fn tar_octal(bytes: &[u8]) -> Result<u64, LocalTtsRuntimeError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value =
        std::str::from_utf8(&bytes[..end]).map_err(|_| LocalTtsRuntimeError::model_load())?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(value, 8).map_err(|_| LocalTtsRuntimeError::model_load())
}

fn padded_tar_size(size: u64) -> u64 {
    size.saturating_add((TAR_BLOCK_BYTES - (size % TAR_BLOCK_BYTES)) % TAR_BLOCK_BYTES)
}

fn read_exact(reader: &mut impl Read, buffer: &mut [u8]) -> Result<(), LocalTtsRuntimeError> {
    reader
        .read_exact(buffer)
        .map_err(|_| LocalTtsRuntimeError::model_load())
}

fn skip_exact(reader: &mut impl Read, bytes: u64) -> Result<(), LocalTtsRuntimeError> {
    let copied = io::copy(&mut reader.take(bytes), &mut io::sink())
        .map_err(|_| LocalTtsRuntimeError::model_load())?;
    if copied == bytes {
        Ok(())
    } else {
        Err(LocalTtsRuntimeError::model_load())
    }
}

fn copy_exact(
    reader: &mut impl Read,
    writer: &mut impl Write,
    bytes: u64,
) -> Result<(), LocalTtsRuntimeError> {
    let copied = io::copy(&mut reader.take(bytes), writer)
        .map_err(|_| LocalTtsRuntimeError::model_load())?;
    if copied == bytes {
        Ok(())
    } else {
        Err(LocalTtsRuntimeError::model_load())
    }
}
