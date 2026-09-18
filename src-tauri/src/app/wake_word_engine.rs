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

impl NativeKwsSession {
    pub fn new(paths: NativeKwsSessionPaths) -> Result<Self, WakeWordError> {
        let config = SherpaKwsConfig::default();
        config.validate()?;
        verify_model_artifacts(&paths.model_dir)?;
        verify_runtime_artifacts(&paths.runtime_dir)?;
        Ok(Self {
            config,
            paths,
            native_runtime: None,
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

    pub fn required_native_c_api_symbols(&self) -> &'static [&'static str; 10] {
        &SHERPA_KWS_C_API_SYMBOLS
    }

    fn native_runtime(&mut self) -> Result<&mut NativeKwsRuntime, WakeWordError> {
        if self.native_runtime.is_none() {
            self.native_runtime = Some(NativeKwsRuntime::load(&self.paths, &self.config)?);
        }
        self.native_runtime.as_mut().ok_or_else(|| {
            WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "Wake Word native runtime was not initialized",
                true,
            )
        })
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
        self.native_runtime()?.accept_pcm16_mono(sample_rate_hz, samples)
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        if self.shutdown {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::Cancelled,
                "wake KWS native session is shut down",
                false,
            ));
        }
        if let Some(runtime) = self.native_runtime.as_ref() {
            runtime.reset_stream()?;
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        self.shutdown = true;
        self.native_runtime = None;
        Ok(())
    }
}

#[repr(C)]
struct SherpaOnnxOnlineTransducerModelConfig {
    encoder: *const c_char,
    decoder: *const c_char,
    joiner: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineParaformerModelConfig {
    encoder: *const c_char,
    decoder: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineZipformer2CtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineNemoCtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineToneCtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct SherpaOnnxOnlineModelConfig {
    transducer: SherpaOnnxOnlineTransducerModelConfig,
    paraformer: SherpaOnnxOnlineParaformerModelConfig,
    zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig,
    tokens: *const c_char,
    num_threads: c_int,
    provider: *const c_char,
    debug: c_int,
    model_type: *const c_char,
    modeling_unit: *const c_char,
    bpe_vocab: *const c_char,
    tokens_buf: *const c_char,
    tokens_buf_size: c_int,
    nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig,
    t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig,
}

#[repr(C)]
struct SherpaOnnxFeatureConfig {
    sample_rate: c_int,
    feature_dim: c_int,
}

#[repr(C)]
struct SherpaOnnxKeywordSpotterConfig {
    feat_config: SherpaOnnxFeatureConfig,
    model_config: SherpaOnnxOnlineModelConfig,
    max_active_paths: c_int,
    num_trailing_blanks: c_int,
    keywords_score: c_float,
    keywords_threshold: c_float,
    keywords_file: *const c_char,
    keywords_buf: *const c_char,
    keywords_buf_size: c_int,
}

#[repr(C)]
struct SherpaOnnxKeywordResult {
    keyword: *const c_char,
    tokens: *const c_char,
    tokens_arr: *const *const c_char,
    count: c_int,
    timestamps: *mut c_float,
    start_time: c_float,
    json: *const c_char,
}

#[repr(C)]
struct SherpaOnnxKeywordSpotter {
    _private: [u8; 0],
}

#[repr(C)]
struct SherpaOnnxOnlineStream {
    _private: [u8; 0],
}

type CreateKeywordSpotterFn =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotterConfig) -> *const SherpaOnnxKeywordSpotter;
type CreateKeywordStreamFn =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter) -> *const SherpaOnnxOnlineStream;
type AcceptWaveformFn =
    unsafe extern "C" fn(*const SherpaOnnxOnlineStream, c_int, *const c_float, c_int);
type IsKeywordStreamReadyFn =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter, *const SherpaOnnxOnlineStream) -> c_int;
type DecodeKeywordStreamFn =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter, *const SherpaOnnxOnlineStream);
type ResetKeywordStreamFn =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter, *const SherpaOnnxOnlineStream);
type GetKeywordResultFn = unsafe extern "C" fn(
    *const SherpaOnnxKeywordSpotter,
    *const SherpaOnnxOnlineStream,
) -> *const SherpaOnnxKeywordResult;
type DestroyKeywordResultFn = unsafe extern "C" fn(*const SherpaOnnxKeywordResult);
type DestroyOnlineStreamFn = unsafe extern "C" fn(*const SherpaOnnxOnlineStream);
type DestroyKeywordSpotterFn = unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter);

