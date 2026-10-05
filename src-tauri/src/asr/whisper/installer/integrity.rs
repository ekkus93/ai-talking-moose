use super::super::manifest::WHISPER_MODEL_BYTES;
use super::super::manifest::WHISPER_MODEL_SHA256;
use super::transport::DownloadSink;
use super::WhisperModelInstallError;
use ring::digest::{Context as Sha256Context, SHA256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub(super) struct WhisperVerifyingFileSink {
    file: std::fs::File,
    pub(super) bytes_written: u64,
    sha256: Sha256Context,
}

impl WhisperVerifyingFileSink {
    pub(super) fn create(path: &Path) -> Result<Self, WhisperModelInstallError> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| WhisperModelInstallError::io("create the staging file"))?;
        Ok(Self {
            file,
            bytes_written: 0,
            sha256: Sha256Context::new(&SHA256),
        })
    }

    pub(super) fn finish(mut self) -> Result<(), WhisperModelInstallError> {
        self.file
            .flush()
            .and_then(|()| self.file.sync_all())
            .map_err(|_| WhisperModelInstallError::io("flush the staging file"))?;
        if self.bytes_written != WHISPER_MODEL_BYTES {
            return Err(WhisperModelInstallError::size_mismatch());
        }
        if digest_hex(self.sha256.finish().as_ref()) != WHISPER_MODEL_SHA256 {
            return Err(WhisperModelInstallError::sha256_mismatch());
        }
        Ok(())
    }
}

impl DownloadSink for WhisperVerifyingFileSink {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), WhisperModelInstallError> {
        let chunk_bytes =
            u64::try_from(chunk.len()).map_err(|_| WhisperModelInstallError::size_mismatch())?;
        let next_size = self.bytes_written.saturating_add(chunk_bytes);
        if next_size > WHISPER_MODEL_BYTES {
            return Err(WhisperModelInstallError::size_mismatch());
        }
        self.file
            .write_all(chunk)
            .map_err(|_| WhisperModelInstallError::io("write the staging file"))?;
        self.sha256.update(chunk);
        self.bytes_written = next_size;
        Ok(())
    }
}

pub(super) fn digest_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_hex_roundtrip() {
        let bytes = b"hello world";
        let hex = digest_hex(bytes);
        assert_eq!(hex.len(), bytes.len() * 2);
    }
}
