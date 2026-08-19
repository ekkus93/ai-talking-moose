use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::*;
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::character::state::CharacterState;
use crate::tools::router::ToolRouter;
use parking_lot::{Mutex as SyncMutex, RwLock};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationLifecycle {
    Idle,
    Connecting,
    Listening,
    Responding,
    Stopping,
    Failed,
}

pub struct ConversationCallbacks {
    state: Arc<dyn Fn(CharacterState) + Send + Sync>,
    lifecycle: Arc<dyn Fn(ConversationLifecycle) + Send + Sync>,
    transcript: Arc<dyn Fn(String, String, String) + Send + Sync>,
    speech_bubble: Arc<dyn Fn(String) + Send + Sync>,
    input_level: Arc<dyn Fn(f32) + Send + Sync>,
}

impl ConversationCallbacks {
    pub fn new<S, L, T, B, I>(
        state: S,
        lifecycle: L,
        transcript: T,
        speech_bubble: B,
        input_level: I,
    ) -> Self
    where
        S: Fn(CharacterState) + Send + Sync + 'static,
        L: Fn(ConversationLifecycle) + Send + Sync + 'static,
        T: Fn(String, String, String) + Send + Sync + 'static,
        B: Fn(String) + Send + Sync + 'static,
        I: Fn(f32) + Send + Sync + 'static,
    {
        Self {
            state: Arc::new(state),
            lifecycle: Arc::new(lifecycle),
            transcript: Arc::new(transcript),
            speech_bubble: Arc::new(speech_bubble),
            input_level: Arc::new(input_level),
        }
    }
}

pub struct ConversationStartRequest {
    pub provider: Arc<dyn RealtimeConversationProvider>,
    pub config: LiveSessionConfig,
    pub capture: Arc<SyncMutex<AudioCapture>>,
    pub input_device: Option<String>,
    pub playback: Arc<AudioPlayback>,
    pub output_device: Option<String>,
    pub muted: Arc<RwLock<bool>>,
    pub tool_router: Arc<ToolRouter>,
    pub callbacks: ConversationCallbacks,
}

