pub use super::wake_word::policy::{
    DEFAULT_WAKE_PHRASE, V1_KWS_CHANNELS, V1_KWS_FEATURE_DIM, V1_KWS_KEYWORD,
    V1_KWS_SAMPLE_RATE_HZ, V1_KWS_THREADS, V1_WAKE_SCORE, V1_WAKE_THRESHOLD,
};
use crate::asr::wake_word_sherpa_manifest::{
    SHERPA_KWS_KEYWORD_BYTES, SHERPA_KWS_KEYWORD_FILE, SHERPA_KWS_KEYWORD_REPRESENTATION,
    SHERPA_KWS_KEYWORD_SHA256, SHERPA_KWS_REQUIRED_FILES, V1_SHERPA_KWS_MODEL_FILES,
};
use ring::digest::{Context, SHA256};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Read;
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeWordErrorKind {
    MissingArtifact,
    InvalidArtifact,
    InvalidConfiguration,
    RuntimeUnavailable,
    Inference,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeWordError {
    pub kind: WakeWordErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl WakeWordError {
    pub fn sanitized(kind: WakeWordErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        let message = sanitize_error_message(&message.into());
        Self {
            kind,
            message,
            retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SherpaKwsConfig {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub feature_dim: u16,
    pub threads: u16,
    pub keyword: String,
    pub score: f32,
    pub threshold: f32,
}

impl Default for SherpaKwsConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: V1_KWS_SAMPLE_RATE_HZ,
            channels: V1_KWS_CHANNELS,
            feature_dim: V1_KWS_FEATURE_DIM,
            threads: V1_KWS_THREADS,
            keyword: V1_KWS_KEYWORD.to_string(),
            score: V1_WAKE_SCORE,
            threshold: V1_WAKE_THRESHOLD,
        }
    }
}

impl SherpaKwsConfig {
    pub fn required_artifact_files(&self) -> &'static [&'static str; 5] {
        &SHERPA_KWS_REQUIRED_FILES
    }

    pub fn validate(&self) -> Result<(), WakeWordError> {
        if self.sample_rate_hz != V1_KWS_SAMPLE_RATE_HZ {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS sample rate must be 16000 Hz",
                false,
            ));
        }
        if self.channels != V1_KWS_CHANNELS {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS input must be mono",
                false,
            ));
        }
        if self.feature_dim != V1_KWS_FEATURE_DIM {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS feature dimension must be 80",
                false,
            ));
        }
        if self.threads != V1_KWS_THREADS {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS V1 must use one inference thread",
                false,
            ));
        }
        if !self.keyword.trim().eq_ignore_ascii_case(V1_KWS_KEYWORD) {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                format!("wake KWS keyword must match {DEFAULT_WAKE_PHRASE}"),
                false,
            ));
        }
        if !self.score.is_finite() || self.score != V1_WAKE_SCORE {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS V1 score must be 1.0",
                false,
            ));
        }
        if !self.threshold.is_finite() || self.threshold != V1_WAKE_THRESHOLD {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS V1 threshold must be 0.25",
                false,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WakeWordDetection {
    pub keyword: String,
    pub score: f32,
}

impl WakeWordDetection {
    pub fn v1_detected(score: f32) -> Self {
        Self {
            keyword: DEFAULT_WAKE_PHRASE.to_string(),
            score,
        }
    }
}

pub trait SherpaKwsEngine {
    fn config(&self) -> &SherpaKwsConfig;
    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError>;
    fn reset_stream(&mut self) -> Result<(), WakeWordError>;
    fn shutdown(&mut self) -> Result<(), WakeWordError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerifiedArtifact {
    relative_path: &'static str,
    bytes: u64,
    sha256: &'static str,
    architecture: Option<NativeArchitecture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeArchitecture {
    ElfX86_64,
    MachOArm64,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const V1_RUNTIME_PLATFORM: &str = "linux-x86_64";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const V1_RUNTIME_FILES: [VerifiedArtifact; 2] = [
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-linux-x64-shared/lib/libonnxruntime.so",
        bytes: 27_026_609,
        sha256: "4b3607aebd1784b26b6f9b20e4bd974c7ab8287043e4d095cb7d2cb40b5e566e",
        architecture: Some(NativeArchitecture::ElfX86_64),
    },
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-linux-x64-shared/lib/libsherpa-onnx-c-api.so",
        bytes: 5_124_192,
        sha256: "b8351ca1632571ac108adbb317bcc4bf7cfe84b72690e3017316b0da3e1e344f",
        architecture: Some(NativeArchitecture::ElfX86_64),
    },
];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const V1_RUNTIME_PLATFORM: &str = "macos-arm64";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const V1_RUNTIME_FILES: [VerifiedArtifact; 2] = [
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-osx-arm64-shared/lib/libonnxruntime.dylib",
        bytes: 28_775_120,
        sha256: "3567d114f7299d559993e536d605a6f46d7bc9d2542004accc80ee9bf5457f0b",
        architecture: Some(NativeArchitecture::MachOArm64),
    },
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-osx-arm64-shared/lib/libsherpa-onnx-c-api.dylib",
        bytes: 4_172_832,
        sha256: "ee098d8b419d49b92101cde3c970a333b361066eb2d79a11ab480a116552b908",
        architecture: Some(NativeArchitecture::MachOArm64),
    },
];

const SHERPA_KWS_C_API_SYMBOLS: [&str; 10] = [
    "SherpaOnnxCreateKeywordSpotter",
    "SherpaOnnxCreateKeywordStream",
    "SherpaOnnxOnlineStreamAcceptWaveform",
    "SherpaOnnxIsKeywordStreamReady",
    "SherpaOnnxDecodeKeywordStream",
    "SherpaOnnxResetKeywordStream",
    "SherpaOnnxGetKeywordResult",
    "SherpaOnnxDestroyKeywordResult",
    "SherpaOnnxDestroyOnlineStream",
    "SherpaOnnxDestroyKeywordSpotter",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeCapiContract {
    library_relative_path: &'static str,
    required_symbols: &'static [&'static str; 10],
}

fn native_capi_contract() -> Result<NativeCapiContract, WakeWordError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok(NativeCapiContract {
            library_relative_path:
                "sherpa-onnx-v1.13.8-linux-x64-shared/lib/libsherpa-onnx-c-api.so",
            required_symbols: &SHERPA_KWS_C_API_SYMBOLS,
        })
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Ok(NativeCapiContract {
            library_relative_path:
                "sherpa-onnx-v1.13.8-osx-arm64-shared/lib/libsherpa-onnx-c-api.dylib",
            required_symbols: &SHERPA_KWS_C_API_SYMBOLS,
        })
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native runtime is unsupported on this platform",
            false,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeKwsSessionPaths {
    pub model_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

#[derive(Debug)]
pub struct NativeKwsSession {
    config: SherpaKwsConfig,
    paths: NativeKwsSessionPaths,
    native_runtime: Option<NativeKwsRuntime>,
    shutdown: bool,
}
