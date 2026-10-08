use super::*;

#[test]
fn sensitive_asr_payloads_are_processed_without_entering_tracing() {
    const TRANSCRIPT: &str = "PRIVATE_ASR_TRANSCRIPT_7f0f9c";
    const RAW_PCM: &[u8] = b"RAW_AUDIO_PCM_181!";
    let pcm_bytes = RAW_PCM.to_vec();
    let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
    let state = Arc::new(FakeState::default());
    state
        .updates
        .lock()
        .unwrap()
        .push(StreamingTranscriptUpdate::Final {
            segment_id: 181,
            text: TRANSCRIPT.to_string(),
            latency_ms: 1,
        });
    let (callback, events) = callback_events();
    let worker_state = state.clone();

    let (_, logs) = capture_logs(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
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
            pipeline.test_sender().try_send(pcm_bytes).unwrap();
            wait_until(|| events.lock().unwrap().len() == 3);

            assert!(matches!(
                events.lock().unwrap().as_slice(),
                [
                    AsrEvent::SpeechStarted { .. },
                    AsrEvent::FinalTranscript { text },
                    AsrEvent::SpeechEnded { .. },
                ] if text.as_str() == TRANSCRIPT
            ));
            {
                let received = state.received_pcm.lock().unwrap();
                assert_eq!(
                    received.len(),
                    1,
                    "raw PCM must cross the production worker boundary"
                );
                assert!(!received[0].is_empty());
            }
            pipeline.stop_and_join().await.unwrap();
        });
    });

    assert_log_capture_live(&logs);
    assert!(!logs.contains(TRANSCRIPT));
    assert!(!logs.contains(std::str::from_utf8(RAW_PCM).unwrap()));
    assert!(!logs.contains(&pcm_base64));
}
