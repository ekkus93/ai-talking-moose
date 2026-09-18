pub use super::wake_word::policy::{
    DEFAULT_WAKE_PHRASE, V1_KWS_CHANNELS, V1_KWS_FEATURE_DIM, V1_KWS_KEYWORD,
    V1_KWS_SAMPLE_RATE_HZ, V1_KWS_THREADS, V1_WAKE_SCORE, V1_WAKE_THRESHOLD,
};
use crate::asr::wake_word_sherpa_manifest::{
    SHERPA_KWS_KEYWORD_BYTES, SHERPA_KWS_KEYWORD_FILE, SHERPA_KWS_KEYWORD_REPRESENTATION,
    SHERPA_KWS_KEYWORD_SHA256, SHERPA_KWS_REQUIRED_FILES, V1_SHERPA_KWS_MODEL_FILES,
};
use ring::digest::{Context, SHA256};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
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
        relative_path: "sherpa-onnx/native/linux-x64/libonnxruntime.so",
        bytes: 27_026_609,
        sha256: "4b3607aebd1784b26b6f9b20e4bd974c7ab8287043e4d095cb7d2cb40b5e566e",
        architecture: Some(NativeArchitecture::ElfX86_64),
    },
    VerifiedArtifact {
        relative_path: "sherpa-onnx/native/linux-x64/libsherpa-onnx-jni.so",
        bytes: 5_166_360,
        sha256: "adcabd1866f667ec78796a504ff96030eff30fbd80792e892752c64a861bf231",
        architecture: Some(NativeArchitecture::ElfX86_64),
    },
];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const V1_RUNTIME_PLATFORM: &str = "macos-arm64";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const V1_RUNTIME_FILES: [VerifiedArtifact; 2] = [
    VerifiedArtifact {
        relative_path: "sherpa-onnx/native/osx-aarch64/libonnxruntime.dylib",
        bytes: 29_006_384,
        sha256: "b0613d0ae53199a83b05fa48e169211498e9d40d54beaa372068ebe5ec5b0929",
        architecture: Some(NativeArchitecture::MachOArm64),
    },
    VerifiedArtifact {
        relative_path: "sherpa-onnx/native/osx-aarch64/libsherpa-onnx-jni.dylib",
        bytes: 4_218_024,
        sha256: "e8025656a2680b838dd7ccd7d7ee7e88e5da42a35ad010d8717c22dd7b851ca1",
        architecture: Some(NativeArchitecture::MachOArm64),
    },
];

const SHERPA_KWS_C_API_SYMBOLS: [&str; 7] = [
    "SherpaOnnxCreateKeywordSpotter",
    "SherpaOnnxCreateOnlineStream",
    "SherpaOnnxOnlineStreamAcceptWaveform",
    "SherpaOnnxDecodeKeywordSpotter",
    "SherpaOnnxGetKeywordResult",
    "SherpaOnnxDestroyOnlineStream",
    "SherpaOnnxDestroyKeywordSpotter",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeCapiContract {
    library_relative_path: &'static str,
    required_symbols: &'static [&'static str; 7],
}

fn native_capi_contract() -> Result<NativeCapiContract, WakeWordError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok(NativeCapiContract {
            library_relative_path: "sherpa-onnx/native/linux-x64/libsherpa-onnx-c-api.so",
            required_symbols: &SHERPA_KWS_C_API_SYMBOLS,
        })
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Ok(NativeCapiContract {
            library_relative_path: "sherpa-onnx/native/osx-aarch64/libsherpa-onnx-c-api.dylib",
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

#[derive(Debug, Clone)]
pub struct NativeKwsSession {
    config: SherpaKwsConfig,
    paths: NativeKwsSessionPaths,
    native_api_checked: bool,
    shutdown: bool,
}

impl NativeKwsSession {
    pub fn new(paths: NativeKwsSessionPaths) -> Result<Self, WakeWordError> {
        let config = SherpaKwsConfig::default();
        config.validate()?;
        verify_model_artifacts(&paths.model_dir)?;
        verify_runtime_artifacts(&paths.runtime_dir)?;
        Ok(Self {
            config,
            paths,
            native_api_checked: false,
            shutdown: false,
        })
    }

    pub fn paths(&self) -> &NativeKwsSessionPaths {
        &self.paths
    }

    pub fn runtime_platform(&self) -> &'static str {
        runtime_platform_name()
    }

    pub fn keyword_representation(&self) -> &'static str {
        SHERPA_KWS_KEYWORD_REPRESENTATION
    }

    pub fn required_native_c_api_symbols(&self) -> &'static [&'static str; 7] {
        &SHERPA_KWS_C_API_SYMBOLS
    }

    fn verify_native_c_api_ready(&mut self) -> Result<(), WakeWordError> {
        if self.native_api_checked {
            return Ok(());
        }
        verify_native_c_api_symbols(&self.paths.runtime_dir)?;
        self.native_api_checked = true;
        Ok(())
    }
}

