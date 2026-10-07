use super::artifacts::{
    runtime_platform_name, validate_pcm_frame, verify_model_artifacts, verify_runtime_artifacts,
};
use super::types::{
    SherpaKwsConfig, SherpaKwsEngine, WakeWordDetection, WakeWordError, WakeWordErrorKind,
};
use super::V1_WAKE_SCORE;
use crate::asr::wake_word_sherpa_manifest::{
    SHERPA_KWS_KEYWORD_FILE, SHERPA_KWS_KEYWORD_REPRESENTATION,
};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::path::{Path, PathBuf};

pub(super) const SHERPA_KWS_C_API_SYMBOLS: [&str; 10] = [
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
pub(super) struct NativeCapiContract {
    pub(super) library_relative_path: &'static str,
    pub(super) required_symbols: &'static [&'static str; 10],
}

pub(super) fn native_capi_contract() -> Result<NativeCapiContract, WakeWordError> {
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

#[repr(C)]
pub(super) struct SherpaOnnxOnlineTransducerModelConfig {
    pub(super) encoder: *const c_char,
    pub(super) decoder: *const c_char,
    pub(super) joiner: *const c_char,
}

#[repr(C)]
pub(super) struct SherpaOnnxOnlineParaformerModelConfig {
    pub(super) encoder: *const c_char,
    pub(super) decoder: *const c_char,
}

#[repr(C)]
pub(super) struct SherpaOnnxOnlineZipformer2CtcModelConfig {
    pub(super) model: *const c_char,
}

#[repr(C)]
pub(super) struct SherpaOnnxOnlineNemoCtcModelConfig {
    pub(super) model: *const c_char,
}

#[repr(C)]
pub(super) struct SherpaOnnxOnlineToneCtcModelConfig {
    pub(super) model: *const c_char,
}

#[repr(C)]
pub(super) struct SherpaOnnxOnlineModelConfig {
    pub(super) transducer: SherpaOnnxOnlineTransducerModelConfig,
    pub(super) paraformer: SherpaOnnxOnlineParaformerModelConfig,
    pub(super) zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig,
    pub(super) tokens: *const c_char,
    pub(super) num_threads: c_int,
    pub(super) provider: *const c_char,
    pub(super) debug: c_int,
    pub(super) model_type: *const c_char,
    pub(super) modeling_unit: *const c_char,
    pub(super) bpe_vocab: *const c_char,
    pub(super) tokens_buf: *const c_char,
    pub(super) tokens_buf_size: c_int,
    pub(super) nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig,
    pub(super) t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig,
}

#[repr(C)]
pub(super) struct SherpaOnnxFeatureConfig {
    pub(super) sample_rate: c_int,
    pub(super) feature_dim: c_int,
}

#[repr(C)]
pub(super) struct SherpaOnnxKeywordSpotterConfig {
    pub(super) feat_config: SherpaOnnxFeatureConfig,
    pub(super) model_config: SherpaOnnxOnlineModelConfig,
    pub(super) max_active_paths: c_int,
    pub(super) num_trailing_blanks: c_int,
    pub(super) keywords_score: c_float,
    pub(super) keywords_threshold: c_float,
    pub(super) keywords_file: *const c_char,
    pub(super) keywords_buf: *const c_char,
    pub(super) keywords_buf_size: c_int,
}

#[repr(C)]
pub(super) struct SherpaOnnxKeywordResult {
    pub(super) keyword: *const c_char,
    pub(super) tokens: *const c_char,
    pub(super) tokens_arr: *const *const c_char,
    pub(super) count: c_int,
    pub(super) timestamps: *mut c_float,
    pub(super) start_time: c_float,
    pub(super) json: *const c_char,
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

pub(super) struct NativeKwsCStringStore {
    pub(super) encoder: CString,
    pub(super) decoder: CString,
    pub(super) joiner: CString,
    pub(super) tokens: CString,
    pub(super) bpe_vocab: CString,
    pub(super) keywords_file: CString,
    pub(super) provider: CString,
    pub(super) empty: CString,
}

pub(super) struct NativeKwsRuntime {
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
    fn load(
        paths: &NativeKwsSessionPaths,
        config: &SherpaKwsConfig,
    ) -> Result<Self, WakeWordError> {
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

    pub(super) fn accept_pcm16_mono(
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

    pub(super) fn reset_stream(&self) -> Result<(), WakeWordError> {
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
    pub(super) fn new(model_dir: &Path) -> Result<Self, WakeWordError> {
        Ok(Self {
            encoder: path_to_cstring(
                &model_dir.join("encoder-epoch-12-avg-2-chunk-16-left-64.onnx"),
            )?,
            decoder: path_to_cstring(
                &model_dir.join("decoder-epoch-12-avg-2-chunk-16-left-64.onnx"),
            )?,
            joiner: path_to_cstring(
                &model_dir.join("joiner-epoch-12-avg-2-chunk-16-left-64.onnx"),
            )?,
            tokens: path_to_cstring(&model_dir.join("tokens.txt"))?,
            bpe_vocab: path_to_cstring(&model_dir.join("bpe.model"))?,
            keywords_file: path_to_cstring(&model_dir.join(SHERPA_KWS_KEYWORD_FILE))?,
            provider: CString::new("cpu").expect("static provider has no interior NUL"),
            empty: CString::new("").expect("static empty string has no interior NUL"),
        })
    }
}

pub(super) fn build_keyword_spotter_config(
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

pub(super) unsafe fn keyword_result_has_detection(result: *const SherpaOnnxKeywordResult) -> bool {
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
unsafe fn load_required_symbol<T: Copy>(
    handle: *mut c_void,
    symbol: &str,
) -> Result<T, WakeWordError> {
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

pub(super) fn verify_native_c_api_symbols(runtime_dir: &Path) -> Result<(), WakeWordError> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeKwsSessionPaths {
    pub model_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

#[derive(Debug)]
pub struct NativeKwsSession {
    pub(super) config: SherpaKwsConfig,
    pub(super) paths: NativeKwsSessionPaths,
    pub(super) native_runtime: Option<NativeKwsRuntime>,
    pub(super) shutdown: bool,
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
        self.native_runtime()?
            .accept_pcm16_mono(sample_rate_hz, samples)
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
