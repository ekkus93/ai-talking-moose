//! Real, model-free FFI invocation for cross-platform native link smoke.
use super::{NativeWhisperApi, WhisperApi};

/// Call a function from the pinned native whisper.cpp library and validate
/// its response; compile-only native link is not sufficient to pass this gate.
pub fn runtime_version() -> Result<String, String> {
    NativeWhisperApi
        .runtime_version()
        .map_err(|error| error.message)
        .and_then(|version| {
            if version.trim().is_empty() {
                Err("Native Whisper runtime returned an empty version".to_string())
            } else {
                Ok(version)
            }
        })
}
