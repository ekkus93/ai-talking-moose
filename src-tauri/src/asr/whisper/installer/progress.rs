use super::integrity::WhisperVerifyingFileSink;
use super::transport::DownloadSink;
use super::WhisperModelInstallError;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModelInstallPhase {
    Downloading,
    Verifying,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperModelInstallProgress {
    pub phase: WhisperModelInstallPhase,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

pub type WhisperModelInstallProgressCallback =
    Arc<dyn Fn(WhisperModelInstallProgress) + Send + Sync>;

pub(super) struct InstallerFileSink {
    inner: WhisperVerifyingFileSink,
    callback: Option<WhisperModelInstallProgressCallback>,
    expected_bytes: u64,
}

impl InstallerFileSink {
    pub(super) fn create(
        path: &Path,
        callback: Option<WhisperModelInstallProgressCallback>,
        expected_bytes: u64,
        expected_sha256: impl Into<String>,
    ) -> Result<Self, WhisperModelInstallError> {
        let inner = WhisperVerifyingFileSink::create(path, expected_bytes, expected_sha256)?;
        if let Some(callback) = callback.as_ref() {
            callback(WhisperModelInstallProgress {
                phase: WhisperModelInstallPhase::Downloading,
                downloaded_bytes: 0,
                total_bytes: expected_bytes,
            });
        }
        Ok(Self {
            inner,
            callback,
            expected_bytes,
        })
    }

    pub(super) fn finish(self) -> Result<(), WhisperModelInstallError> {
        let result = self.inner.finish();
        if result.is_ok() {
            if let Some(callback) = self.callback.as_ref() {
                callback(WhisperModelInstallProgress {
                    phase: WhisperModelInstallPhase::Verifying,
                    downloaded_bytes: self.expected_bytes,
                    total_bytes: self.expected_bytes,
                });
            }
        }
        result
    }
}

impl DownloadSink for InstallerFileSink {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), WhisperModelInstallError> {
        self.inner.write_chunk(chunk)?;
        if let Some(callback) = self.callback.as_ref() {
            callback(WhisperModelInstallProgress {
                phase: WhisperModelInstallPhase::Downloading,
                downloaded_bytes: self.inner.bytes_written,
                total_bytes: self.expected_bytes,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_file_creation_failure_is_reported_as_sanitized_io_error() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path_is_directory = directory.path().join("staging.bin");
        std::fs::create_dir(&path_is_directory).expect("create conflicting directory");
        let error = match InstallerFileSink::create(&path_is_directory, None, 1, "00") {
            Ok(_) => panic!("file sink must reject a directory path"),
            Err(error) => error,
        };
        assert_eq!(error.kind, super::super::WhisperModelInstallErrorKind::Io);
        assert_eq!(
            error.message,
            "The Whisper model installer could not create the staging file."
        );
    }
}
