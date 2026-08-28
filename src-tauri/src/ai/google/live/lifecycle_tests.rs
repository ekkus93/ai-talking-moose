use super::*;

fn lifecycle_config() -> LiveSessionConfig {
    LiveSessionConfig {
        model: "gemini-3.1-flash-live-preview".to_string(),
        voice_name: Some("Puck".to_string()),
        system_instruction: None,
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: Vec::new(),
    }
}

#[tokio::test]
async fn websocket_write_wait_is_bounded_and_cancellation_aware() {
    let cancellation = CancellationToken::new();
    let timed_out = await_bounded_write(
        std::future::pending::<Result<(), ()>>(),
        Some(&cancellation),
        Duration::from_millis(10),
    )
    .await;
    assert_eq!(timed_out, Err(WriteWaitError::TimedOut));

    cancellation.cancel();
    let cancelled = await_bounded_write(
        std::future::pending::<Result<(), ()>>(),
        Some(&cancellation),
        Duration::from_secs(1),
    )
    .await;
    assert_eq!(cancelled, Err(WriteWaitError::Cancelled));
}

#[tokio::test]
async fn explicit_close_uses_dedicated_control_path_and_waits_for_supervisor_ack() {
    let (out_tx, _out_rx) = mpsc::channel(1);
    let (close_tx, mut close_rx) = mpsc::channel(1);
    let is_active = Arc::new(AtomicBool::new(true));
    let cancellation = CancellationToken::new();
    let diagnostics = Arc::new(LiveOutboundDiagnosticsStore::default());
    let active_probe = is_active.clone();
    let cancellation_probe = cancellation.clone();
    let mut session = GoogleLiveSession {
        sender: out_tx,
        close_sender: close_tx,
        is_active,
        cancellation,
        diagnostics,
        sample_rate_in: 16_000,
    };

    // Saturate the ordinary outbound queue. Protocol close must not wait behind it.
    session.send_text_turn("queued before close").await.unwrap();

    let close = tokio::spawn(async move { session.close().await });
    let request = tokio::time::timeout(Duration::from_millis(100), close_rx.recv())
        .await
        .expect("dedicated close control path must remain responsive")
        .expect("close request");
    assert!(active_probe.load(Ordering::SeqCst));
    assert!(cancellation_probe.is_cancelled());

    request.complete(Ok(()));
    close.await.unwrap().unwrap();
    assert!(!active_probe.load(Ordering::SeqCst));
}

#[test]
fn dropping_live_session_force_cancels_supervisor() {
    let (out_tx, _out_rx) = mpsc::channel(1);
    let (close_tx, _close_rx) = mpsc::channel(1);
    let is_active = Arc::new(AtomicBool::new(true));
    let cancellation = CancellationToken::new();
    let active_probe = is_active.clone();
    let cancellation_probe = cancellation.clone();
    let session = GoogleLiveSession {
        sender: out_tx,
        close_sender: close_tx,
        is_active,
        cancellation,
        diagnostics: Arc::new(LiveOutboundDiagnosticsStore::default()),
        sample_rate_in: 16_000,
    };

    drop(session);

    assert!(!active_probe.load(Ordering::SeqCst));
    assert!(cancellation_probe.is_cancelled());
}

#[tokio::test]
async fn network_denial_harness_blocks_live_before_websocket_connect() {
    let _guard = crate::test_support::deny_network_for_scope();
    let provider = GoogleLiveProvider::new(GoogleAuth::new("valid-test-key".to_string()));
    let (events, _receiver) = mpsc::channel(1);

    let error = match provider.connect(lifecycle_config(), events).await {
        Ok(_) => panic!("network denial must stop Live before WebSocket I/O"),
        Err(error) => error,
    };
    assert_eq!(error.kind, ProviderErrorKind::Network);
}
