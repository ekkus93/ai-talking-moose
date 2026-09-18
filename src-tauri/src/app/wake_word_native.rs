//! Verified dynamic sherpa-onnx C API adapter for Wake Word V1.
//!
//! This module intentionally binds only the keyword-spotting API surface. It has
//! no networking or full-transcription entry points.

use super::wake_word_engine::{
    validate_pcm_frame, SherpaKwsConfig, SherpaKwsEngine, WakeWordDetection, WakeWordError,
    WakeWordErrorKind, V1_KWS_SAMPLE_RATE_HZ,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

const ARTIFACT_MANIFEST: &str = include_str!("../../../wake-word-artifacts.json");
const EXPECTED_RUNTIME_VERSION: &str = "1.13.8";
const MAX_DECODE_STEPS_PER_FEED: usize = 64;

#[derive(Debug, Clone)]
pub struct NativeSherpaPaths {
    pub runtime_c_api: PathBuf,
    pub runtime_onnx: PathBuf,
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
    pub bpe_model: PathBuf,
    pub keyword_file: PathBuf,
}

impl NativeSherpaPaths {
    pub fn from_roots(runtime_root: &Path, model_root: &Path) -> Result<Self, WakeWordError> {
        let document = manifest()?;
        let platform = platform_key()?;
        let runtime = document
            .get("runtime")
            .and_then(|v| v.get("platforms"))
            .and_then(|v| v.get(platform))
            .ok_or_else(unsupported_platform)?;
        let install_root = runtime
            .get("install_root")
            .and_then(Value::as_str)
            .ok_or_else(invalid_manifest)?;
        let files = runtime
            .get("files")
            .and_then(Value::as_array)
            .ok_or_else(invalid_manifest)?;
        let c_api = runtime_file_by_basename(files, c_api_library_basename())?;
        let onnx = runtime_file_by_basename(files, onnx_library_basename())?;

        let model = document
            .get("artifacts")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .ok_or_else(invalid_manifest)?;
        let keyword = model
            .get("keyword")
            .and_then(Value::as_object)
            .ok_or_else(invalid_manifest)?;
        let keyword_file = keyword
            .get("filename")
            .and_then(Value::as_str)
            .ok_or_else(invalid_manifest)?;

        Ok(Self {
            runtime_c_api: runtime_root.join(install_root).join(c_api),
            runtime_onnx: runtime_root.join(install_root).join(onnx),
            encoder: model_root.join("encoder-epoch-12-avg-2-chunk-16-left-64.onnx"),
            decoder: model_root.join("decoder-epoch-12-avg-2-chunk-16-left-64.onnx"),
            joiner: model_root.join("joiner-epoch-12-avg-2-chunk-16-left-64.onnx"),
            tokens: model_root.join("tokens.txt"),
            bpe_model: model_root.join("bpe.model"),
            keyword_file: model_root.join(keyword_file),
        })
    }
}

fn manifest() -> Result<Value, WakeWordError> {
    serde_json::from_str(ARTIFACT_MANIFEST).map_err(|_| invalid_manifest())
}

fn invalid_manifest() -> WakeWordError {
    WakeWordError::sanitized(
        WakeWordErrorKind::InvalidArtifact,
        "Wake Word production artifact manifest is invalid",
        false,
    )
}

fn unsupported_platform() -> WakeWordError {
    WakeWordError::sanitized(
        WakeWordErrorKind::RuntimeUnavailable,
        "Wake Word native runtime is unsupported on this platform",
        false,
    )
}

fn platform_key() -> Result<&'static str, WakeWordError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok("linux-x86_64");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok("macos-arm64");
    }
    #[allow(unreachable_code)]
    Err(unsupported_platform())
}

fn c_api_library_basename() -> &'static str {
    if cfg!(target_os = "macos") {
        "libsherpa-onnx-c-api.dylib"
    } else {
        "libsherpa-onnx-c-api.so"
    }
}

fn onnx_library_basename() -> &'static str {
    if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    }
}

fn runtime_file_by_basename(files: &[Value], basename: &str) -> Result<String, WakeWordError> {
    let mut matches = files.iter().filter_map(|item| {
        let path = item.get("path")?.as_str()?;
        (Path::new(path).file_name()?.to_str()? == basename).then(|| path.to_string())
    });
    let first = matches.next().ok_or_else(invalid_manifest)?;
    if matches.next().is_some() {
        return Err(invalid_manifest());
    }
    Ok(first)
}

