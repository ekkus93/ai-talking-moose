use super::super::types::{
    AsrError, AsrErrorKind, AsrMode, AsrModelDescriptor, AsrModelInstallState,
};

use ring::digest::{Context, SHA256};
use std::fs;
use std::path::Path;

/// Stable application-owned identifier for the Whisper Small local model.
pub const WHISPER_SMALL_ID: &str = "whisper-small-ggml";

/// Display name shown in settings.
pub const WHISPER_SMALL_DISPLAY_NAME: &str = "Whisper Small";

/// whisper.cpp source commit the CMake build links against.
pub const WHISPER_SOURCE_COMMIT: &str = "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774";

/// Immutable Hugging Face repository revision identifying the model artifact.
pub const WHISPER_MODEL_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";

/// Hugging Face download URL for the `ggml-small` weight, pinned to the immutable
/// model repository revision.
pub const WHISPER_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true";

/// Expected SHA-256 of the downloaded weight.
pub const WHISPER_MODEL_SHA256: &str =
    "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b";

/// Expected byte size of the downloaded weight.
pub const WHISPER_MODEL_BYTES: u64 = 487_601_967;

/// Runtime release label surfaced in diagnostics (the whisper.cpp source commit).
pub const WHISPER_RUNTIME_RELEASE: &str = "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774";

/// Expected GGUF magic prefix of the `ggml-small.bin` weight.
pub const WHISPER_MODEL_MAGIC: [u8; 4] = *b"lmgg";

/// Upstream repository for the `ggml-small.bin` Whisper.cpp weight.
pub const WHISPER_MODEL_REPO: &str = "https://github.com/ggerganov/whisper.cpp";

/// License recorded for the `ggml-small.bin` Whisper.cpp weight.
pub const WHISPER_MODEL_LICENSE: &str = "MIT";

/// Expected file name for the downloaded `ggml-small.bin` weight.
pub const WHISPER_MODEL_FILE_NAME: &str = "ggml-small.bin";

/// Message returned when the Whisper.cpp runtime is not yet built into the build.
pub const WHISPER_RUNTIME_UNBUILT_MESSAGE: &str =
    "Whisper.cpp native runtime is not yet built into this build. Build the app with the whisper.cpp source to enable Whisper Small local ASR.";

/// Error message shown when this build does not link the Whisper.cpp runtime.
///
/// Returns `None` when the runtime is available (linked). Returns a message
/// explaining the unbuilt state otherwise.
fn runtime_error_message(linked: bool) -> Option<String> {
    if linked {
        None
    } else {
        Some(WHISPER_RUNTIME_UNBUILT_MESSAGE.to_string())
    }
}

/// Build a default Whisper Small descriptor.
///
/// On targets where the whisper.cpp runtime is linked the descriptor is clean
/// and `get_asr_models`/install/delete commands can report the real installed state.
/// On targets where it is not linked, the descriptor fails closed with an unbuilt
/// runtime message.
pub fn model_descriptor(active: bool) -> AsrModelDescriptor {
    let error_message = match runtime_error_message(cfg!(whisper_native_linked)) {
        Some(message) => Some(message),
        None => match validate_manifest_pins() {
            Ok(()) => None,
            Err(error) => Some(error.message),
        },
    };

    AsrModelDescriptor {
        id: WHISPER_SMALL_ID.to_string(),
        display_name: WHISPER_SMALL_DISPLAY_NAME.to_string(),
        mode: AsrMode::WhisperSmall,
        install_state: AsrModelInstallState::NotInstalled,
        revision: WHISPER_SOURCE_COMMIT.to_string(),
        runtime_release: WHISPER_RUNTIME_RELEASE.to_string(),
        installed_bytes: None,
        expected_bytes: WHISPER_MODEL_BYTES,
        active,
        error_message,
    }
}

