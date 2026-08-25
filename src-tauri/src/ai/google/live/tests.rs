use super::*;
use crate::tools::policy::{
    ToolConfirmationPolicy, ToolDeclaration, ToolExecutionPolicy, ToolPermissionLevel,
    ToolPrivacyGate,
};

fn config() -> LiveSessionConfig {
    LiveSessionConfig {
        model: "gemini-3.1-flash-live-preview".to_string(),
        voice_name: Some("Puck".to_string()),
        system_instruction: Some("Be a moose".to_string()),
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: vec![ToolDeclaration {
            name: "get_current_time".to_string(),
            description: "Get time".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            permission: ToolPermissionLevel::SafeReadOnly,
            privacy_gate: ToolPrivacyGate::None,
            confirmation: ToolConfirmationPolicy::None,
            execution: ToolExecutionPolicy::new(250, 64, 1_024),
        }],
    }
}

async fn expect_transcript_event(
    receiver: &mut mpsc::Receiver<LiveServerEvent>,
    user: bool,
    expected_text: &str,
    expected_final: bool,
) {
    let event = receiver.recv().await.expect("transcript event");
    let update = match (user, event) {
        (true, LiveServerEvent::UserTranscript(update))
        | (false, LiveServerEvent::ModelTranscript(update)) => update,
        (_, other) => panic!("unexpected transcript event: {other:?}"),
    };
    assert_eq!(update.text, expected_text);
    assert_eq!(update.is_final, expected_final);
}

#[test]
fn setup_enables_transcription_tools_resumption_and_compression() {
    let value = serde_json::to_value(setup_message(&config(), None)).unwrap();
    let setup = &value["setup"];
    assert_eq!(setup["model"], "models/gemini-3.1-flash-live-preview");
    assert!(setup.get("inputAudioTranscription").is_some());
    assert!(setup.get("outputAudioTranscription").is_some());
    assert!(setup.get("sessionResumption").is_some());
    assert!(setup.get("contextWindowCompression").is_some());
    let declaration = &setup["tools"][0]["functionDeclarations"][0];
    assert_eq!(declaration["name"], "get_current_time");
    assert!(declaration.get("parametersJsonSchema").is_some());
    assert!(declaration.get("permission").is_none());
    assert!(declaration.get("privacy_gate").is_none());
    assert!(declaration.get("confirmation").is_none());
    assert!(declaration.get("execution").is_none());
}

#[test]
fn audio_uses_current_realtime_audio_shape() {
    let value = serde_json::to_value(audio_message(&[0, 1, 2, 3], 16_000)).unwrap();
    assert!(value["realtimeInput"].get("audio").is_some());
    assert!(value["realtimeInput"].get("mediaChunks").is_none());
}

#[test]
fn tool_response_preserves_call_id_and_name() {
    let value = serde_json::to_value(tool_response_message(ToolCallResponse {
        id: "call-7".to_string(),
        name: "get_current_time".to_string(),
        output: json!({ "time": "noon" }),
    }))
    .unwrap();
    let response = &value["toolResponse"]["functionResponses"][0];
    assert_eq!(response["id"], "call-7");
    assert_eq!(response["name"], "get_current_time");
}

#[test]
fn websocket_auth_uses_documented_query_parameter_with_url_encoding() {
    const KEY: &str = "AIzaSyLIVE key+with/specials";
    let url = live_websocket_url(KEY).unwrap();
    let parsed = Url::parse(&url).unwrap();

    assert_eq!(
        parsed
            .query_pairs()
            .find(|(name, _)| name == "key")
            .unwrap()
            .1,
        KEY
    );
    assert!(!url.contains("LIVE key+with/specials"));
}

#[test]
fn reconnect_backoff_is_bounded() {
    assert_eq!(reconnect_delay(1, 0), Duration::from_millis(250));
    assert!(reconnect_delay(8, 500) <= Duration::from_millis(2_125));
}

#[tokio::test]
async fn transcription_fragments_finalize_at_turn_boundaries() {
    let (tx, mut rx) = mpsc::channel(8);
    let mut input = String::new();
    let mut output = String::new();
    let mut resume = None;
    let server: LiveServerMessage = serde_json::from_value(json!({
        "serverContent": { "inputTranscription": { "text": "hello moose" } }
    }))
    .unwrap();
    assert!(matches!(
        handle_server_message(server, &tx, &mut input, &mut output, &mut resume).await,
        ServerAction::Continue
    ));
    assert!(matches!(
        rx.recv().await,
        Some(LiveServerEvent::UserTranscript(TranscriptUpdate {
            is_final: false,
            ..
        }))
    ));

    let server: LiveServerMessage = serde_json::from_value(json!({
        "serverContent": { "outputTranscription": { "text": "hello human" }, "turnComplete": true }
    }))
    .unwrap();
    handle_server_message(server, &tx, &mut input, &mut output, &mut resume).await;
    assert!(matches!(
        rx.recv().await,
        Some(LiveServerEvent::UserTranscript(TranscriptUpdate {
            is_final: true,
            ..
        }))
    ));
    assert!(matches!(
        rx.recv().await,
        Some(LiveServerEvent::ModelTranscript(TranscriptUpdate {
            is_final: false,
            ..
        }))
    ));
    assert!(matches!(
        rx.recv().await,
        Some(LiveServerEvent::ModelTranscript(TranscriptUpdate {
            is_final: true,
            ..
        }))
    ));
    assert!(matches!(
        rx.recv().await,
        Some(LiveServerEvent::TurnComplete)
    ));
}