#[derive(Debug, Clone)]
struct ExpectedIdentity {
    bytes: u64,
    sha256: String,
    architecture: Option<String>,
}

fn expected_model_identity(name: &str) -> Result<ExpectedIdentity, WakeWordError> {
    let document = manifest()?;
    let model = document
        .get("artifacts")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(invalid_manifest)?;
    if let Some(files) = model.get("files").and_then(Value::as_array) {
        if let Some(item) = files
            .iter()
            .find(|item| item.get("name").and_then(Value::as_str) == Some(name))
        {
            return identity_from_value(item, None);
        }
    }
    if model
        .get("keyword")
        .and_then(|item| item.get("filename"))
        .and_then(Value::as_str)
        == Some(name)
    {
        return identity_from_value(model.get("keyword").ok_or_else(invalid_manifest)?, None);
    }
    Err(invalid_manifest())
}

fn expected_runtime_identity(basename: &str) -> Result<ExpectedIdentity, WakeWordError> {
    let document = manifest()?;
    let platform = platform_key()?;
    let cfg = document
        .get("runtime")
        .and_then(|v| v.get("platforms"))
        .and_then(|v| v.get(platform))
        .ok_or_else(unsupported_platform)?;
    let architecture = cfg
        .get("architecture")
        .and_then(Value::as_str)
        .ok_or_else(invalid_manifest)?
        .to_string();
    let item = cfg
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|item| {
                item.get("path")
                    .and_then(Value::as_str)
                    .and_then(|path| Path::new(path).file_name())
                    .and_then(|name| name.to_str())
                    == Some(basename)
            })
        })
        .ok_or_else(invalid_manifest)?;
    identity_from_value(item, Some(architecture))
}

fn identity_from_value(
    item: &Value,
    architecture: Option<String>,
) -> Result<ExpectedIdentity, WakeWordError> {
    Ok(ExpectedIdentity {
        bytes: item
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(invalid_manifest)?,
        sha256: item
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or_else(invalid_manifest)?
            .to_string(),
        architecture,
    })
}

fn verify_file(path: &Path, expected: &ExpectedIdentity) -> Result<(), WakeWordError> {
    let mut file = File::open(path).map_err(|_| missing_artifact())?;
    let metadata = file.metadata().map_err(|_| missing_artifact())?;
    if metadata.len() != expected.bytes {
        return Err(invalid_artifact());
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| invalid_artifact())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = format!("{:x}", hasher.finalize());
    if digest != expected.sha256 {
        return Err(invalid_artifact());
    }
    if let Some(architecture) = &expected.architecture {
        file.seek(SeekFrom::Start(0)).map_err(|_| invalid_artifact())?;
        let mut header = [0_u8; 64];
        let read = file.read(&mut header).map_err(|_| invalid_artifact())?;
        if !architecture_matches(&header[..read], architecture) {
            return Err(invalid_artifact());
        }
    }
    Ok(())
}

fn architecture_matches(data: &[u8], expected: &str) -> bool {
    if expected == "elf-x86_64" {
        return data.len() >= 20
            && data[..4] == *b"\x7fELF"
            && data[4] == 2
            && data[5] == 1
            && u16::from_le_bytes([data[18], data[19]]) == 62;
    }
    if expected == "macho-arm64" {
        return data.len() >= 8
            && data[..4] == [0xcf, 0xfa, 0xed, 0xfe]
            && u32::from_le_bytes([data[4], data[5], data[6], data[7]]) == 0x0100_000c;
    }
    false
}

fn missing_artifact() -> WakeWordError {
    WakeWordError::sanitized(
        WakeWordErrorKind::MissingArtifact,
        "Wake Word required artifact is missing",
        false,
    )
}

fn invalid_artifact() -> WakeWordError {
    WakeWordError::sanitized(
        WakeWordErrorKind::InvalidArtifact,
        "Wake Word required artifact failed identity verification",
        false,
    )
}

