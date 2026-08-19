#[cfg(moonshine_native_linked)]
use std::ffi::{c_char, c_void};
use std::ffi::{CStr, CString};
#[cfg(moonshine_native_linked)]
use std::{ptr, slice};

pub(super) const MOONSHINE_HEADER_VERSION: i32 = 30_000;
pub(super) const MOONSHINE_MODEL_ARCH_TINY_STREAMING: u32 = 2;
pub(super) const MOONSHINE_MODEL_ARCH_SMALL_STREAMING: u32 = 4;
#[cfg(moonshine_native_linked)]
pub(super) const MOONSHINE_FLAG_FORCE_UPDATE: u32 = 1;

#[cfg(moonshine_native_linked)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MoonshineTranscriptLine {
    text: *const c_char,
    _audio_data: *const f32,
    _audio_data_count: usize,
    start_time: f32,
    duration: f32,
    id: u64,
    is_complete: i8,
    is_updated: i8,
    is_new: i8,
    has_text_changed: i8,
    _have_speakers_changed: i8,
    _speaker_spans: *const c_void,
    _speaker_span_count: u64,
    last_transcription_latency_ms: u32,
    _words: *const c_void,
    _word_count: u64,
}

#[cfg(moonshine_native_linked)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MoonshineTranscript {
    lines: *mut MoonshineTranscriptLine,
    line_count: u64,
}

#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 88] = [(); std::mem::size_of::<MoonshineTranscriptLine>()];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 8] = [(); std::mem::align_of::<MoonshineTranscriptLine>()];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 16] = [(); std::mem::size_of::<MoonshineTranscript>()];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 0] = [(); std::mem::offset_of!(MoonshineTranscriptLine, text)];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 24] = [(); std::mem::offset_of!(MoonshineTranscriptLine, start_time)];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 32] = [(); std::mem::offset_of!(MoonshineTranscriptLine, id)];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 40] = [(); std::mem::offset_of!(MoonshineTranscriptLine, is_complete)];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 64] =
    [(); std::mem::offset_of!(MoonshineTranscriptLine, last_transcription_latency_ms)];
#[cfg(all(moonshine_native_linked, target_pointer_width = "64"))]
const _: [(); 8] = [(); std::mem::offset_of!(MoonshineTranscript, line_count)];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FfiError {
    pub code: i32,
    pub message: String,
}

