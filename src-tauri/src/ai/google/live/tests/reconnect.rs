#[test]
fn retry_policy_has_hard_bounds() {
    assert_eq!(MAX_RECONNECT_ATTEMPTS, 3);
    assert_eq!(MAX_RECONNECT_ELAPSED, Duration::from_secs(20));
    assert_eq!(reconnect_delay(1, 0), Duration::from_millis(250));
    assert_eq!(reconnect_delay(2, 0), Duration::from_millis(500));
    assert_eq!(reconnect_delay(3, 0), Duration::from_millis(1_000));
    assert!(reconnect_delay(8, u64::MAX) <= Duration::from_millis(2_125));
}

#[tokio::test]
async fn explicit_close_cancels_in_flight_reconnect_attempt() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let error = await_reconnect_attempt(
        std::future::pending::<Result<(), ProviderError>>(),
        &cancellation,
    )
    .await
    .expect_err("explicit close must cancel a pending reconnect attempt");

    assert_eq!(error.kind, ProviderErrorKind::Closed);
}

#[tokio::test]
async fn reconnect_wait_does_not_consume_queued_tool_response() {
    let diagnostics = Arc::new(LiveOutboundDiagnosticsStore::default());
    let (tx, mut rx) = mpsc::channel(2);
    let is_active = Arc::new(AtomicBool::new(true));
    let cancellation = CancellationToken::new();
    let (close_tx, _close_rx) = mpsc::channel(1);
    let mut session = GoogleLiveSession {
        sender: tx,
        close_sender: close_tx,
        is_active,
        cancellation: cancellation.clone(),
        diagnostics: diagnostics.clone(),
        sample_rate_in: 16_000,
    };

    session
        .send_tool_response(ToolCallResponse {
            id: "call-reconnect".to_string(),
            name: "get_current_time".to_string(),
            output: json!({"time": "noon"}),
        })
        .await
        .unwrap();

    let reconnect = tokio::spawn({
        let cancellation = cancellation.clone();
        async move {
            await_reconnect_attempt(
                std::future::pending::<Result<(), ProviderError>>(),
                &cancellation,
            )
            .await
        }
    });
    tokio::task::yield_now().await;

    let retained = rx
        .try_recv()
        .expect("reconnect must leave outbound queue intact");
    assert!(matches!(retained.message, Message::Text(_)));
    assert_eq!(diagnostics.snapshot().queued, 1);

    cancellation.cancel();
    let error = reconnect.await.unwrap().unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Closed);
}

#[test]
fn failed_tool_response_is_retained_for_retry_then_counted_on_terminal_failure() {
    let diagnostics = LiveOutboundDiagnosticsStore::default();
    let mut pending = None;
    retain_for_retry(
        &mut pending,
        OutboundEnvelope::new(Message::Text("tool-response".to_string())),
        &diagnostics,
    );
    assert!(matches!(
        pending.as_ref().map(|item| &item.message),
        Some(Message::Text(text)) if text == "tool-response"
    ));
    assert_eq!(diagnostics.snapshot().retried, 1);

    let (_tx, mut rx) = mpsc::channel(1);
    assert_eq!(
        record_terminal_outbound_failures(&mut pending, &mut rx, &diagnostics),
        1
    );
    let snapshot = diagnostics.snapshot();
    assert_eq!(snapshot.terminally_failed, 1);
    assert_eq!(snapshot.dropped, 1);
}