pub fn verify_production_paths(paths: &NativeSherpaPaths) -> Result<(), WakeWordError> {
    verify_file(
        &paths.runtime_c_api,
        &expected_runtime_identity(c_api_library_basename())?,
    )?;
    verify_file(
        &paths.runtime_onnx,
        &expected_runtime_identity(onnx_library_basename())?,
    )?;
    for (path, name) in [
        (&paths.encoder, "encoder-epoch-12-avg-2-chunk-16-left-64.onnx"),
        (&paths.decoder, "decoder-epoch-12-avg-2-chunk-16-left-64.onnx"),
        (&paths.joiner, "joiner-epoch-12-avg-2-chunk-16-left-64.onnx"),
        (&paths.tokens, "tokens.txt"),
        (&paths.bpe_model, "bpe.model"),
        (&paths.keyword_file, "hey-moose.tokens.txt"),
    ] {
        verify_file(path, &expected_model_identity(name)?)?;
    }
    let keyword = std::fs::read_to_string(&paths.keyword_file).map_err(|_| invalid_artifact())?;
    if keyword != "▁HE Y ▁MO O SE\n" {
        return Err(invalid_artifact());
    }
    Ok(())
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
    num_threads: i32,
    provider: *const c_char,
    debug: i32,
    model_type: *const c_char,
    modeling_unit: *const c_char,
    bpe_vocab: *const c_char,
    tokens_buf: *const c_char,
    tokens_buf_size: i32,
    nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig,
    t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig,
}

#[repr(C)]
struct SherpaOnnxFeatureConfig {
    sample_rate: i32,
    feature_dim: i32,
}

#[repr(C)]
struct SherpaOnnxKeywordSpotterConfig {
    feat_config: SherpaOnnxFeatureConfig,
    model_config: SherpaOnnxOnlineModelConfig,
    max_active_paths: i32,
    num_trailing_blanks: i32,
    keywords_score: f32,
    keywords_threshold: f32,
    keywords_file: *const c_char,
    keywords_buf: *const c_char,
    keywords_buf_size: i32,
}

#[repr(C)]
struct SherpaOnnxKeywordSpotter {
    _opaque: [u8; 0],
}

#[repr(C)]
struct SherpaOnnxOnlineStream {
    _opaque: [u8; 0],
}

#[repr(C)]
struct SherpaOnnxKeywordResult {
    keyword: *const c_char,
    tokens: *const c_char,
    tokens_arr: *const *const c_char,
    count: i32,
    timestamps: *mut f32,
    start_time: f32,
    json: *const c_char,
}

type CreateKeywordSpotter =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotterConfig) -> *const SherpaOnnxKeywordSpotter;
type DestroyKeywordSpotter = unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter);
type CreateKeywordStream =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter) -> *const SherpaOnnxOnlineStream;
type DestroyOnlineStream = unsafe extern "C" fn(*const SherpaOnnxOnlineStream);
type AcceptWaveform =
    unsafe extern "C" fn(*const SherpaOnnxOnlineStream, i32, *const f32, i32);
type IsReady =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter, *const SherpaOnnxOnlineStream) -> i32;
type Decode =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter, *const SherpaOnnxOnlineStream);
type Reset =
    unsafe extern "C" fn(*const SherpaOnnxKeywordSpotter, *const SherpaOnnxOnlineStream);
type GetResult = unsafe extern "C" fn(
    *const SherpaOnnxKeywordSpotter,
    *const SherpaOnnxOnlineStream,
) -> *const SherpaOnnxKeywordResult;
type DestroyResult = unsafe extern "C" fn(*const SherpaOnnxKeywordResult);
type GetVersion = unsafe extern "C" fn() -> *const c_char;

#[cfg(unix)]
struct DynamicLibrary(*mut c_void);

#[cfg(unix)]
impl DynamicLibrary {
    fn open(path: &Path, global: bool) -> Result<Self, WakeWordError> {
        let path = path.to_str().ok_or_else(runtime_load_error)?;
        let path = CString::new(path).map_err(|_| runtime_load_error())?;
        let flags = libc::RTLD_NOW | if global { libc::RTLD_GLOBAL } else { libc::RTLD_LOCAL };
        let handle = unsafe { libc::dlopen(path.as_ptr(), flags) };
        if handle.is_null() {
            Err(runtime_load_error())
        } else {
            Ok(Self(handle))
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T, WakeWordError> {
        let symbol = libc::dlsym(self.0, name.as_ptr().cast());
        if symbol.is_null() {
            return Err(runtime_load_error());
        }
        Ok(std::mem::transmute_copy::<*mut c_void, T>(&symbol))
    }
}

#[cfg(unix)]
impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                libc::dlclose(self.0);
            }
            self.0 = ptr::null_mut();
        }
    }
}

