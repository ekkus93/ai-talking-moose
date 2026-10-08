use super::*;

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
async fn stop_drains_accepted_queued_audio_before_finalization() {
    let state = Arc::new(FakeState::default());
    let gate = Arc::new(PushGate::default());
    *state.push_gate.lock().unwrap() = Some(gate.clone());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 42,
            text: "drained utterance".to_string(),
            latency_ms: 3,
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
    let sender = pipeline.test_sender();
    // AudioCapture emits 100 ms mono PCM at 16 kHz: 1,600 signed 16-bit
    // samples, or 3,200 bytes per accepted chunk. Distinct values prove FIFO.
    let chunks: Vec<Vec<u8>> = [1_i16, 2, 3]
        .into_iter()
        .map(|sample| sample.to_le_bytes().repeat(1_600))
        .collect();

    // The worker deterministically blocks inside the first accepted push. The
    // next two chunks therefore remain queued when normal stop is requested.
    sender.try_send(chunks[0].clone()).unwrap();
    gate.wait_until_entered();
    sender.try_send(chunks[1].clone()).unwrap();
    sender.try_send(chunks[2].clone()).unwrap();

    pipeline.request_stop();
    *state.push_gate.lock().unwrap() = None;
    gate.release();
    pipeline.stop_and_join().await.unwrap();

    assert_eq!(state.pushes.load(Ordering::SeqCst), 3);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert!(
        !pipeline.is_running(),
        "worker must be retired after finalization"
    );
    let received = state.received_pcm.lock().unwrap();
    assert_eq!(received.len(), 3);
    for (actual, chunk) in received.iter().zip(chunks.iter()) {
        assert_eq!(actual.len(), 1_600);
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32_768.0;
        assert!(actual
            .iter()
            .all(|value| (*value - sample).abs() < f32::EPSILON));
    }
    drop(received);
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::FinalTranscript {
                text: "drained utterance".to_string(),
            },
            AsrEvent::SpeechEnded { monotonic_ms: None },
        ]
    );
}
#[tokio::test]
async fn sub_threshold_stop_final_crosses_normal_path_exactly_once() {
    let state = Arc::new(FakeState::default());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 77,
            text: "tail words".to_string(),
            latency_ms: 4,
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

    // One two-byte PCM sample is far below Whisper's partial cadence. It is
    // accepted before stop and the final produced by engine.stop still crosses
    // the ordinary transcript state machine exactly once.
    pipeline.test_sender().try_send(vec![0, 0]).unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    pipeline.stop_and_join().await.unwrap();
    pipeline.stop_and_join().await.unwrap();

    assert_eq!(state.pushes.load(Ordering::SeqCst), 1);
    assert_eq!(state.stops.load(Ordering::SeqCst), 1);
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::FinalTranscript {
                text: "tail words".to_string(),
            },
            AsrEvent::SpeechEnded { monotonic_ms: None },
        ]
    );
}
#[tokio::test]
async fn graceful_stop_waits_for_scheduled_final_delivery_acknowledgement() {
    let state = Arc::new(FakeState::default());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 79,
            text: "delayed final".to_string(),
            latency_ms: 3,
        });
    let (ack_tx, mut ack_rx) = tokio::sync::mpsc::unbounded_channel();
    let callback: LocalAsrPipelineEventCallback = Arc::new(move |event, ack| {
        if matches!(event, AsrEvent::FinalTranscript { .. }) {
            ack_tx.send(ack).unwrap();
        }
    });
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
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    let stop = tokio::spawn(async move { pipeline.stop_and_join().await });
    let final_ack = tokio::time::timeout(Duration::from_secs(2), ack_rx.recv())
        .await
        .expect("worker should schedule its stop-time final")
        .expect("worker event channel should remain open");
    assert!(
        !stop.is_finished(),
        "graceful stop must wait while the conversation handoff is pending"
    );

    drop(final_ack);
    stop.await.unwrap().unwrap();
}
#[tokio::test]
async fn whitespace_stop_final_closes_speech_without_user_transcript() {
    let state = Arc::new(FakeState::default());
    state
        .stop_updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 78,
            text: "   ".to_string(),
            latency_ms: 2,
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

    pipeline.stop_and_join().await.unwrap();

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        [
            AsrEvent::SpeechStarted { monotonic_ms: None },
            AsrEvent::SpeechEnded { monotonic_ms: None },
        ]
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, AsrEvent::FinalTranscript { .. })));
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
