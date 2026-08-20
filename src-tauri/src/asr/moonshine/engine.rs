use super::installer::{
    MoonshineModelInstallError, MoonshineModelInstallErrorKind, MoonshineModelInstaller,
};
use super::runtime::{
    MoonshineModelArchitecture, MoonshineStream, MoonshineTranscriber, MoonshineTranscript,
};
use crate::asr::{AsrError, AsrErrorKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Moonshine's V1 local-ASR input contract.
///
/// V1R-ASR-009 owns microphone format conversion and must feed this engine mono
/// `f32` PCM at exactly 16 kHz. This engine never sends microphone audio to a
/// cloud provider and never falls back to Gemini Live audio recognition.
pub const MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ: u32 = 16_000;

/// One meaningful transcript change emitted by the Tiny streaming engine.
///
/// Native line IDs are preserved so V1R-ASR-010 can build the higher-level
/// utterance accumulator without inferring identity from text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoonshineTinyTranscriptUpdate {
    Partial {
        line_id: u64,
        text: String,
        latency_ms: u32,
    },
    Final {
        line_id: u64,
        text: String,
        latency_ms: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmittedLineState {
    text: String,
    is_final: bool,
}

trait TinyModelResolver {
    fn resolve_verified_tiny_model(&self) -> Result<PathBuf, AsrError>;
}

impl TinyModelResolver for MoonshineModelInstaller {
    fn resolve_verified_tiny_model(&self) -> Result<PathBuf, AsrError> {
        match self.verify_installed(MoonshineModelArchitecture::TinyStreaming) {
            Ok(Some(installed)) => Ok(installed.model_path),
            Ok(None) => Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: "Moonshine Tiny is not installed. Download it in Settings before starting local speech recognition. No microphone audio was sent to Google."
                    .to_string(),
                retryable: true,
            }),
            Err(error) => Err(map_installer_error(error)),
        }
    }
}

fn map_installer_error(error: MoonshineModelInstallError) -> AsrError {
    match error.kind {
        MoonshineModelInstallErrorKind::CorruptInstall
        | MoonshineModelInstallErrorKind::SizeMismatch
        | MoonshineModelInstallErrorKind::Sha256Mismatch
        | MoonshineModelInstallErrorKind::Crc32cMismatch => AsrError {
            kind: AsrErrorKind::ModelCorrupt,
            message: "Moonshine Tiny is incomplete or corrupt. Reinstall it in Settings before starting local speech recognition. No microphone audio was sent to Google."
                .to_string(),
            retryable: true,
        },
        MoonshineModelInstallErrorKind::Cancelled => AsrError {
            kind: AsrErrorKind::Cancelled,
            message: "Moonshine Tiny model verification was cancelled.".to_string(),
            retryable: true,
        },
        MoonshineModelInstallErrorKind::InvalidManifest
        | MoonshineModelInstallErrorKind::UnsupportedArtifact => AsrError {
            kind: AsrErrorKind::Internal,
            message: "The bundled Moonshine Tiny model metadata is invalid. Update or reinstall the application."
                .to_string(),
            retryable: false,
        },
        MoonshineModelInstallErrorKind::InsufficientDiskSpace
        | MoonshineModelInstallErrorKind::Network
        | MoonshineModelInstallErrorKind::Http
        | MoonshineModelInstallErrorKind::Io
        | MoonshineModelInstallErrorKind::Promotion => AsrError {
            kind: AsrErrorKind::ModelLoadFailed,
            message: "Moonshine Tiny could not be verified before local speech recognition started."
                .to_string(),
            retryable: true,
        },
    }
}

trait TinyStreamBackend: Send {
    fn add_audio(&mut self, pcm: &[f32], sample_rate_hz: u32) -> Result<(), AsrError>;
    fn transcribe(&mut self, force_update: bool) -> Result<MoonshineTranscript, AsrError>;
    fn stop(&mut self) -> Result<(), AsrError>;
}

struct NativeTinyStreamBackend {
    stream: MoonshineStream,
}

impl TinyStreamBackend for NativeTinyStreamBackend {
    fn add_audio(&mut self, pcm: &[f32], sample_rate_hz: u32) -> Result<(), AsrError> {
        self.stream.add_audio(pcm, sample_rate_hz)
    }

    fn transcribe(&mut self, force_update: bool) -> Result<MoonshineTranscript, AsrError> {
        self.stream.transcribe(force_update)
    }

