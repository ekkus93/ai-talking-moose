//! Whisper.cpp local ASR support.
//!
//! Whisper is a local-only command-ASR engine. It shares the bounded microphone
//! queue and transcript state machine with Moonshine but runs a batch whisper.cpp
//! inference over the queued PCM instead of a streaming endpoint. The whisper.cpp
//! C runtime is built from source via CMake in `src-tauri/build.rs`; model weights
//! are user-downloaded and never committed.
pub mod acceptance;
pub(crate) mod engine;
pub(crate) mod ffi;
pub(crate) mod installer;
pub(crate) mod manifest;

pub(crate) use ffi::{path_to_cstring, NativeWhisperApi, WhisperApi, WhisperModel, WhisperSegment};
pub(crate) use installer::WhisperModelInstaller;

use crate::asr::types::{AsrMode, AsrModelDescriptor};

#[allow(dead_code)]
pub(crate) fn descriptor(mode: &AsrMode) -> AsrModelDescriptor {
    match mode {
        AsrMode::WhisperSmall => manifest::model_descriptor(true),
        _ => AsrModelDescriptor::default(),
    }
}
