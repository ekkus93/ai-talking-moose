use super::*;

#[tokio::test]
async fn moonshine_mode_never_calls_provider_audio_upload_api() {
    let manager = ConversationManager::new();
    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count: audio_upload_count.clone(),
        text_turns,
    }));

    manager
        .forward_microphone_chunk(0, AsrMode::MoonshineTinyStreaming, &[1, 2, 3, 4])
        .await
        .unwrap();
    manager
        .forward_microphone_chunk(0, AsrMode::MoonshineSmallStreaming, &[5, 6, 7, 8])
        .await
        .unwrap();

    assert_eq!(audio_upload_count.load(AtomicOrdering::SeqCst), 0);

    manager.generation.store(1, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_asr_mode.lock() = Some(AsrMode::GeminiLiveAudio);
    manager
        .forward_microphone_chunk(1, AsrMode::GeminiLiveAudio, &[9, 10])
        .await
        .unwrap();
    assert_eq!(audio_upload_count.load(AtomicOrdering::SeqCst), 1);
}

#[tokio::test]
async fn local_final_transcript_is_rejected_in_gemini_cloud_audio_mode() {
    let manager = ConversationManager::new();
    let generation = 58;
    let session_id = "cloud-audio-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::GeminiLiveAudio);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, generation).await;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count,
        text_turns: text_turns.clone(),
    }));

    let routed = manager
        .handle_local_asr_event(
            generation,
            session_id,
            AsrEvent::FinalTranscript {
                text: "must stay on the cloud-ASR path".to_string(),
            },
        )
        .await
        .unwrap();

    assert!(!routed);
    assert!(text_turns.lock().is_empty());
}
