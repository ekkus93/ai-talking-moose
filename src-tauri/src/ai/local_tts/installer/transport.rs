use super::LocalTtsInstallError;
use crate::ai::local_tts::manifest::LocalTtsArtifact;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

const MAX_MODEL_REDIRECTS: usize = 5;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedirectDecision {
    Follow,
    RejectInsecureScheme,
    RejectLimit,
}

fn redirect_decision(url: &reqwest::Url, previous_count: usize) -> RedirectDecision {
    if url.scheme() != "https" {
        RedirectDecision::RejectInsecureScheme
    } else if previous_count > MAX_MODEL_REDIRECTS {
        RedirectDecision::RejectLimit
    } else {
        RedirectDecision::Follow
    }
}

impl ReqwestLocalTtsDownloadTransport {
    pub(super) fn new() -> Result<Self, LocalTtsInstallError> {
        let redirect = reqwest::redirect::Policy::custom(|attempt| {
            match redirect_decision(attempt.url(), attempt.previous().len()) {
                RedirectDecision::Follow => attempt.follow(),
                RedirectDecision::RejectInsecureScheme => {
                    attempt.error("Local TTS redirect target must use HTTPS")
                }
                RedirectDecision::RejectLimit => attempt.error("Local TTS redirect limit exceeded"),
            }
        });
        let client = reqwest::Client::builder()
            .redirect(redirect)
            .build()
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
                response.map_err(|_| LocalTtsInstallError::network())?
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
            let chunk = chunk.map_err(|_| LocalTtsInstallError::network())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_policy_rejects_insecure_targets_and_excess_hops() {
        let https = reqwest::Url::parse("https://example.com/model").unwrap();
        let http = reqwest::Url::parse("http://example.com/model").unwrap();
        assert_eq!(redirect_decision(&https, 0), RedirectDecision::Follow);
        assert_eq!(
            redirect_decision(&http, 0),
            RedirectDecision::RejectInsecureScheme
        );
        assert_eq!(
            redirect_decision(&https, MAX_MODEL_REDIRECTS + 1),
            RedirectDecision::RejectLimit
        );
    }
}