#[derive(Clone, Copy)]
struct NativeKwsApi {
    create_keyword_spotter: CreateKeywordSpotterFn,
    create_keyword_stream: CreateKeywordStreamFn,
    accept_waveform: AcceptWaveformFn,
    is_keyword_stream_ready: IsKeywordStreamReadyFn,
    decode_keyword_stream: DecodeKeywordStreamFn,
    reset_keyword_stream: ResetKeywordStreamFn,
    get_keyword_result: GetKeywordResultFn,
    destroy_keyword_result: DestroyKeywordResultFn,
    destroy_online_stream: DestroyOnlineStreamFn,
    destroy_keyword_spotter: DestroyKeywordSpotterFn,
}

struct NativeKwsCStringStore {
    encoder: CString,
    decoder: CString,
    joiner: CString,
    tokens: CString,
    bpe_vocab: CString,
    keywords_file: CString,
    provider: CString,
    empty: CString,
}

struct NativeKwsRuntime {
    api: NativeKwsApi,
    spotter: *const SherpaOnnxKeywordSpotter,
    stream: *const SherpaOnnxOnlineStream,
    _strings: NativeKwsCStringStore,
    _library: DynamicLibraryHandle,
}

impl std::fmt::Debug for NativeKwsRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeKwsRuntime")
            .field("spotter", &self.spotter)
            .field("stream", &self.stream)
            .finish_non_exhaustive()
    }
}

impl NativeKwsRuntime {
    fn load(paths: &NativeKwsSessionPaths, config: &SherpaKwsConfig) -> Result<Self, WakeWordError> {
        let contract = native_capi_contract()?;
        let library_path = paths.runtime_dir.join(contract.library_relative_path);
        if !library_path.is_file() {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "missing required Wake Word native C API library",
                true,
            ));
        }
        let library = open_dynamic_library(&library_path)?;
        let api = unsafe { load_native_kws_api(library.0)? };
        let strings = NativeKwsCStringStore::new(&paths.model_dir)?;
        let keyword_config = build_keyword_spotter_config(config, &strings);
        let spotter = unsafe { (api.create_keyword_spotter)(&keyword_config) };
        if spotter.is_null() {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "failed to create Wake Word native keyword spotter",
                true,
            ));
        }
        let stream = unsafe { (api.create_keyword_stream)(spotter) };
        if stream.is_null() {
            unsafe {
                (api.destroy_keyword_spotter)(spotter);
            }
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "failed to create Wake Word native keyword stream",
                true,
            ));
        }
        Ok(Self {
            api,
            spotter,
            stream,
            _strings: strings,
            _library: library,
        })
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        let samples_f32: Vec<c_float> = samples
            .iter()
            .map(|sample| f32::from(*sample) / 32768.0)
            .collect();
        unsafe {
            (self.api.accept_waveform)(
                self.stream,
                sample_rate_hz as c_int,
                samples_f32.as_ptr(),
                samples_f32.len() as c_int,
            );
            while (self.api.is_keyword_stream_ready)(self.spotter, self.stream) != 0 {
                (self.api.decode_keyword_stream)(self.spotter, self.stream);
            }
            let result = (self.api.get_keyword_result)(self.spotter, self.stream);
            if result.is_null() {
                return Ok(None);
            }
            let detected = keyword_result_has_detection(result);
            (self.api.destroy_keyword_result)(result);
            if detected {
                (self.api.reset_keyword_stream)(self.spotter, self.stream);
                Ok(Some(WakeWordDetection::v1_detected(V1_WAKE_SCORE)))
            } else {
                Ok(None)
            }
        }
    }

    fn reset_stream(&self) -> Result<(), WakeWordError> {
        unsafe {
            (self.api.reset_keyword_stream)(self.spotter, self.stream);
        }
        Ok(())
    }
}

impl Drop for NativeKwsRuntime {
    fn drop(&mut self) {
        unsafe {
            if !self.stream.is_null() {
                (self.api.destroy_online_stream)(self.stream);
            }
            if !self.spotter.is_null() {
                (self.api.destroy_keyword_spotter)(self.spotter);
            }
        }
    }
}

impl NativeKwsCStringStore {
    fn new(model_dir: &Path) -> Result<Self, WakeWordError> {
        Ok(Self {
            encoder: path_to_cstring(&model_dir.join("encoder-epoch-12-avg-2-chunk-16-left-64.onnx"))?,
            decoder: path_to_cstring(&model_dir.join("decoder-epoch-12-avg-2-chunk-16-left-64.onnx"))?,
            joiner: path_to_cstring(&model_dir.join("joiner-epoch-12-avg-2-chunk-16-left-64.onnx"))?,
            tokens: path_to_cstring(&model_dir.join("tokens.txt"))?,
            bpe_vocab: path_to_cstring(&model_dir.join("bpe.model"))?,
            keywords_file: path_to_cstring(&model_dir.join(SHERPA_KWS_KEYWORD_FILE))?,
            provider: CString::new("cpu").expect("static provider has no interior NUL"),
            empty: CString::new("").expect("static empty string has no interior NUL"),
        })
    }
}

