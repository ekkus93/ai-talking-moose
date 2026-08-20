use crate::ai::google::auth::GoogleAuth;
use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::*;
use async_trait::async_trait;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tracing::{error, info, warn};

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

fn provider_error_from_server_payload(value: &serde_json::Value) -> Option<ProviderError> {
    let error = value.get("error")?;
    let code = error.get("code").and_then(serde_json::Value::as_u64);
    let status = error
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

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

    Some(ProviderError::from_kind(kind))
}

pub struct GoogleLiveSession {
    sender: mpsc::Sender<Message>,
    is_active: Arc<AtomicBool>,
}

fn text_turn_message(text: &str) -> serde_json::Value {
    json!({
        "realtimeInput": {
            "text": text
        }
    })
}

#[async_trait]
impl LiveSession for GoogleLiveSession {
    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }

        let b64 = base64::engine::general_purpose::STANDARD.encode(pcm_bytes);
        let msg = json!({
            "realtimeInput": {
                "mediaChunks": [
                    {
                        "mimeType": "audio/pcm;rate=16000",
                        "data": b64
                    }
                ]
            }
        });

        self.sender
            .send(Message::Text(msg.to_string()))
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        if !self.is_active.load(Ordering::SeqCst) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }

        self.sender
            .send(Message::Text(text_turn_message(text).to_string()))
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

        let msg = json!({
            "toolResponse": {
                "functionResponses": [
                    {
                        "id": response.id,
                        "response": response.output
                    }
                ]
            }
        });

        self.sender
            .send(Message::Text(msg.to_string()))
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        info!("Barge-in / Interrupt requested on Gemini Live session");
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

        let url = format!(
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}",
            self.auth.api_key
        );

        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(provider_error_for_connect)?;

        let (mut write, mut read) = ws_stream.split();
        let (out_tx, mut out_rx) = mpsc::channel::<Message>(64);

        let is_active = Arc::new(AtomicBool::new(true));
        let is_active_clone = is_active.clone();

        // 1. Send Setup message
        let voice_name = config.voice_name.unwrap_or_else(|| "Puck".to_string());
        let model_name = if config.model.is_empty() || config.model.contains("gemini-2.0") {
            "models/gemini-2.5-flash-native-audio-latest".to_string()
        } else if config.model.starts_with("models/") {
            config.model.clone()
        } else {
            format!("models/{}", config.model)
        };

        let setup_msg = json!({
            "setup": {
                "model": model_name,
                "generationConfig": {
                    "responseModalities": ["AUDIO"],
                    "speechConfig": {
                        "voiceConfig": {
                            "prebuiltVoiceConfig": {
                                "voiceName": voice_name
                            }
                        }
                    }
                },
                "systemInstruction": {
                    "parts": [
                        { "text": config.system_instruction.unwrap_or_default() }
                    ]
                }
            }
        });

        info!("Sending Gemini Live setup frame");

        write
            .send(Message::Text(setup_msg.to_string()))
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?;

        // 2. Outgoing forwarder task
        let send_error_tx = event_sender.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if write.send(msg).await.is_err() {
                    warn!("Gemini Live WebSocket send failed");
                    let _ = send_error_tx
                        .send(LiveServerEvent::Error(ProviderError::from_kind(
                            ProviderErrorKind::Network,
                        )))
                        .await;
                    break;
                }
            }
        });

        // 3. Incoming receiver task
        let ev_tx = event_sender.clone();
        tauri::async_runtime::spawn(async move {
            let _ = ev_tx.send(LiveServerEvent::Connected).await;

            while let Some(msg_result) = read.next().await {
                if !is_active_clone.load(Ordering::SeqCst) {
                    break;
                }

                let json_text = match msg_result {
                    Ok(Message::Text(txt)) => Some(txt),
                    Ok(Message::Binary(bin)) => String::from_utf8(bin).ok(),
                    Ok(Message::Close(frame)) => {
                        let code = frame.as_ref().map(|value| u16::from(value.code));
                        info!(?code, "Gemini Live WebSocket closed by server");
                        if matches!(code, None | Some(1000 | 1001)) {
                            let _ = ev_tx.send(LiveServerEvent::Closed).await;
                        } else {
                            let _ = ev_tx
                                .send(LiveServerEvent::Error(ProviderError::from_kind(
                                    ProviderErrorKind::Closed,
                                )))
                                .await;
                        }
                        break;
                    }
                    Err(_) => {
                        error!("Gemini Live WebSocket receive failed");
                        let _ = ev_tx
                            .send(LiveServerEvent::Error(ProviderError::from_kind(
                                ProviderErrorKind::Network,
                            )))
                            .await;
                        break;
                    }
                    _ => None,
                };

                if let Some(txt) = json_text {
                    let val = match serde_json::from_str::<serde_json::Value>(&txt) {
                        Ok(value) => value,
                        Err(_) => {
                            let _ = ev_tx
                                .send(LiveServerEvent::Error(ProviderError::from_kind(
                                    ProviderErrorKind::Protocol,
                                )))
                                .await;
                            break;
                        }
                    };

                    if let Some(provider_error) = provider_error_from_server_payload(&val) {
                        let _ = ev_tx.send(LiveServerEvent::Error(provider_error)).await;
                        break;
                    }

                    if let Some(content) = val.get("serverContent") {
                        if let Some(interrupted) =
                            content.get("interrupted").and_then(|value| value.as_bool())
                        {
                            if interrupted {
                                let _ = ev_tx.send(LiveServerEvent::Interrupted).await;
                            }
                        }

                        if let Some(parts) = content
                            .get("modelTurn")
                            .and_then(|model_turn| model_turn.get("parts"))
                            .and_then(|parts| parts.as_array())
                        {
                            for part in parts {
                                let is_thought = part
                                    .get("thought")
                                    .and_then(|thought| thought.as_bool())
                                    .unwrap_or(false);
                                if is_thought {
                                    continue;
                                }

                                if let Some(text) = part.get("text").and_then(|text| text.as_str())
                                {
                                    let _ = ev_tx
                                        .send(LiveServerEvent::ModelTranscript(text.to_string()))
                                        .await;
                                }
                                if let Some(b64) = part
                                    .get("inlineData")
                                    .and_then(|inline_data| inline_data.get("data"))
                                    .and_then(|data| data.as_str())
                                {
                                    if let Ok(pcm_bytes) =
                                        base64::engine::general_purpose::STANDARD.decode(b64)
                                    {
                                        info!(
                                            "Received model audio chunk: {} bytes",
                                            pcm_bytes.len()
                                        );
                                        let _ =
                                            ev_tx.send(LiveServerEvent::AudioData(pcm_bytes)).await;
                                    }
                                }
                            }
                        }

                        if let Some(true) = content
                            .get("turnComplete")
                            .and_then(|complete| complete.as_bool())
                        {
                            let _ = ev_tx.send(LiveServerEvent::TurnComplete).await;
                        }
                    }

                    if let Some(tool_call) = val.get("toolCall") {
                        if let Some(calls) = tool_call
                            .get("functionCalls")
                            .and_then(|calls| calls.as_array())
                        {
                            for call in calls {
                                let id = call
                                    .get("id")
                                    .and_then(|id| id.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = call
                                    .get("name")
                                    .and_then(|name| name.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let args =
                                    call.get("args").cloned().unwrap_or(serde_json::json!({}));
                                let _ = ev_tx
                                    .send(LiveServerEvent::ToolCall { id, name, args })
                                    .await;
                            }
                        }
                    }
                }
            }

            is_active_clone.store(false, Ordering::SeqCst);
        });

        Ok(Box::new(GoogleLiveSession {
            sender: out_tx,
            is_active,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_statuses_map_to_stable_provider_categories() {
        assert_eq!(
            provider_error_for_http_status(401).kind,
            ProviderErrorKind::Auth
        );
        assert_eq!(
            provider_error_for_http_status(429).kind,
            ProviderErrorKind::Quota
        );
        assert_eq!(
            provider_error_for_http_status(404).kind,
            ProviderErrorKind::Model
        );
        assert_eq!(
            provider_error_for_http_status(400).kind,
            ProviderErrorKind::Setup
        );
        assert_eq!(
            provider_error_for_http_status(503).kind,
            ProviderErrorKind::Network
        );
    }

    #[test]
    fn server_error_payload_is_classified_without_copying_private_message() {
        let private = "PRIVATE_TRANSCRIPT api-key-should-never-escape";
        let value = serde_json::json!({
            "error": {
                "code": 429,
                "status": "RESOURCE_EXHAUSTED",
                "message": private
            }
        });

        let error = provider_error_from_server_payload(&value).unwrap();
        assert_eq!(error.kind, ProviderErrorKind::Quota);
        assert!(!error.message.contains(private));
        assert!(!error.message.contains("api-key-should-never-escape"));
    }

    #[test]
    fn unknown_server_error_is_protocol_error_without_raw_payload() {
        let value = serde_json::json!({
            "error": {
                "code": 499,
                "status": "SOMETHING_NEW",
                "message": "sensitive provider detail"
            }
        });

        let error = provider_error_from_server_payload(&value).unwrap();
        assert_eq!(error.kind, ProviderErrorKind::Protocol);
        assert!(!error.message.contains("sensitive provider detail"));
    }

    #[test]
    fn text_turn_uses_realtime_input_without_audio_payload() {
        let message = text_turn_message("hello from Moonshine");
        assert_eq!(
            message,
            serde_json::json!({
                "realtimeInput": {
                    "text": "hello from Moonshine"
                }
            })
        );
        assert!(message["realtimeInput"].get("mediaChunks").is_none());
        assert!(message["realtimeInput"].get("audio").is_none());
        assert!(message["realtimeInput"].get("video").is_none());
    }

    #[tokio::test]
    async fn missing_api_key_fails_before_network_with_auth_category() {
        let provider = GoogleLiveProvider::new(GoogleAuth::new(String::new()));
        let (event_tx, _event_rx) = mpsc::channel(1);
        let error = provider
            .connect(
                LiveSessionConfig {
                    model: "test-model".to_string(),
                    voice_name: None,
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                },
                event_tx,
            )
            .await
            .err()
            .expect("missing credentials must fail before network access");

        assert_eq!(error.kind, ProviderErrorKind::Auth);
        assert!(!error.retryable);
    }
}
