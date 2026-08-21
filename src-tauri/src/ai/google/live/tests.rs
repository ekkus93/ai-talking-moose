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
