#[cfg(any(whisper_native_linked, test))]
use std::ffi::c_char;
#[cfg(whisper_native_linked)]
use std::ffi::c_void;
use std::ffi::{CStr, CString};

#[cfg(whisper_native_linked)]
pub(super) const WHISPER_SAMPLING_GREEDY: i32 = 0;
// NUL-terminated C string backing the forced English language code.
// Static so its address is stable for the lifetime of a `whisper_full` call.
#[cfg(whisper_native_linked)]
static WHISPER_LANGUAGE_EN: [u8; 3] = [b'e', b'n', 0];

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperAhead {
    n_text_layer: i32,
    n_head: i32,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperAheads {
    n_heads: usize,
    heads: *const WhisperAhead,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperVadParams {
    threshold: f32,
    min_speech_duration_ms: i32,
    min_silence_duration_ms: i32,
    max_speech_duration_s: f32,
    speech_pad_ms: i32,
    samples_overlap: f32,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperGrammarElement {
    type_: i32,
    value: u32,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperGreedy {
    best_of: i32,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperBeamSearch {
    beam_size: i32,
    patience: f32,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperFullParams {
    strategy: i32,
    n_threads: i32,
    n_max_text_ctx: i32,
    offset_ms: i32,
    duration_ms: i32,
    translate: bool,
    no_context: bool,
    no_timestamps: bool,
    single_segment: bool,
    print_special: bool,
    print_progress: bool,
    print_realtime: bool,
    print_timestamps: bool,
    token_timestamps: bool,
    thold_pt: f32,
    thold_ptsum: f32,
    max_len: i32,
    split_on_word: bool,
    max_tokens: i32,
    debug_mode: bool,
    audio_ctx: i32,
    tdrz_enable: bool,
    suppress_regex: *const c_char,
    initial_prompt: *const c_char,
    carry_initial_prompt: bool,
    prompt_tokens: *const i32,
    prompt_n_tokens: i32,
    language: *const c_char,
    detect_language: bool,
    suppress_blank: bool,
    suppress_nst: bool,
    temperature: f32,
    max_initial_ts: f32,
    length_penalty: f32,
    temperature_inc: f32,
    entropy_thold: f32,
    logprob_thold: f32,
    no_speech_thold: f32,
    greedy: WhisperGreedy,
    beam_search: WhisperBeamSearch,
    new_segment_callback: *const (),
    new_segment_callback_user_data: *const (),
    progress_callback: *const (),
    progress_callback_user_data: *const (),
    encoder_begin_callback: *const (),
    encoder_begin_callback_user_data: *const (),
    abort_callback: *const (),
    abort_callback_user_data: *const (),
    logits_filter_callback: *const (),
    logits_filter_callback_user_data: *const (),
    grammar_rules: *const *const WhisperGrammarElement,
    n_grammar_rules: usize,
    i_start_rule: usize,
    grammar_penalty: f32,
    vad: bool,
    vad_model_path: *const c_char,
    vad_params: WhisperVadParams,
}

#[cfg(any(whisper_native_linked, test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct WhisperContextParams {
    use_gpu: bool,
    flash_attn: bool,
    gpu_device: i32,
    dtw_token_timestamps: bool,
    dtw_aheads_preset: i32,
    dtw_n_top: i32,
    dtw_aheads: WhisperAheads,
    dtw_mem_size: usize,
}

#[cfg(any(whisper_native_linked, test))]
const _: [(); 304] = [(); std::mem::size_of::<WhisperFullParams>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 8] = [(); std::mem::align_of::<WhisperFullParams>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 48] = [(); std::mem::size_of::<WhisperContextParams>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 16] = [(); std::mem::size_of::<WhisperAheads>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 8] = [(); std::mem::size_of::<WhisperAhead>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 24] = [(); std::mem::size_of::<WhisperVadParams>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 8] = [(); std::mem::size_of::<WhisperGrammarElement>()];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 0] = [(); std::mem::offset_of!(WhisperFullParams, strategy)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 4] = [(); std::mem::offset_of!(WhisperFullParams, n_threads)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 16] = [(); std::mem::offset_of!(WhisperFullParams, duration_ms)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 20] = [(); std::mem::offset_of!(WhisperFullParams, translate)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 28] = [(); std::mem::offset_of!(WhisperFullParams, token_timestamps)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 32] = [(); std::mem::offset_of!(WhisperFullParams, thold_pt)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 40] = [(); std::mem::offset_of!(WhisperFullParams, max_len)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 52] = [(); std::mem::offset_of!(WhisperFullParams, debug_mode)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 56] = [(); std::mem::offset_of!(WhisperFullParams, audio_ctx)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 60] = [(); std::mem::offset_of!(WhisperFullParams, tdrz_enable)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 64] = [(); std::mem::offset_of!(WhisperFullParams, suppress_regex)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 104] = [(); std::mem::offset_of!(WhisperFullParams, language)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 112] = [(); std::mem::offset_of!(WhisperFullParams, detect_language)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 116] = [(); std::mem::offset_of!(WhisperFullParams, temperature)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 144] = [(); std::mem::offset_of!(WhisperFullParams, greedy)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 148] = [(); std::mem::offset_of!(WhisperFullParams, beam_search)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 160] = [(); std::mem::offset_of!(WhisperFullParams, new_segment_callback)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 264] = [(); std::mem::offset_of!(WhisperFullParams, grammar_penalty)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 268] = [(); std::mem::offset_of!(WhisperFullParams, vad)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 272] = [(); std::mem::offset_of!(WhisperFullParams, vad_model_path)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 280] = [(); std::mem::offset_of!(WhisperFullParams, vad_params)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 0] = [(); std::mem::offset_of!(WhisperContextParams, use_gpu)];
#[cfg(any(whisper_native_linked, test))]
const _: [(); 24] = [(); std::mem::offset_of!(WhisperContextParams, dtw_aheads)];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FfiError {
    pub code: i32,
    pub message: String,
}

impl FfiError {
    #[cfg(not(whisper_native_linked))]
    fn unavailable() -> Self {
        Self {
            code: -1,
            message: "Whisper.cpp native runtime is not linked into this build".to_string(),
        }
    }

    fn invalid_response(message: impl Into<String>) -> Self {
        Self {
            code: -1,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WhisperSegment {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub no_speech_prob: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct WhisperTranscript {
    pub segments: Vec<WhisperSegment>,
    pub no_speech_prob: f32,
}

pub(crate) trait WhisperApi: Send + Sync {
    #[allow(dead_code)]
    fn runtime_version(&self) -> Result<String, FfiError>;
    fn load_model(&self, model_path: &CStr) -> Result<WhisperModel, FfiError>;
    fn transcribe(
        &self,
        model: &WhisperModel,
        audio: &[f32],
    ) -> Result<WhisperTranscript, FfiError>;
}

#[derive(Debug)]
pub(crate) struct WhisperModel {
    ctx: *mut std::ffi::c_void,
}

// SAFETY: WhisperModel is constructed on the local-ASR worker and then remains
// exclusively owned by that worker until Drop. Moving ownership to that worker
// is sound; shared concurrent access is deliberately not promised.
unsafe impl Send for WhisperModel {}

impl WhisperModel {
    #[cfg(whisper_native_linked)]
    fn new(ctx: *mut c_void) -> Self {
        Self { ctx }
    }
}

impl Drop for WhisperModel {
    fn drop(&mut self) {
        #[cfg(whisper_native_linked)]
        {
            // SAFETY: `ctx` is a non-null handle returned by `whisper_init_from_file_with_params`,
            // owned by this RAII wrapper, and dropped exactly once.
            unsafe { whisper_free(self.ctx) }
        }
        #[cfg(not(whisper_native_linked))]
        {
            let _ = self.ctx;
        }
    }
}

#[derive(Debug)]
pub(crate) struct NativeWhisperApi;

impl NativeWhisperApi {
    #[cfg(whisper_native_linked)]
    fn language_ptr() -> *const c_char {
        WHISPER_LANGUAGE_EN.as_ptr() as *const c_char
    }

    #[cfg(whisper_native_linked)]
    fn transcribe(
        &self,
        model: &WhisperModel,
        audio: &[f32],
    ) -> Result<WhisperTranscript, FfiError> {
        if audio.is_empty() {
            return Ok(WhisperTranscript::default());
        }

        // The context was already loaded by `load_model`; `whisper_full` consumes the
        // model from `model.ctx` and re-uses the per-call `params` we build below.
        let n_samples = i32::try_from(audio.len()).map_err(|_| {
            FfiError::invalid_response("Whisper audio buffer exceeds the native sample-count limit")
        })?;

        // SAFETY: `whisper_full_default_params` returns a by-value C struct with every
        // field at the exact C width declared in the pinned whisper.h. We only mutate
        // fields that are safe to own here (`language` points to a static), and the
        // struct itself is moved by value into `whisper_full`.
        let mut params = unsafe { whisper_full_default_params(WHISPER_SAMPLING_GREEDY) };
        params.language = Self::language_ptr();
        params.detect_language = false;
        params.suppress_blank = true;

        let ret = unsafe { whisper_full(model.ctx, params, audio.as_ptr(), n_samples) };
        if ret != 0 {
            return Err(FfiError::invalid_response(format!(
                "whisper_full returned non-zero status {ret}"
            )));
        }

        let n_segments = unsafe { whisper_full_n_segments(model.ctx) };
        if n_segments < 0 {
            return Err(FfiError::invalid_response(
                "whisper_full_n_segments returned a negative count",
            ));
        }

        let mut segments = Vec::with_capacity(n_segments.max(0) as usize);
        for i in 0..n_segments {
            // SAFETY: a successful `whisper_full` leaves `n_segments` segments owned by
            // the context and valid until the next `whisper_full` call on the same
            // context. We read and copy all fields before returning.
            let text_ptr = unsafe { whisper_full_get_segment_text(model.ctx, i) };
            let text = if text_ptr.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(text_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            let start_ms = unsafe { whisper_full_get_segment_t0(model.ctx, i) };
            let end_ms = unsafe { whisper_full_get_segment_t1(model.ctx, i) };
            let no_speech_prob = unsafe { whisper_full_get_segment_no_speech_prob(model.ctx, i) };
            segments.push(WhisperSegment {
                text,
                start_ms,
                end_ms,
                no_speech_prob,
            });
        }

        let no_speech_prob = if segments.is_empty() {
            0.0
        } else {
            segments.iter().map(|s| s.no_speech_prob).sum::<f32>() / segments.len() as f32
        };

        Ok(WhisperTranscript {
            segments,
            no_speech_prob,
        })
    }
}

impl WhisperApi for NativeWhisperApi {
    #[allow(dead_code)]
    fn runtime_version(&self) -> Result<String, FfiError> {
        #[cfg(whisper_native_linked)]
        {
            // SAFETY: `whisper_version` takes no arguments and returns a library-owned
            // NUL-terminated string that we copy immediately.
            let ptr = unsafe { whisper_version() };
            if ptr.is_null() {
                Err(FfiError::invalid_response("whisper_version returned null"))
            } else {
                Ok(unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned())
            }
        }
        #[cfg(not(whisper_native_linked))]
        {
            Err(FfiError::unavailable())
        }
    }

    fn load_model(&self, model_path: &CStr) -> Result<WhisperModel, FfiError> {
        #[cfg(whisper_native_linked)]
        {
            // SAFETY: `model_path` remains alive for the duration of the call; `cparams`
            // is a by-value C struct with every field at the exact C width.
            let cparams = unsafe { whisper_context_default_params() };
            let ctx = unsafe { whisper_init_from_file_with_params(model_path.as_ptr(), cparams) };
            if ctx.is_null() {
                return Err(FfiError::invalid_response(
                    "whisper_init_from_file_with_params failed to load the verified model",
                ));
            }
            Ok(WhisperModel::new(ctx))
        }
        #[cfg(not(whisper_native_linked))]
        {
            let _ = model_path;
            Err(FfiError::unavailable())
        }
    }

    fn transcribe(
        &self,
        model: &WhisperModel,
        audio: &[f32],
    ) -> Result<WhisperTranscript, FfiError> {
        #[cfg(whisper_native_linked)]
        {
            self.transcribe(model, audio)
        }
        #[cfg(not(whisper_native_linked))]
        {
            let _ = (model, audio);
            Err(FfiError::unavailable())
        }
    }
}

pub(crate) fn path_to_cstring(path: &std::path::Path) -> Result<CString, FfiError> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| FfiError::invalid_response("Whisper model path contains an interior NUL byte"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_params_layout_matches_compiler_probe() {
        assert_eq!(std::mem::size_of::<WhisperFullParams>(), 304);
        assert_eq!(std::mem::align_of::<WhisperFullParams>(), 8);
        assert_eq!(std::mem::offset_of!(WhisperFullParams, language), 104);
        assert_eq!(
            std::mem::offset_of!(WhisperFullParams, detect_language),
            112
        );
        assert_eq!(std::mem::offset_of!(WhisperFullParams, greedy), 144);
        assert_eq!(
            std::mem::offset_of!(WhisperFullParams, new_segment_callback),
            160
        );
        assert_eq!(std::mem::offset_of!(WhisperFullParams, vad_params), 280);
    }

    #[test]
    fn context_params_layout_matches_compiler_probe() {
        assert_eq!(std::mem::size_of::<WhisperContextParams>(), 48);
        assert_eq!(std::mem::offset_of!(WhisperContextParams, dtw_aheads), 24);
    }
}

// SAFETY: declarations below are copied from the pinned whisper.cpp header
// `include/whisper.h`. The build script only enables this block when the pinned
// static runtime staging directory is complete.
#[cfg(whisper_native_linked)]
unsafe extern "C" {
    #[allow(dead_code)]
    fn whisper_version() -> *const c_char;
    fn whisper_init_from_file_with_params(
        path_model: *const c_char,
        cparams: WhisperContextParams,
    ) -> *mut c_void;
    fn whisper_full(
        ctx: *mut c_void,
        wparams: WhisperFullParams,
        samples: *const f32,
        n_samples: i32,
    ) -> i32;
    fn whisper_full_n_segments(ctx: *mut c_void) -> i32;
    fn whisper_full_get_segment_text(ctx: *mut c_void, i_segment: i32) -> *const c_char;
    fn whisper_full_get_segment_t0(ctx: *mut c_void, i_segment: i32) -> i64;
    fn whisper_full_get_segment_t1(ctx: *mut c_void, i_segment: i32) -> i64;
    fn whisper_full_get_segment_no_speech_prob(ctx: *mut c_void, i_segment: i32) -> f32;
    fn whisper_context_default_params() -> WhisperContextParams;
    fn whisper_full_default_params(strategy: i32) -> WhisperFullParams;
    fn whisper_free(ctx: *mut c_void);
}
