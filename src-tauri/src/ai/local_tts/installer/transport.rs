use super::{LocalTtsInstallError, LocalTtsInstallErrorKind};
use crate::ai::local_tts::manifest::LocalTtsArtifact;
use crate::installer_http::{
    classify_installer_http_failure, InstallerHttpFailure, INSTALLER_HTTP_TIMEOUT_MESSAGE,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

pub(super) type ArtifactProgressCallback = Arc<dyn Fn(u64) + Send + Sync>;

#[async_trait]
pub(super) trait LocalTtsDownloadTransport: Send + Sync {
    async fn download(
        &self,
        artifact: &'static LocalTtsArtifact,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&ArtifactProgressCallback>,
    ) -> Result<(), LocalTtsInstallError>;
}

pub(super) struct ReqwestLocalTtsDownloadTransport {
    client: reqwest::Client,
}

fn reqwest_install_error(error: reqwest::Error) -> LocalTtsInstallError {
    match classify_installer_http_failure(&error) {
        InstallerHttpFailure::Timeout => LocalTtsInstallError {
            kind: LocalTtsInstallErrorKind::Network,
            message: INSTALLER_HTTP_TIMEOUT_MESSAGE.to_string(),
            retryable: true,
        },
        InstallerHttpFailure::Network => LocalTtsInstallError::network(),
    }
}

impl ReqwestLocalTtsDownloadTransport {
    pub(super) fn new() -> Result<Self, LocalTtsInstallError> {
        let client = crate::installer_http::build_secure_installer_http_client()
            .map_err(|_| LocalTtsInstallError::network())?;
        Ok(Self { client })
    }
}

#[async_trait]
impl LocalTtsDownloadTransport for ReqwestLocalTtsDownloadTransport {
    async fn download(
        &self,
        artifact: &'static LocalTtsArtifact,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&ArtifactProgressCallback>,
    ) -> Result<(), LocalTtsInstallError> {
        let response = tokio::select! {
            _ = cancellation.cancelled() => return Err(LocalTtsInstallError::cancelled()),
            response = self.client.get(artifact.source_url).send() => {
                response.map_err(reqwest_install_error)?
            }
        };
        if !response.status().is_success() {
            return Err(LocalTtsInstallError::http(response.status().as_u16()));
        }
        if let Some(length) = response.content_length() {
            if length != artifact.expected_bytes {
                return Err(LocalTtsInstallError::size_mismatch());
            }
        }

        let mut output = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .await
            .map_err(|_| LocalTtsInstallError::io("create a staging artifact"))?;
        let mut downloaded = 0_u64;
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                _ = cancellation.cancelled() => return Err(LocalTtsInstallError::cancelled()),
                next = stream.next() => next,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(reqwest_install_error)?;
            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .ok_or_else(LocalTtsInstallError::size_mismatch)?;
            if downloaded > artifact.expected_bytes {
                return Err(LocalTtsInstallError::size_mismatch());
            }
            output
                .write_all(&chunk)
                .await
                .map_err(|_| LocalTtsInstallError::io("write a staging artifact"))?;
            if let Some(callback) = progress {
                callback(downloaded);
            }
        }
        output
            .flush()
            .await
            .map_err(|_| LocalTtsInstallError::io("flush a staging artifact"))?;
        output
            .sync_all()
            .await
            .map_err(|_| LocalTtsInstallError::io("sync a staging artifact"))?;
        if downloaded != artifact.expected_bytes {
            return Err(LocalTtsInstallError::size_mismatch());
        }
        Ok(())
    }
}
