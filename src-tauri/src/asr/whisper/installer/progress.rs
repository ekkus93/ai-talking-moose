use super::integrity::WhisperVerifyingFileSink;
use super::transport::DownloadSink;
use super::WhisperModelInstallError;
use std::path::Path;
use std::sync::Arc;

use super::super::manifest::WHISPER_MODEL_BYTES;

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
}

impl InstallerFileSink {
    pub(super) fn create(
        path: &Path,
        callback: Option<WhisperModelInstallProgressCallback>,
    ) -> Result<Self, WhisperModelInstallError> {
        let inner = WhisperVerifyingFileSink::create(path)?;
        if let Some(callback) = callback.as_ref() {
            callback(WhisperModelInstallProgress {
                phase: WhisperModelInstallPhase::Downloading,
                downloaded_bytes: 0,
                total_bytes: WHISPER_MODEL_BYTES,
            });
        }
        Ok(Self { inner, callback })
    }

    pub(super) fn finish(self) -> Result<(), WhisperModelInstallError> {
        let result = self.inner.finish();
        if result.is_ok() {
            if let Some(callback) = self.callback.as_ref() {
                callback(WhisperModelInstallProgress {
                    phase: WhisperModelInstallPhase::Verifying,
                    downloaded_bytes: WHISPER_MODEL_BYTES,
                    total_bytes: WHISPER_MODEL_BYTES,
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
                total_bytes: WHISPER_MODEL_BYTES,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_order() {
        assert!(std::mem::size_of::<WhisperModelInstallPhase>() > 0);
    }
}
