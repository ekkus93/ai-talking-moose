use super::*;

#[tokio::test]
async fn starting_a_new_provider_tears_down_previous_local_asr_before_connect() {
    let manager = ConversationManager::new();
    manager.generation.store(30, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("old-session".to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let local_asr_stop_count = attach_counting_local_asr(&manager, 30).await;

    let result = manager.start_session(test_request(false)).await;

    assert!(result.is_err());
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn barge_in_keeps_current_local_asr_active() {
    let manager = ConversationManager::new();
    let playback = Arc::new(AudioPlayback::new());
    let generation = 35;
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("local-barge-session".to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;

    let local_asr_stop_count = attach_counting_local_asr(&manager, generation).await;
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count,
        interrupt_count: interrupt_count.clone(),
    }));

    manager.barge_in(playback).await.unwrap();

    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 0);
    assert!(manager.local_asr_lifecycle().is_active().await);
    assert!(manager.local_asr_callback_is_current(generation).await);
}

#[tokio::test]
async fn stale_local_asr_callback_cannot_mutate_newer_generation() {
    let manager = ConversationManager::new();
    manager.generation.store(40, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, 40).await;

    assert!(manager.local_asr_callback_is_current(40).await);
    manager.generation.store(41, Ordering::SeqCst);
    assert!(!manager.local_asr_callback_is_current(40).await);
    assert!(!manager.local_asr_callback_is_current(41).await);
}

#[tokio::test]
async fn cancelled_local_asr_preparation_failure_cannot_emit_stale_error_state() {
    let manager = ConversationManager::new();
    let gate = Arc::new(LocalAsrPreparationTestGate::failing());
    manager.set_local_asr_preparation_test_gate(Some(gate.clone()));

    let state_events = Arc::new(SyncMutex::new(Vec::new()));
    let lifecycle_events = Arc::new(SyncMutex::new(Vec::new()));
    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineTinyStreaming;
    let capture = request.capture.clone();
    let playback = request.playback.clone();
    let state_events_for_callback = state_events.clone();
    let lifecycle_events_for_callback = lifecycle_events.clone();
    request.callbacks = ConversationCallbacks::new(
        move |state| state_events_for_callback.lock().push(state),
        move |lifecycle| lifecycle_events_for_callback.lock().push(lifecycle),
        |_, _, _| {},
        |_| {},
        |_| {},
        |_| {},
    );

    let manager_for_start = manager.clone();
    let start_task = tokio::spawn(async move { manager_for_start.start_session(request).await });

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        gate.entered.notified(),
    )
    .await
    .expect("local ASR preparation barrier was not entered");

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        manager.stop_session(capture, playback),
    )
    .await
    .expect("Stop must invalidate blocked local ASR preparation promptly");

    gate.release.notify_one();
    let error = tokio::time::timeout(std::time::Duration::from_millis(500), start_task)
        .await
        .expect("released stale preparation must finish")
        .unwrap()
        .expect_err("stale preparation failure must resolve as cancellation");

    assert_eq!(error, "Conversation start was cancelled");
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    assert!(!manager.is_active());
    assert!(
        !state_events.lock().contains(&CharacterState::Error),
        "cancelled preparation must not emit a stale Error state"
    );
    assert!(
        !lifecycle_events
            .lock()
            .contains(&ConversationLifecycle::Failed),
        "cancelled preparation must not emit a stale Failed lifecycle"
    );
}

#[tokio::test]
async fn blocked_local_asr_preparation_does_not_hold_operation_lock_against_stop() {
    let manager = ConversationManager::new();
    let gate = Arc::new(LocalAsrPreparationTestGate::default());
    manager.set_local_asr_preparation_test_gate(Some(gate.clone()));

    let connect_count = Arc::new(AtomicUsize::new(0));
    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineTinyStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    let capture = request.capture.clone();
    let playback = request.playback.clone();

    let manager_for_start = manager.clone();
    let start_task = tokio::spawn(async move { manager_for_start.start_session(request).await });

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        gate.entered.notified(),
    )
    .await
    .expect("local ASR preparation barrier was not entered");

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        manager.stop_session(capture.clone(), playback),
    )
    .await
    .expect("Stop must invalidate pending local ASR preparation without waiting for startup");

    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);

    gate.release.notify_one();
    let start_error = tokio::time::timeout(std::time::Duration::from_millis(500), start_task)
        .await
        .expect("released local ASR preparation must finish")
        .unwrap()
        .expect_err("stale local ASR preparation must not activate");
    assert_eq!(start_error, "Conversation start was cancelled");
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!manager.local_asr_lifecycle().is_active().await);
}

