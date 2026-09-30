use super::{MoonshineModelInstallCancellation, MoonshineModelInstallError};
use async_trait::async_trait;
use futures_util::StreamExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DownloadMetadata {
    pub(super) content_length: Option<u64>,
}

pub(super) trait DownloadSink: Send {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), MoonshineModelInstallError>;
}

#[async_trait]
pub(super) trait ModelDownloadTransport: Send + Sync {
    async fn stream(
        &self,
        url: &str,
        cancellation: &MoonshineModelInstallCancellation,
        sink: &mut dyn DownloadSink,
    ) -> Result<DownloadMetadata, MoonshineModelInstallError>;
}

pub(super) struct ReqwestModelDownloadTransport {
    client: reqwest::Client,
}

impl ReqwestModelDownloadTransport {
    pub(super) fn new() -> Result<Self, MoonshineModelInstallError> {
        let client = crate::installer_http::build_secure_installer_http_client()
            .map_err(|_| MoonshineModelInstallError::network())?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ModelDownloadTransport for ReqwestModelDownloadTransport {
    async fn stream(
        &self,
        url: &str,
        cancellation: &MoonshineModelInstallCancellation,
        sink: &mut dyn DownloadSink,
    ) -> Result<DownloadMetadata, MoonshineModelInstallError> {
        if !url.starts_with("https://") {
            return Err(MoonshineModelInstallError::invalid_manifest());
        }
        cancellation.check()?;

        let response = tokio::select! {
            () = cancellation.cancelled() => return Err(MoonshineModelInstallError::cancelled()),
            response = self
                .client
                .get(url)
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .send() => response.map_err(|_| MoonshineModelInstallError::network())?,
        };

        let status = response.status();
        if !status.is_success() {
            return Err(MoonshineModelInstallError::http(status.as_u16()));
        }
        if response.url().scheme() != "https" {
            return Err(MoonshineModelInstallError::network());
        }

        let content_length = response.content_length();
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                () = cancellation.cancelled() => return Err(MoonshineModelInstallError::cancelled()),
                next = stream.next() => next,
            };
            let Some(chunk_result) = next else {
                break;
            };
            let chunk = chunk_result.map_err(|_| MoonshineModelInstallError::network())?;
            cancellation.check()?;
            sink.write_chunk(&chunk)?;
        }

        Ok(DownloadMetadata { content_length })
    }
}
