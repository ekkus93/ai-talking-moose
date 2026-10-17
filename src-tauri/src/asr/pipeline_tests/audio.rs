use super::*;

#[tokio::test]
async fn converts_capture_i16_le_to_engine_f32() {
    let state = Arc::new(FakeState::default());
    let mut pipeline = fake_pipeline(state.clone()).await;
    let samples = [i16::MIN, 0, i16::MAX];
    let bytes = AudioResampler::i16_to_bytes(&samples);
    pipeline.test_sender().try_send(bytes).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);

    {
        let received = state.received_pcm.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert!((received[0][0] + 1.0).abs() < 0.0001);
        assert!(received[0][1].abs() < 0.0001);
        assert!((received[0][2] - (32767.0 / 32768.0)).abs() < 0.0001);
    }
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
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
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
        [AsrEvent::Error {
            error: AsrError {
                kind: AsrErrorKind::AudioInput,
                ..
            }
        }]
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
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
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
        [AsrEvent::Error {
            error: AsrError {
                kind: AsrErrorKind::Inference,
                ..
            }
        }]
    ));
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::Inference
    );
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn whisper_malformed_pcm_fails_closed_and_does_not_invoke_inference() {
    let state = Arc::new(FakeState::default());
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_architecture(
        LocalAsrArchitecture::WhisperSmall,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }) as Box<dyn PipelineEngine>)
        },
        callback,
        WORKER_STARTUP_TIMEOUT,
        None,
    )
    .await
    .unwrap();

    pipeline.test_sender().try_send(vec![1, 2, 3]).unwrap();
    wait_until(|| !pipeline.is_running());
    assert_eq!(state.pushes.load(Ordering::SeqCst), 0);
    assert_eq!(
        pipeline.diagnostics().last_error.unwrap().kind,
        AsrErrorKind::AudioInput
    );
    assert!(matches!(
        events.lock().unwrap().as_slice(),
        [AsrEvent::Error {
            error: AsrError {
                kind: AsrErrorKind::AudioInput,
                ..
            }
        }]
    ));
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::AudioInput
    );
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn whisper_inference_failure_after_partial_never_emits_a_stale_final() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 23,
            text: "incomplete".to_string(),
            latency_ms: 1,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();
    let mut pipeline = LocalAsrPipeline::start_architecture(
        LocalAsrArchitecture::WhisperSmall,
        WHISPER_LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        move || {
            Ok(Box::new(FakeEngine {
                state: worker_state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }) as Box<dyn PipelineEngine>)
        },
        callback,
        WORKER_STARTUP_TIMEOUT,
        None,
    )
    .await
    .unwrap();

    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| events.lock().unwrap().len() >= 2);
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::PartialTranscript {
                text: "incomplete".to_string(),
            },
        ]
    );

    *state.fail_push.lock().unwrap() = Some(AsrError {
        kind: AsrErrorKind::Inference,
        message: "simulated Whisper inference failure".to_string(),
        retryable: false,
    });
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| !pipeline.is_running());

    assert_eq!(
        pipeline.diagnostics().last_error.unwrap().kind,
        AsrErrorKind::Inference
    );
    assert_eq!(
        pipeline.stop_and_join().await.unwrap_err().kind,
        AsrErrorKind::Inference
    );
    assert_eq!(state.pushes.load(Ordering::SeqCst), 2);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    let recorded = events.lock().unwrap();
    assert_eq!(
        recorded
            .iter()
            .filter(|event| matches!(event, AsrEvent::Error { .. }))
            .count(),
        1
    );
    assert!(
        !recorded
            .iter()
            .any(|event| matches!(event, AsrEvent::FinalTranscript { .. })),
        "failed Whisper inference must not promote a partial transcript to a final"
    );
}
