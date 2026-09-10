use super::LocalTtsInstallError;
use ring::digest::{Context as Sha256Context, SHA256};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

const VERIFY_BUFFER_BYTES: usize = 1024 * 1024;

pub(super) async fn verify_artifact_async(
    path: PathBuf,
    expected_bytes: u64,
    expected_sha256: String,
    cancellation: CancellationToken,
) -> Result<(), LocalTtsInstallError> {
    let worker_cancellation = cancellation.clone();
    let result = tokio::task::spawn_blocking(move || {
        verify_artifact_cancellable(
            &path,
            expected_bytes,
            &expected_sha256,
            &worker_cancellation,
        )
    })
    .await
    .map_err(|_| LocalTtsInstallError::io("verify a downloaded Local TTS artifact"))?;
    if cancellation.is_cancelled() {
        return Err(LocalTtsInstallError::cancelled());
    }
    result
}

fn verify_artifact_cancellable(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    cancellation: &CancellationToken,
) -> Result<(), LocalTtsInstallError> {
    if cancellation.is_cancelled() {
        return Err(LocalTtsInstallError::cancelled());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| LocalTtsInstallError::io("inspect a downloaded Local TTS artifact"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LocalTtsInstallError::corrupt_install());
    }
    if metadata.len() != expected_bytes {
        return Err(LocalTtsInstallError::size_mismatch());
    }
    let file = fs::File::open(path)
        .map_err(|_| LocalTtsInstallError::io("open a downloaded Local TTS artifact"))?;
    let mut reader = BufReader::with_capacity(VERIFY_BUFFER_BYTES, file);
    let mut context = Sha256Context::new(&SHA256);
    let mut buffer = vec![0_u8; VERIFY_BUFFER_BYTES];
    loop {
        if cancellation.is_cancelled() {
            return Err(LocalTtsInstallError::cancelled());
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|_| LocalTtsInstallError::io("verify a downloaded Local TTS artifact"))?;
        if read == 0 {
            break;
        }
        if cancellation.is_cancelled() {
            return Err(LocalTtsInstallError::cancelled());
        }
        context.update(&buffer[..read]);
    }
    if cancellation.is_cancelled() {
        return Err(LocalTtsInstallError::cancelled());
    }
    let actual = context.finish();
    let actual_hex = actual
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual_hex != expected_sha256 {
        return Err(LocalTtsInstallError::sha256_mismatch());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn verifier_rejects_wrong_size_and_wrong_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("artifact.bin");
        fs::write(&path, b"abc").unwrap();
        let wrong_size =
            verify_artifact_cancellable(&path, 4, "00", &CancellationToken::new()).unwrap_err();
        assert_eq!(
            wrong_size.kind,
            super::super::LocalTtsInstallErrorKind::SizeMismatch
        );

        let wrong_hash = verify_artifact_cancellable(
            &path,
            3,
            "0000000000000000000000000000000000000000000000000000000000000000",
            &CancellationToken::new(),
        )
        .unwrap_err();
        assert_eq!(
            wrong_hash.kind,
            super::super::LocalTtsInstallErrorKind::Sha256Mismatch
        );
    }
}
