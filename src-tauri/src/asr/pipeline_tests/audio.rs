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
