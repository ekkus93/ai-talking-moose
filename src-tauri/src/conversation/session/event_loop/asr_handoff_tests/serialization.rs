use super::*;

#[tokio::test]
async fn local_final_does_not_wait_for_global_operation_lock() {
    let manager = ConversationManager::new();
    let generation = 55;
    let session_id = "serialized-moonshine-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, generation).await;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count,
        text_turns: text_turns.clone(),
    }));
    *manager.state_callback.lock() = Some(Arc::new(|_| {}));
    *manager.lifecycle_callback.lock() = Some(Arc::new(|_| {}));
    *manager.transcript_callback.lock() = Some(Arc::new(|_, _, _| {}));

    let operation_guard = manager.operation_lock.lock().await;
    let handled = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        manager.handle_local_asr_event(
            generation,
            session_id,
            AsrEvent::FinalTranscript {
                text: "serialized final".to_string(),
            },
        ),
    )
    .await
    .expect("local final must not wait for the global conversation operation lock")
    .unwrap();

    assert!(handled);
    assert_eq!(
        text_turns.lock().as_slice(),
        &["serialized final".to_string()]
    );
    drop(operation_guard);
}

#[tokio::test]
async fn stop_time_final_is_committed_once_before_session_teardown() {
    let manager = Arc::new(ConversationManager::new());
    let generation = 56;
    let session_id = "stop-time-whisper-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::WhisperSmall);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count,
        text_turns: text_turns.clone(),
    }));
    *manager.state_callback.lock() = Some(Arc::new(|_| {}));
    *manager.lifecycle_callback.lock() = Some(Arc::new(|_| {}));
    *manager.transcript_callback.lock() = Some(Arc::new(|_, _, _| {}));
    manager
        .local_asr_lifecycle()
        .attach(
            generation,
            Box::new(FinalOnStopLocalAsrResource {
                manager: manager.clone(),
                generation,
                session_id: session_id.to_string(),
            }),
        )
        .await
        .unwrap();

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    manager
        .stop_session(capture.clone(), playback.clone())
        .await;
    manager.stop_session(capture, playback).await;

    assert_eq!(text_turns.lock().as_slice(), &["stop-time final"]);
    assert!(manager.generation.load(Ordering::SeqCst) > generation);
    assert!(!manager.is_active());
}
