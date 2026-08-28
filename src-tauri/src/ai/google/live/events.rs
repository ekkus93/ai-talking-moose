fn append_transcript(buffer: &mut String, fragment: &str) -> bool {
    let fragment = fragment.trim();
    if fragment.is_empty() {
        return false;
    }
    if buffer.is_empty() {
        buffer.push_str(fragment);
        return true;
    }
    if fragment == buffer || buffer.ends_with(fragment) {
        return false;
    }
    if fragment.starts_with(buffer.as_str()) {
        buffer.clear();
        buffer.push_str(fragment);
        return true;
    }
    let buffer_ends_space = buffer.chars().last().is_some_and(char::is_whitespace);
    let fragment_starts_space = fragment.chars().next().is_some_and(char::is_whitespace);
    if !buffer_ends_space && !fragment_starts_space {
        buffer.push(' ');
    }
    buffer.push_str(fragment);
    true
}

async fn emit_transcript(
    sender: &mpsc::Sender<LiveServerEvent>,
    user: bool,
    text: &str,
    is_final: bool,
) {
    if text.trim().is_empty() {
        return;
    }
    let update = TranscriptUpdate {
        text: text.trim().to_string(),
        is_final,
    };
    let event = if user {
        LiveServerEvent::UserTranscript(update)
    } else {
        LiveServerEvent::ModelTranscript(update)
    };
    let _ = sender.send(event).await;
}

async fn finalize_transcript(
    sender: &mpsc::Sender<LiveServerEvent>,
    user: bool,
    buffer: &mut String,
) {
    if !buffer.trim().is_empty() {
        emit_transcript(sender, user, buffer, true).await;
        buffer.clear();
    }
}

enum ServerAction {
    Continue,
    Reconnect,
    Terminal(ProviderError),
}

async fn handle_server_message(
    server: LiveServerMessage,
    sender: &mpsc::Sender<LiveServerEvent>,
    input_transcript: &mut String,
    output_transcript: &mut String,
    resume_handle: &mut Option<String>,
) -> ServerAction {
    if let Some(error) = server.error.as_ref() {
        return ServerAction::Terminal(provider_error_from_server_error(error));
    }
    if let Some(update) = server.session_resumption_update.as_ref() {
        if update.resumable != Some(false) {
            if let Some(handle) = update.handle() {
                *resume_handle = Some(handle.to_string());
            }
        }
    }
    if server.go_away.is_some() {
        return ServerAction::Reconnect;
    }

    if let Some(content) = server.server_content {
        if content.interrupted == Some(true) {
            output_transcript.clear();
            let _ = sender.send(LiveServerEvent::Interrupted).await;
        }

        let input_fragment = content
            .interim_input_transcription
            .as_ref()
            .or(content.input_transcription.as_ref());
        if let Some(transcription) = input_fragment {
            if append_transcript(input_transcript, &transcription.text) {
                emit_transcript(sender, true, input_transcript, false).await;
            }
            if transcription.finished == Some(true) {
                finalize_transcript(sender, true, input_transcript).await;
            }
        }

        if let Some(transcription) = content.output_transcription.as_ref() {
            finalize_transcript(sender, true, input_transcript).await;
            if append_transcript(output_transcript, &transcription.text) {
                emit_transcript(sender, false, output_transcript, false).await;
            }
            if transcription.finished == Some(true) {
                finalize_transcript(sender, false, output_transcript).await;
            }
        }

        if let Some(model_turn) = content.model_turn {
            finalize_transcript(sender, true, input_transcript).await;
            for part in model_turn.parts {
                if let Some(inline_data) = part.inline_data {
                    match base64::engine::general_purpose::STANDARD.decode(inline_data.data) {
                        Ok(pcm) => {
                            let _ = sender.send(LiveServerEvent::AudioData(pcm)).await;
                        }
                        Err(_) => {
                            return ServerAction::Terminal(ProviderError::from_kind(
                                ProviderErrorKind::Protocol,
                            ));
                        }
                    }
                }
                if content.output_transcription.is_none() {
                    if let Some(text) = part.text {
                        if append_transcript(output_transcript, &text) {
                            emit_transcript(sender, false, output_transcript, false).await;
                        }
                    }
                }
            }
        }

        if content.turn_complete == Some(true) || content.generation_complete == Some(true) {
            finalize_transcript(sender, true, input_transcript).await;
            finalize_transcript(sender, false, output_transcript).await;
            let _ = sender.send(LiveServerEvent::TurnComplete).await;
        }
    }

    if let Some(tool_call) = server.tool_call {
        for call in tool_call.function_calls {
            if call.id.is_empty() || call.name.is_empty() {
                return ServerAction::Terminal(ProviderError::from_kind(
                    ProviderErrorKind::Protocol,
                ));
            }
            let _ = sender
                .send(LiveServerEvent::ToolCall {
                    id: call.id,
                    name: call.name,
                    args: call.args,
                })
                .await;
        }
    }

    ServerAction::Continue
}
