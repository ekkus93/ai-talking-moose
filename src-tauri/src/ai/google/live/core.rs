use crate::ai::google::auth::{trace_google_provider_failure, GoogleAuth};
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
use reqwest::Url;
use serde_json::json;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use tracing::info;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const SETUP_TIMEOUT: Duration = Duration::from_secs(8);
const SOCKET_WRITE_TIMEOUT: Duration = Duration::from_secs(8);
const CLOSE_FRAME_TIMEOUT: Duration = Duration::from_secs(2);
const CLOSE_COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_REQUEST_TIMEOUT: Duration = Duration::from_millis(250);
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

#[derive(Debug, Clone)]
struct OutboundEnvelope {
    message: Message,
}

impl OutboundEnvelope {
    fn new(message: Message) -> Self {
        Self { message }
    }
}

struct CloseRequest {
    completion: oneshot::Sender<Result<(), ProviderError>>,
}

impl CloseRequest {
    fn complete(self, result: Result<(), ProviderError>) {
        let _ = self.completion.send(result);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteWaitError {
    Cancelled,
    TimedOut,
}

async fn await_bounded_write<E, F>(
    write: F,
    cancellation: Option<&CancellationToken>,
    timeout: Duration,
) -> Result<Result<(), E>, WriteWaitError>
where
    F: Future<Output = Result<(), E>>,
{
    if let Some(cancellation) = cancellation {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(WriteWaitError::Cancelled),
            result = tokio::time::timeout(timeout, write) => {
                result.map_err(|_| WriteWaitError::TimedOut)
            }
        }
    } else {
        tokio::time::timeout(timeout, write)
            .await
            .map_err(|_| WriteWaitError::TimedOut)
    }
}

async fn write_socket_message(
    socket: &mut GeminiSocket,
    message: Message,
    cancellation: Option<&CancellationToken>,
    timeout: Duration,
) -> Result<(), ProviderError> {
    match await_bounded_write(socket.send(message), cancellation, timeout).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            let provider_error = provider_error_for_connect(error);
            trace_live_provider_failure(&provider_error);
            Err(provider_error)
        }
        Err(WriteWaitError::Cancelled) => Err(ProviderError::from_kind(ProviderErrorKind::Closed)),
        Err(WriteWaitError::TimedOut) => Err(ProviderError::from_kind(ProviderErrorKind::Network)),
    }
}

async fn complete_protocol_close(socket: &mut GeminiSocket, request: CloseRequest) {
    let result =
        write_socket_message(socket, Message::Close(None), None, CLOSE_FRAME_TIMEOUT).await;
    request.complete(result);
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
