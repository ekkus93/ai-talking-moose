use crate::ai::google::auth::GoogleAuth;
use crate::ai::google::config::{validate_live_model, LIVE_WEBSOCKET_ENDPOINT};
use crate::ai::google::protocol::{
    LiveBlob, LiveClientMessage, LiveContent, LiveFunctionResponse, LiveGenerationConfig, LivePart,
    LivePrebuiltVoiceConfig, LiveRealtimeInput, LiveServerError, LiveServerMessage,
    LiveSessionResumptionConfig, LiveSetupConfig, LiveSpeechConfig, LiveToolResponse,
    LiveVoiceConfig,
};
use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::{
    LiveServerEvent, LiveSessionConfig, ProviderError, ProviderErrorKind, ToolCallResponse,
    TranscriptUpdate,
};
use async_trait::async_trait;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const SETUP_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_RECONNECT_ATTEMPTS: u32 = 3;
const MAX_RECONNECT_ELAPSED: Duration = Duration::from_secs(20);
const BASE_RECONNECT_DELAY_MS: u64 = 250;
const MAX_RECONNECT_DELAY_MS: u64 = 2_000;
const MAX_RECONNECT_JITTER_MS: u64 = 125;

type GeminiSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct GoogleLiveProvider {
    auth: GoogleAuth,
}

impl GoogleLiveProvider {
    pub fn new(auth: GoogleAuth) -> Self {
        Self { auth }
    }
}

fn provider_error_for_http_status(status: u16) -> ProviderError {
    let kind = match status {
        401 | 403 => ProviderErrorKind::Auth,
        404 => ProviderErrorKind::Model,
        429 => ProviderErrorKind::Quota,
        400 => ProviderErrorKind::Setup,
        _ if status >= 500 => ProviderErrorKind::Network,
        _ => ProviderErrorKind::Protocol,
    };
    ProviderError::from_kind(kind)
}

fn provider_error_for_connect(error: WebSocketError) -> ProviderError {
    match error {
        WebSocketError::Http(response) => {
            provider_error_for_http_status(response.status().as_u16())
        }
        _ => ProviderError::from_kind(ProviderErrorKind::Network),
    }
}

fn provider_error_from_server_error(error: &LiveServerError) -> ProviderError {
    let code = error.code;
    let status = error.status.as_deref().unwrap_or_default();
    let kind = if matches!(code, Some(401 | 403))
        || matches!(status, "UNAUTHENTICATED" | "PERMISSION_DENIED")
    {
        ProviderErrorKind::Auth
    } else if code == Some(429) || status == "RESOURCE_EXHAUSTED" {
        ProviderErrorKind::Quota
    } else if code == Some(404) || status == "NOT_FOUND" {
        ProviderErrorKind::Model
    } else if code == Some(400) || matches!(status, "INVALID_ARGUMENT" | "FAILED_PRECONDITION") {
        ProviderErrorKind::Setup
    } else {
        ProviderErrorKind::Protocol
    };
    ProviderError::from_kind(kind)
}

fn provider_error_for_close(code: Option<u16>) -> Option<ProviderError> {
    match code {
        None | Some(1000 | 1001) => None,
        Some(1008) => Some(ProviderError::from_kind(ProviderErrorKind::Auth)),
        Some(1011 | 1006) => Some(ProviderError::from_kind(ProviderErrorKind::Network)),
        Some(_) => Some(ProviderError::from_kind(ProviderErrorKind::Closed)),
    }
}

fn tool_declarations(config: &LiveSessionConfig) -> Option<Vec<serde_json::Value>> {
    if config.tools.is_empty() {
        return None;
    }
    let declarations = config
        .tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "parametersJsonSchema": tool.parameters,
            })
        })
        .collect::<Vec<_>>();
    Some(vec![json!({ "functionDeclarations": declarations })])
}

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

