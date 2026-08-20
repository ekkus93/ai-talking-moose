use super::ffi::{
    path_to_cstring, FfiError, MoonshineApi, NativeMoonshineApi, OwnedNativeTranscript,
    MOONSHINE_HEADER_VERSION, MOONSHINE_MODEL_ARCH_SMALL_STREAMING,
    MOONSHINE_MODEL_ARCH_TINY_STREAMING,
};
use super::manifest::manifest_for_architecture;
use crate::asr::{AsrError, AsrErrorKind};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonshineModelArchitecture {
    TinyStreaming,
    SmallStreaming,
}

impl MoonshineModelArchitecture {
    fn native_value(self) -> u32 {
        match self {
            Self::TinyStreaming => MOONSHINE_MODEL_ARCH_TINY_STREAMING,
            Self::SmallStreaming => MOONSHINE_MODEL_ARCH_SMALL_STREAMING,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoonshineLine {
    pub text: String,
    pub start_time_seconds: f32,
    pub duration_seconds: f32,
    pub id: u64,
    pub is_complete: bool,
    pub is_updated: bool,
    pub is_new: bool,
    pub has_text_changed: bool,
    pub last_transcription_latency_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MoonshineTranscript {
    pub lines: Vec<MoonshineLine>,
}

struct TranscriberInner {
    api: Arc<dyn MoonshineApi>,
    handle: i32,
    runtime_version: i32,
}

impl Drop for TranscriberInner {
    fn drop(&mut self) {
        self.api.free_transcriber(self.handle);
    }
}

#[derive(Clone)]
pub struct MoonshineTranscriber {
    inner: Arc<TranscriberInner>,
}

impl MoonshineTranscriber {
    pub fn load(
        model_path: &Path,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Self, AsrError> {
        Self::load_with_api(model_path, architecture, Arc::new(NativeMoonshineApi))
    }

    fn load_with_api(
        model_path: &Path,
        architecture: MoonshineModelArchitecture,
        api: Arc<dyn MoonshineApi>,
    ) -> Result<Self, AsrError> {
        manifest_for_architecture(architecture)
            .validate()
            .map_err(|_| AsrError {
                kind: AsrErrorKind::Internal,
                message:
                    "Moonshine model metadata is invalid. Update or reinstall the application."
                        .to_string(),
                retryable: false,
            })?;

        let runtime_version = api
            .runtime_version()
            .map_err(|error| map_ffi_error(AsrErrorKind::RuntimeUnavailable, error, true))?;
        if runtime_version < MOONSHINE_HEADER_VERSION {
            return Err(AsrError {
                kind: AsrErrorKind::RuntimeUnavailable,
                message: format!(
                    "Moonshine runtime version {runtime_version} is older than required header version {MOONSHINE_HEADER_VERSION}"
                ),
                retryable: false,
            });
        }

        let model_path = path_to_cstring(model_path)
            .map_err(|error| map_ffi_error(AsrErrorKind::ModelLoadFailed, error, false))?;
        let handle = api
            .load_transcriber(&model_path, architecture.native_value())
            .map_err(|error| map_ffi_error(AsrErrorKind::ModelLoadFailed, error, false))?;

        Ok(Self {
            inner: Arc::new(TranscriberInner {
                api,
                handle,
                runtime_version,
            }),
        })
    }

    pub fn runtime_version(&self) -> i32 {
        self.inner.runtime_version
    }

    pub fn create_stream(&self) -> Result<MoonshineStream, AsrError> {
        let handle = self
            .inner
            .api
            .create_stream(self.inner.handle)
            .map_err(|error| map_ffi_error(AsrErrorKind::InvalidState, error, true))?;
        Ok(MoonshineStream {
            parent: self.inner.clone(),
            handle,
            started: false,
        })
    }
}

pub struct MoonshineStream {
    parent: Arc<TranscriberInner>,
    handle: i32,
    started: bool,
}

impl MoonshineStream {
    pub fn start(&mut self) -> Result<(), AsrError> {
        if self.started {
            return Ok(());
        }
        self.parent
            .api
            .start_stream(self.parent.handle, self.handle)
            .map_err(|error| map_ffi_error(AsrErrorKind::InvalidState, error, true))?;
        self.started = true;
        Ok(())
    }

    pub fn add_audio(&mut self, audio: &[f32], sample_rate_hz: u32) -> Result<(), AsrError> {
        if !self.started {
            return Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: "Moonshine stream must be started before adding audio".to_string(),
                retryable: true,
            });
        }
        let sample_rate = i32::try_from(sample_rate_hz).map_err(|_| AsrError {
            kind: AsrErrorKind::AudioInput,
            message: "audio sample rate does not fit Moonshine i32".to_string(),
            retryable: false,
        })?;
        self.parent
            .api
            .add_audio(self.parent.handle, self.handle, audio, sample_rate)
            .map_err(|error| map_ffi_error(AsrErrorKind::AudioInput, error, true))
    }

    pub fn transcribe(&mut self, force_update: bool) -> Result<MoonshineTranscript, AsrError> {
        if !self.started {
            return Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: "Moonshine stream must be started before transcription".to_string(),
                retryable: true,
            });
        }
        let transcript = self
            .parent
            .api
            .transcribe_stream(self.parent.handle, self.handle, force_update)
            .map_err(|error| map_ffi_error(AsrErrorKind::Inference, error, true))?;
        Ok(copy_owned_transcript(transcript))
    }

    pub fn stop(&mut self) -> Result<(), AsrError> {
        if !self.started {
            return Ok(());
        }
        self.parent
            .api
            .stop_stream(self.parent.handle, self.handle)
            .map_err(|error| map_ffi_error(AsrErrorKind::InvalidState, error, true))?;
        self.started = false;
        Ok(())
    }
}

impl Drop for MoonshineStream {
    fn drop(&mut self) {
        if self.started {
            let _ = self.parent.api.stop_stream(self.parent.handle, self.handle);
            self.started = false;
        }
        let _ = self.parent.api.free_stream(self.parent.handle, self.handle);
    }
}

fn copy_owned_transcript(transcript: OwnedNativeTranscript) -> MoonshineTranscript {
    MoonshineTranscript {
        lines: transcript
            .lines
            .into_iter()
            .map(|line| MoonshineLine {
                text: line.text,
                start_time_seconds: line.start_time,
                duration_seconds: line.duration,
                id: line.id,
                is_complete: line.is_complete,
                is_updated: line.is_updated,
                is_new: line.is_new,
                has_text_changed: line.has_text_changed,
                last_transcription_latency_ms: line.last_transcription_latency_ms,
            })
            .collect(),
    }
}

fn map_ffi_error(kind: AsrErrorKind, error: FfiError, retryable: bool) -> AsrError {
    AsrError {
        kind,
        message: format!("{} (Moonshine code {})", error.message, error.code),
        retryable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::moonshine::ffi::{FfiError, OwnedNativeLine};
    use parking_lot::Mutex;
    use std::ffi::CStr;

    struct FakeApi {
        calls: Mutex<Vec<String>>,
        runtime_version: i32,
        load_result: Mutex<Result<i32, FfiError>>,
        create_stream_result: Mutex<Result<i32, FfiError>>,
        transcript: Mutex<OwnedNativeTranscript>,
    }

    impl FakeApi {
        fn healthy() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                runtime_version: MOONSHINE_HEADER_VERSION,
                load_result: Mutex::new(Ok(10)),
                create_stream_result: Mutex::new(Ok(20)),
                transcript: Mutex::new(OwnedNativeTranscript::default()),
            }
        }

        fn record(&self, call: impl Into<String>) {
            self.calls.lock().push(call.into());
        }
    }

