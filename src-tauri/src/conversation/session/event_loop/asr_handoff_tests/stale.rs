use super::*;

#[tokio::test]
async fn stale_local_final_transcript_cannot_reach_newer_session() {
    let manager = ConversationManager::new();
    manager.generation.store(62, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("new-session".to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, 62).await;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count,
        text_turns: text_turns.clone(),
    }));
    let transcripts = Arc::new(SyncMutex::new(Vec::<(String, String, String)>::new()));
    let transcripts_for_callback = transcripts.clone();
    *manager.transcript_callback.lock() = Some(Arc::new(move |session, role, text| {
        transcripts_for_callback.lock().push((session, role, text));
    }));

    let routed = manager
        .handle_local_asr_event(
            61,
            "old-session",
            AsrEvent::FinalTranscript {
                text: "stale words".to_string(),
            },
        )
        .await
        .unwrap();

    assert!(!routed);
    assert!(text_turns.lock().is_empty());
    assert!(transcripts.lock().is_empty());
}