#[tokio::test]
async fn cancelled_wake_local_asr_start_cannot_mutate_newer_committed_session() {
    let manager = ConversationManager::new();
    let gate = Arc::new(LocalAsrPreparationTestGate::default());
    manager.set_local_asr_preparation_test_gate(Some(gate.clone()));

    let mut stale_request = test_request(false);
    stale_request.asr_mode = AsrMode::MoonshineTinyStreaming;
    let stale_capture = stale_request.capture.clone();
    let stale_playback = stale_request.playback.clone();
    let stale_handoff = WakeCommandHandoffAudio::new(16_000, vec![101, 202, 303, 404]).unwrap();

    let manager_for_stale = manager.clone();
    let stale_task = tokio::spawn(async move {
        manager_for_stale
            .start_session_with_wake_handoff(stale_request, Some(stale_handoff))
            .await
    });

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        gate.entered.notified(),
    )
    .await
    .expect("stale local ASR preparation barrier was not entered");

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        manager.stop_session(stale_capture, stale_playback),
    )
    .await
    .expect("Stop must invalidate the blocked Wake start promptly");

    // A fresh non-Wake conversation must be able to commit while the old local-ASR preparation
    // remains blocked. The old Wake handoff belongs only to its stale generation.
    manager.set_local_asr_preparation_test_gate(None);
    let connect_count = Arc::new(AtomicUsize::new(0));
    let mut fresh_request = test_request(false);
    fresh_request.provider = Arc::new(ReadyProvider {
        connect_count: connect_count.clone(),
    });
    fresh_request.playback = Arc::new(AudioPlayback::new_mock());
    let fresh_capture = fresh_request.capture.clone();
    let fresh_playback = fresh_request.playback.clone();

    let fresh_session_id = manager
        .start_session(fresh_request)
        .await
        .expect("fresh generation must commit independently of the stale preparation");
    assert!(manager.is_active());
    assert_eq!(
        manager.current_session_id().as_deref(),
        Some(fresh_session_id.as_str())
    );
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Listening);
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 1);

    gate.release.notify_one();
    let stale_error = tokio::time::timeout(std::time::Duration::from_millis(500), stale_task)
        .await
        .expect("released stale preparation must finish")
        .unwrap()
        .expect_err("stale Wake start must not commit");
    assert_eq!(stale_error, "Conversation start was cancelled");

    assert!(manager.is_active());
    assert_eq!(
        manager.current_session_id().as_deref(),
        Some(fresh_session_id.as_str()),
        "stale completion must not replace or fail the newer committed session"
    );
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Listening);
    assert!(fresh_capture.lock().is_active());

    manager.stop_session(fresh_capture, fresh_playback).await;
}

#[tokio::test]
async fn whisper_preparation_does_not_require_moonshine_installer() {
    let manager = ConversationManager::new();
    let temp = tempfile::TempDir::new().unwrap();
    let whisper_installer = Arc::new(
        crate::asr::whisper::WhisperModelInstaller::new(
            temp.path()
                .join("models")
                .join("whisper")
                .join("whisper-small"),
        )
        .unwrap(),
    );
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new_mock());

    let error = match manager
        .prepare_local_asr(LocalAsrPreparation {
            generation: 73,
            asr_mode: AsrMode::WhisperSmall,
            installer: None,
            whisper_installer: Some(whisper_installer),
            session_id: "whisper-without-moonshine".to_string(),
            capture,
            playback,
            state_callback: Arc::new(|_| {}),
            provider_error_callback: Arc::new(|_| {}),
        })
        .await
    {
        Ok(_) => panic!("empty Whisper profile should fail closed"),
        Err(error) => error,
    };

    assert!(error.contains("Whisper") || error.contains("whisper"));
    assert!(
        !error.contains("Moonshine"),
        "Whisper startup must not depend on Moonshine-only installer state: {error}"
    );

    let (diagnostics, _) = manager
        .last_local_asr_diagnostics(AsrMode::WhisperSmall)
        .expect("Whisper failure should be retained in local diagnostics");
    let kind = diagnostics
        .last_error
        .expect("Whisper diagnostics should include the startup failure")
        .kind;
    assert!(matches!(
        kind,
        AsrErrorKind::ModelNotInstalled | AsrErrorKind::RuntimeUnavailable
    ));
}