    impl MoonshineApi for FakeApi {
        fn runtime_version(&self) -> Result<i32, FfiError> {
            self.record("runtime_version");
            Ok(self.runtime_version)
        }

        fn load_transcriber(&self, _model_path: &CStr, model_arch: u32) -> Result<i32, FfiError> {
            self.record(format!("load:{model_arch}"));
            self.load_result.lock().clone()
        }

        fn free_transcriber(&self, transcriber_handle: i32) {
            self.record(format!("free_transcriber:{transcriber_handle}"));
        }

        fn create_stream(&self, transcriber_handle: i32) -> Result<i32, FfiError> {
            self.record(format!("create_stream:{transcriber_handle}"));
            self.create_stream_result.lock().clone()
        }

        fn free_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError> {
            self.record(format!("free_stream:{transcriber_handle}:{stream_handle}"));
            Ok(())
        }

        fn start_stream(
            &self,
            transcriber_handle: i32,
            stream_handle: i32,
        ) -> Result<(), FfiError> {
            self.record(format!("start:{transcriber_handle}:{stream_handle}"));
            Ok(())
        }

        fn stop_stream(&self, transcriber_handle: i32, stream_handle: i32) -> Result<(), FfiError> {
            self.record(format!("stop:{transcriber_handle}:{stream_handle}"));
            Ok(())
        }

        fn add_audio(
            &self,
            transcriber_handle: i32,
            stream_handle: i32,
            audio: &[f32],
            sample_rate: i32,
        ) -> Result<(), FfiError> {
            self.record(format!(
                "audio:{transcriber_handle}:{stream_handle}:{}:{sample_rate}",
                audio.len()
            ));
            Ok(())
        }

        fn transcribe_stream(
            &self,
            transcriber_handle: i32,
            stream_handle: i32,
            force_update: bool,
        ) -> Result<OwnedNativeTranscript, FfiError> {
            self.record(format!(
                "transcribe:{transcriber_handle}:{stream_handle}:{force_update}"
            ));
            Ok(self.transcript.lock().clone())
        }
    }