fn build_keyword_spotter_config(
    config: &SherpaKwsConfig,
    strings: &NativeKwsCStringStore,
) -> SherpaOnnxKeywordSpotterConfig {
    let null = std::ptr::null();
    SherpaOnnxKeywordSpotterConfig {
        feat_config: SherpaOnnxFeatureConfig {
            sample_rate: config.sample_rate_hz as c_int,
            feature_dim: config.feature_dim as c_int,
        },
        model_config: SherpaOnnxOnlineModelConfig {
            transducer: SherpaOnnxOnlineTransducerModelConfig {
                encoder: strings.encoder.as_ptr(),
                decoder: strings.decoder.as_ptr(),
                joiner: strings.joiner.as_ptr(),
            },
            paraformer: SherpaOnnxOnlineParaformerModelConfig {
                encoder: null,
                decoder: null,
            },
            zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig { model: null },
            tokens: strings.tokens.as_ptr(),
            num_threads: config.threads as c_int,
            provider: strings.provider.as_ptr(),
            debug: 0,
            model_type: strings.empty.as_ptr(),
            modeling_unit: strings.empty.as_ptr(),
            bpe_vocab: strings.bpe_vocab.as_ptr(),
            tokens_buf: null,
            tokens_buf_size: 0,
            nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig { model: null },
            t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig { model: null },
        },
        max_active_paths: 4,
        num_trailing_blanks: 1,
        keywords_score: config.score,
        keywords_threshold: config.threshold,
        keywords_file: strings.keywords_file.as_ptr(),
        keywords_buf: null,
        keywords_buf_size: 0,
    }
}

fn path_to_cstring(path: &Path) -> Result<CString, WakeWordError> {
    let text = path.to_str().ok_or_else(|| {
        WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native path is not valid UTF-8",
            false,
        )
    })?;
    CString::new(text).map_err(|_| {
        WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native path is invalid",
            false,
        )
    })
}

unsafe fn keyword_result_has_detection(result: *const SherpaOnnxKeywordResult) -> bool {
    let keyword = (*result).keyword;
    !keyword.is_null() && !CStr::from_ptr(keyword).to_bytes().is_empty()
}

#[cfg(unix)]
fn open_dynamic_library(library_path: &Path) -> Result<DynamicLibraryHandle, WakeWordError> {
    let path = path_to_cstring(library_path)?;
    unsafe {
        let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        if handle.is_null() {
            Err(WakeWordError::sanitized(
                WakeWordErrorKind::RuntimeUnavailable,
                "failed to load Wake Word native C API library",
                true,
            ))
        } else {
            Ok(DynamicLibraryHandle(handle))
        }
    }
}

#[cfg(not(unix))]
fn open_dynamic_library(_library_path: &Path) -> Result<DynamicLibraryHandle, WakeWordError> {
    Err(WakeWordError::sanitized(
        WakeWordErrorKind::RuntimeUnavailable,
        "Wake Word native C API loading is unsupported on this platform",
        false,
    ))
}

#[cfg(unix)]
unsafe fn load_native_kws_api(handle: *mut c_void) -> Result<NativeKwsApi, WakeWordError> {
    Ok(NativeKwsApi {
        create_keyword_spotter: load_required_symbol(handle, "SherpaOnnxCreateKeywordSpotter")?,
        create_keyword_stream: load_required_symbol(handle, "SherpaOnnxCreateKeywordStream")?,
        accept_waveform: load_required_symbol(handle, "SherpaOnnxOnlineStreamAcceptWaveform")?,
        is_keyword_stream_ready: load_required_symbol(handle, "SherpaOnnxIsKeywordStreamReady")?,
        decode_keyword_stream: load_required_symbol(handle, "SherpaOnnxDecodeKeywordStream")?,
        reset_keyword_stream: load_required_symbol(handle, "SherpaOnnxResetKeywordStream")?,
        get_keyword_result: load_required_symbol(handle, "SherpaOnnxGetKeywordResult")?,
        destroy_keyword_result: load_required_symbol(handle, "SherpaOnnxDestroyKeywordResult")?,
        destroy_online_stream: load_required_symbol(handle, "SherpaOnnxDestroyOnlineStream")?,
        destroy_keyword_spotter: load_required_symbol(handle, "SherpaOnnxDestroyKeywordSpotter")?,
    })
}

