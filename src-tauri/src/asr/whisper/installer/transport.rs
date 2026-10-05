use super::{
    WhisperModelInstallCancellation, WhisperModelInstallError, WhisperModelInstallErrorKind,
};
use crate::installer_http::{classify_installer_http_failure, InstallerHttpFailure};
use async_trait::async_trait;
use futures_util::StreamExt;

pub(super) trait DownloadSink: Send {
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), WhisperModelInstallError>;
}

#[async_trait]
pub(super) trait ModelDownloadTransport: Send + Sync {
    async fn stream(
        &self,
        url: &str,
        cancellation: &WhisperModelInstallCancellation,
        sink: &mut dyn DownloadSink,
    ) -> Result<(), WhisperModelInstallError>;
}

pub(super) struct ReqwestModelDownloadTransport {
    client: reqwest::Client,
}

fn reqwest_install_error(error: reqwest::Error) -> WhisperModelInstallError {
    match classify_installer_http_failure(&error) {
        InstallerHttpFailure::Timeout => WhisperModelInstallError::new(
            WhisperModelInstallErrorKind::Network,
            "The Whisper model download timed out. Please try again.".to_string(),
            true,
        ),
        InstallerHttpFailure::Network => WhisperModelInstallError::network(),
    }
}

impl ReqwestModelDownloadTransport {
    pub(super) fn new() -> Result<Self, WhisperModelInstallError> {
        let client = crate::installer_http::build_secure_installer_http_client()
            .map_err(|_| WhisperModelInstallError::network())?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ModelDownloadTransport for ReqwestModelDownloadTransport {
    async fn stream(
        &self,
        url: &str,
        cancellation: &WhisperModelInstallCancellation,
        sink: &mut dyn DownloadSink,
    ) -> Result<(), WhisperModelInstallError> {
        if !url.starts_with("https://") {
            return Err(WhisperModelInstallError::invalid_manifest());
        }
        if cancellation.is_cancelled() {
            return Err(WhisperModelInstallError::cancelled());
        }

        let response = tokio::select! {
            () = cancellation.cancelled() => return Err(WhisperModelInstallError::cancelled()),
            response = self.client.get(url).header(reqwest::header::ACCEPT_ENCODING, "identity").send() => response.map_err(reqwest_install_error)?,
        };

        let status = response.status();
        if !status.is_success() {
            return Err(WhisperModelInstallError::http(status.as_u16()));
        }
        if response.url().scheme() != "https" {
            return Err(WhisperModelInstallError::network());
        }

        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                () = cancellation.cancelled() => return Err(WhisperModelInstallError::cancelled()),
                next = stream.next() => next,
            };
            let Some(chunk_result) = next else {
                break;
            };
            let chunk = chunk_result.map_err(reqwest_install_error)?;
            if cancellation.is_cancelled() {
                return Err(WhisperModelInstallError::cancelled());
            }
            sink.write_chunk(&chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_struct_exists() {
        let _ = std::mem::size_of::<ReqwestModelDownloadTransport>();
    }
}