fn runtime_load_error() -> WakeWordError {
    WakeWordError::sanitized(
        WakeWordErrorKind::RuntimeUnavailable,
        "Wake Word native runtime could not be loaded",
        true,
    )
}

struct NativeApi {
    create_spotter: CreateKeywordSpotter,
    destroy_spotter: DestroyKeywordSpotter,
    create_stream: CreateKeywordStream,
    destroy_stream: DestroyOnlineStream,
    accept_waveform: AcceptWaveform,
    is_ready: IsReady,
    decode: Decode,
    reset: Reset,
    get_result: GetResult,
    destroy_result: DestroyResult,
}

struct NativeStrings {
    _encoder: CString,
    _decoder: CString,
    _joiner: CString,
    _tokens: CString,
    _keywords: CString,
    _cpu: CString,
}

pub struct NativeSherpaKwsSession {
    api: NativeApi,
    spotter: *const SherpaOnnxKeywordSpotter,
    stream: *const SherpaOnnxOnlineStream,
    cancelled: AtomicBool,
    shutdown: bool,
    _strings: NativeStrings,
    #[cfg(unix)]
    _sherpa_library: DynamicLibrary,
    #[cfg(unix)]
    _onnx_library: DynamicLibrary,
}

unsafe impl Send for NativeSherpaKwsSession {}

impl NativeSherpaKwsSession {
    pub fn new(paths: &NativeSherpaPaths, config: &SherpaKwsConfig) -> Result<Self, WakeWordError> {
        config.validate()?;
        verify_production_paths(paths)?;
        Self::load(paths, config)
    }

    #[cfg(unix)]
    fn load(paths: &NativeSherpaPaths, config: &SherpaKwsConfig) -> Result<Self, WakeWordError> {
        let onnx_library = DynamicLibrary::open(&paths.runtime_onnx, true)?;
        let sherpa_library = DynamicLibrary::open(&paths.runtime_c_api, false)?;

        let get_version: GetVersion =
            unsafe { sherpa_library.symbol(b"SherpaOnnxGetVersionStr\0")? };
        let version_ptr = unsafe { get_version() };
        if version_ptr.is_null()
            || unsafe { CStr::from_ptr(version_ptr) }.to_str().ok() != Some(EXPECTED_RUNTIME_VERSION)
        {
            return Err(runtime_load_error());
        }

        let api = NativeApi {
            create_spotter: unsafe { sherpa_library.symbol(b"SherpaOnnxCreateKeywordSpotter\0")? },
            destroy_spotter: unsafe { sherpa_library.symbol(b"SherpaOnnxDestroyKeywordSpotter\0")? },
            create_stream: unsafe { sherpa_library.symbol(b"SherpaOnnxCreateKeywordStream\0")? },
            destroy_stream: unsafe { sherpa_library.symbol(b"SherpaOnnxDestroyOnlineStream\0")? },
            accept_waveform: unsafe {
                sherpa_library.symbol(b"SherpaOnnxOnlineStreamAcceptWaveform\0")?
            },
            is_ready: unsafe { sherpa_library.symbol(b"SherpaOnnxIsKeywordStreamReady\0")? },
            decode: unsafe { sherpa_library.symbol(b"SherpaOnnxDecodeKeywordStream\0")? },
            reset: unsafe { sherpa_library.symbol(b"SherpaOnnxResetKeywordStream\0")? },
            get_result: unsafe { sherpa_library.symbol(b"SherpaOnnxGetKeywordResult\0")? },
            destroy_result: unsafe {
                sherpa_library.symbol(b"SherpaOnnxDestroyKeywordResult\0")?
            },
        };

        let strings = NativeStrings {
            _encoder: path_cstring(&paths.encoder)?,
            _decoder: path_cstring(&paths.decoder)?,
            _joiner: path_cstring(&paths.joiner)?,
            _tokens: path_cstring(&paths.tokens)?,
            _keywords: path_cstring(&paths.keyword_file)?,
            _cpu: CString::new("cpu").expect("literal contains no NUL"),
        };
        let native_config = SherpaOnnxKeywordSpotterConfig {
            feat_config: SherpaOnnxFeatureConfig {
                sample_rate: config.sample_rate_hz as i32,
                feature_dim: config.feature_dim as i32,
            },
            model_config: SherpaOnnxOnlineModelConfig {
                transducer: SherpaOnnxOnlineTransducerModelConfig {
                    encoder: strings._encoder.as_ptr(),
                    decoder: strings._decoder.as_ptr(),
                    joiner: strings._joiner.as_ptr(),
                },
                paraformer: SherpaOnnxOnlineParaformerModelConfig {
                    encoder: ptr::null(),
                    decoder: ptr::null(),
                },
                zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig { model: ptr::null() },
                tokens: strings._tokens.as_ptr(),
                num_threads: config.threads as i32,
                provider: strings._cpu.as_ptr(),
                debug: 0,
                model_type: ptr::null(),
                modeling_unit: ptr::null(),
                bpe_vocab: ptr::null(),
                tokens_buf: ptr::null(),
                tokens_buf_size: 0,
                nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig { model: ptr::null() },
                t_one_ctc: SherpaOnnxOnlineToneCtcModelConfig { model: ptr::null() },
            },
            max_active_paths: 4,
            num_trailing_blanks: 1,
            keywords_score: config.score,
            keywords_threshold: config.threshold,
            keywords_file: strings._keywords.as_ptr(),
            keywords_buf: ptr::null(),
            keywords_buf_size: 0,
        };

        let spotter = unsafe { (api.create_spotter)(&native_config) };
        if spotter.is_null() {
            return Err(runtime_load_error());
        }
        let stream = unsafe { (api.create_stream)(spotter) };
        if stream.is_null() {
            unsafe {
                (api.destroy_spotter)(spotter);
            }
            return Err(runtime_load_error());
        }

        Ok(Self {
            api,
            spotter,
            stream,
            cancelled: AtomicBool::new(false),
            shutdown: false,
            _strings: strings,
            _sherpa_library: sherpa_library,
            _onnx_library: onnx_library,
        })
    }