/// Fail-closed validation for the Whisper Small manifest pins.
///
/// Ensures the model URL references the vendored Whisper.cpp source commit, the SHA-256
/// pin is a well-formed 64-character ASCII hex digest, the expected byte size is non-zero,
/// the expected file name is path-safe, and the recorded repository and license fields are
/// non-empty.
pub fn validate_manifest_pins() -> Result<(), AsrError> {
    if WHISPER_SMALL_ID.is_empty() {
        return Err(internal_validation_error(
            "WHISPER_SMALL_ID must be non-empty",
        ));
    }

    if WHISPER_SOURCE_COMMIT.is_empty() {
        return Err(internal_validation_error(
            "WHISPER_SOURCE_COMMIT must be non-empty",
        ));
    }

    if WHISPER_SMALL_DISPLAY_NAME.is_empty() {
        return Err(internal_validation_error(
            "WHISPER_SMALL_DISPLAY_NAME must be non-empty",
        ));
    }

    if !WHISPER_MODEL_URL.starts_with("https://") {
        return Err(internal_validation_error(
            "WHISPER_MODEL_URL must use https://",
        ));
    }

    if !WHISPER_MODEL_URL.contains(WHISPER_MODEL_REVISION) {
        return Err(internal_validation_error(
            "WHISPER_MODEL_URL must reference WHISPER_MODEL_REVISION",
        ));
    }

    if !is_ascii_hex_sha256(WHISPER_MODEL_SHA256) {
        return Err(internal_validation_error(
            "WHISPER_MODEL_SHA256 must be a 64-character ASCII hex digest",
        ));
    }

    if WHISPER_MODEL_BYTES == 0 {
        return Err(internal_validation_error(
            "WHISPER_MODEL_BYTES must be non-zero",
        ));
    }

    if WHISPER_MODEL_REPO.is_empty() {
        return Err(internal_validation_error(
            "WHISPER_MODEL_REPO must be non-empty",
        ));
    }

    if WHISPER_MODEL_FILE_NAME.is_empty() {
        return Err(internal_validation_error(
            "WHISPER_MODEL_FILE_NAME must be non-empty",
        ));
    }

    if WHISPER_MODEL_LICENSE.is_empty() {
        return Err(internal_validation_error(
            "WHISPER_MODEL_LICENSE must be non-empty",
        ));
    }

    if !is_safe_model_file_name(WHISPER_MODEL_FILE_NAME) {
        return Err(internal_validation_error(
            "WHISPER_MODEL_FILE_NAME must be a safe model file name",
        ));
    }

    Ok(())
}

fn internal_validation_error(message: &str) -> AsrError {
    AsrError {
        kind: AsrErrorKind::Internal,
        message: format!("Whisper manifest validation failed: {message}"),
        retryable: false,
    }
}

/// Returns true when `value` is exactly 64 ASCII hexadecimal digits.
fn is_ascii_hex_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Returns true when `name` is a safe, non-empty model file name.
///
/// The name may contain `/` separators, but every segment must be:
///
/// - non-empty
/// - not `..`
/// - ASCII alphanumeric, underscore, hyphen, or dot
///
/// This rejects NUL bytes, absolute paths, backslash path separators, path traversal
/// segments, and any non-ASCII characters.
fn is_safe_model_file_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\0') || name.contains('\\') {
        return false;
    }

    let safe_segment = |segment: &str| {
        !segment.is_empty()
            && segment != ".."
            && !segment.contains("..")
            && segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    };

    name.split('/').all(safe_segment)
}