async fn open_and_setup(
    api_key: &str,
    config: &LiveSessionConfig,
    resume_handle: Option<&str>,
) -> Result<GeminiSocket, ProviderError> {
    let url = format!("{LIVE_WEBSOCKET_ENDPOINT}?key={api_key}");
    let connect_result = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(&url))
        .await
        .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?;
    let (mut socket, _) = connect_result.map_err(provider_error_for_connect)?;

    let setup = encode_client_message(&setup_message(config, resume_handle))?;
    socket
        .send(setup)
        .await
        .map_err(provider_error_for_connect)?;

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
            Err(error) => return Err(provider_error_for_connect(error)),
        }
    }
}

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

fn reconnect_delay(attempt: u32, jitter_ms: u64) -> Duration {
    let exponent = attempt.saturating_sub(1).min(6);
    let base = BASE_RECONNECT_DELAY_MS.saturating_mul(1_u64 << exponent);
    Duration::from_millis(base.min(MAX_RECONNECT_DELAY_MS) + jitter_ms.min(MAX_RECONNECT_JITTER_MS))
}

fn drain_disconnected_messages(
    receiver: &mut mpsc::Receiver<Message>,
    is_active: &AtomicBool,
) -> bool {
    while let Ok(message) = receiver.try_recv() {
        if matches!(message, Message::Close(_)) {
            is_active.store(false, Ordering::SeqCst);
            return false;
        }
    }
    is_active.load(Ordering::SeqCst)
}

async fn reconnect(
    api_key: &str,
    config: &LiveSessionConfig,
    resume_handle: Option<&str>,
    receiver: &mut mpsc::Receiver<Message>,
    is_active: &AtomicBool,
) -> Result<GeminiSocket, ProviderError> {
    let started = Instant::now();
    let mut last_error = ProviderError::from_kind(ProviderErrorKind::Network);
    for attempt in 1..=MAX_RECONNECT_ATTEMPTS {
        if !is_active.load(Ordering::SeqCst) || started.elapsed() >= MAX_RECONNECT_ELAPSED {
            break;
        }
        if !drain_disconnected_messages(receiver, is_active) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        let jitter = rand::thread_rng().gen_range(0..=MAX_RECONNECT_JITTER_MS);
        let delay = reconnect_delay(attempt, jitter);
        let sleep = tokio::time::sleep(delay);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                () = &mut sleep => break,
                outbound = receiver.recv() => {
                    match outbound {
                        Some(Message::Close(_)) | None => {
                            is_active.store(false, Ordering::SeqCst);
                            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
                        }
                        Some(_) => {}
                    }
                }
            }
        }
        match open_and_setup(api_key, config, resume_handle).await {
            Ok(socket) => {
                drain_disconnected_messages(receiver, is_active);
                return Ok(socket);
            }
            Err(error) => {
                if !error.retryable {
                    return Err(error);
                }
                last_error = error;
            }
        }
    }
    Err(last_error)
}