    #[cfg(not(unix))]
    fn load(_paths: &NativeSherpaPaths, _config: &SherpaKwsConfig) -> Result<Self, WakeWordError> {
        Err(unsupported_platform())
    }

    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn check_cancelled(&self) -> Result<(), WakeWordError> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(WakeWordError::sanitized(
                WakeWordErrorKind::Cancelled,
                "Wake Word native inference was cancelled",
                true,
            ))
        } else {
            Ok(())
        }
    }

    fn accept_f32(&mut self, samples: &[f32]) -> Result<bool, WakeWordError> {
        if self.shutdown {
            return Err(runtime_load_error());
        }
        self.check_cancelled()?;
        let n = i32::try_from(samples.len()).map_err(|_| {
            WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "Wake Word PCM frame is too large",
                false,
            )
        })?;
        unsafe {
            (self.api.accept_waveform)(self.stream, V1_KWS_SAMPLE_RATE_HZ as i32, samples.as_ptr(), n);
        }
        let mut decode_steps = 0_usize;
        loop {
            self.check_cancelled()?;
            let ready = unsafe { (self.api.is_ready)(self.spotter, self.stream) };
            if ready == 0 {
                break;
            }
            if decode_steps >= MAX_DECODE_STEPS_PER_FEED {
                return Err(WakeWordError::sanitized(
                    WakeWordErrorKind::Inference,
                    "Wake Word native decoder exceeded its bounded work limit",
                    true,
                ));
            }
            unsafe {
                (self.api.decode)(self.spotter, self.stream);
            }
            decode_steps += 1;
        }
        let result = unsafe { (self.api.get_result)(self.spotter, self.stream) };
        if result.is_null() {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::Inference,
                "Wake Word native inference returned no result",
                true,
            ));
        }
        let detected = unsafe {
            let keyword = (*result).keyword;
            !keyword.is_null() && CStr::from_ptr(keyword).to_bytes().len() > 0
        };
        unsafe {
            (self.api.destroy_result)(result);
        }
        Ok(detected)
    }

    fn reset_native_stream(&mut self) -> Result<(), WakeWordError> {
        if self.shutdown {
            return Err(runtime_load_error());
        }
        unsafe {
            (self.api.reset)(self.spotter, self.stream);
        }
        self.cancelled.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn shutdown_native(&mut self) {
        if self.shutdown {
            return;
        }
        self.shutdown = true;
        self.cancelled.store(true, Ordering::SeqCst);
        unsafe {
            if !self.stream.is_null() {
                (self.api.destroy_stream)(self.stream);
                self.stream = ptr::null();
            }
            if !self.spotter.is_null() {
                (self.api.destroy_spotter)(self.spotter);
                self.spotter = ptr::null();
            }
        }
    }
}

