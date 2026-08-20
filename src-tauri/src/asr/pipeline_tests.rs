use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

#[derive(Default)]
struct FakeState {
    pushes: AtomicUsize,
    stops: AtomicUsize,
    received_pcm: StdMutex<Vec<Vec<f32>>>,
    fail_push: StdMutex<Option<AsrError>>,
    updates: StdMutex<Vec<MoonshineTinyTranscriptUpdate>>,
    block_push: AtomicBool,
    worker_thread: StdMutex<Option<thread::ThreadId>>,
}

struct FakeEngine {
    state: Arc<FakeState>,
    sample_rate: u32,
}

impl PipelineEngine for FakeEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        self.sample_rate
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError> {
        *self.state.worker_thread.lock().unwrap() = Some(thread::current().id());
        self.state.pushes.fetch_add(1, Ordering::SeqCst);
        self.state.received_pcm.lock().unwrap().push(pcm.to_vec());
        while self.state.block_push.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
        }
        if let Some(error) = self.state.fail_push.lock().unwrap().clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut *self.state.updates.lock().unwrap()))
    }

    fn stop(&mut self) -> Result<(), AsrError> {
        self.state.stops.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn callback_events() -> (
    LocalAsrPipelineEventCallback,
    Arc<StdMutex<Vec<LocalAsrPipelineEvent>>>,
) {
    let events = Arc::new(StdMutex::new(Vec::new()));
    let callback_events = events.clone();
    let callback: LocalAsrPipelineEventCallback = Arc::new(move |event| {
        callback_events.lock().unwrap().push(event);
    });
    (callback, events)
}

async fn fake_pipeline(state: Arc<FakeState>) -> LocalAsrPipeline {
    let (callback, _) = callback_events();
    LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap()
}

fn wait_until(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for worker");
        thread::sleep(Duration::from_millis(1));
    }
}

#[tokio::test]
async fn worker_runs_off_the_async_caller_thread() {
    let caller_thread = thread::current().id();
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    assert_ne!(
        state.worker_thread.lock().unwrap().as_ref(),
        Some(&caller_thread)
    );
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn ingress_is_hard_bounded_and_drops_newest() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline = fake_pipeline(state.clone()).await;
    let sender = pipeline.test_sender();

    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    for _ in 0..LOCAL_ASR_QUEUE_CAPACITY_CHUNKS {
        sender.try_send(vec![0, 0]).unwrap();
    }
    assert_eq!(
        pipeline.diagnostics().queue_depth,
        LOCAL_ASR_QUEUE_CAPACITY_CHUNKS
    );
    assert!(matches!(
        sender.try_send(vec![0, 0]),
        Err(mpsc::error::TrySendError::Full(_))
    ));

    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(pipeline.diagnostics().queue_depth, 0);
}

#[tokio::test]
async fn converts_capture_i16_le_to_engine_f32() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    let samples = [i16::MIN, 0, i16::MAX];
    let bytes = AudioResampler::i16_to_bytes(&samples);
    pipeline.test_sender().try_send(bytes).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);

    let received = state.received_pcm.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert!((received[0][0] + 1.0).abs() < 0.0001);
    assert!(received[0][1].abs() < 0.0001);
    assert!((received[0][2] - (32767.0 / 32768.0)).abs() < 0.0001);
    drop(received);
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn malformed_pcm_is_typed_terminal_audio_error() {
    let state = Arc::new(FakeState::default());
    let (callback, events) = callback_events();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![1]).unwrap();
    wait_until(|| !pipeline.is_running());
    assert_eq!(
        pipeline.diagnostics().last_error.unwrap().kind,
        AsrErrorKind::AudioInput
    );
    assert!(matches!(
        events.lock().unwrap().as_slice(),
        [LocalAsrPipelineEvent::Error(AsrError {
            kind: AsrErrorKind::AudioInput,
            ..
        })]
    ));
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::AudioInput
    );
}