impl FfiError {
    fn unavailable() -> Self {
        Self {
            code: -1,
            message: "Moonshine native runtime is not linked into this build".to_string(),
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
pub(super) struct OwnedNativeLine {
    pub text: String,
    pub start_time: f32,
    pub duration: f32,
    pub id: u64,
    pub is_complete: bool,
    pub is_updated: bool,
    pub is_new: bool,
    pub has_text_changed: bool,
    pub last_transcription_latency_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct OwnedNativeTranscript {
    pub lines: Vec<OwnedNativeLine>,
}

pub(super) trait MoonshineApi: Send + Sync {
    fn runtime_version(&self) -> Result<i32, FfiError>;
    fn load_transcriber(&self, model_path: &CStr, model_arch: u32) -> Result<i32, FfiError>;
    fn free_transcriber(&self, transcriber_handle: i32);
    fn create_stream(&self, transcriber_handle: i32) -> Result<i32, FfiError>;
    fn free_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError>;
    fn start_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError>;
    fn stop_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError>;
    fn add_audio(
        &self,
        transcriber_handle: i32,
        stream_handle: i32,
        audio: &[f32],
        sample_rate: i32,
    ) -> Result<(), FfiError>;
    fn transcribe_stream(
        &self,
        transcriber_handle: i32,
        stream_handle: i32,
        force_update: bool,
    ) -> Result<OwnedNativeTranscript, FfiError>;
}

#[derive(Debug, Default)]
pub(super) struct NativeMoonshineApi;

impl NativeMoonshineApi {
    #[cfg(moonshine_native_linked)]
    fn error(&self, code: i32) -> FfiError {
        // SAFETY: `moonshine_error_to_string` returns either NULL or a pointer to a
        // library-owned NUL-terminated string. We copy it immediately and never free it.
        let message = unsafe {
            let ptr = moonshine_error_to_string(code);
            if ptr.is_null() {
                format!("Moonshine native error {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        FfiError { code, message }
    }

    #[cfg(moonshine_native_linked)]
    fn status(&self, code: i32) -> Result<(), FfiError> {
        if code == 0 {
            Ok(())
        } else {
            Err(self.error(code))
        }
    }

    #[cfg(moonshine_native_linked)]
    fn handle(&self, handle: i32) -> Result<i32, FfiError> {
        if handle >= 0 {
            Ok(handle)
        } else {
            Err(self.error(handle))
        }
    }

    #[cfg(moonshine_native_linked)]
    fn copy_transcript(
        &self,
        transcript: *mut MoonshineTranscript,
    ) -> Result<OwnedNativeTranscript, FfiError> {
        if transcript.is_null() {
            return Err(FfiError::invalid_response(
                "Moonshine returned a null transcript pointer",
            ));
        }

        // SAFETY: a successful `moonshine_transcribe_stream` returns a transcript
        // owned by the transcriber and valid until the next call on that transcriber.
        // This helper is invoked before any subsequent native call and copies all V1
        // fields into Rust-owned values before returning.
        let transcript = unsafe { &*transcript };
        let line_count = usize::try_from(transcript.line_count).map_err(|_| {
            FfiError::invalid_response("Moonshine transcript line count does not fit usize")
        })?;

        if line_count == 0 {
            return Ok(OwnedNativeTranscript::default());
        }
        if transcript.lines.is_null() {
            return Err(FfiError::invalid_response(
                "Moonshine returned a null line array with a non-zero line count",
            ));
        }

        // SAFETY: the upstream transcript contract guarantees `lines` references
        // `line_count` contiguous `transcript_line_t` values for the lifetime stated
        // above. We only read that slice during this copy operation.
        let lines = unsafe { slice::from_raw_parts(transcript.lines, line_count) };
        let mut owned = Vec::with_capacity(line_count);
        for line in lines {
            let text = if line.text.is_null() {
                String::new()
            } else {
                // SAFETY: upstream documents `text` as UTF-8, NUL-terminated, and
                // transcriber-owned until the next native call. We copy it now.
                unsafe { CStr::from_ptr(line.text) }
                    .to_string_lossy()
                    .into_owned()
            };
            owned.push(OwnedNativeLine {
                text,
                start_time: line.start_time,
                duration: line.duration,
                id: line.id,
                is_complete: line.is_complete != 0,
                is_updated: line.is_updated != 0,
                is_new: line.is_new != 0,
                has_text_changed: line.has_text_changed != 0,
                last_transcription_latency_ms: line.last_transcription_latency_ms,
            });
        }
        Ok(OwnedNativeTranscript { lines: owned })
    }
}

impl MoonshineApi for NativeMoonshineApi {
    fn runtime_version(&self) -> Result<i32, FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: this function takes no arguments and returns a plain integer.
            return Ok(unsafe { moonshine_get_version() });
        }
        #[cfg(not(moonshine_native_linked))]
        {
            Err(FfiError::unavailable())
        }
    }

    fn load_transcriber(&self, model_path: &CStr, model_arch: u32) -> Result<i32, FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: `model_path` remains alive for the duration of the call; the
            // options pointer is NULL because the count is zero; all scalars use the
            // exact C widths declared in the pinned v3 header.
            let handle = unsafe {
                moonshine_load_transcriber_from_files(
                    model_path.as_ptr(),
                    model_arch,
                    ptr::null(),
                    0,
                    MOONSHINE_HEADER_VERSION,
                )
            };
            return self.handle(handle);
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = (model_path, model_arch);
            Err(FfiError::unavailable())
        }
    }

    fn free_transcriber(&self, transcriber_handle: i32) {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: callers only pass a non-negative handle owned by the RAII
            // wrapper, and Drop invokes this exactly once for that owner.
            unsafe { moonshine_free_transcriber(transcriber_handle) };
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = transcriber_handle;
        }
    }

    fn create_stream(&self, transcriber_handle: i32) -> Result<i32, FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: the transcriber handle is kept alive by the owning wrapper.
            let handle = unsafe { moonshine_create_stream(transcriber_handle, 0) };
            return self.handle(handle);
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = transcriber_handle;
            Err(FfiError::unavailable())
        }
    }

