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
