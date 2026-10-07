use super::types::LocalModelInstallError;
use super::{VerificationObserver, VERIFY_BUFFER_BYTES};
use ring::digest::{Context as Sha256Context, SHA256};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub(super) async fn verify_artifact_async(
    path: PathBuf,
    expected_bytes: u64,
    expected_sha256: String,
    cancellation: CancellationToken,
    observer: Option<VerificationObserver>,
) -> Result<(), LocalModelInstallError> {
    let worker_cancellation = cancellation.clone();
    let result = tokio::task::spawn_blocking(move || {
        verify_artifact_cancellable(
            &path,
            expected_bytes,
            &expected_sha256,
            &worker_cancellation,
            observer.as_ref(),
        )
    })
    .await
    .map_err(|_| LocalModelInstallError::io("verify the downloaded local model"))?;

    if cancellation.is_cancelled() {
        return Err(LocalModelInstallError::cancelled());
    }
    result
}

#[cfg(test)]
pub(super) fn verify_artifact(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<(), LocalModelInstallError> {
    verify_artifact_cancellable(
        path,
        expected_bytes,
        expected_sha256,
        &CancellationToken::new(),
        None,
    )
}

pub(super) fn verify_artifact_cancellable(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    cancellation: &CancellationToken,
    observer: Option<&VerificationObserver>,
) -> Result<(), LocalModelInstallError> {
    if cancellation.is_cancelled() {
        return Err(LocalModelInstallError::cancelled());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| LocalModelInstallError::io("inspect the downloaded local model"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LocalModelInstallError::corrupt_install());
    }
    let file = fs::File::open(path)
        .map_err(|_| LocalModelInstallError::io("open the downloaded local model"))?;
    if file
        .metadata()
        .map_err(|_| LocalModelInstallError::io("inspect the downloaded local model"))?
        .len()
        != expected_bytes
    {
        return Err(LocalModelInstallError::size_mismatch());
    }
    let mut reader = BufReader::with_capacity(VERIFY_BUFFER_BYTES, file);
    let mut context = Sha256Context::new(&SHA256);
    let mut buffer = vec![0_u8; VERIFY_BUFFER_BYTES];
    loop {
        if cancellation.is_cancelled() {
            return Err(LocalModelInstallError::cancelled());
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|_| LocalModelInstallError::io("verify the downloaded local model"))?;
        if read == 0 {
            break;
        }
        if let Some(observer) = observer {
            observer();
        }
        if cancellation.is_cancelled() {
            return Err(LocalModelInstallError::cancelled());
        }
        context.update(&buffer[..read]);
    }
    if cancellation.is_cancelled() {
        return Err(LocalModelInstallError::cancelled());
    }
    let actual = context.finish();
    let actual_hex = actual
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual_hex != expected_sha256 {
        return Err(LocalModelInstallError::sha256_mismatch());
    }
    Ok(())
}
