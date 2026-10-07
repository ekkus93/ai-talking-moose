use super::*;

#[test]
fn lifecycle_transition_table_rejects_invalid_jumps() {
    assert!(ConversationLifecycle::Idle.can_transition_to(ConversationLifecycle::Connecting));
    assert!(ConversationLifecycle::Connecting.can_transition_to(ConversationLifecycle::Failed));
    assert!(ConversationLifecycle::Responding.can_transition_to(ConversationLifecycle::Listening));
    assert!(!ConversationLifecycle::Idle.can_transition_to(ConversationLifecycle::Responding));
    assert!(!ConversationLifecycle::Failed.can_transition_to(ConversationLifecycle::Responding));
}

#[tokio::test]
async fn failed_provider_connect_never_becomes_active() {
    let manager = ConversationManager::new();
    let result = manager.start_session(test_request(false)).await;

    assert!(result.is_err());
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn muted_state_blocks_transactional_start_inside_operation_lock() {
    let manager = ConversationManager::new();
    let result = manager.start_session(test_request(true)).await;

    assert_eq!(result.unwrap_err(), "Moose is currently muted");
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
}

#[tokio::test]
async fn missing_tiny_model_fails_before_provider_connect_or_microphone_capture() {
    let manager = ConversationManager::new();
    let connect_count = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineTinyStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    request.moonshine_installer =
        Some(Arc::new(MoonshineModelInstaller::new(temp.path()).unwrap()));
    let capture = request.capture.clone();

    let error = manager
        .start_session(request)
        .await
        .expect_err("missing Tiny model must fail before provider or microphone startup");

    assert!(error.contains("not installed"));
    assert!(error.contains("No microphone audio was sent"));
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!capture.lock().is_active());
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn missing_small_model_fails_before_provider_connect_or_microphone_capture() {
    let manager = ConversationManager::new();
    let connect_count = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineSmallStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    request.moonshine_installer =
        Some(Arc::new(MoonshineModelInstaller::new(temp.path()).unwrap()));
    let capture = request.capture.clone();

    let error = manager
        .start_session(request)
        .await
        .expect_err("missing Small model must fail before provider or microphone startup");

    assert!(error.contains("Moonshine Small"));
    assert!(error.contains("not installed"));
    assert!(error.contains("No microphone audio was sent"));
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!capture.lock().is_active());
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn corrupt_tiny_model_fails_before_provider_connect_or_microphone_capture() {
    let manager = ConversationManager::new();
    let connect_count = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let installer = Arc::new(MoonshineModelInstaller::new(temp.path()).unwrap());
    let corrupt_model =
        installer.model_path(crate::asr::moonshine::MoonshineModelArchitecture::TinyStreaming);
    std::fs::create_dir_all(&corrupt_model).unwrap();
    std::fs::write(
        corrupt_model.join("corrupt-placeholder"),
        b"not a verified model",
    )
    .unwrap();

    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineTinyStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    request.moonshine_installer = Some(installer);
    let capture = request.capture.clone();

    let error = manager
        .start_session(request)
        .await
        .expect_err("corrupt Tiny model must fail before provider or microphone startup");

    assert!(error.contains("corrupt") || error.contains("incomplete"));
    assert!(error.contains("No microphone audio was sent"));
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!capture.lock().is_active());
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn centralized_shutdown_is_idempotent_and_closes_session_once() {
    let manager = ConversationManager::new();
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let (pcm_tx, _pcm_rx) = mpsc::channel(1);
    capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count: close_count.clone(),
        interrupt_count,
    }));
    *manager.active_session_id.lock() = Some("test-session".to_string());
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let local_asr_stop_count = attach_counting_local_asr(&manager, 0).await;

    manager
        .stop_session(capture.clone(), playback.clone())
        .await;
    manager.stop_session(capture.clone(), playback).await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
}

#[tokio::test]
async fn application_shutdown_is_idempotent_and_closes_backend_resources() {
    let manager = ConversationManager::new();
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let (pcm_tx, _pcm_rx) = mpsc::channel(1);
    capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count: close_count.clone(),
        interrupt_count,
    }));
    *manager.active_session_id.lock() = Some("shutdown-session".to_string());
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let local_asr_stop_count = attach_counting_local_asr(&manager, 0).await;

    manager
        .shutdown_application(capture.clone(), playback.clone())
        .await;
    manager
        .shutdown_application(capture.clone(), playback.clone())
        .await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    assert_eq!(playback.diagnostics().sample_rate_hz, None);
    assert!(!playback.is_playing());
}

