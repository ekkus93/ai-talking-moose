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
    LiveOutboundDeliveryState, LiveOutboundDiagnostics, LiveServerEvent, LiveSessionConfig,
    ProviderError, ProviderErrorKind, ToolCallResponse, TranscriptUpdate,
};
use async_trait::async_trait;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::json;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use tracing::info;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const SETUP_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_RECONNECT_ATTEMPTS: u32 = 3;
const MAX_RECONNECT_ELAPSED: Duration = Duration::from_secs(20);
const BASE_RECONNECT_DELAY_MS: u64 = 250;
const MAX_RECONNECT_DELAY_MS: u64 = 2_000;
const MAX_RECONNECT_JITTER_MS: u64 = 125;
const OUTBOUND_QUEUE_CAPACITY: usize = 64;

type GeminiSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct GoogleLiveProvider {
    auth: GoogleAuth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutboundKind {
    Audio,
    TextTurn,
    ToolResponse,
    Close,
}

#[derive(Debug, Clone)]
struct OutboundEnvelope {
    message: Message,
    kind: OutboundKind,
}

impl OutboundEnvelope {
    fn new(message: Message, kind: OutboundKind) -> Self {
        Self { message, kind }
    }

    fn is_close(&self) -> bool {
        self.kind == OutboundKind::Close
    }
}

#[derive(Default)]
struct LiveOutboundDiagnosticsStore {
    queued: AtomicU64,
    written: AtomicU64,
    retried: AtomicU64,
    dropped: AtomicU64,
    terminally_failed: AtomicU64,
}

impl LiveOutboundDiagnosticsStore {
    fn record(&self, state: LiveOutboundDeliveryState) {
        let counter = match state {
            LiveOutboundDeliveryState::Queued => &self.queued,
            LiveOutboundDeliveryState::Written => &self.written,
            LiveOutboundDeliveryState::Retried => &self.retried,
            LiveOutboundDeliveryState::TerminallyFailed => &self.terminally_failed,
        };
        counter.fetch_add(1, Ordering::SeqCst);
    }

    fn record_drop(&self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }

    fn snapshot(&self) -> LiveOutboundDiagnostics {
        LiveOutboundDiagnostics {
            queued: self.queued.load(Ordering::SeqCst),
            written: self.written.load(Ordering::SeqCst),
            retried: self.retried.load(Ordering::SeqCst),
            dropped: self.dropped.load(Ordering::SeqCst),
            terminally_failed: self.terminally_failed.load(Ordering::SeqCst),
        }
    }
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

async fn await_reconnect_attempt<T, F>(
    attempt: F,
    cancellation: &CancellationToken,
) -> Result<T, ProviderError>
where
    F: Future<Output = Result<T, ProviderError>>,
{
    tokio::select! {
        result = attempt => result,
        () = cancellation.cancelled() => Err(ProviderError::from_kind(ProviderErrorKind::Closed)),
    }
}

async fn reconnect(
    api_key: &str,
    config: &LiveSessionConfig,
    resume_handle: Option<&str>,
    is_active: &AtomicBool,
    cancellation: &CancellationToken,
) -> Result<GeminiSocket, ProviderError> {
    let started = Instant::now();
    let mut last_error = ProviderError::from_kind(ProviderErrorKind::Network);
    for attempt in 1..=MAX_RECONNECT_ATTEMPTS {
        if !is_active.load(Ordering::SeqCst)
            || cancellation.is_cancelled()
            || started.elapsed() >= MAX_RECONNECT_ELAPSED
        {
            break;
        }
        let jitter = rand::thread_rng().gen_range(0..=MAX_RECONNECT_JITTER_MS);
        let delay = reconnect_delay(attempt, jitter);
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            () = cancellation.cancelled() => {
                return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
            }
        }

        let reconnect_result = await_reconnect_attempt(
            open_and_setup(api_key, config, resume_handle),
            cancellation,
        )
        .await;
        match reconnect_result {
            Ok(socket) => return Ok(socket),
            Err(error) => {
                if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() {
                    return Err(error);
                }
                if !error.retryable {
                    return Err(error);
                }
                last_error = error;
            }
        }
    }
    Err(last_error)
}

fn retain_for_retry(
    pending_outbound: &mut Option<OutboundEnvelope>,
    envelope: OutboundEnvelope,
    diagnostics: &LiveOutboundDiagnosticsStore,
) {
    debug_assert!(!envelope.is_close());
    diagnostics.record(LiveOutboundDeliveryState::Retried);
    *pending_outbound = Some(envelope);
}

fn record_terminal_outbound_failures(
    pending_outbound: &mut Option<OutboundEnvelope>,
    receiver: &mut mpsc::Receiver<OutboundEnvelope>,
    diagnostics: &LiveOutboundDiagnosticsStore,
) -> usize {
    let mut failed = 0;
    if let Some(envelope) = pending_outbound.take() {
        if !envelope.is_close() {
            diagnostics.record(LiveOutboundDeliveryState::TerminallyFailed);
            diagnostics.record_drop();
            failed += 1;
        }
    }
    while let Ok(envelope) = receiver.try_recv() {
        if !envelope.is_close() {
            diagnostics.record(LiveOutboundDeliveryState::TerminallyFailed);
            diagnostics.record_drop();
            failed += 1;
        }
    }
    failed
}

async fn supervise_socket(
    mut socket: GeminiSocket,
    api_key: String,
    config: LiveSessionConfig,
    mut receiver: mpsc::Receiver<OutboundEnvelope>,
    sender: mpsc::Sender<LiveServerEvent>,
    is_active: Arc<AtomicBool>,
    cancellation: CancellationToken,
    diagnostics: Arc<LiveOutboundDiagnosticsStore>,
) {
    let mut input_transcript = String::new();
    let mut output_transcript = String::new();
    let mut resume_handle = None;
    let mut pending_outbound: Option<OutboundEnvelope> = None;

    'connection: loop {
        if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() {
            break;
        }

        let action = if let Some(envelope) = pending_outbound.take() {
            let is_close = envelope.is_close();
            if socket.send(envelope.message.clone()).await.is_err() {
                if is_close {
                    is_active.store(false, Ordering::SeqCst);
                    break 'connection;
                }
                retain_for_retry(&mut pending_outbound, envelope, &diagnostics);
                ServerAction::Reconnect
            } else if is_close {
                is_active.store(false, Ordering::SeqCst);
                break 'connection;
            } else {
                diagnostics.record(LiveOutboundDeliveryState::Written);
                ServerAction::Continue
            }
        } else {
            tokio::select! {
                () = cancellation.cancelled() => break 'connection,
                outbound = receiver.recv() => {
                    match outbound {
                        Some(envelope) => {
                            let is_close = envelope.is_close();
                            if socket.send(envelope.message.clone()).await.is_err() {
                                if is_close {
                                    is_active.store(false, Ordering::SeqCst);
                                    break 'connection;
                                }
                                retain_for_retry(&mut pending_outbound, envelope, &diagnostics);
                                ServerAction::Reconnect
                            } else if is_close {
                                is_active.store(false, Ordering::SeqCst);
                                break 'connection;
                            } else {
                                diagnostics.record(LiveOutboundDeliveryState::Written);
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
                                    let failed = record_terminal_outbound_failures(
                                        &mut pending_outbound,
                                        &mut receiver,
                                        &diagnostics,
                                    );
                                    if failed > 0 {
                                        let _ = sender
                                            .send(LiveServerEvent::Error(ProviderError::from_kind(
                                                ProviderErrorKind::Closed,
                                            )))
                                            .await;
                                    } else {
                                        let _ = sender.send(LiveServerEvent::Closed).await;
                                    }
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
                            if error.retryable {
                                ServerAction::Reconnect
                            } else {
                                ServerAction::Terminal(error)
                            }
                        }
                        None => ServerAction::Reconnect,
                    }
                }
            }
        };

        match action {
            ServerAction::Continue => {}
            ServerAction::Terminal(error) => {
                record_terminal_outbound_failures(
                    &mut pending_outbound,
                    &mut receiver,
                    &diagnostics,
                );
                let _ = sender.send(LiveServerEvent::Error(error)).await;
                break;
            }
            ServerAction::Reconnect => {
                if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() {
                    break;
                }
                info!("Reconnecting Gemini Live session with bounded backoff");
                match reconnect(
                    &api_key,
                    &config,
                    resume_handle.as_deref(),
                    &is_active,
                    &cancellation,
                )
                .await
                {
                    Ok(new_socket) => {
                        socket = new_socket;
                        let _ = sender.send(LiveServerEvent::Connected).await;
                    }
                    Err(_error)
                        if !is_active.load(Ordering::SeqCst) || cancellation.is_cancelled() =>
                    {
                        break;
                    }
                    Err(error) => {
                        record_terminal_outbound_failures(
                            &mut pending_outbound,
                            &mut receiver,
                            &diagnostics,
                        );
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
    sender: mpsc::Sender<OutboundEnvelope>,
    is_active: Arc<AtomicBool>,
    cancellation: CancellationToken,
    diagnostics: Arc<LiveOutboundDiagnosticsStore>,
    sample_rate_in: u32,
}

impl GoogleLiveSession {
    /// `Ok(())` means the message was accepted into this bounded local outbound queue. It does
    /// not mean the WebSocket write completed, nor that Gemini acknowledged or processed it.
    async fn enqueue(&self, envelope: OutboundEnvelope) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) || self.cancellation.is_cancelled() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        self.sender
            .send(envelope)
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))?;
        self.diagnostics.record(LiveOutboundDeliveryState::Queued);
        Ok(())
    }
}

#[async_trait]
impl LiveSession for GoogleLiveSession {
    fn outbound_diagnostics(&self) -> Option<LiveOutboundDiagnostics> {
        Some(self.diagnostics.snapshot())
    }

    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        let message = encode_client_message(&audio_message(pcm_bytes, self.sample_rate_in))?;
        self.enqueue(OutboundEnvelope::new(message, OutboundKind::Audio))
            .await
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        let message = encode_client_message(&text_turn_message(text))?;
        self.enqueue(OutboundEnvelope::new(message, OutboundKind::TextTurn))
            .await
    }

    async fn send_tool_response(
        &mut self,
        response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        let message = encode_client_message(&tool_response_message(response))?;
        self.enqueue(OutboundEnvelope::new(message, OutboundKind::ToolResponse))
            .await
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        info!("Barge-in requested on Gemini Live session; server VAD owns interruption");
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        self.is_active.store(false, Ordering::SeqCst);
        let _ = self.sender.try_send(OutboundEnvelope::new(
            Message::Close(None),
            OutboundKind::Close,
        ));
        self.cancellation.cancel();
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
        let (out_tx, out_rx) = mpsc::channel::<OutboundEnvelope>(OUTBOUND_QUEUE_CAPACITY);
        let is_active = Arc::new(AtomicBool::new(true));
        let supervisor_active = is_active.clone();
        let supervisor_sender = event_sender.clone();
        let supervisor_key = self.auth.api_key.clone();
        let supervisor_config = config.clone();
        let cancellation = CancellationToken::new();
        let supervisor_cancellation = cancellation.clone();
        let diagnostics = Arc::new(LiveOutboundDiagnosticsStore::default());
        let supervisor_diagnostics = diagnostics.clone();
        tauri::async_runtime::spawn(async move {
            supervise_socket(
                socket,
                supervisor_key,
                supervisor_config,
                out_rx,
                supervisor_sender,
                supervisor_active,
                supervisor_cancellation,
                supervisor_diagnostics,
            )
            .await;
        });
        let _ = event_sender.send(LiveServerEvent::Connected).await;

        Ok(Box::new(GoogleLiveSession {
            sender: out_tx,
            is_active,
            cancellation,
            diagnostics,
            sample_rate_in: config.sample_rate_in,
        }))
    }
}

#[cfg(test)]
mod tests;