    #[cfg(not(moonshine_native_linked))]
    #[test]
    fn normal_build_reports_native_runtime_unavailable_without_linking() {
        let result = MoonshineTranscriber::load(
            Path::new("/tmp/not-installed"),
            MoonshineModelArchitecture::TinyStreaming,
        );
        let error = result
            .err()
            .expect("build without native linkage must fail closed");
        assert_eq!(error.kind, AsrErrorKind::RuntimeUnavailable);
    }

    #[test]
    fn rejects_runtime_older_than_header() {
        let api = Arc::new(FakeApi {
            runtime_version: MOONSHINE_HEADER_VERSION - 1,
            ..FakeApi::healthy()
        });
        let result = MoonshineTranscriber::load_with_api(
            Path::new("/tmp/model"),
            MoonshineModelArchitecture::TinyStreaming,
            api,
        );
        let error = result.err().expect("old runtime should be rejected");
        assert_eq!(error.kind, AsrErrorKind::RuntimeUnavailable);
    }

    #[test]
    fn maps_negative_load_error_to_model_load_error() {
        let api = Arc::new(FakeApi::healthy());
        *api.load_result.lock() = Err(FfiError {
            code: -3,
            message: "bad model".to_string(),
        });
        let result = MoonshineTranscriber::load_with_api(
            Path::new("/tmp/model"),
            MoonshineModelArchitecture::SmallStreaming,
            api,
        );
        let error = result.err().expect("load failure should be surfaced");
        assert_eq!(error.kind, AsrErrorKind::ModelLoadFailed);
        assert!(error.message.contains("-3"));
    }

    #[test]
    fn invalid_stream_handle_is_typed_and_does_not_free_a_child() {
        let api = Arc::new(FakeApi::healthy());
        *api.create_stream_result.lock() = Err(FfiError {
            code: -7,
            message: "invalid transcriber handle".to_string(),
        });
        let transcriber = MoonshineTranscriber::load_with_api(
            Path::new("/tmp/model"),
            MoonshineModelArchitecture::TinyStreaming,
            api.clone(),
        )
        .unwrap();

        let error = transcriber
            .create_stream()
            .err()
            .expect("invalid stream handle should be surfaced");
        assert_eq!(error.kind, AsrErrorKind::InvalidState);
        assert!(error.message.contains("-7"));
        drop(transcriber);

        let calls = api.calls.lock();
        assert!(!calls.iter().any(|call| call.starts_with("free_stream:")));
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.as_str() == "free_transcriber:10")
                .count(),
            1
        );
    }

    #[test]
    fn stream_drop_precedes_parent_drop_and_happens_once() {
        let api = Arc::new(FakeApi::healthy());
        let transcriber = MoonshineTranscriber::load_with_api(
            Path::new("/tmp/model"),
            MoonshineModelArchitecture::TinyStreaming,
            api.clone(),
        )
        .unwrap();
        let mut stream = transcriber.create_stream().unwrap();
        stream.start().unwrap();
        drop(transcriber);
        assert!(!api
            .calls
            .lock()
            .iter()
            .any(|call| call.starts_with("free_transcriber:")));

        drop(stream);
        let calls = api.calls.lock().clone();
        let stop = calls.iter().position(|call| call == "stop:10:20").unwrap();
        let free_stream = calls
            .iter()
            .position(|call| call == "free_stream:10:20")
            .unwrap();
        let free_transcriber = calls
            .iter()
            .position(|call| call == "free_transcriber:10")
            .unwrap();
        assert!(stop < free_stream);
        assert!(free_stream < free_transcriber);
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.as_str() == "free_stream:10:20")
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.as_str() == "free_transcriber:10")
                .count(),
            1
        );
    }

    #[test]
    fn transcript_is_rust_owned_and_preserves_v1_metadata() {
        let api = Arc::new(FakeApi::healthy());
        *api.transcript.lock() = OwnedNativeTranscript {
            lines: vec![OwnedNativeLine {
                text: "hello moose".to_string(),
                start_time: 1.25,
                duration: 0.75,
                id: 42,
                is_complete: true,
                is_updated: true,
                is_new: true,
                has_text_changed: true,
                last_transcription_latency_ms: 17,
            }],
        };
        let transcriber = MoonshineTranscriber::load_with_api(
            Path::new("/tmp/model"),
            MoonshineModelArchitecture::SmallStreaming,
            api,
        )
        .unwrap();
        let mut stream = transcriber.create_stream().unwrap();
        stream.start().unwrap();
        stream.add_audio(&[0.0, 0.25, -0.25], 16_000).unwrap();
        let transcript = stream.transcribe(true).unwrap();

        assert_eq!(transcript.lines.len(), 1);
        let line = &transcript.lines[0];
        assert_eq!(line.text, "hello moose");
        assert_eq!(line.id, 42);
        assert!(line.is_complete);
        assert_eq!(line.last_transcription_latency_ms, 17);
    }
}