#[cfg(unix)]
unsafe fn load_required_symbol<T: Copy>(handle: *mut c_void, symbol: &str) -> Result<T, WakeWordError> {
    let c_symbol = CString::new(symbol).map_err(|_| {
        WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native C API symbol is invalid",
            false,
        )
    })?;
    let pointer = libc::dlsym(handle, c_symbol.as_ptr());
    if pointer.is_null() {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native C API is missing a required symbol",
            true,
        ));
    }
    Ok(std::mem::transmute_copy(&pointer))
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
            native_runtime: None,
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
            native_runtime: None,
            shutdown: false,
        };
        let symbols = session.required_native_c_api_symbols();
        assert_eq!(symbols.len(), 10);
        assert!(symbols.contains(&"SherpaOnnxCreateKeywordSpotter"));
        assert!(symbols.contains(&"SherpaOnnxDecodeKeywordStream"));
        assert!(symbols.contains(&"SherpaOnnxResetKeywordStream"));
        assert!(!symbols.iter().any(|symbol| symbol.contains("Transducer")));
        assert!(!symbols
            .iter()
            .any(|symbol| symbol.contains("OfflineRecognizer")));
        assert!(!symbols.iter().any(|symbol| symbol.contains("Whisper")));
    }

    #[test]
    fn runtime_contract_uses_shared_c_api_artifacts_not_jni() {
        let files = runtime_files().unwrap();
        assert!(files.iter().any(|f| f.relative_path.contains("c-api")));
        assert!(!files.iter().any(|f| f.relative_path.contains("jni")));
        let contract = native_capi_contract().unwrap();
        assert!(contract
            .library_relative_path
            .contains("shared/lib/libsherpa-onnx-c-api"));
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
        assert!(!error
            .message
            .contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn native_session_shutdown_is_idempotent_and_feed_errors_are_sanitized() {
        let mut session = NativeKwsSession {
            config: SherpaKwsConfig::default(),
            paths: NativeKwsSessionPaths {
                model_dir: PathBuf::from("model"),
                runtime_dir: PathBuf::from("runtime"),
            },
            native_runtime: None,
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
    fn native_keyword_config_uses_frozen_policy_and_verified_paths() {
        let model_dir = PathBuf::from("model");
        let strings = NativeKwsCStringStore::new(&model_dir).unwrap();
        let config = build_keyword_spotter_config(&SherpaKwsConfig::default(), &strings);
        assert_eq!(config.feat_config.sample_rate, 16_000);
        assert_eq!(config.feat_config.feature_dim, 80);
        assert_eq!(config.model_config.num_threads, 1);
        assert_eq!(config.keywords_score, 1.0);
        assert_eq!(config.keywords_threshold, 0.25);
        unsafe {
            assert!(CStr::from_ptr(config.model_config.provider)
                .to_str()
                .unwrap()
                .eq("cpu"));
            assert!(CStr::from_ptr(config.model_config.transducer.encoder)
                .to_str()
                .unwrap()
                .ends_with("encoder-epoch-12-avg-2-chunk-16-left-64.onnx"));
            assert!(CStr::from_ptr(config.model_config.transducer.decoder)
                .to_str()
                .unwrap()
                .ends_with("decoder-epoch-12-avg-2-chunk-16-left-64.onnx"));
            assert!(CStr::from_ptr(config.model_config.transducer.joiner)
                .to_str()
                .unwrap()
                .ends_with("joiner-epoch-12-avg-2-chunk-16-left-64.onnx"));
            assert!(CStr::from_ptr(config.model_config.tokens)
                .to_str()
                .unwrap()
                .ends_with("tokens.txt"));
            assert!(CStr::from_ptr(config.model_config.bpe_vocab)
                .to_str()
                .unwrap()
                .ends_with("bpe.model"));
            assert!(CStr::from_ptr(config.keywords_file)
                .to_str()
                .unwrap()
                .ends_with(SHERPA_KWS_KEYWORD_FILE));
        }
    }

    #[test]
    fn keyword_result_detection_is_bounded_to_keyword_presence() {
        let keyword = CString::new("HEY MOOSE").unwrap();
        let result = SherpaOnnxKeywordResult {
            keyword: keyword.as_ptr(),
            tokens: std::ptr::null(),
            tokens_arr: std::ptr::null(),
            count: 0,
            timestamps: std::ptr::null_mut(),
            start_time: 0.0,
            json: std::ptr::null(),
        };
        assert!(unsafe { keyword_result_has_detection(&result) });

        let empty = CString::new("").unwrap();
        let result = SherpaOnnxKeywordResult {
            keyword: empty.as_ptr(),
            tokens: std::ptr::null(),
            tokens_arr: std::ptr::null(),
            count: 0,
            timestamps: std::ptr::null_mut(),
            start_time: 0.0,
            json: std::ptr::null(),
        };
        assert!(!unsafe { keyword_result_has_detection(&result) });
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