    fn free_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: both handles are owned by the RAII wrapper and the parent
            // transcriber is retained until after this stream has been freed.
            return self
                .status(unsafe { moonshine_free_stream(transcriber_handle, stream_handle) });
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = (transcriber_handle, stream_handle);
            Err(FfiError::unavailable())
        }
    }

    fn start_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: both handles are valid for this owned parent/child pair.
            return self
                .status(unsafe { moonshine_start_stream(transcriber_handle, stream_handle) });
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = (transcriber_handle, stream_handle);
            Err(FfiError::unavailable())
        }
    }

    fn stop_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            // SAFETY: both handles are valid for this owned parent/child pair.
            return self
                .status(unsafe { moonshine_stop_stream(transcriber_handle, stream_handle) });
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = (transcriber_handle, stream_handle);
            Err(FfiError::unavailable())
        }
    }

    fn add_audio(
        &self,
        transcriber_handle: i32,
        stream_handle: i32,
        audio: &[f32],
        sample_rate: i32,
    ) -> Result<(), FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            let audio_len = u64::try_from(audio.len()).map_err(|_| {
                FfiError::invalid_response("audio buffer length does not fit Moonshine u64")
            })?;
            // SAFETY: the slice pointer remains valid for the duration of this call,
            // and the upstream API only buffers/copies the supplied samples.
            return self.status(unsafe {
                moonshine_transcribe_add_audio_to_stream(
                    transcriber_handle,
                    stream_handle,
                    audio.as_ptr(),
                    audio_len,
                    sample_rate,
                    0,
                )
            });
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = (transcriber_handle, stream_handle, audio, sample_rate);
            Err(FfiError::unavailable())
        }
    }

    fn transcribe_stream(
        &self,
        transcriber_handle: i32,
        stream_handle: i32,
        force_update: bool,
    ) -> Result<OwnedNativeTranscript, FfiError> {
        #[cfg(moonshine_native_linked)]
        {
            let flags = if force_update {
                MOONSHINE_FLAG_FORCE_UPDATE
            } else {
                0
            };
            let mut transcript = ptr::null_mut();
            // SAFETY: both handles are live and `out_transcript` points to writable
            // stack storage for the duration of the call. The returned library-owned
            // pointer is copied into Rust-owned data before this method returns.
            self.status(unsafe {
                moonshine_transcribe_stream(
                    transcriber_handle,
                    stream_handle,
                    flags,
                    &mut transcript,
                )
            })?;
            return self.copy_transcript(transcript);
        }
        #[cfg(not(moonshine_native_linked))]
        {
            let _ = (transcriber_handle, stream_handle, force_update);
            Err(FfiError::unavailable())
        }
    }
}

pub(super) fn path_to_cstring(path: &std::path::Path) -> Result<CString, FfiError> {
    CString::new(path.to_string_lossy().as_bytes()).map_err(|_| {
        FfiError::invalid_response("Moonshine model path contains an interior NUL byte")
    })
}

// SAFETY: declarations below are copied from Moonshine Voice v0.1.3
// `core/moonshine-c-api.h` (header version 3.0.0). The build script only enables
// this block when an explicit native library directory is supplied.
#[cfg(moonshine_native_linked)]
unsafe extern "C" {
    fn moonshine_get_version() -> i32;
    fn moonshine_error_to_string(error: i32) -> *const c_char;
    fn moonshine_load_transcriber_from_files(
        path: *const c_char,
        model_arch: u32,
        options: *const c_void,
        options_count: u64,
        moonshine_version: i32,
    ) -> i32;
    fn moonshine_free_transcriber(transcriber_handle: i32);
    fn moonshine_create_stream(transcriber_handle: i32, flags: u32) -> i32;
    fn moonshine_free_stream(transcriber_handle: i32, stream_handle: i32) -> i32;
    fn moonshine_start_stream(transcriber_handle: i32, stream_handle: i32) -> i32;
    fn moonshine_stop_stream(transcriber_handle: i32, stream_handle: i32) -> i32;
    fn moonshine_transcribe_add_audio_to_stream(
        transcriber_handle: i32,
        stream_handle: i32,
        new_audio_data: *const f32,
        audio_length: u64,
        sample_rate: i32,
        flags: u32,
    ) -> i32;
    fn moonshine_transcribe_stream(
        transcriber_handle: i32,
        stream_handle: i32,
        flags: u32,
        out_transcript: *mut *mut MoonshineTranscript,
    ) -> i32;
}