#[tokio::test]
async fn inference_error_is_preserved_and_emitted() {
    let state = Arc::new(FakeState::default());
    *state.fail_push.lock().unwrap() = Some(AsrError {
        kind: AsrErrorKind::Inference,
        message: "fake inference failure".to_string(),
        retryable: true,
    });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| !pipeline.is_running());
    assert_eq!(
        pipeline.diagnostics().last_error.unwrap().kind,
        AsrErrorKind::Inference
    );
    assert!(matches!(
        events.lock().unwrap().as_slice(),
        [LocalAsrPipelineEvent::Error(AsrError {
            kind: AsrErrorKind::Inference,
            ..
        })]
    ));
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::Inference
    );
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn transcript_updates_cross_worker_boundary() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(MoonshineTinyTranscriptUpdate::Partial {
            line_id: 7,
            text: "hello".to_string(),
            latency_ms: 9,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
    )
    .await
    .unwrap();
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| !events.lock().unwrap().is_empty());
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [LocalAsrPipelineEvent::Transcript(
            MoonshineTinyTranscriptUpdate::Partial {
                line_id: 7,
                text: "hello".to_string(),
                latency_ms: 9,
            }
        )]
    );
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn stop_is_idempotent_and_joins_worker() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.stop_and_join().await.unwrap();
    pipeline.stop_and_join().await.unwrap();
    assert!(!pipeline.is_running());
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stop_discards_queued_audio_instead_of_draining_it() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    let mut pipeline = fake_pipeline(state.clone()).await;
    let sender = pipeline.test_sender();
    sender.try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    sender.try_send(vec![0, 0]).unwrap();
    sender.try_send(vec![0, 0]).unwrap();
    pipeline.request_stop();
    state.block_push.store(false, Ordering::SeqCst);
    pipeline.stop_and_join().await.unwrap();
    assert_eq!(state.pushes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn startup_failure_never_exposes_running_pipeline() {
    let (callback, _) = callback_events();
    let result = LocalAsrPipeline::start_with_factory(
        || {
            Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: "missing".to_string(),
                retryable: true,
            })
        },
        callback,
    )
    .await;
    let error = match result {
        Ok(_) => panic!("startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, AsrErrorKind::ModelNotInstalled);
}

#[tokio::test]
async fn unexpected_engine_sample_rate_fails_before_capture() {
    let state = Arc::new(FakeState::default());
    let (callback, _) = callback_events();
    let result = LocalAsrPipeline::start_with_factory(
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: 48_000,
            }))
        },
        callback,
    )
    .await;
    let error = match result {
        Ok(_) => panic!("startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.kind, AsrErrorKind::Internal);
}

#[tokio::test]
async fn mock_capture_uses_same_authoritative_capture_at_16khz() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let mut capture = AudioCapture::new_mock();
    pipeline.start_capture(&mut capture, None, None).unwrap();
    let diagnostics = capture.diagnostics();
    assert!(diagnostics.active);
    assert_eq!(
        diagnostics.sample_rate_hz,
        Some(MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ)
    );
    assert_eq!(diagnostics.channels, Some(1));
    capture.stop();
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn processed_chunk_releases_queue_depth() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    wait_until(|| pipeline.diagnostics().queue_depth == 0);
    pipeline.stop_and_join().await.unwrap();
}

#[tokio::test]
async fn stopped_pipeline_refuses_to_start_microphone() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    pipeline.stop_and_join().await.unwrap();
    let mut capture = AudioCapture::new_mock();
    let error = pipeline
        .start_capture(&mut capture, None, None)
        .unwrap_err();
    assert_eq!(error.kind, AsrErrorKind::InvalidState);
    assert!(!capture.is_active());
}

#[tokio::test]
async fn drop_stops_and_joins_worker_as_safety_net() {
    let state = Arc::new(FakeState::default());
    {
        let pipeline = fake_pipeline(state.clone()).await;
        assert!(pipeline.is_running());
    }
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pipeline_diagnostics_report_bound_and_running_state() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state).await;
    let diagnostics = pipeline.diagnostics();
    assert_eq!(diagnostics.input_sample_rate_hz, 16_000);
    assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
    assert_eq!(diagnostics.queue_depth, 0);
    assert!(diagnostics.running);
    assert!(diagnostics.last_error.is_none());
    pipeline.stop_and_join().await.unwrap();
}
