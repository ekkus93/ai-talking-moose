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

include!("tests/protocol.rs");
include!("tests/transcription.rs");
include!("tests/reconnect.rs");
include!("tests/errors_tools.rs");