pub struct ConversationManager {
    active_session_id: Arc<SyncMutex<Option<String>>>,
    live_session: Arc<AsyncMutex<Option<Box<dyn LiveSession>>>>,
    lifecycle: Arc<RwLock<ConversationLifecycle>>,
    is_in_conversation: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    operation_lock: AsyncMutex<()>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            active_session_id: Arc::new(SyncMutex::new(None)),
            live_session: Arc::new(AsyncMutex::new(None)),
            lifecycle: Arc::new(RwLock::new(ConversationLifecycle::Idle)),
            is_in_conversation: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            operation_lock: AsyncMutex::new(()),
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_in_conversation.load(Ordering::SeqCst)
    }

    pub fn lifecycle(&self) -> ConversationLifecycle {
        *self.lifecycle.read()
    }

    pub fn current_session_id(&self) -> Option<String> {
        if self.is_active() {
            self.active_session_id.lock().clone()
        } else {
            None
        }
    }

    fn set_lifecycle(
        lifecycle: &RwLock<ConversationLifecycle>,
        target: ConversationLifecycle,
        callback: Option<&Arc<dyn Fn(ConversationLifecycle) + Send + Sync>>,
    ) {
        *lifecycle.write() = target;
        if let Some(callback) = callback {
            callback(target);
        }
    }

    async fn close_provisional_session(session: &mut Box<dyn LiveSession>) {
        if let Err(error_value) = session.close().await {
            warn!(error = %error_value, "Failed to close provisional conversation session");
        }
    }

    async fn stop_session_locked(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
    ) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.is_in_conversation.store(false, Ordering::SeqCst);
        *self.lifecycle.write() = ConversationLifecycle::Stopping;

        capture.lock().stop();
        playback.flush();

        let mut session_lock = self.live_session.lock().await;
        if let Some(mut session) = session_lock.take() {
            if let Err(error_value) = session.close().await {
                warn!(error = %error_value, "Failed to close conversation session");
            }
        }
        *self.active_session_id.lock() = None;
        *self.lifecycle.write() = ConversationLifecycle::Idle;
    }

    pub async fn start_session(&self, request: ConversationStartRequest) -> Result<String, String> {
        let _operation_guard = self.operation_lock.lock().await;
        self.stop_session_locked(request.capture.clone(), request.playback.clone())
            .await;

        if *request.muted.read() {
            return Err("Moose is currently muted".to_string());
        }

        let ConversationStartRequest {
            provider,
            config,
            capture,
            input_device,
            playback,
            output_device,
            muted: _,
            tool_router,
            callbacks,
        } = request;
        let ConversationCallbacks {
            state: state_callback,
            lifecycle: lifecycle_callback,
            transcript: transcript_callback,
            speech_bubble: speech_bubble_callback,
            input_level: input_level_callback,
        } = callbacks;

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Connecting,
            Some(&lifecycle_callback),
        );

        let input_sample_rate = config.sample_rate_in;
        let output_sample_rate = config.sample_rate_out;
        let (server_ev_tx, mut server_ev_rx) = mpsc::channel::<LiveServerEvent>(64);
        let mut session = match provider.connect(config, server_ev_tx).await {
            Ok(session) => session,
            Err(error_value) => {
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                return Err(format!("conversation provider connection failed: {error_value}"));
            }
        };

        if let Err(error_value) = playback.start(output_device) {
            Self::close_provisional_session(&mut session).await;
            playback.flush();
            Self::set_lifecycle(
                &self.lifecycle,
                ConversationLifecycle::Failed,
                Some(&lifecycle_callback),
            );
            state_callback(CharacterState::Error);
            return Err(format!("failed to start audio output: {error_value}"));
        }

        let (pcm_tx, mut pcm_rx) = mpsc::channel::<Vec<u8>>(32);
        let (level_tx, mut level_rx) = mpsc::channel::<f32>(32);
        let capture_result =
            capture
                .lock()
                .start(input_device, input_sample_rate, pcm_tx, Some(level_tx));
        if let Err(error_value) = capture_result {
            playback.flush();
            Self::close_provisional_session(&mut session).await;
            Self::set_lifecycle(
                &self.lifecycle,
                ConversationLifecycle::Failed,
                Some(&lifecycle_callback),
            );
            state_callback(CharacterState::Error);
            return Err(format!("failed to start microphone: {error_value}"));
        }

        // The session becomes authoritative only after provider, output, and microphone
        // startup all succeed. Before this point `is_active()` remains false.
        let session_id = Uuid::new_v4().to_string();
        *self.live_session.lock().await = Some(session);
        *self.active_session_id.lock() = Some(session_id.clone());
        self.is_in_conversation.store(true, Ordering::SeqCst);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Listening,
            Some(&lifecycle_callback),
        );
        state_callback(CharacterState::Listening);

        let is_running_for_pcm = self.is_in_conversation.clone();
        let generation_for_pcm = self.generation.clone();
        let live_session_for_pcm = self.live_session.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(chunk) = pcm_rx.recv().await {
                if !is_running_for_pcm.load(Ordering::SeqCst)
                    || generation_for_pcm.load(Ordering::SeqCst) != generation
                {
                    break;
                }

                let mut session_lock = live_session_for_pcm.lock().await;
                if let Some(ref mut live_session) = *session_lock {
                    if let Err(error_value) = live_session.send_audio_chunk(&chunk).await {
                        warn!(error = %error_value, "Failed to stream microphone audio frame");
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        let is_running_for_level = self.is_in_conversation.clone();
        let generation_for_level = self.generation.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(level) = level_rx.recv().await {
                if !is_running_for_level.load(Ordering::SeqCst)
                    || generation_for_level.load(Ordering::SeqCst) != generation
                {
                    break;
                }
                input_level_callback(level);
            }
        });

        let is_running = self.is_in_conversation.clone();
        let generation_ref = self.generation.clone();
        let active_session_id = self.active_session_id.clone();
        let live_sess_ref = self.live_session.clone();
        let lifecycle_ref = self.lifecycle.clone();
        let capture_ref = capture;
        let sess_id_clone = session_id.clone();

        tauri::async_runtime::spawn(async move {
            info!(session_id = %sess_id_clone, "Conversation event loop started");
            let mut terminal_error: Option<String> = None;

            while let Some(event) = server_ev_rx.recv().await {
                if !is_running.load(Ordering::SeqCst)
                    || generation_ref.load(Ordering::SeqCst) != generation
                {
                    break;
                }

                match event {
                    LiveServerEvent::Connected => {
                        info!(session_id = %sess_id_clone, "Conversation provider connected");
                    }
                    LiveServerEvent::UserTranscript(text) => {
                        transcript_callback(
                            sess_id_clone.clone(),
                            "user".to_string(),
                            text,
                        );
                        Self::set_lifecycle(
                            &lifecycle_ref,
                            ConversationLifecycle::Responding,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Thinking);
                    }
                    LiveServerEvent::ModelTranscript(text) => {
                        transcript_callback(
                            sess_id_clone.clone(),
                            "moose".to_string(),
                            text.clone(),
                        );
                        speech_bubble_callback(text);
                    }
                    LiveServerEvent::AudioData(pcm_bytes) => {
                        Self::set_lifecycle(
                            &lifecycle_ref,
                            ConversationLifecycle::Responding,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Talking);
                        match playback.enqueue_pcm_bytes(&pcm_bytes, output_sample_rate) {
                            Ok(report) if report.dropped_samples > 0 => {
                                warn!(
                                    dropped_samples = report.dropped_samples,
                                    "Conversation playback queue overflowed"
                                );
                            }
                            Ok(_) => {}
                            Err(error_value) => {
                                terminal_error = Some(format!(
                                    "failed to enqueue conversation audio: {error_value}"
                                ));
                                break;
                            }
                        }
                    }
                    LiveServerEvent::Interrupted => {
                        playback.flush();
                        state_callback(CharacterState::Interrupted);
                        Self::set_lifecycle(
                            &lifecycle_ref,
                            ConversationLifecycle::Listening,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Listening);
                    }
                    LiveServerEvent::TurnComplete => {
                        if is_running.load(Ordering::SeqCst)
                            && generation_ref.load(Ordering::SeqCst) == generation
                        {
                            Self::set_lifecycle(
                                &lifecycle_ref,
                                ConversationLifecycle::Listening,
                                Some(&lifecycle_callback),
                            );
                            state_callback(CharacterState::Listening);
                        }
                    }
                    LiveServerEvent::ToolCall { id, name, args } => {
                        info!(tool_name = %name, "Handling model tool call");
                        let router = tool_router.clone();
                        let live_ref = live_sess_ref.clone();
                        let generation_for_tool = generation_ref.clone();

                        tauri::async_runtime::spawn(async move {
                            let result = router
                                .dispatch(&name, &args)
                                .await
                                .unwrap_or_else(|error_value| {
                                    serde_json::json!({ "error": error_value })
                                });

                            if generation_for_tool.load(Ordering::SeqCst) != generation {
                                return;
                            }
                            let mut session_lock = live_ref.lock().await;
                            if let Some(ref mut live_session) = *session_lock {
                                let _ = live_session
                                    .send_tool_response(ToolCallResponse { id, output: result })
                                    .await;
                            }
                        });
                    }
                    LiveServerEvent::Error(error_value) => {
                        terminal_error = Some(error_value);
                        break;
                    }
                    LiveServerEvent::Closed => break,
                }
            }

            // An obsolete event loop must never tear down a newer session.
            if generation_ref.load(Ordering::SeqCst) != generation {
                return;
            }

            is_running.store(false, Ordering::SeqCst);
            capture_ref.lock().stop();
            playback.flush();

            let mut session_lock = live_sess_ref.lock().await;
            if let Some(mut live_session) = session_lock.take() {
                let _ = live_session.close().await;
            }
            *active_session_id.lock() = None;

            if let Some(error_value) = terminal_error {
                error!(error = %error_value, "Conversation session terminated with an error");
                Self::set_lifecycle(
                    &lifecycle_ref,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
            } else {
                Self::set_lifecycle(
                    &lifecycle_ref,
                    ConversationLifecycle::Idle,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Idle);
            }

            info!(session_id = %sess_id_clone, "Conversation event loop exited");
        });

        Ok(session_id)
    }

    pub async fn barge_in(&self, playback: Arc<AudioPlayback>) -> Result<(), String> {
        if !self.is_active() {
            return Ok(());
        }

        playback.flush();
        let mut session_lock = self.live_session.lock().await;
        if let Some(ref mut session) = *session_lock {
            session.interrupt().await?;
        }
        Ok(())
    }

    pub async fn stop_session(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
    ) {
        let _operation_guard = self.operation_lock.lock().await;
        self.stop_session_locked(capture, playback).await;
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct FailingProvider;

    #[async_trait]
    impl RealtimeConversationProvider for FailingProvider {
        async fn connect(
            &self,
            _config: LiveSessionConfig,
            _event_sender: mpsc::Sender<LiveServerEvent>,
        ) -> Result<Box<dyn LiveSession>, String> {
            Err("injected connect failure".to_string())
        }
    }

    fn test_tool_router() -> Arc<ToolRouter> {
        let settings = Arc::new(RwLock::new(crate::app::state::AppSettings::default()));
        let memory = Arc::new(crate::memory::MemoryManager::new(Arc::new(
            crate::persistence::Database::new_in_memory().unwrap(),
        )));
        Arc::new(ToolRouter::new(Arc::new(
            crate::tools::builtin::BuiltinTools {
                memory_manager: memory,
                character_config: crate::character::personality::CharacterConfig::default(),
                settings,
            },
        )))
    }

    fn test_request(muted: bool) -> ConversationStartRequest {
        ConversationStartRequest {
            provider: Arc::new(FailingProvider),
            config: LiveSessionConfig {
                model: "fake".to_string(),
                voice_name: None,
                system_instruction: None,
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
            },
            capture: Arc::new(SyncMutex::new(AudioCapture::new_mock())),
            input_device: None,
            playback: Arc::new(AudioPlayback::new()),
            output_device: None,
            muted: Arc::new(RwLock::new(muted)),
            tool_router: test_tool_router(),
            callbacks: ConversationCallbacks::new(|_| {}, |_| {}, |_, _, _| {}, |_| {}, |_| {}),
        }
    }

    #[tokio::test]
    async fn failed_provider_connect_never_becomes_active() {
        let manager = ConversationManager::new();
        let result = manager.start_session(test_request(false)).await;

        assert!(result.is_err());
        assert!(!manager.is_active());
        assert_eq!(manager.current_session_id(), None);
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
    }

    #[tokio::test]
    async fn muted_state_blocks_transactional_start_inside_operation_lock() {
        let manager = ConversationManager::new();
        let result = manager.start_session(test_request(true)).await;

        assert_eq!(result.unwrap_err(), "Moose is currently muted");
        assert!(!manager.is_active());
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    }
}
