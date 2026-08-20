use super::super::manifest::MOONSHINE_MODEL_REVISION;
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

impl ModelResolver for FakeResolver {
    fn resolve_verified_model(
        &self,
        _architecture: MoonshineModelArchitecture,
    ) -> Result<PathBuf, AsrError> {
        self.result.clone()
    }
}

#[derive(Default)]
struct BackendState {
    opened_path: Option<PathBuf>,
    opened_architecture: Option<MoonshineModelArchitecture>,
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

impl StreamBackend for FakeBackend {
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

impl StreamFactory for FakeFactory {
    fn open(
        &self,
        model_path: &Path,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Box<dyn StreamBackend>, AsrError> {
        if let Some(error) = &self.open_error {
            return Err(error.clone());
        }
        let mut state = self.state.lock().unwrap();
        state.opened_path = Some(model_path.to_path_buf());
        state.opened_architecture = Some(architecture);
        drop(state);
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
fn production_small_open_reports_missing_verified_install_without_native_fallback() {
    let root = temporary_model_root("small-missing");
    let installer = MoonshineModelInstaller::new(&root).unwrap();

    let error = MoonshineSmallEngine::open_small(&installer)
        .err()
        .expect("missing Small model must fail before native startup");
    assert_eq!(error.kind, AsrErrorKind::ModelNotInstalled);
    assert!(error.message.contains("Moonshine Small"));
    assert!(error
        .message
        .contains("No microphone audio was sent to Google"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tiny_and_small_use_independent_install_directories() {
    let root = temporary_model_root("independent-paths");
    let installer = MoonshineModelInstaller::new(&root).unwrap();
    let tiny = installer.model_path(MoonshineModelArchitecture::TinyStreaming);
    let small = installer.model_path(MoonshineModelArchitecture::SmallStreaming);

    assert_ne!(tiny, small);
    assert_eq!(tiny.file_name().unwrap(), MOONSHINE_MODEL_REVISION);
    assert_eq!(small.file_name().unwrap(), MOONSHINE_MODEL_REVISION);
    assert_eq!(
        tiny.parent().unwrap().file_name().unwrap(),
        "moonshine-tiny-streaming-en"
    );
    assert_eq!(
        small.parent().unwrap().file_name().unwrap(),
        "moonshine-small-streaming-en"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_small_open_reports_corrupt_install_without_native_fallback() {
    let root = temporary_model_root("small-corrupt");
    let installer = MoonshineModelInstaller::new(&root).unwrap();
    std::fs::create_dir_all(installer.model_path(MoonshineModelArchitecture::SmallStreaming))
        .unwrap();

    let error = MoonshineSmallEngine::open_small(&installer)
        .err()
        .expect("corrupt Small model must fail before native startup");
    assert_eq!(error.kind, AsrErrorKind::ModelCorrupt);
    assert!(error.message.contains("Moonshine Small"));
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
    let state = state.lock().unwrap();
    assert_eq!(
        state.opened_path.as_deref(),
        Some(Path::new("/verified/tiny"))
    );
    assert_eq!(
        state.opened_architecture,
        Some(MoonshineModelArchitecture::TinyStreaming)
    );
}

#[test]
fn small_uses_the_small_native_architecture_with_the_same_streaming_contract() {
    let (resolver, factory, state) = fixture();
    let engine = MoonshineStreamingEngine::open_with_architecture_components(
        MoonshineModelArchitecture::SmallStreaming,
        &resolver,
        &factory,
    )
    .unwrap();

    assert_eq!(
        engine.architecture(),
        MoonshineModelArchitecture::SmallStreaming
    );
    assert_eq!(
        engine.input_sample_rate_hz(),
        MOONSHINE_SMALL_INPUT_SAMPLE_RATE_HZ
    );
    let state = state.lock().unwrap();
    assert_eq!(
        state.opened_architecture,
        Some(MoonshineModelArchitecture::SmallStreaming)
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
