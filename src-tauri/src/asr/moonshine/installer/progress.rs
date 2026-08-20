use super::super::manifest::MoonshineModelFile;
use super::integrity::VerifyingFileSink;
use super::transport::DownloadSink;
use super::MoonshineModelInstallError;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonshineModelInstallPhase {
    Downloading,
    Verifying,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoonshineModelInstallProgress {
    pub phase: MoonshineModelInstallPhase,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub current_file: Option<String>,
}

pub type MoonshineModelInstallProgressCallback =
    Arc<dyn Fn(MoonshineModelInstallProgress) + Send + Sync>;

pub(super) struct InstallerFileSink<'a> {
    inner: VerifyingFileSink<'a>,
    callback: Option<MoonshineModelInstallProgressCallback>,
    downloaded_before_file: u64,
    downloaded_in_file: u64,
    total_bytes: u64,
    current_file: String,
}

impl<'a> InstallerFileSink<'a> {
    pub(super) fn create(
        path: &Path,
        manifest_file: &'a MoonshineModelFile,
        downloaded_before_file: u64,
        total_bytes: u64,
        callback: Option<MoonshineModelInstallProgressCallback>,
    ) -> Result<Self, MoonshineModelInstallError> {
        let inner = VerifyingFileSink::create(path, manifest_file)?;
        if let Some(callback) = callback.as_ref() {
            callback(MoonshineModelInstallProgress {
                phase: MoonshineModelInstallPhase::Downloading,
                downloaded_bytes: downloaded_before_file,
                total_bytes,
                current_file: Some(manifest_file.name.to_string()),
            });
        }
        Ok(Self {
            inner,
            callback,
            downloaded_before_file,
            downloaded_in_file: 0,
            total_bytes,
            current_file: manifest_file.name.to_string(),
        })
    }

    pub(super) fn finish(self) -> Result<(), MoonshineModelInstallError> {
        self.inner.finish()
    }
}

impl DownloadSink for InstallerFileSink<'_> {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), MoonshineModelInstallError> {
        self.inner.write_chunk(chunk)?;
        if let Some(callback) = self.callback.as_ref() {
            let chunk_bytes = u64::try_from(chunk.len())
                .map_err(|_| MoonshineModelInstallError::size_mismatch(&self.current_file))?;
            self.downloaded_in_file = self.downloaded_in_file.saturating_add(chunk_bytes);
            callback(MoonshineModelInstallProgress {
                phase: MoonshineModelInstallPhase::Downloading,
                downloaded_bytes: self
                    .downloaded_before_file
                    .saturating_add(self.downloaded_in_file),
                total_bytes: self.total_bytes,
                current_file: Some(self.current_file.clone()),
            });
        }
        Ok(())
    }
}
