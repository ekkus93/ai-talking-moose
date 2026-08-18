use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::*;
use crate::audio::playback::AudioPlayback;
use crate::character::state::CharacterState;
use crate::tools::router::ToolRouter;
use parking_lot::Mutex as SyncMutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct ConversationManager {
    active_session_id: Arc<SyncMutex<Option<String>>>,
    live_session: Arc<AsyncMutex<Option<Box<dyn LiveSession>>>>,
    is_in_conversation: Arc<AtomicBool>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            active_session_id: Arc::new(SyncMutex::new(None)),
            live_session: Arc::new(AsyncMutex::new(None)),
            is_in_conversation: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_in_conversation.load(Ordering::SeqCst)
    }

    pub fn current_session_id(&self) -> Option<String> {
        self.active_session_id.lock().clone()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_session(
        &self,
        provider: Arc<dyn RealtimeConversationProvider>,
        config: LiveSessionConfig,
        playback: Arc<AudioPlayback>,
        tool_router: Arc<ToolRouter>,
        state_callback: impl Fn(CharacterState) + Send + Sync + 'static,
        transcript_callback: impl Fn(String, String) + Send + Sync + 'static, // (role, text)
        speech_bubble_callback: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<String, String> {
        // End any prior session
        self.stop_session(playback.clone()).await;

        let session_id = Uuid::new_v4().to_string();
        *self.active_session_id.lock() = Some(session_id.clone());
        self.is_in_conversation.store(true, Ordering::SeqCst);

        state_callback(CharacterState::Listening);

        let (server_ev_tx, mut server_ev_rx) = mpsc::channel::<LiveServerEvent>(64);
        let session = provider.connect(config, server_ev_tx).await?;
        *self.live_session.lock().await = Some(session);

        let is_running = self.is_in_conversation.clone();
        let live_sess_ref = self.live_session.clone();
        let sess_id_clone = session_id.clone();

        tauri::async_runtime::spawn(async move {
            info!(
                "Conversation event processing loop started: {}",
                sess_id_clone
            );

            while let Some(event) = server_ev_rx.recv().await {
                if !is_running.load(Ordering::SeqCst) {
                    break;
                }

                match event {
                    LiveServerEvent::Connected => {
                        info!("Gemini Live session connected");
                    }
                    LiveServerEvent::UserTranscript(text) => {
                        info!("User said: {}", text);
                        transcript_callback("user".to_string(), text);
                        state_callback(CharacterState::Thinking);
                    }
                    LiveServerEvent::ModelTranscript(text) => {
                        info!("Moose said: {}", text);
                        transcript_callback("moose".to_string(), text.clone());
                        speech_bubble_callback(text);
                    }
                    LiveServerEvent::AudioData(pcm_bytes) => {
                        state_callback(CharacterState::Talking);
                        playback.enqueue_pcm_bytes(&pcm_bytes, 24000, 24000);
                    }
                    LiveServerEvent::Interrupted => {
                        info!("Interruption triggered! Barge-in active.");
                        playback.flush();
                        state_callback(CharacterState::Interrupted);
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        state_callback(CharacterState::Listening);
                    }
                    LiveServerEvent::TurnComplete => {
                        // After talking finishes, return to listening
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        if is_running.load(Ordering::SeqCst) {
                            state_callback(CharacterState::Listening);
                        }
                    }
                    LiveServerEvent::ToolCall { id, name, args } => {
                        info!("Handling tool call: {} (id: {})", name, id);
                        let router = tool_router.clone();
                        let live_ref = live_sess_ref.clone();

                        tauri::async_runtime::spawn(async move {
                            let result = router
                                .dispatch(&name, &args)
                                .await
                                .unwrap_or_else(|e| serde_json::json!({ "error": e }));

                            let mut sess_lock = live_ref.lock().await;
                            if let Some(ref mut sess) = *sess_lock {
                                let _ = sess
                                    .send_tool_response(ToolCallResponse { id, output: result })
                                    .await;
                            }
                        });
                    }
                    LiveServerEvent::Error(err) => {
                        error!("Live session error: {}", err);
                        state_callback(CharacterState::Error);
                    }
                    LiveServerEvent::Closed => {
                        info!("Live session closed by server");
                        break;
                    }
                }
            }

            info!("Conversation loop exited: {}", sess_id_clone);
        });

        Ok(session_id)
    }

    pub async fn send_audio_frame(&self, pcm_bytes: &[u8]) {
        let mut sess_lock = self.live_session.lock().await;
        if let Some(ref mut sess) = *sess_lock {
            if let Err(e) = sess.send_audio_chunk(pcm_bytes).await {
                warn!("Failed to stream audio frame: {}", e);
            }
        }
    }

    pub async fn barge_in(&self, playback: Arc<AudioPlayback>) {
        info!("Barge-in requested by user/client");
        playback.flush();
        let mut sess_lock = self.live_session.lock().await;
        if let Some(ref mut sess) = *sess_lock {
            let _ = sess.interrupt().await;
        }
    }

    pub async fn stop_session(&self, playback: Arc<AudioPlayback>) {
        self.is_in_conversation.store(false, Ordering::SeqCst);
        playback.flush();

        let mut sess_lock = self.live_session.lock().await;
        if let Some(mut sess) = sess_lock.take() {
            let _ = sess.close().await;
        }
        *self.active_session_id.lock() = None;
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}
