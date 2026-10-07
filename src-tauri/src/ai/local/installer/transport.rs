use super::types::{
    LocalModelInstallError, LocalModelInstallErrorKind, LocalModelInstallProgress,
    LocalModelInstallProgressCallback, LocalModelInstallState,
};
use crate::ai::local::catalog::LocalModelCatalogEntry;
use crate::installer_http::{
    classify_installer_http_failure, InstallerHttpFailure, INSTALLER_HTTP_TIMEOUT_MESSAGE,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub(super) trait LocalModelDownloadTransport: Send + Sync {
    async fn download(
        &self,
        entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError>;
}

pub(super) struct ReqwestLocalModelDownloadTransport {
    client: reqwest::Client,
}

fn reqwest_install_error(error: reqwest::Error) -> LocalModelInstallError {
    match classify_installer_http_failure(&error) {
        InstallerHttpFailure::Timeout => LocalModelInstallError {
            kind: LocalModelInstallErrorKind::Network,
            message: INSTALLER_HTTP_TIMEOUT_MESSAGE.to_string(),
            retryable: true,
        },
        InstallerHttpFailure::Network => LocalModelInstallError::network(),
    }
}

impl ReqwestLocalModelDownloadTransport {
    pub(super) fn new() -> Result<Self, LocalModelInstallError> {
        let client = crate::installer_http::build_secure_installer_http_client()
            .map_err(|_| LocalModelInstallError::network())?;
        Ok(Self { client })
    }
}

#[async_trait]
impl LocalModelDownloadTransport for ReqwestLocalModelDownloadTransport {
    async fn download(
        &self,
        entry: &'static LocalModelCatalogEntry,
        destination: &Path,
        cancellation: &CancellationToken,
        progress: Option<&LocalModelInstallProgressCallback>,
    ) -> Result<(), LocalModelInstallError> {
        let response = tokio::select! {
            _ = cancellation.cancelled() => return Err(LocalModelInstallError::cancelled()),
            response = self.client.get(entry.source_url).send() => {
                response.map_err(reqwest_install_error)?
            }
        };
        if !response.status().is_success() {
            return Err(LocalModelInstallError::http(response.status().as_u16()));
        }
        if let Some(length) = response.content_length() {
            if length != entry.expected_bytes {
                return Err(LocalModelInstallError::size_mismatch());
            }
        }

        let mut output = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .await
            .map_err(|_| LocalModelInstallError::io("create the staging model file"))?;
        let mut downloaded = 0_u64;
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                _ = cancellation.cancelled() => return Err(LocalModelInstallError::cancelled()),
                next = stream.next() => next,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(reqwest_install_error)?;
            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .ok_or_else(LocalModelInstallError::size_mismatch)?;
            if downloaded > entry.expected_bytes {
                return Err(LocalModelInstallError::size_mismatch());
            }
            output
                .write_all(&chunk)
                .await
                .map_err(|_| LocalModelInstallError::io("write the staging model file"))?;
            if let Some(callback) = progress {
                callback(LocalModelInstallProgress {
                    model_id: entry.id.to_string(),
                    install_state: LocalModelInstallState::Downloading,
                    downloaded_bytes: downloaded,
                    total_bytes: entry.expected_bytes,
                });
            }
        }
        output
            .flush()
            .await
            .map_err(|_| LocalModelInstallError::io("flush the staging model file"))?;
        output
            .sync_all()
            .await
            .map_err(|_| LocalModelInstallError::io("sync the staging model file"))?;
        Ok(())
    }
}