    fn stop(&mut self) -> Result<(), AsrError> {
        self.stream.stop()
    }
}

trait TinyStreamFactory {
    fn open(&self, model_path: &Path) -> Result<Box<dyn TinyStreamBackend>, AsrError>;
}

struct NativeTinyStreamFactory;

impl TinyStreamFactory for NativeTinyStreamFactory {
    fn open(&self, model_path: &Path) -> Result<Box<dyn TinyStreamBackend>, AsrError> {
        let transcriber =
            MoonshineTranscriber::load(model_path, MoonshineModelArchitecture::TinyStreaming)?;
        let mut stream = transcriber.create_stream()?;
        stream.start()?;
        Ok(Box::new(NativeTinyStreamBackend { stream }))
    }
}

/// Persistent local Moonshine Tiny streaming recognizer.
///
/// The engine is intentionally synchronous. V1R-ASR-009 owns the dedicated
/// inference worker so native inference cannot block Tokio or the UI thread.
/// Construction verifies the ASR-006 installation before the native runtime is
/// touched. No local failure is converted into a cloud recognition request.
pub struct MoonshineTinyEngine {
    model_path: PathBuf,
    stream: Option<Box<dyn TinyStreamBackend>>,
    emitted_lines: HashMap<u64, EmittedLineState>,
    cancelled: bool,
}

impl MoonshineTinyEngine {
    pub fn open(installer: &MoonshineModelInstaller) -> Result<Self, AsrError> {
        Self::open_with_components(installer, &NativeTinyStreamFactory)
    }

    fn open_with_components(
        resolver: &dyn TinyModelResolver,
        factory: &dyn TinyStreamFactory,
    ) -> Result<Self, AsrError> {
        let model_path = resolver.resolve_verified_tiny_model()?;
        let stream = factory.open(&model_path)?;
        Ok(Self {
            model_path,
            stream: Some(stream),
            emitted_lines: HashMap::new(),
            cancelled: false,
        })
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub const fn input_sample_rate_hz(&self) -> u32 {
        MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ
    }

    pub fn is_active(&self) -> bool {
        self.stream.is_some() && !self.cancelled
    }

    /// Append one increment of 16 kHz mono `f32` PCM and return only
    /// transcript lines whose text/finality changed since the previous call.
    pub fn push_pcm(
        &mut self,
        pcm: &[f32],
    ) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError> {
        self.ensure_active()?;
        if pcm.is_empty() {
            return Ok(Vec::new());
        }

        let transcript = {
            let stream = self.active_stream()?;
            stream.add_audio(pcm, MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ)?;
            stream.transcribe(false)?
        };
        Ok(self.collect_updates(transcript))
    }

    /// Force the native streaming decoder to expose its latest transcript
    /// state without appending synthetic audio.
    pub fn flush(&mut self) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError> {
        self.ensure_active()?;
        let transcript = self.active_stream()?.transcribe(true)?;
        Ok(self.collect_updates(transcript))
    }

    /// Stop the native stream. Safe to call repeatedly.
    pub fn stop(&mut self) -> Result<(), AsrError> {
        let Some(mut stream) = self.stream.take() else {
            return Ok(());
        };
        stream.stop()
    }

    /// Cancel this engine and tear down its native stream. A cancelled engine
    /// cannot be reused; callers must construct a new engine for a new session.
    pub fn cancel(&mut self) -> Result<(), AsrError> {
        self.cancelled = true;
        self.stop()
    }

    fn ensure_active(&self) -> Result<(), AsrError> {
        if self.cancelled {
            return Err(AsrError {
                kind: AsrErrorKind::Cancelled,
                message: "Moonshine Tiny recognition was cancelled.".to_string(),
                retryable: true,
            });
        }
        if self.stream.is_none() {
            return Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: "Moonshine Tiny recognition is not active.".to_string(),
                retryable: true,
            });
        }
        Ok(())
    }

    fn active_stream(&mut self) -> Result<&mut (dyn TinyStreamBackend + '_), AsrError> {
        match self.stream.as_mut() {
            Some(stream) => Ok(stream.as_mut()),
            None => Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: "Moonshine Tiny recognition is not active.".to_string(),
                retryable: true,
            }),
        }
    }

    fn collect_updates(
        &mut self,
        transcript: MoonshineTranscript,
    ) -> Vec<MoonshineTinyTranscriptUpdate> {
        let mut updates = Vec::new();
        for line in transcript.lines {
            if line.text.is_empty() {
                continue;
            }

            let changed = self.emitted_lines.get(&line.id).is_none_or(|previous| {
                previous.text != line.text || previous.is_final != line.is_complete
            });
            if !changed {
                continue;
            }

            let update = if line.is_complete {
                MoonshineTinyTranscriptUpdate::Final {
                    line_id: line.id,
                    text: line.text.clone(),
                    latency_ms: line.last_transcription_latency_ms,
                }
            } else {
                MoonshineTinyTranscriptUpdate::Partial {
                    line_id: line.id,
                    text: line.text.clone(),
                    latency_ms: line.last_transcription_latency_ms,
                }
            };
            self.emitted_lines.insert(
                line.id,
                EmittedLineState {
                    text: line.text,
                    is_final: line.is_complete,
                },
            );
            updates.push(update);
        }
        updates
    }
}