#[tokio::test]
async fn input_transcription_deduplicates_updates_and_emits_one_final() {
    let (tx, mut rx) = mpsc::channel(8);
    let mut input = String::new();
    let mut output = String::new();
    let mut resume = None;

    for frame in [
        json!({
            "serverContent": { "interimInputTranscription": { "text": "hello" } }
        }),
        json!({
            "serverContent": { "interimInputTranscription": { "text": "hello" } }
        }),
        json!({
            "serverContent": {
                "inputTranscription": { "text": "hello moose", "finished": true }
            }
        }),
    ] {
        let server: LiveServerMessage = serde_json::from_value(frame).unwrap();
        assert!(matches!(
            handle_server_message(server, &tx, &mut input, &mut output, &mut resume).await,
            ServerAction::Continue
        ));
    }

    expect_transcript_event(&mut rx, true, "hello", false).await;
    expect_transcript_event(&mut rx, true, "hello moose", false).await;
    expect_transcript_event(&mut rx, true, "hello moose", true).await;
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn output_transcription_deduplicates_updates_and_emits_one_final() {
    let (tx, mut rx) = mpsc::channel(8);
    let mut input = String::new();
    let mut output = String::new();
    let mut resume = None;

    for frame in [
        json!({
            "serverContent": { "outputTranscription": { "text": "hello" } }
        }),
        json!({
            "serverContent": { "outputTranscription": { "text": "hello" } }
        }),
        json!({
            "serverContent": {
                "outputTranscription": { "text": "hello human", "finished": true }
            }
        }),
    ] {
        let server: LiveServerMessage = serde_json::from_value(frame).unwrap();
        assert!(matches!(
            handle_server_message(server, &tx, &mut input, &mut output, &mut resume).await,
            ServerAction::Continue
        ));
    }

    expect_transcript_event(&mut rx, false, "hello", false).await;
    expect_transcript_event(&mut rx, false, "hello human", false).await;
    expect_transcript_event(&mut rx, false, "hello human", true).await;
    assert!(rx.try_recv().is_err());
}

#[test]
fn malformed_known_frame_is_protocol_error_without_private_payload() {
    let error = decode_server_frame(Message::Text("{ private transcript".to_string())).unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Protocol);
    assert!(!error.message.contains("private transcript"));
}

#[test]
fn representative_text_and_binary_frames_parse() {
    let setup_frame = json!({
        "setupComplete": {},
        "futureEnvelopeField": true
    })
    .to_string();
    let setup = decode_server_frame(Message::Text(setup_frame))
        .unwrap()
        .unwrap();
    assert!(setup.setup_complete.is_some());

    let binary_frame = json!({
        "serverContent": {
            "outputTranscription": { "text": "binary hello" },
            "futureField": 7
        }
    })
    .to_string()
    .into_bytes();
    let binary = decode_server_frame(Message::Binary(binary_frame))
        .unwrap()
        .unwrap();
    assert_eq!(
        binary
            .server_content
            .unwrap()
            .output_transcription
            .unwrap()
            .text,
        "binary hello"
    );
}

#[test]
fn setup_rejection_is_typed_and_private_payload_is_discarded() {
    let frame = json!({
        "error": {
            "code": 400,
            "status": "INVALID_ARGUMENT",
            "message": "private transcript and api material"
        }
    })
    .to_string();
    let server = decode_server_frame(Message::Text(frame)).unwrap().unwrap();
    let error = provider_error_from_server_error(server.error.as_ref().unwrap());
    assert_eq!(error.kind, ProviderErrorKind::Setup);
    assert!(!error.retryable);
    assert!(!error.message.contains("private transcript"));
    assert!(!error.message.contains("api material"));
}

#[test]
fn duplicate_transcript_fragments_are_suppressed_or_replaced() {
    let mut buffer = String::new();
    assert!(append_transcript(&mut buffer, "hello"));
    assert!(!append_transcript(&mut buffer, "hello"));
    assert!(append_transcript(&mut buffer, "hello human"));
    assert_eq!(buffer, "hello human");
    assert!(!append_transcript(&mut buffer, "human"));
}

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
    let mut session = GoogleLiveSession {
        sender: tx,
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
    assert_eq!(retained.kind, OutboundKind::ToolResponse);
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
        OutboundEnvelope::new(
            Message::Text("tool-response".to_string()),
            OutboundKind::ToolResponse,
        ),
        &diagnostics,
    );
    assert_eq!(
        pending.as_ref().map(|item| item.kind),
        Some(OutboundKind::ToolResponse)
    );
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
