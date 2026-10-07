use std::ffi::{CStr, CString};
#[cfg(whisper_native_linked)]
use std::ffi::{c_char, c_void};

pub(super) const WHISPER_SAMPLING_GREEDY: i32 = 0;
// NUL-terminated C string backing the forced English language code.
// Static so its address is stable for the lifetime of a `whisper_full` call.
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