impl Drop for NativeSherpaKwsSession {
    fn drop(&mut self) {
        self.shutdown_native();
    }
}

fn path_cstring(path: &Path) -> Result<CString, WakeWordError> {
    let value = path.to_str().ok_or_else(invalid_artifact)?;
    CString::new(value).map_err(|_| invalid_artifact())
}

pub struct NativeSherpaKwsEngine {
    config: SherpaKwsConfig,
    session: NativeSherpaKwsSession,
}

impl NativeSherpaKwsEngine {
    pub fn new(paths: NativeSherpaPaths, config: SherpaKwsConfig) -> Result<Self, WakeWordError> {
        config.validate()?;
        let session = NativeSherpaKwsSession::new(&paths, &config)?;
        Ok(Self { config, session })
    }

    pub fn request_cancel(&self) {
        self.session.request_cancel();
    }
}

impl SherpaKwsEngine for NativeSherpaKwsEngine {
    fn config(&self) -> &SherpaKwsConfig {
        &self.config
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        validate_pcm_frame(sample_rate_hz, samples)?;
        let normalized: Vec<f32> = samples
            .iter()
            .map(|sample| *sample as f32 / 32768.0)
            .collect();
        if self.session.accept_f32(&normalized)? {
            self.session.reset_native_stream()?;
            Ok(Some(WakeWordDetection::v1_detected(self.config.score)))
        } else {
            Ok(None)
        }
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        self.session.reset_native_stream()
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        self.session.shutdown_native();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn synthetic_elf() -> Vec<u8> {
        let mut data = vec![0_u8; 64];
        data[..4].copy_from_slice(b"\x7fELF");
        data[4] = 2;
        data[5] = 1;
        data[18..20].copy_from_slice(&62_u16.to_le_bytes());
        data
    }

    #[test]
    fn architecture_gate_rejects_wrong_native_binary() {
        let mut macho = vec![0_u8; 64];
        macho[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        macho[4..8].copy_from_slice(&0x0100_000c_u32.to_le_bytes());
        assert!(!architecture_matches(&macho, "elf-x86_64"));
        assert!(architecture_matches(&synthetic_elf(), "elf-x86_64"));
    }

    #[test]
    fn identity_gate_rejects_missing_and_corrupt_files_before_load() {
        let root = tempdir().unwrap();
        let missing = root.path().join("missing");
        let expected = ExpectedIdentity {
            bytes: 3,
            sha256: format!("{:x}", Sha256::digest(b"abc")),
            architecture: None,
        };
        assert_eq!(
            verify_file(&missing, &expected).unwrap_err().kind,
            WakeWordErrorKind::MissingArtifact
        );

        let corrupt = root.path().join("artifact");
        File::create(&corrupt)
            .unwrap()
            .write_all(b"abd")
            .unwrap();
        assert_eq!(
            verify_file(&corrupt, &expected).unwrap_err().kind,
            WakeWordErrorKind::InvalidArtifact
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_load_failure_is_sanitized() {
        let error = DynamicLibrary::open(Path::new("/definitely/not/a/library.so"), false)
            .err()
            .unwrap();
        assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
        assert!(!error.message.contains('/'));
        assert!(!error.message.contains(".so"));
    }

    #[test]
    fn v1_native_policy_is_observable() {
        let config = SherpaKwsConfig::default();
        assert_eq!(config.threads, 1);
        assert_eq!(config.score, 1.0);
        assert_eq!(config.threshold, 0.25);
        assert_eq!(config.sample_rate_hz, 16_000);
        assert_eq!(config.feature_dim, 80);
    }

    #[test]
    fn production_manifest_names_c_api_not_jni() {
        let document = manifest().unwrap();
        let serialized = document.to_string();
        assert!(serialized.contains("libsherpa-onnx-c-api"));
        assert!(!serialized.contains("libsherpa-onnx-jni"));
        assert_eq!(
            document
                .get("runtime")
                .and_then(|r| r.get("abi"))
                .and_then(Value::as_str),
            Some("sherpa-onnx-c-api")
        );
    }
}