#[tokio::test]
async fn centralized_shutdown_emits_retained_lifecycle_events() {
    let manager = ConversationManager::new();
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let observed = Arc::new(SyncMutex::new(Vec::new()));
    let observed_for_callback = observed.clone();
    *manager.lifecycle_callback.lock() = Some(Arc::new(move |lifecycle| {
        observed_for_callback.lock().push(lifecycle);
    }));
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("test-session".to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    manager.stop_session(capture, playback).await;

    assert_eq!(
        observed.lock().as_slice(),
        &[ConversationLifecycle::Stopping, ConversationLifecycle::Idle]
    );
    assert!(manager.lifecycle_callback.lock().is_none());
}

#[tokio::test]
async fn provider_closed_event_loop_converges_through_centralized_cleanup() {
    let manager = ConversationManager::new();
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let (pcm_tx, _pcm_rx) = mpsc::channel(1);
    capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count: close_count.clone(),
        interrupt_count,
    }));
    let generation = 11;
    let session_id = "closed-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let local_asr_stop_count = attach_counting_local_asr(&manager, generation).await;

    let lifecycle_events = Arc::new(SyncMutex::new(Vec::new()));
    let lifecycle_events_for_callback = lifecycle_events.clone();
    let lifecycle_callback: LifecycleCallback =
        Arc::new(move |lifecycle| lifecycle_events_for_callback.lock().push(lifecycle));
    *manager.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
    let state_events = Arc::new(SyncMutex::new(Vec::new()));
    let context = test_event_loop_context(
        generation,
        session_id,
        capture.clone(),
        playback.clone(),
        state_events.clone(),
        lifecycle_callback,
    );
    let (event_tx, event_rx) = mpsc::channel(1);
    event_tx.send(LiveServerEvent::Closed).await.unwrap();
    drop(event_tx);

    manager.run_event_loop(event_rx, context).await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    assert_eq!(state_events.lock().as_slice(), &[CharacterState::Idle]);
    assert_eq!(
        lifecycle_events.lock().as_slice(),
        &[ConversationLifecycle::Stopping, ConversationLifecycle::Idle]
    );
}

#[tokio::test]
async fn provider_error_event_loop_converges_to_failed_cleanup() {
    let manager = ConversationManager::new();
    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let (pcm_tx, _pcm_rx) = mpsc::channel(1);
    capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count: close_count.clone(),
        interrupt_count,
    }));
    let generation = 12;
    let session_id = "error-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let local_asr_stop_count = attach_counting_local_asr(&manager, generation).await;

    let lifecycle_events = Arc::new(SyncMutex::new(Vec::new()));
    let lifecycle_events_for_callback = lifecycle_events.clone();
    let lifecycle_callback: LifecycleCallback =
        Arc::new(move |lifecycle| lifecycle_events_for_callback.lock().push(lifecycle));
    *manager.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
    let state_events = Arc::new(SyncMutex::new(Vec::new()));
    let mut context = test_event_loop_context(
        generation,
        session_id,
        capture.clone(),
        playback.clone(),
        state_events.clone(),
        lifecycle_callback,
    );
    let provider_errors = Arc::new(SyncMutex::new(Vec::new()));
    let provider_errors_for_callback = provider_errors.clone();
    context.provider_error_callback = Arc::new(move |error| {
        provider_errors_for_callback.lock().push(error);
    });
    let (event_tx, event_rx) = mpsc::channel(1);
    event_tx
        .send(LiveServerEvent::Error(ProviderError::from_kind(
            ProviderErrorKind::Protocol,
        )))
        .await
        .unwrap();
    drop(event_tx);

    manager.run_event_loop(event_rx, context).await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
    assert_eq!(state_events.lock().as_slice(), &[CharacterState::Error]);
    assert_eq!(provider_errors.lock().len(), 1);
    assert_eq!(provider_errors.lock()[0].kind, ProviderErrorKind::Protocol);
    assert_eq!(
        lifecycle_events.lock().as_slice(),
        &[
            ConversationLifecycle::Stopping,
            ConversationLifecycle::Failed
        ]
    );
}