async fn supervise_socket(
    mut socket: GeminiSocket,
    api_key: String,
    config: LiveSessionConfig,
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<LiveServerEvent>,
    is_active: Arc<AtomicBool>,
) {
    let mut input_transcript = String::new();
    let mut output_transcript = String::new();
    let mut resume_handle = None;

    'connection: loop {
        if !is_active.load(Ordering::SeqCst) {
            break;
        }
        let action = tokio::select! {
            outbound = receiver.recv() => {
                match outbound {
                    Some(message) => {
                        let is_close = matches!(message, Message::Close(_));
                        if socket.send(message).await.is_err() {
                            ServerAction::Reconnect
                        } else if is_close {
                            is_active.store(false, Ordering::SeqCst);
                            break 'connection;
                        } else {
                            ServerAction::Continue
                        }
                    }
                    None => break 'connection,
                }
            }
            inbound = socket.next() => {
                match inbound {
                    Some(Ok(Message::Close(frame))) => {
                        let code = frame.as_ref().map(|value| u16::from(value.code));
                        match provider_error_for_close(code) {
                            Some(error) if error.retryable => ServerAction::Reconnect,
                            Some(error) => ServerAction::Terminal(error),
                            None => {
                                let _ = sender.send(LiveServerEvent::Closed).await;
                                break 'connection;
                            }
                        }
                    }
                    Some(Ok(message)) => match decode_server_frame(message) {
                        Ok(Some(server)) => handle_server_message(
                            server,
                            &sender,
                            &mut input_transcript,
                            &mut output_transcript,
                            &mut resume_handle,
                        ).await,
                        Ok(None) => ServerAction::Continue,
                        Err(error) => ServerAction::Terminal(error),
                    },
                    Some(Err(error)) => {
                        let error = provider_error_for_connect(error);
                        if error.retryable { ServerAction::Reconnect } else { ServerAction::Terminal(error) }
                    }
                    None => ServerAction::Reconnect,
                }
            }
        };

        match action {
            ServerAction::Continue => {}
            ServerAction::Terminal(error) => {
                let _ = sender.send(LiveServerEvent::Error(error)).await;
                break;
            }
            ServerAction::Reconnect => {
                if !is_active.load(Ordering::SeqCst) {
                    break;
                }
                info!("Reconnecting Gemini Live session with bounded backoff");
                match reconnect(
                    &api_key,
                    &config,
                    resume_handle.as_deref(),
                    &mut receiver,
                    &is_active,
                )
                .await
                {
                    Ok(new_socket) => {
                        socket = new_socket;
                        let _ = sender.send(LiveServerEvent::Connected).await;
                    }
                    Err(_error) if !is_active.load(Ordering::SeqCst) => break,
                    Err(error) => {
                        let _ = sender.send(LiveServerEvent::Error(error)).await;
                        break;
                    }
                }
            }
        }
    }

    is_active.store(false, Ordering::SeqCst);
}

pub struct GoogleLiveSession {
    sender: mpsc::Sender<Message>,
    is_active: Arc<AtomicBool>,
    sample_rate_in: u32,
}

#[async_trait]
impl LiveSession for GoogleLiveSession {
    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        let message = encode_client_message(&audio_message(pcm_bytes, self.sample_rate_in))?;
        self.sender
            .send(message)
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        let message = encode_client_message(&text_turn_message(text))?;
        self.sender
            .send(message)
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))
    }

    async fn send_tool_response(
        &mut self,
        response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        let message = encode_client_message(&tool_response_message(response))?;
        self.sender
            .send(message)
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        info!("Barge-in requested on Gemini Live session; server VAD owns interruption");
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        self.is_active.store(false, Ordering::SeqCst);
        let _ = self.sender.send(Message::Close(None)).await;
        Ok(())
    }
}

#[async_trait]
impl RealtimeConversationProvider for GoogleLiveProvider {
    async fn connect(
        &self,
        config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        if !self.auth.is_valid() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Auth));
        }
        validate_live_model(&config.model)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Model))?;

        let socket = open_and_setup(&self.auth.api_key, &config, None).await?;
        let (out_tx, out_rx) = mpsc::channel::<Message>(64);
        let is_active = Arc::new(AtomicBool::new(true));
        let supervisor_active = is_active.clone();
        let supervisor_sender = event_sender.clone();
        let supervisor_key = self.auth.api_key.clone();
        let supervisor_config = config.clone();
        tauri::async_runtime::spawn(async move {
            supervise_socket(
                socket,
                supervisor_key,
                supervisor_config,
                out_rx,
                supervisor_sender,
                supervisor_active,
            )
            .await;
        });
        let _ = event_sender.send(LiveServerEvent::Connected).await;

        Ok(Box::new(GoogleLiveSession {
            sender: out_tx,
            is_active,
            sample_rate_in: config.sample_rate_in,
        }))
    }
}

#[cfg(test)]
mod tests;
