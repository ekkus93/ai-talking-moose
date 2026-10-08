use super::*;

#[tokio::test]
async fn transcript_updates_cross_worker_boundary() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 7,
            text: "hello".to_string(),
            latency_ms: 1,
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
    wait_until(|| !events.lock().unwrap().is_empty());
    {
        let events = events.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            [
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::PartialTranscript {
                    text: "hello".to_string(),
                },
            ]
        );
    }
    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn diagnostics_measure_audio_latency_rtf_cpu_and_memory_without_fabrication() {
    let state = Arc::new(FakeState::default());
    state.block_push.store(true, Ordering::SeqCst);
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 11,
            text: "measured".to_string(),
            latency_ms: 17,
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

    let pcm = vec![0_i16; 1_600];
    pipeline
        .test_sender()
        .try_send(AudioResampler::i16_to_bytes(&pcm))
        .unwrap();
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    thread::sleep(Duration::from_millis(5));
    state.block_push.store(false, Ordering::SeqCst);
    wait_until(|| events.lock().unwrap().len() >= 2);

    let diagnostics = pipeline.diagnostics();
    assert_eq!(diagnostics.processed_audio_ms, 100);
    assert!(diagnostics.inference_wall_time_ms >= 5);
    assert!(diagnostics.real_time_factor.is_some_and(|rtf| rtf > 0.0));
    assert!(diagnostics.first_partial_latency_ms.is_some());
    assert_eq!(diagnostics.last_transcription_latency_ms, Some(17));
    assert!(diagnostics.process_cpu_time_ms.is_some());
    assert!(diagnostics.average_cpu_utilization_percent.is_some());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        assert!(diagnostics.baseline_resident_memory_bytes.is_some());
        assert!(diagnostics.resident_memory_bytes.is_some());
        assert!(diagnostics.peak_resident_memory_bytes.is_some());
    }

    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn blank_native_update_does_not_count_as_first_useful_transcript() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Partial {
            segment_id: 12,
            text: "   ".to_string(),
            latency_ms: 13,
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
    wait_until(|| state.pushes.load(Ordering::SeqCst) == 1);
    wait_until(|| pipeline.diagnostics().last_transcription_latency_ms == Some(13));

    let diagnostics = pipeline.diagnostics();
    assert!(events.lock().unwrap().is_empty());
    assert_eq!(diagnostics.first_partial_latency_ms, None);
    assert_eq!(diagnostics.first_final_latency_ms, None);
    assert_eq!(diagnostics.last_transcription_latency_ms, Some(13));

    pipeline.stop_and_join().await.unwrap();
}
#[tokio::test]
async fn final_transcript_crosses_worker_as_provider_neutral_lifecycle() {
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 8,
            text: "complete".to_string(),
            latency_ms: 1,
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
    wait_until(|| events.lock().unwrap().len() == 3);
    {
        let events = events.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            [
                AsrEvent::SpeechStarted { monotonic_ms: None },
                AsrEvent::FinalTranscript {
                    text: "complete".to_string(),
                },
                AsrEvent::SpeechEnded { monotonic_ms: None },
            ]
        );
    }
    pipeline.stop_and_join().await.unwrap();
}
