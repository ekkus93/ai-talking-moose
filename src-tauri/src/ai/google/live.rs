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
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

pub struct GoogleLiveProvider {
    auth: GoogleAuth,
}

impl GoogleLiveProvider {
    pub fn new(auth: GoogleAuth) -> Self {
        Self { auth }
    }
}

pub struct GoogleLiveSession {
    sender: mpsc::Sender<Message>,
    is_active: Arc<AtomicBool>,
}

#[async_trait]
impl LiveSession for GoogleLiveSession {
    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), String> {
        if !self.is_active.load(Ordering::SeqCst) {
            return Ok(());
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
            .map_err(|e| format!("Failed to send audio chunk: {}", e))
    }

    async fn send_tool_response(&mut self, response: ToolCallResponse) -> Result<(), String> {
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
            .map_err(|e| format!("Failed to send tool response: {}", e))
    }

    async fn interrupt(&mut self) -> Result<(), String> {
        info!("Barge-in / Interrupt requested on Gemini Live session");
        Ok(())
    }

    async fn close(&mut self) -> Result<(), String> {
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
    ) -> Result<Box<dyn LiveSession>, String> {
        if !self.auth.is_valid() {
            return Err("Google API key is missing".to_string());
        }

        let url = format!(
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}",
            self.auth.api_key
        );

        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| {
                format!(
                    "WebSocket connection failed: {}",
                    self.auth.redact(&e.to_string())
                )
            })?;

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
            .map_err(|e| {
                format!(
                    "Failed to send setup message: {}",
                    self.auth.redact(&e.to_string())
                )
            })?;

        // 2. Outgoing forwarder task
        tauri::async_runtime::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    warn!("WebSocket send error: {}", e);
                    break;
                }
            }
        });

        // 3. Incoming receiver task
        let ev_tx = event_sender.clone();
        let auth_for_logs = self.auth.clone();
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
                        let _ = ev_tx.send(LiveServerEvent::Closed).await;
                        break;
                    }
                    Err(e) => {
                        let safe_error = auth_for_logs.redact(&e.to_string());
                        error!("WebSocket error in Gemini Live stream: {}", safe_error);
                        let _ = ev_tx.send(LiveServerEvent::Error(safe_error)).await;
                        break;
                    }
                    _ => None,
                };

                if let Some(txt) = json_text {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                        if let Some(content) = val.get("serverContent") {
                            if let Some(interrupted) =
                                content.get("interrupted").and_then(|v| v.as_bool())
                            {
                                if interrupted {
                                    let _ = ev_tx.send(LiveServerEvent::Interrupted).await;
                                }
                            }

                            if let Some(parts) = content
                                .get("modelTurn")
                                .and_then(|mt| mt.get("parts"))
                                .and_then(|p| p.as_array())
                            {
                                for part in parts {
                                    let is_thought = part
                                        .get("thought")
                                        .and_then(|t| t.as_bool())
                                        .unwrap_or(false);
                                    if is_thought {
                                        continue;
                                    }

                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        let _ = ev_tx
                                            .send(LiveServerEvent::ModelTranscript(
                                                text.to_string(),
                                            ))
                                            .await;
                                    }
                                    if let Some(b64) = part
                                        .get("inlineData")
                                        .and_then(|id| id.get("data"))
                                        .and_then(|d| d.as_str())
                                    {
                                        if let Ok(pcm_bytes) =
                                            base64::engine::general_purpose::STANDARD.decode(b64)
                                        {
                                            info!(
                                                "Received model audio chunk: {} bytes",
                                                pcm_bytes.len()
                                            );
                                            let _ = ev_tx
                                                .send(LiveServerEvent::AudioData(pcm_bytes))
                                                .await;
                                        }
                                    }
                                }
                            }

                            if let Some(true) =
                                content.get("turnComplete").and_then(|tc| tc.as_bool())
                            {
                                let _ = ev_tx.send(LiveServerEvent::TurnComplete).await;
                            }
                        }

                        if let Some(tool_call) = val.get("toolCall") {
                            if let Some(calls) =
                                tool_call.get("functionCalls").and_then(|fc| fc.as_array())
                            {
                                for call in calls {
                                    let id = call
                                        .get("id")
                                        .and_then(|i| i.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = call
                                        .get("name")
                                        .and_then(|n| n.as_str())
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
            }

            is_active_clone.store(false, Ordering::SeqCst);
        });

        Ok(Box::new(GoogleLiveSession {
            sender: out_tx,
            is_active,
        }))
    }
}
