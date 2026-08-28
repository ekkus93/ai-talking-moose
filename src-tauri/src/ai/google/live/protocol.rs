fn setup_message(config: &LiveSessionConfig, resume_handle: Option<&str>) -> LiveClientMessage {
    let voice_name = config.voice_name.as_deref().unwrap_or("Puck").to_string();
    LiveClientMessage {
        setup: Some(LiveSetupConfig {
            model: format!("models/{}", config.model),
            generation_config: Some(LiveGenerationConfig {
                response_modalities: Some(vec!["AUDIO".to_string()]),
                speech_config: Some(LiveSpeechConfig {
                    voice_config: Some(LiveVoiceConfig {
                        prebuilt_voice_config: Some(LivePrebuiltVoiceConfig { voice_name }),
                    }),
                }),
            }),
            system_instruction: Some(LiveContent {
                parts: vec![LivePart {
                    text: config.system_instruction.clone(),
                    inline_data: None,
                }],
            }),
            tools: tool_declarations(config),
            input_audio_transcription: Some(json!({})),
            output_audio_transcription: Some(json!({})),
            session_resumption: Some(LiveSessionResumptionConfig {
                handle: resume_handle.map(ToOwned::to_owned),
            }),
            context_window_compression: Some(json!({ "slidingWindow": {} })),
        }),
        realtime_input: None,
        tool_response: None,
    }
}

fn audio_message(pcm_bytes: &[u8], sample_rate: u32) -> LiveClientMessage {
    LiveClientMessage {
        setup: None,
        realtime_input: Some(LiveRealtimeInput {
            audio: Some(LiveBlob {
                mime_type: format!("audio/pcm;rate={sample_rate}"),
                data: base64::engine::general_purpose::STANDARD.encode(pcm_bytes),
            }),
            text: None,
        }),
        tool_response: None,
    }
}

fn text_turn_message(text: &str) -> LiveClientMessage {
    LiveClientMessage {
        setup: None,
        realtime_input: Some(LiveRealtimeInput {
            audio: None,
            text: Some(text.to_string()),
        }),
        tool_response: None,
    }
}

fn tool_response_message(response: ToolCallResponse) -> LiveClientMessage {
    LiveClientMessage {
        setup: None,
        realtime_input: None,
        tool_response: Some(LiveToolResponse {
            function_responses: vec![LiveFunctionResponse {
                id: response.id,
                name: response.name,
                response: response.output,
            }],
        }),
    }
}

fn encode_client_message(message: &LiveClientMessage) -> Result<Message, ProviderError> {
    serde_json::to_string(message)
        .map(Message::Text)
        .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Internal))
}

fn decode_server_frame(message: Message) -> Result<Option<LiveServerMessage>, ProviderError> {
    let text = match message {
        Message::Text(text) => text,
        Message::Binary(binary) => String::from_utf8(binary)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Protocol))?,
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => return Ok(None),
        Message::Close(_) => return Ok(None),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Protocol))
}

fn live_websocket_url(api_key: &str) -> Result<String, ProviderError> {
    let mut url = Url::parse(LIVE_WEBSOCKET_ENDPOINT)
        .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Internal))?;
    url.query_pairs_mut().append_pair("key", api_key);
    Ok(url.to_string())
}

fn trace_live_provider_failure(error: &ProviderError) {
    trace_google_provider_failure("live", error);
}

async fn open_and_setup(
    api_key: &str,
    config: &LiveSessionConfig,
    resume_handle: Option<&str>,
    cancellation: Option<&CancellationToken>,
) -> Result<GeminiSocket, ProviderError> {
    #[cfg(test)]
    if crate::test_support::network_denied() {
        return Err(ProviderError::from_kind(ProviderErrorKind::Network));
    }

    let url = live_websocket_url(api_key)?;
    let connect_result = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(&url))
        .await
        .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?;
    let (mut socket, _) = connect_result.map_err(|error| {
        let provider_error = provider_error_for_connect(error);
        trace_live_provider_failure(&provider_error);
        provider_error
    })?;

    let setup = encode_client_message(&setup_message(config, resume_handle))?;
    write_socket_message(&mut socket, setup, cancellation, SOCKET_WRITE_TIMEOUT).await?;

    let setup_deadline = tokio::time::Instant::now() + SETUP_TIMEOUT;
    loop {
        let next = tokio::time::timeout_at(setup_deadline, socket.next())
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Setup))?;
        let Some(frame) = next else {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        };
        match frame {
            Ok(Message::Close(frame)) => {
                let code = frame.as_ref().map(|value| u16::from(value.code));
                return Err(provider_error_for_close(code)
                    .unwrap_or_else(|| ProviderError::from_kind(ProviderErrorKind::Closed)));
            }
            Ok(message) => {
                let Some(server) = decode_server_frame(message)? else {
                    continue;
                };
                if let Some(error) = server.error.as_ref() {
                    return Err(provider_error_from_server_error(error));
                }
                if server.setup_complete.is_some() {
                    return Ok(socket);
                }
                return Err(ProviderError::from_kind(ProviderErrorKind::Setup));
            }
            Err(error) => {
                let provider_error = provider_error_for_connect(error);
                trace_live_provider_failure(&provider_error);
                return Err(provider_error);
            }
        }
    }
}