impl SherpaKwsEngine for NativeKwsSession {
    fn config(&self) -> &SherpaKwsConfig {
        &self.config
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        validate_pcm_frame(sample_rate_hz, samples)?;
        if self.shutdown {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::Cancelled,
                "wake KWS native session is shut down",
                false,
            ));
        }
        self.verify_native_c_api_ready()?;
        Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "wake KWS native inference loop is not implemented yet",
            true,
        ))
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        if self.shutdown {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::Cancelled,
                "wake KWS native session is shut down",
                false,
            ));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        self.shutdown = true;
        Ok(())
    }
}

fn verify_native_c_api_symbols(runtime_dir: &Path) -> Result<(), WakeWordError> {
    let contract = native_capi_contract()?;
    let library_path = runtime_dir.join(contract.library_relative_path);
    if !library_path.is_file() {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "missing required Wake Word native C API library",
            true,
        ));
    }
    verify_dynamic_symbols(&library_path, contract.required_symbols)
}

#[cfg(unix)]
fn verify_dynamic_symbols(
    library_path: &Path,
    required_symbols: &[&str],
) -> Result<(), WakeWordError> {
    let path = library_path.to_str().ok_or_else(|| {
        WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native library path is not valid UTF-8",
            false,
        )
    })?;
    let c_path = CString::new(path).map_err(|_| {
        WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native library path is invalid",
            false,
        )
    })?;
    unsafe {
        let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        if handle.is_null() {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "failed to load Wake Word native C API library",
                true,
            ));
        }
        let close_guard = DynamicLibraryHandle(handle);
        for symbol in required_symbols {
            let c_symbol = CString::new(*symbol).map_err(|_| {
                WakeWordError::sanitized(
                    WakeWordErrorKind::RuntimeUnavailable,
                    "Wake Word native C API symbol is invalid",
                    false,
                )
            })?;
            if libc::dlsym(close_guard.0, c_symbol.as_ptr()).is_null() {
                return Err(WakeWordError::sanitized(
                    WakeWordErrorKind::RuntimeUnavailable,
                    "Wake Word native C API is missing a required symbol",
                    true,
                ));
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
struct DynamicLibraryHandle(*mut libc::c_void);

#[cfg(unix)]
impl Drop for DynamicLibraryHandle {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self.0);
        }
    }
}

#[cfg(not(unix))]
fn verify_dynamic_symbols(
    _library_path: &Path,
    _required_symbols: &[&str],
) -> Result<(), WakeWordError> {
    Err(WakeWordError::sanitized(
        WakeWordErrorKind::RuntimeUnavailable,
        "Wake Word native C API loading is unsupported on this platform",
        false,
    ))
}

pub fn validate_pcm_frame(sample_rate_hz: u32, samples: &[i16]) -> Result<(), WakeWordError> {
    if sample_rate_hz != V1_KWS_SAMPLE_RATE_HZ {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidConfiguration,
            "wake KWS frame sample rate must be 16000 Hz",
            false,
        ));
    }
    if samples.is_empty() {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidConfiguration,
            "wake KWS frame must contain at least one sample",
            false,
        ));
    }
    Ok(())
}

fn verify_model_artifacts(model_dir: &Path) -> Result<(), WakeWordError> {
    for file in V1_SHERPA_KWS_MODEL_FILES {
        verify_file_identity(
            &model_dir.join(file.name),
            file.bytes,
            file.sha256,
            WakeWordErrorKind::MissingArtifact,
            None,
        )?;
    }
    verify_file_identity(
        &model_dir.join(SHERPA_KWS_KEYWORD_FILE),
        SHERPA_KWS_KEYWORD_BYTES,
        SHERPA_KWS_KEYWORD_SHA256,
        WakeWordErrorKind::MissingArtifact,
        None,
    )
}

