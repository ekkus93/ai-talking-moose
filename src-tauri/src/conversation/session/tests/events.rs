use super::*;

#[tokio::test]
async fn tool_call_event_dispatches_through_router_and_preserves_call_identity() {
    let manager = ConversationManager::new();
    let generation = 60;
    let session_id = "tool-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let responses = Arc::new(SyncMutex::new(Vec::<ToolCallResponse>::new()));
    *manager.live_session.lock().await = Some(Box::new(ToolRecordingSession {
        responses: responses.clone(),
    }));

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let states = Arc::new(SyncMutex::new(Vec::new()));
    let lifecycle_callback: LifecycleCallback = Arc::new(|_| {});
    let context = test_event_loop_context(
        generation,
        session_id,
        capture,
        playback,
        states,
        lifecycle_callback,
    );
    let (event_tx, event_rx) = mpsc::channel(4);
    let manager_for_loop = manager.clone();
    let loop_task = tokio::spawn(async move {
        manager_for_loop.run_event_loop(event_rx, context).await;
    });

    event_tx
        .send(LiveServerEvent::ToolCall {
            id: "call-42".to_string(),
            name: "get_current_time".to_string(),
            args: serde_json::json!({}),
        })
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if !responses.lock().is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("tool response was not sent");

    let response = responses.lock()[0].clone();
    assert_eq!(response.id, "call-42");
    assert_eq!(response.name, "get_current_time");
    assert!(response.output.get("time").is_some());

    event_tx.send(LiveServerEvent::Closed).await.unwrap();
    drop(event_tx);
    loop_task.await.unwrap();
}