impl Drop for MoonshineTinyEngine {
    fn drop(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::runtime::MoonshineLine;
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    fn asr_error(kind: AsrErrorKind, message: &str) -> AsrError {
        AsrError {
            kind,
            message: message.to_string(),
            retryable: true,
        }
    }

    struct FakeResolver {
        result: Result<PathBuf, AsrError>,
    }

    impl TinyModelResolver for FakeResolver {
        fn resolve_verified_tiny_model(&self) -> Result<PathBuf, AsrError> {
            self.result.clone()
        }
    }

    #[derive(Default)]
    struct BackendState {
        opened_path: Option<PathBuf>,
        add_calls: Vec<(Vec<f32>, u32)>,
        transcribe_calls: Vec<bool>,
        stop_calls: usize,
        transcripts: VecDeque<Result<MoonshineTranscript, AsrError>>,
        add_error: Option<AsrError>,
        stop_error: Option<AsrError>,
    }

    struct FakeBackend {
        state: Arc<Mutex<BackendState>>,
    }

    impl TinyStreamBackend for FakeBackend {
        fn add_audio(&mut self, pcm: &[f32], sample_rate_hz: u32) -> Result<(), AsrError> {
            let mut state = self.state.lock().unwrap();
            state.add_calls.push((pcm.to_vec(), sample_rate_hz));
            match &state.add_error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }

        fn transcribe(&mut self, force_update: bool) -> Result<MoonshineTranscript, AsrError> {
            let mut state = self.state.lock().unwrap();
            state.transcribe_calls.push(force_update);
            state
                .transcripts
                .pop_front()
                .unwrap_or_else(|| Ok(MoonshineTranscript::default()))
        }

        fn stop(&mut self) -> Result<(), AsrError> {
            let mut state = self.state.lock().unwrap();
            state.stop_calls += 1;
            match &state.stop_error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }
    }

    struct FakeFactory {
        state: Arc<Mutex<BackendState>>,
        open_error: Option<AsrError>,
    }

    impl TinyStreamFactory for FakeFactory {
        fn open(&self, model_path: &Path) -> Result<Box<dyn TinyStreamBackend>, AsrError> {
            if let Some(error) = &self.open_error {
                return Err(error.clone());
            }
            self.state.lock().unwrap().opened_path = Some(model_path.to_path_buf());
            Ok(Box::new(FakeBackend {
                state: self.state.clone(),
            }))
        }
    }

    fn fixture() -> (FakeResolver, FakeFactory, Arc<Mutex<BackendState>>) {
        let state = Arc::new(Mutex::new(BackendState::default()));
        (
            FakeResolver {
                result: Ok(PathBuf::from("/verified/tiny")),
            },
            FakeFactory {
                state: state.clone(),
                open_error: None,
            },
            state,
        )
    }

    fn line(id: u64, text: &str, is_complete: bool, latency_ms: u32) -> MoonshineLine {
        MoonshineLine {
            text: text.to_string(),
            start_time_seconds: 0.0,
            duration_seconds: 0.0,
            id,
            is_complete,
            is_updated: true,
            is_new: true,
            has_text_changed: true,
            last_transcription_latency_ms: latency_ms,
        }
    }

    fn temporary_model_root(label: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "talking-moose-asr007-{label}-{}-{nonce}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn production_open_reports_missing_verified_install_without_native_fallback() {
        let root = temporary_model_root("missing");
        let installer = MoonshineModelInstaller::new(&root).unwrap();

        let error = MoonshineTinyEngine::open(&installer)
            .err()
            .expect("missing model must fail before native startup");
        assert_eq!(error.kind, AsrErrorKind::ModelNotInstalled);
        assert!(error
            .message
            .contains("No microphone audio was sent to Google"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn production_open_reports_corrupt_install_without_native_fallback() {
        let root = temporary_model_root("corrupt");
        let installer = MoonshineModelInstaller::new(&root).unwrap();
        std::fs::create_dir_all(installer.model_path(MoonshineModelArchitecture::TinyStreaming))
            .unwrap();

        let error = MoonshineTinyEngine::open(&installer)
            .err()
            .expect("corrupt model must fail before native startup");
        assert_eq!(error.kind, AsrErrorKind::ModelCorrupt);
        assert!(error
            .message
            .contains("No microphone audio was sent to Google"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_model_fails_before_native_factory_is_called() {
        let state = Arc::new(Mutex::new(BackendState::default()));
        let resolver = FakeResolver {
            result: Err(asr_error(AsrErrorKind::ModelNotInstalled, "missing")),
        };
        let factory = FakeFactory {
            state: state.clone(),
            open_error: None,
        };

        let error = MoonshineTinyEngine::open_with_components(&resolver, &factory)
            .err()
            .expect("missing model must fail closed");
        assert_eq!(error.kind, AsrErrorKind::ModelNotInstalled);
        assert!(state.lock().unwrap().opened_path.is_none());
    }

    #[test]
    fn native_load_error_is_typed_and_not_replaced_with_fallback() {
        let (resolver, mut factory, state) = fixture();
        factory.open_error = Some(asr_error(
            AsrErrorKind::RuntimeUnavailable,
            "native missing",
        ));

        let error = MoonshineTinyEngine::open_with_components(&resolver, &factory)
            .err()
            .expect("native load error must be returned");
        assert_eq!(error.kind, AsrErrorKind::RuntimeUnavailable);
        assert!(state.lock().unwrap().opened_path.is_none());
    }

    #[test]
    fn opens_the_verified_tiny_model_path() {
        let (resolver, factory, state) = fixture();
        let engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        assert_eq!(engine.model_path(), Path::new("/verified/tiny"));
        assert_eq!(engine.input_sample_rate_hz(), 16_000);
        assert!(engine.is_active());
        assert_eq!(
            state.lock().unwrap().opened_path.as_deref(),
            Some(Path::new("/verified/tiny"))
        );
    }

    #[test]
    fn incremental_pcm_reuses_one_stream_and_the_16khz_contract() {
        let (resolver, factory, state) = fixture();
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        assert!(engine.push_pcm(&[0.1, 0.2]).unwrap().is_empty());
        assert!(engine.push_pcm(&[-0.3]).unwrap().is_empty());

        let state = state.lock().unwrap();
        assert_eq!(state.add_calls.len(), 2);
        assert_eq!(state.add_calls[0], (vec![0.1, 0.2], 16_000));
        assert_eq!(state.add_calls[1], (vec![-0.3], 16_000));
        assert_eq!(state.transcribe_calls, vec![false, false]);
    }

    #[test]
    fn partial_changes_replace_by_line_id_without_duplicate_emission() {
        let (resolver, factory, state) = fixture();
        {
            let mut state = state.lock().unwrap();
            state.transcripts.push_back(Ok(MoonshineTranscript {
                lines: vec![line(7, "hel", false, 8)],
            }));
            state.transcripts.push_back(Ok(MoonshineTranscript {
                lines: vec![line(7, "hello", false, 9)],
            }));
            state.transcripts.push_back(Ok(MoonshineTranscript {
                lines: vec![line(7, "hello", false, 10)],
            }));
        }
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        assert_eq!(
            engine.push_pcm(&[0.1]).unwrap(),
            vec![MoonshineTinyTranscriptUpdate::Partial {
                line_id: 7,
                text: "hel".to_string(),
                latency_ms: 8,
            }]
        );
        assert_eq!(
            engine.push_pcm(&[0.1]).unwrap(),
            vec![MoonshineTinyTranscriptUpdate::Partial {
                line_id: 7,
                text: "hello".to_string(),
                latency_ms: 9,
            }]
        );
        assert!(engine.push_pcm(&[0.1]).unwrap().is_empty());
    }

    #[test]
    fn complete_line_emits_final_once_until_content_changes() {
        let (resolver, factory, state) = fixture();
        {
            let mut state = state.lock().unwrap();
            state.transcripts.push_back(Ok(MoonshineTranscript {
                lines: vec![line(42, "hello", false, 11)],
            }));
            state.transcripts.push_back(Ok(MoonshineTranscript {
                lines: vec![line(42, "hello", true, 12)],
            }));
            state.transcripts.push_back(Ok(MoonshineTranscript {
                lines: vec![line(42, "hello", true, 13)],
            }));
        }
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        let _ = engine.push_pcm(&[0.1]).unwrap();
        assert_eq!(
            engine.push_pcm(&[0.1]).unwrap(),
            vec![MoonshineTinyTranscriptUpdate::Final {
                line_id: 42,
                text: "hello".to_string(),
                latency_ms: 12,
            }]
        );
        assert!(engine.push_pcm(&[0.1]).unwrap().is_empty());
    }

    #[test]
    fn flush_forces_transcription_without_appending_audio() {
        let (resolver, factory, state) = fixture();
        state
            .lock()
            .unwrap()
            .transcripts
            .push_back(Ok(MoonshineTranscript {
                lines: vec![line(5, "done", true, 14)],
            }));
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        assert_eq!(
            engine.flush().unwrap(),
            vec![MoonshineTinyTranscriptUpdate::Final {
                line_id: 5,
                text: "done".to_string(),
                latency_ms: 14,
            }]
        );
        let state = state.lock().unwrap();
        assert!(state.add_calls.is_empty());
        assert_eq!(state.transcribe_calls, vec![true]);
    }

    #[test]
    fn inference_error_is_returned_with_its_typed_category() {
        let (resolver, factory, state) = fixture();
        state
            .lock()
            .unwrap()
            .transcripts
            .push_back(Err(asr_error(AsrErrorKind::Inference, "failed")));
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        let error = engine
            .push_pcm(&[0.1])
            .expect_err("inference error must propagate");
        assert_eq!(error.kind, AsrErrorKind::Inference);
    }

    #[test]
    fn audio_input_error_is_returned_without_transcribing() {
        let (resolver, factory, state) = fixture();
        state.lock().unwrap().add_error = Some(asr_error(AsrErrorKind::AudioInput, "bad pcm"));
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        let error = engine
            .push_pcm(&[0.1])
            .expect_err("audio error must propagate");
        assert_eq!(error.kind, AsrErrorKind::AudioInput);
        assert!(state.lock().unwrap().transcribe_calls.is_empty());
    }

    #[test]
    fn cancellation_stops_once_and_prevents_reuse() {
        let (resolver, factory, state) = fixture();
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        engine.cancel().unwrap();
        engine.cancel().unwrap();
        assert!(!engine.is_active());
        assert_eq!(state.lock().unwrap().stop_calls, 1);
        let error = engine
            .push_pcm(&[0.1])
            .expect_err("cancelled engine cannot be reused");
        assert_eq!(error.kind, AsrErrorKind::Cancelled);
    }

    #[test]
    fn stop_is_idempotent_and_changes_later_calls_to_invalid_state() {
        let (resolver, factory, state) = fixture();
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        engine.stop().unwrap();
        engine.stop().unwrap();
        assert_eq!(state.lock().unwrap().stop_calls, 1);
        let error = engine
            .flush()
            .expect_err("stopped engine cannot transcribe");
        assert_eq!(error.kind, AsrErrorKind::InvalidState);
    }

    #[test]
    fn dropping_an_active_engine_stops_the_stream_once() {
        let (resolver, factory, state) = fixture();
        {
            let _engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();
        }
        assert_eq!(state.lock().unwrap().stop_calls, 1);
    }

    #[test]
    fn empty_pcm_is_a_noop_but_still_requires_an_active_engine() {
        let (resolver, factory, state) = fixture();
        let mut engine = MoonshineTinyEngine::open_with_components(&resolver, &factory).unwrap();

        assert!(engine.push_pcm(&[]).unwrap().is_empty());
        assert!(state.lock().unwrap().add_calls.is_empty());
        engine.stop().unwrap();
        assert_eq!(
            engine.push_pcm(&[]).unwrap_err().kind,
            AsrErrorKind::InvalidState
        );
    }

    #[test]
    fn engine_is_send_for_the_future_dedicated_inference_worker() {
        fn assert_send<T: Send>() {}
        assert_send::<MoonshineTinyEngine>();
    }
}