fn verify_runtime_artifacts(runtime_dir: &Path) -> Result<(), WakeWordError> {
    let runtime_files = runtime_files()?;
    for file in runtime_files {
        verify_file_identity(
            &runtime_dir.join(file.relative_path),
            file.bytes,
            file.sha256,
            WakeWordErrorKind::RuntimeUnavailable,
            file.architecture,
        )?;
    }
    Ok(())
}

fn runtime_files() -> Result<&'static [VerifiedArtifact], WakeWordError> {
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    ))]
    {
        Ok(&V1_RUNTIME_FILES)
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

fn runtime_platform_name() -> &'static str {
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    ))]
    {
        V1_RUNTIME_PLATFORM
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        "unsupported"
    }
}

fn verify_file_identity(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    missing_kind: WakeWordErrorKind,
    expected_architecture: Option<NativeArchitecture>,
) -> Result<(), WakeWordError> {
    let mut file = File::open(path).map_err(|_| {
        WakeWordError::sanitized(
            missing_kind.clone(),
            "missing required wake artifact",
            false,
        )
    })?;
    let metadata = file.metadata().map_err(|_| {
        WakeWordError::sanitized(
            WakeWordErrorKind::InvalidArtifact,
            "invalid wake artifact",
            false,
        )
    })?;
    if metadata.len() != expected_bytes {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidArtifact,
            "wake artifact identity mismatch",
            false,
        ));
    }

    let mut context = Context::new(&SHA256);
    let mut header = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|_| {
            WakeWordError::sanitized(
                WakeWordErrorKind::InvalidArtifact,
                "failed to read wake artifact",
                false,
            )
        })?;
        if count == 0 {
            break;
        }
        if header.len() < 64 {
            let needed = 64 - header.len();
            header.extend_from_slice(&buffer[..count.min(needed)]);
        }
        context.update(&buffer[..count]);
    }
    let digest = hex_digest(context.finish().as_ref());
    if digest != expected_sha256 {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidArtifact,
            "wake artifact identity mismatch",
            false,
        ));
    }
    if let Some(expected) = expected_architecture {
        verify_native_architecture(&header, expected)?;
    }
    Ok(())
}

