use super::*;

#[tokio::test]
async fn stale_generation_cannot_tear_down_newer_session() {
    let manager = ConversationManager::new();
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    manager.generation.store(8, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("new-session".to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let cleaned = manager
        .shutdown_if_generation_current(7, capture, playback, ConversationLifecycle::Idle)
        .await;

    assert!(!cleaned);
    assert!(manager.is_active());
    assert_eq!(manager.current_session_id().as_deref(), Some("new-session"));
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Listening);
}

#[tokio::test]
async fn barge_in_suppresses_stale_output_until_next_user_turn_boundary() {
    let manager = ConversationManager::new();
    let playback = Arc::new(AudioPlayback::new());
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count,
        interrupt_count: interrupt_count.clone(),
    }));
    manager.generation.store(21, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;
    let local_asr_stop_count = attach_counting_local_asr(&manager, 21).await;

    manager.barge_in(playback).await.unwrap();

    assert!(manager.output_suppressed.load(Ordering::SeqCst));
    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 0);
    assert!(manager.local_asr_callback_is_current(21).await);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Responding);
}

#[test]
fn interrupted_response_suppression_rejects_stale_non_audio_callbacks() {
    let manager = ConversationManager::new();
    manager.output_suppressed.store(true, Ordering::SeqCst);

    assert!(
        manager.should_suppress_interrupted_response_event(&LiveServerEvent::ModelTranscript(
            TranscriptUpdate {
                text: "stale transcript".to_string(),
                is_final: false,
            }
        ))
    );
    assert!(manager
        .should_suppress_interrupted_response_event(&LiveServerEvent::AudioData(vec![1, 2, 3])));
    assert!(manager.should_suppress_interrupted_response_event(&LiveServerEvent::TurnComplete));
    assert!(
        manager.should_suppress_interrupted_response_event(&LiveServerEvent::ToolCall {
            id: "stale-tool".to_string(),
            name: "remember".to_string(),
            args: serde_json::json!({"fact": "stale"}),
        })
    );

    assert!(
        !manager.should_suppress_interrupted_response_event(&LiveServerEvent::UserTranscript(
            TranscriptUpdate {
                text: "new user turn".to_string(),
                is_final: false,
            }
        ))
    );
    assert!(!manager.should_suppress_interrupted_response_event(&LiveServerEvent::Interrupted));
    assert!(!manager.should_suppress_interrupted_response_event(&LiveServerEvent::Closed));
}

#[tokio::test]
async fn barge_in_waits_for_the_serialized_operation_boundary() {
    let manager = ConversationManager::new();
    let playback = Arc::new(AudioPlayback::new());
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count,
        interrupt_count: interrupt_count.clone(),
    }));
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;

    let operation_guard = manager.operation_lock.lock().await;
    let manager_for_barge = manager.clone();
    let barge_task = tokio::spawn(async move { manager_for_barge.barge_in(playback).await });

    tokio::task::yield_now().await;
    assert!(!barge_task.is_finished());
    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 0);

    drop(operation_guard);
    barge_task.await.unwrap().unwrap();
    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
}

#[tokio::test]
async fn stale_generation_tool_response_cannot_enter_replacement_session() {
    let manager = ConversationManager::new();
    let responses = Arc::new(SyncMutex::new(Vec::<ToolCallResponse>::new()));
    *manager.live_session.lock().await = Some(Box::new(ToolRecordingSession {
        responses: responses.clone(),
    }));
    manager.generation.store(71, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("replacement-session".to_string());

    let accepted = manager
        .send_tool_response_if_current(
            70,
            "stale-session",
            ToolCallResponse {
                id: "stale-call".to_string(),
                name: "get_current_time".to_string(),
                output: serde_json::json!({"time": "stale"}),
            },
        )
        .await
        .unwrap();

    assert!(!accepted);
    assert!(responses.lock().is_empty());
}

#[tokio::test]
async fn stale_generation_microphone_chunk_cannot_enter_replacement_session() {
    let manager = ConversationManager::new();
    let chunks = Arc::new(SyncMutex::new(Vec::<Vec<u8>>::new()));
    *manager.live_session.lock().await = Some(Box::new(AudioRecordingSession {
        chunks: chunks.clone(),
    }));
    manager.generation.store(81, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_asr_mode.lock() = Some(AsrMode::GeminiLiveAudio);

    let accepted = manager
        .forward_microphone_chunk(80, AsrMode::GeminiLiveAudio, &[1, 2, 3, 4])
        .await
        .unwrap();

    assert!(!accepted);
    assert!(chunks.lock().is_empty());
}

#[tokio::test]
async fn stalled_provider_send_cannot_block_stop_indefinitely() {
    let manager = ConversationManager::new();
    *manager.live_session.lock().await = Some(Box::new(StallingSession));
    manager.generation.store(91, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_asr_mode.lock() = Some(AsrMode::GeminiLiveAudio);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let manager_for_send = manager.clone();
    let send_task = tokio::spawn(async move {
        manager_for_send
            .forward_microphone_chunk(91, AsrMode::GeminiLiveAudio, &[7, 8])
            .await
    });
    tokio::task::yield_now().await;

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        manager.stop_session(capture, playback),
    )
    .await
    .expect("bounded provider I/O must allow Stop to complete");

    let send_error = send_task
        .await
        .unwrap()
        .expect_err("stalled send must reach the provider timeout");
    assert_eq!(send_error.kind, ProviderErrorKind::Network);
    assert!(!manager.is_active());
}

#[tokio::test]
async fn stalled_provider_interrupt_is_bounded() {
    let manager = ConversationManager::new();
    *manager.live_session.lock().await = Some(Box::new(StallingSession));
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;

    let error = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        manager.barge_in(Arc::new(AudioPlayback::new())),
    )
    .await
    .expect("bounded provider I/O must allow barge-in to return")
    .expect_err("stalled interrupt must report a timeout");

    assert!(error.contains("operation timeout"));
}

#[tokio::test]
async fn stalled_provider_connect_does_not_hold_operation_lock_against_stop() {
    let manager = ConversationManager::new();
    let entered = Arc::new(AtomicBool::new(false));
    let mut request = test_request(false);
    request.provider = Arc::new(StallingProvider {
        entered: entered.clone(),
    });
    let capture = request.capture.clone();
    let playback = request.playback.clone();

    let manager_for_start = manager.clone();
    let start_task = tokio::spawn(async move { manager_for_start.start_session(request).await });
    tokio::time::timeout(std::time::Duration::from_millis(250), async {
        while !entered.load(AtomicOrdering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("provider connect was not entered");

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        manager.stop_session(capture, playback),
    )
    .await
    .expect("Stop must not wait for a stalled provider connect");

    let start_error = tokio::time::timeout(std::time::Duration::from_millis(500), start_task)
        .await
        .expect("bounded connect must finish")
        .unwrap()
        .expect_err("the invalidated start must not activate");
    assert_eq!(start_error, "Conversation start was cancelled");
}
