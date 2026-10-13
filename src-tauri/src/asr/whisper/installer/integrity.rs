use super::transport::DownloadSink;
use super::WhisperModelInstallError;
use ring::digest::{Context as Sha256Context, SHA256};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

pub(super) trait StagingFile: Write + Send {
    fn sync_all(&mut self) -> io::Result<()>;
}

impl StagingFile for std::fs::File {
    fn sync_all(&mut self) -> io::Result<()> {
        std::fs::File::sync_all(self)
    }
}

pub(super) trait StagingFileFactory: Send + Sync {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn StagingFile>>;
}

pub(super) struct SystemStagingFileFactory;

impl StagingFileFactory for SystemStagingFileFactory {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn StagingFile>> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Box::new(file))
    }
}

pub(super) struct WhisperVerifyingFileSink {
    file: Box<dyn StagingFile>,
    pub(super) bytes_written: u64,
    expected_bytes: u64,
    expected_sha256: String,
    sha256: Sha256Context,
}

impl WhisperVerifyingFileSink {
    pub(super) fn create(
        path: &Path,
        expected_bytes: u64,
        expected_sha256: impl Into<String>,
        file_factory: &dyn StagingFileFactory,
    ) -> Result<Self, WhisperModelInstallError> {
        let file = file_factory
            .create_new(path)
            .map_err(|_| WhisperModelInstallError::io("create the staging file"))?;
        Ok(Self {
            file,
            bytes_written: 0,
            expected_bytes,
            expected_sha256: expected_sha256.into(),
            sha256: Sha256Context::new(&SHA256),
        })
    }

    pub(super) fn finish(mut self) -> Result<(), WhisperModelInstallError> {
        self.file
            .flush()
            .and_then(|()| self.file.sync_all())
            .map_err(|_| WhisperModelInstallError::io("flush the staging file"))?;
        if self.bytes_written != self.expected_bytes {
            return Err(WhisperModelInstallError::size_mismatch());
        }
        if digest_hex(self.sha256.finish().as_ref()) != self.expected_sha256 {
            return Err(WhisperModelInstallError::sha256_mismatch());
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn sha256_so_far(&self) -> String {
        digest_hex(self.sha256.clone().finish().as_ref())
    }
}

impl DownloadSink for WhisperVerifyingFileSink {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), WhisperModelInstallError> {
        let chunk_bytes =
            u64::try_from(chunk.len()).map_err(|_| WhisperModelInstallError::size_mismatch())?;
        let next_size = self.bytes_written.saturating_add(chunk_bytes);
        if next_size > self.expected_bytes {
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
