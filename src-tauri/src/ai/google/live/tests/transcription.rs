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
