#[test]
fn retryability_is_explicit_by_provider_error_category() {
    for kind in [
        ProviderErrorKind::Quota,
        ProviderErrorKind::Network,
        ProviderErrorKind::Closed,
    ] {
        assert!(ProviderError::from_kind(kind).retryable, "{kind:?}");
    }
    for kind in [
        ProviderErrorKind::Auth,
        ProviderErrorKind::Protocol,
        ProviderErrorKind::Setup,
        ProviderErrorKind::Model,
        ProviderErrorKind::Internal,
    ] {
        assert!(!ProviderError::from_kind(kind).retryable, "{kind:?}");
    }
}

#[tokio::test]
async fn fake_tool_call_frame_emits_provider_neutral_call_identity() {
    let server: LiveServerMessage = serde_json::from_value(json!({
        "toolCall": {
            "functionCalls": [{
                "id": "call-42",
                "name": "get_current_time",
                "args": {}
            }]
        }
    }))
    .unwrap();
    let (tx, mut rx) = mpsc::channel(2);
    let mut input = String::new();
    let mut output = String::new();
    let mut resume = None;

    assert!(matches!(
        handle_server_message(server, &tx, &mut input, &mut output, &mut resume).await,
        ServerAction::Continue
    ));
    match rx.recv().await {
        Some(LiveServerEvent::ToolCall { id, name, args }) => {
            assert_eq!(id, "call-42");
            assert_eq!(name, "get_current_time");
            assert_eq!(args, json!({}));
        }
        other => panic!("unexpected tool event: {other:?}"),
    }
}
