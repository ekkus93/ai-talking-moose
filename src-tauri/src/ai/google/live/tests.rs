use super::*;
use crate::tools::policy::{ToolDeclaration, ToolPermissionLevel};

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
            parameters: json!({ "type": "object", "properties": {} }),
            permission: ToolPermissionLevel::SafeReadOnly,
        }],
    }
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
    assert_eq!(
        setup["tools"][0]["functionDeclarations"][0]["name"],
        "get_current_time"
    );
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

#[test]
fn malformed_known_frame_is_protocol_error_without_private_payload() {
    let error = decode_server_frame(Message::Text("{ private transcript".to_string())).unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Protocol);
    assert!(!error.message.contains("private transcript"));
}

#[test]
fn representative_text_and_binary_frames_parse() {
    let setup = decode_server_frame(Message::Text(
        r#"{"setupComplete":{},"futureEnvelopeField":true}"#.to_string(),
    ))
    .unwrap()
    .unwrap();
    assert!(setup.setup_complete.is_some());

    let binary = decode_server_frame(Message::Binary(
        br#"{"serverContent":{"outputTranscription":{"text":"binary hello"},"futureField":7}}"#
            .to_vec(),
    ))
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
    let server = decode_server_frame(Message::Text(
        r#"{"error":{"code":400,"status":"INVALID_ARGUMENT","message":"private transcript and api material"}}"#
            .to_string(),
    ))
    .unwrap()
    .unwrap();
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
fn retry_policy_has_hard_bounds_and_explicit_close_cancels_reconnect() {
    assert_eq!(MAX_RECONNECT_ATTEMPTS, 3);
    assert_eq!(MAX_RECONNECT_ELAPSED, Duration::from_secs(20));
    assert_eq!(reconnect_delay(1, 0), Duration::from_millis(250));
    assert_eq!(reconnect_delay(2, 0), Duration::from_millis(500));
    assert_eq!(reconnect_delay(3, 0), Duration::from_millis(1_000));
    assert!(reconnect_delay(8, u64::MAX) <= Duration::from_millis(2_125));

    let active = AtomicBool::new(true);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(Message::Close(None)).unwrap();
    assert!(!drain_disconnected_messages(&mut rx, &active));
    assert!(!active.load(Ordering::SeqCst));
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