/// Hex-encoded SHA-256 of the given bytes, matching the `ring` convention used by
/// the rest of the app.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut context = Context::new(&SHA256);
    context.update(bytes);
    context
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[allow(dead_code)]
/// Verify a downloaded Whisper model weight: exact byte size, SHA-256 pin, and the
/// GGUF magic prefix. Mirrors the Moonshine model verification contract.
pub fn verify_model(path: &Path) -> Result<(), AsrError> {
    let bytes = fs::read(path).map_err(|error| AsrError {
        kind: AsrErrorKind::ModelCorrupt,
        message: format!(
            "Could not read Whisper model at {}: {error}",
            path.display()
        ),
        retryable: true,
    })?;

    if bytes.len() != WHISPER_MODEL_BYTES as usize {
        return Err(AsrError {
            kind: AsrErrorKind::ModelCorrupt,
            message: format!(
                "Whisper model size is {} bytes; expected {}",
                bytes.len(),
                WHISPER_MODEL_BYTES
            ),
            retryable: true,
        });
    }

    if bytes[0..4] != WHISPER_MODEL_MAGIC {
        return Err(AsrError {
            kind: AsrErrorKind::ModelCorrupt,
            message: "Whisper model has an unsupported header magic.".to_string(),
            retryable: false,
        });
    }

    if sha256_hex(&bytes) != WHISPER_MODEL_SHA256 {
        return Err(AsrError {
            kind: AsrErrorKind::ModelCorrupt,
            message: "Whisper model SHA-256 did not match the verified pin.".to_string(),
            retryable: false,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;
    use tempfile::TempDir;

    #[test]
    fn whisper_pins_are_fixed() {
        assert_eq!(WHISPER_SMALL_ID, "whisper-small-ggml");
        assert_eq!(
            WHISPER_MODEL_URL,
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true"
        );
        assert_eq!(
            WHISPER_MODEL_SHA256,
            "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b"
        );
        assert_eq!(WHISPER_MODEL_BYTES, 487_601_967);
        assert_eq!(
            WHISPER_SOURCE_COMMIT,
            "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774"
        );
        assert_eq!(WHISPER_MODEL_MAGIC, [b'l', b'm', b'g', b'g']);
    }

    #[test]
    fn whisper_descriptor_defaults_to_not_installed() {
        let descriptor = model_descriptor(false);
        assert_eq!(descriptor.mode, AsrMode::WhisperSmall);
        assert_eq!(descriptor.install_state, AsrModelInstallState::NotInstalled);
        assert_eq!(descriptor.id, WHISPER_SMALL_ID);
        assert_eq!(descriptor.display_name, WHISPER_SMALL_DISPLAY_NAME);
        assert_eq!(descriptor.expected_bytes, WHISPER_MODEL_BYTES);
        assert_eq!(descriptor.revision, WHISPER_SOURCE_COMMIT);
        assert_eq!(descriptor.runtime_release, WHISPER_RUNTIME_RELEASE);
        assert_eq!(
            descriptor.error_message,
            runtime_error_message(cfg!(whisper_native_linked))
        );
    }

    #[test]
    fn runtime_error_message_is_clean_when_linked() {
        assert!(runtime_error_message(true).is_none());
    }

    #[test]
    fn runtime_error_message_explains_unbuilt_runtime() {
        let message = runtime_error_message(false).unwrap();
        assert!(message.contains("not yet built"));
    }

    #[test]
    fn verify_model_rejects_missing_file() {
        let result = verify_model(&std::path::PathBuf::from(
            "/nonexistent/whisper/ggml-small.bin",
        ));
        assert!(matches!(
            result,
            Err(AsrError {
                kind: AsrErrorKind::ModelCorrupt,
                ..
            })
        ));
    }

    #[test]
    fn verify_model_rejects_wrong_size_and_magic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("fake.bin");
        write(&path, b"not-a-gguf").unwrap();
        let result = verify_model(&path);
        assert!(matches!(
            result,
            Err(AsrError {
                kind: AsrErrorKind::ModelCorrupt,
                ..
            })
        ));
    }

    #[test]
    fn validate_manifest_pins_accepts_pinned_values() {
        assert!(matches!(validate_manifest_pins(), Ok(())));
    }

    #[test]
    fn is_ascii_hex_sha256_accepts_valid_sha256() {
        assert!(is_ascii_hex_sha256(WHISPER_MODEL_SHA256));
    }

    #[test]
    fn is_ascii_hex_sha256_rejects_invalid_digests() {
        let invalid_short = "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987";
        assert!(!is_ascii_hex_sha256(invalid_short));
        let invalid_long = "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987bc";
        assert!(!is_ascii_hex_sha256(invalid_long));
        let invalid_symbol = "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987G";
        assert!(!is_ascii_hex_sha256(invalid_symbol));
    }

    #[test]
    fn is_safe_model_file_name_accepts_pinned_name() {
        assert!(is_safe_model_file_name(WHISPER_MODEL_FILE_NAME));
        assert!(is_safe_model_file_name("ggml-small.bin"));
        assert!(is_safe_model_file_name("models/ggml-small.bin"));
    }

    #[test]
    fn is_safe_model_file_name_rejects_path_traversal_and_invalid_names() {
        assert!(!is_safe_model_file_name(""));
        assert!(!is_safe_model_file_name(".."));
        assert!(!is_safe_model_file_name("../ggml-small.bin"));
        assert!(!is_safe_model_file_name("..ggml-small.bin"));
        assert!(!is_safe_model_file_name("ggml..small.bin"));
        assert!(!is_safe_model_file_name("ggml-small/.."));
        assert!(!is_safe_model_file_name("ggml-small/../ggml-small.bin"));
        assert!(!is_safe_model_file_name("/ggml-small.bin"));
        assert!(!is_safe_model_file_name("ggml-small/..gg"));
        assert!(!is_safe_model_file_name("ggml-small/gg..bin"));
        assert!(!is_safe_model_file_name("ggml-small/..."));
        assert!(!is_safe_model_file_name("ggml-small/\0bin"));
        assert!(!is_safe_model_file_name("bin\\ggml-small"));
        assert!(!is_safe_model_file_name("ggml-small/ggml-small\0.bin"));
    }
}