fn verify_native_architecture(
    header: &[u8],
    expected: NativeArchitecture,
) -> Result<(), WakeWordError> {
    let actual = if header.len() >= 20
        && &header[..4] == b"\x7fELF"
        && header.get(4..6) == Some(&[2, 1])
        && u16::from_le_bytes([header[18], header[19]]) == 62
    {
        Some(NativeArchitecture::ElfX86_64)
    } else if header.len() >= 8
        && &header[..4] == b"\xcf\xfa\xed\xfe"
        && u32::from_le_bytes([header[4], header[5], header[6], header[7]]) == 0x0100000C
    {
        Some(NativeArchitecture::MachOArm64)
    } else {
        None
    };
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "native runtime architecture mismatch",
            false,
        ))
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn sanitize_error_message(message: &str) -> String {
    let mut sanitized = String::with_capacity(message.len());
    for token in message.split_whitespace() {
        let looks_like_path = token.contains('/') || token.contains('\\');
        let looks_like_secret = token.len() >= 24
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
        if looks_like_path {
            sanitized.push_str("<path>");
        } else if looks_like_secret {
            sanitized.push_str("<redacted>");
        } else {
            sanitized.push_str(token);
        }
        sanitized.push(' ');
    }
    sanitized.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn default_config_freezes_v1_sherpa_policy() {
        let config = SherpaKwsConfig::default();
        assert_eq!(config.sample_rate_hz, 16_000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.feature_dim, 80);
        assert_eq!(config.threads, 1);
        assert_eq!(config.keyword, "HEY MOOSE");
        assert_eq!(config.score, 1.0);
        assert_eq!(config.threshold, 0.25);
        config.validate().unwrap();
    }

    #[test]
    fn config_rejects_drift_from_frozen_v1_policy() {
        let config = SherpaKwsConfig {
            sample_rate_hz: 48_000,
            ..Default::default()
        };
        assert_eq!(
            config.validate().unwrap_err().kind,
            WakeWordErrorKind::InvalidConfiguration
        );

        let config = SherpaKwsConfig {
            threads: 2,
            ..Default::default()
        };
        assert_eq!(
            config.validate().unwrap_err().message,
            "wake KWS V1 must use one inference thread"
        );

        let config = SherpaKwsConfig {
            channels: 2,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = SherpaKwsConfig {
            score: 0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = SherpaKwsConfig {
            keyword: "HEY BRUCE".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = SherpaKwsConfig {
            threshold: 0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn engine_artifact_contract_matches_manifest_exactly() {
        let config = SherpaKwsConfig::default();
        assert_eq!(
            config.required_artifact_files(),
            &[
                "encoder-epoch-12-avg-2-chunk-16-left-64.onnx",
                "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
                "joiner-epoch-12-avg-2-chunk-16-left-64.onnx",
                "tokens.txt",
                "bpe.model",
            ]
        );
    }

    #[test]
    fn pcm_frames_must_be_canonical_nonempty_mono_stream_samples() {
        validate_pcm_frame(16_000, &[0, 1, -1]).unwrap();
        assert!(validate_pcm_frame(48_000, &[0]).is_err());
        assert!(validate_pcm_frame(16_000, &[]).is_err());
    }

    #[test]
    fn detection_event_is_bounded_and_never_contains_audio() {
        let detection = WakeWordDetection::v1_detected(0.77);
        assert_eq!(detection.keyword, DEFAULT_WAKE_PHRASE);
        assert_eq!(detection.score, 0.77);
    }

    #[test]
    fn errors_sanitize_paths_and_token_like_secrets() {
        let error = WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "failed /tmp/private/model.onnx token abcdefghijklmnopqrstuvwxyz123456",
            true,
        );
        assert_eq!(error.message, "failed <path> token <redacted>");
        assert!(error.retryable);
    }

    struct FakeEngine {
        config: SherpaKwsConfig,
        triggered: bool,
        shutdowns: u8,
    }

    impl SherpaKwsEngine for FakeEngine {
        fn config(&self) -> &SherpaKwsConfig {
            &self.config
        }

        fn accept_pcm16_mono(
            &mut self,
            sample_rate_hz: u32,
            samples: &[i16],
        ) -> Result<Option<WakeWordDetection>, WakeWordError> {
            validate_pcm_frame(sample_rate_hz, samples)?;
            if self.triggered {
                Ok(None)
            } else {
                self.triggered = true;
                Ok(Some(WakeWordDetection::v1_detected(V1_WAKE_SCORE)))
            }
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
            self.triggered = false;
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            self.shutdowns = self.shutdowns.saturating_add(1);
            Ok(())
        }
    }

    struct CountingEngine {
        config: SherpaKwsConfig,
        accepted_batches: usize,
        accepted_samples: usize,
    }

    impl SherpaKwsEngine for CountingEngine {
        fn config(&self) -> &SherpaKwsConfig {
            &self.config
        }

        fn accept_pcm16_mono(
            &mut self,
            sample_rate_hz: u32,
            samples: &[i16],
        ) -> Result<Option<WakeWordDetection>, WakeWordError> {
            validate_pcm_frame(sample_rate_hz, samples)?;
            self.accepted_batches = self.accepted_batches.saturating_add(1);
            self.accepted_samples = self.accepted_samples.saturating_add(samples.len());
            Ok(None)
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            Ok(())
        }
    }

    #[test]
    fn invalid_pcm_is_rejected_before_kws_feed_mutation() {
        let mut engine = CountingEngine {
            config: SherpaKwsConfig::default(),
            accepted_batches: 0,
            accepted_samples: 0,
        };

        assert!(engine.accept_pcm16_mono(48_000, &[1, 2]).is_err());
        assert!(engine.accept_pcm16_mono(16_000, &[]).is_err());
        assert_eq!(engine.accepted_batches, 0);
        assert_eq!(engine.accepted_samples, 0);

        assert!(engine.accept_pcm16_mono(16_000, &[1, 2]).unwrap().is_none());
        assert_eq!(engine.accepted_batches, 1);
        assert_eq!(engine.accepted_samples, 2);
    }

    #[test]
    fn native_session_rejects_missing_verified_artifacts_before_creation() {
        let temp = TempDir::new().unwrap();
        let error = NativeKwsSession::new(NativeKwsSessionPaths {
            model_dir: temp.path().join("model"),
            runtime_dir: temp.path().join("runtime"),
        })
        .unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::MissingArtifact);
        assert!(!error
            .message
            .contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn native_session_rejects_corrupt_verified_artifact_before_creation() {
        let temp = TempDir::new().unwrap();
        let model_dir = temp.path().join("model");
        fs::create_dir_all(&model_dir).unwrap();
        for file in V1_SHERPA_KWS_MODEL_FILES {
            fs::write(model_dir.join(file.name), b"corrupt").unwrap();
        }
        fs::write(
            model_dir.join(SHERPA_KWS_KEYWORD_FILE),
            SHERPA_KWS_KEYWORD_REPRESENTATION.as_bytes(),
        )
        .unwrap();

        let error = NativeKwsSession::new(NativeKwsSessionPaths {
            model_dir,
            runtime_dir: temp.path().join("runtime"),
        })
        .unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::InvalidArtifact);
        assert_eq!(error.message, "wake artifact identity mismatch");
    }

    #[test]
    fn native_runtime_architecture_check_is_sanitized() {
        let error =
            verify_native_architecture(&[0_u8; 64], NativeArchitecture::ElfX86_64).unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
        assert_eq!(error.message, "native runtime architecture mismatch");
    }

    #[test]
    fn native_session_exposes_frozen_policy_without_network() {
        let paths = NativeKwsSessionPaths {
            model_dir: PathBuf::from("model"),
            runtime_dir: PathBuf::from("runtime"),
        };
        let session = NativeKwsSession {
            config: SherpaKwsConfig::default(),
            paths: paths.clone(),
            native_api_checked: false,
            shutdown: false,
        };
        assert_eq!(session.paths(), &paths);
        assert_eq!(session.config().threads, 1);
        assert_eq!(session.config().threshold, 0.25);
        assert_eq!(session.config().score, 1.0);
        assert_eq!(session.keyword_representation(), "▁HE Y ▁MO O SE");
        assert!(!session.runtime_platform().is_empty());
    }

    #[test]
    fn native_c_api_contract_is_kws_only_and_privacy_bounded() {
        let paths = NativeKwsSessionPaths {
            model_dir: PathBuf::from("model"),
            runtime_dir: PathBuf::from("runtime"),
        };
        let session = NativeKwsSession {
            config: SherpaKwsConfig::default(),
            paths,
            native_api_checked: false,
            shutdown: false,
        };
        let symbols = session.required_native_c_api_symbols();
        assert!(symbols.contains(&"SherpaOnnxCreateKeywordSpotter"));
        assert!(symbols.contains(&"SherpaOnnxDecodeKeywordSpotter"));
        assert!(!symbols.iter().any(|symbol| symbol.contains("Transducer")));
        assert!(!symbols.iter().any(|symbol| symbol.contains("OfflineRecognizer")));
        assert!(!symbols.iter().any(|symbol| symbol.contains("Whisper")));
    }

    #[test]
    fn missing_native_c_api_library_is_sanitized_before_inference() {
        let temp = TempDir::new().unwrap();
        let error = verify_native_c_api_symbols(temp.path()).unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
        assert_eq!(
            error.message,
            "missing required Wake Word native C API library"
        );
        assert!(error.retryable);
        assert!(!error.message.contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn native_session_shutdown_is_idempotent_and_feed_errors_are_sanitized() {
        let mut session = NativeKwsSession {
            config: SherpaKwsConfig::default(),
            paths: NativeKwsSessionPaths {
                model_dir: PathBuf::from("model"),
                runtime_dir: PathBuf::from("runtime"),
            },
            native_api_checked: false,
            shutdown: false,
        };
        let error = session.accept_pcm16_mono(16_000, &[1, 2]).unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
        assert_eq!(
            error.message,
            "missing required Wake Word native C API library"
        );
        assert!(error.retryable);
        session.shutdown().unwrap();
        session.shutdown().unwrap();
        assert_eq!(
            session.accept_pcm16_mono(16_000, &[1]).unwrap_err().kind,
            WakeWordErrorKind::Cancelled
        );
    }

    #[test]
    fn engine_boundary_supports_feed_reset_and_idempotent_shutdown_contract() {
        let mut engine = FakeEngine {
            config: SherpaKwsConfig::default(),
            triggered: false,
            shutdowns: 0,
        };
        engine.config().validate().unwrap();
        assert!(engine
            .accept_pcm16_mono(16_000, &[1, 2, 3])
            .unwrap()
            .is_some());
        assert!(engine
            .accept_pcm16_mono(16_000, &[1, 2, 3])
            .unwrap()
            .is_none());
        engine.reset_stream().unwrap();
        assert!(engine
            .accept_pcm16_mono(16_000, &[1, 2, 3])
            .unwrap()
            .is_some());
        engine.shutdown().unwrap();
        engine.shutdown().unwrap();
        assert_eq!(engine.shutdowns, 2);
    }
}
