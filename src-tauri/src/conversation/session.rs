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

type StateCallback = Arc<dyn Fn(CharacterState) + Send + Sync>;
type LifecycleCallback = Arc<dyn Fn(ConversationLifecycle) + Send + Sync>;
type TranscriptCallback = Arc<dyn Fn(String, String, String) + Send + Sync>;
type SpeechBubbleCallback = Arc<dyn Fn(String) + Send + Sync>;
type InputLevelCallback = Arc<dyn Fn(f32) + Send + Sync>;

impl ConversationLifecycle {
    pub fn can_transition_to(self, target: Self) -> bool {
        if self == target {
            return true;
        }

        matches!(
            (self, target),
            (Self::Idle, Self::Connecting | Self::Stopping)
                | (
                    Self::Connecting,
                    Self::Listening | Self::Stopping | Self::Failed
                )
                | (
                    Self::Listening,
                    Self::Responding | Self::Stopping | Self::Failed
                )
                | (
                    Self::Responding,
                    Self::Listening | Self::Stopping | Self::Failed
                )
                | (Self::Stopping, Self::Idle | Self::Failed)
                | (Self::Failed, Self::Stopping | Self::Idle)
        )
    }
}

pub struct ConversationCallbacks {
    state: StateCallback,
    lifecycle: LifecycleCallback,
    transcript: TranscriptCallback,
    speech_bubble: SpeechBubbleCallback,
    input_level: InputLevelCallback,
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

struct ConversationEventLoopContext {
    generation: u64,
    session_id: String,
    capture: Arc<SyncMutex<AudioCapture>>,
    playback: Arc<AudioPlayback>,
    output_sample_rate: u32,
    tool_router: Arc<ToolRouter>,
    state_callback: StateCallback,
    lifecycle_callback: LifecycleCallback,
    transcript_callback: TranscriptCallback,
    speech_bubble_callback: SpeechBubbleCallback,
}

#[derive(Clone)]
pub struct ConversationManager {
    active_session_id: Arc<SyncMutex<Option<String>>>,
    live_session: Arc<AsyncMutex<Option<Box<dyn LiveSession>>>>,
    lifecycle: Arc<RwLock<ConversationLifecycle>>,
    is_in_conversation: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    output_suppressed: Arc<AtomicBool>,
    lifecycle_callback: Arc<SyncMutex<Option<LifecycleCallback>>>,
    operation_lock: Arc<AsyncMutex<()>>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            active_session_id: Arc::new(SyncMutex::new(None)),
            live_session: Arc::new(AsyncMutex::new(None)),
            lifecycle: Arc::new(RwLock::new(ConversationLifecycle::Idle)),
            is_in_conversation: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            output_suppressed: Arc::new(AtomicBool::new(false)),
            lifecycle_callback: Arc::new(SyncMutex::new(None)),
            operation_lock: Arc::new(AsyncMutex::new(())),
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
        callback: Option<&LifecycleCallback>,
    ) {
        let mut current = lifecycle.write();
        let previous = *current;
        if !previous.can_transition_to(target) {
            warn!(
                ?previous,
                ?target,
                "Rejected invalid conversation lifecycle transition"
            );
            return;
        }

        *current = target;
        drop(current);
        if let Some(callback) = callback {
            callback(target);
        }
    }

    async fn close_provisional_session(session: &mut Box<dyn LiveSession>) {
        if let Err(error_value) = session.close().await {
            warn!(error = %error_value, "Failed to close provisional conversation session");
        }
    }

    async fn shutdown_locked(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        final_lifecycle: ConversationLifecycle,
    ) {
        let lifecycle_callback = self.lifecycle_callback.lock().clone();
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.is_in_conversation.store(false, Ordering::SeqCst);
        self.output_suppressed.store(false, Ordering::SeqCst);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Stopping,
            lifecycle_callback.as_ref(),
        );

        capture.lock().stop();
        playback.flush();

        let mut session_lock = self.live_session.lock().await;
        if let Some(mut session) = session_lock.take() {
            if let Err(error_value) = session.close().await {
                warn!(error = %error_value, "Failed to close conversation session");
            }
        }
        *self.active_session_id.lock() = None;
        Self::set_lifecycle(
            &self.lifecycle,
            final_lifecycle,
            lifecycle_callback.as_ref(),
        );
        *self.lifecycle_callback.lock() = None;
    }

    async fn shutdown_if_generation_current(
        &self,
        expected_generation: u64,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        final_lifecycle: ConversationLifecycle,
    ) -> bool {
        let _operation_guard = self.operation_lock.lock().await;
        if self.generation.load(Ordering::SeqCst) != expected_generation {
            return false;
        }

        self.shutdown_locked(capture, playback, final_lifecycle)
            .await;
        true
    }

    pub async fn start_session(&self, request: ConversationStartRequest) -> Result<String, String> {
        let _operation_guard = self.operation_lock.lock().await;
        self.shutdown_locked(
            request.capture.clone(),
            request.playback.clone(),
            ConversationLifecycle::Idle,
        )
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
        self.output_suppressed.store(false, Ordering::SeqCst);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Connecting,
            Some(&lifecycle_callback),
        );

        let input_sample_rate = config.sample_rate_in;
        let output_sample_rate = config.sample_rate_out;
        let (server_ev_tx, server_ev_rx) = mpsc::channel::<LiveServerEvent>(64);
        let mut session = match provider.connect(config, server_ev_tx).await {
            Ok(session) => session,
            Err(error_value) => {
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                return Err(format!(
                    "conversation provider connection failed: {error_value}"
                ));
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

        let session_id = Uuid::new_v4().to_string();
        *self.live_session.lock().await = Some(session);
        *self.active_session_id.lock() = Some(session_id.clone());
        *self.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
        self.is_in_conversation.store(true, Ordering::SeqCst);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Listening,
            Some(&lifecycle_callback),
        );
        state_callback(CharacterState::Listening);

        let manager_for_pcm = self.clone();
        let capture_for_pcm = capture.clone();
        let playback_for_pcm = playback.clone();
        let state_for_pcm = state_callback.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(chunk) = pcm_rx.recv().await {
                if !manager_for_pcm.is_in_conversation.load(Ordering::SeqCst)
                    || manager_for_pcm.generation.load(Ordering::SeqCst) != generation
                {
                    break;
                }

                let send_failed = {
                    let mut session_lock = manager_for_pcm.live_session.lock().await;
                    match session_lock.as_mut() {
                        Some(live_session) => match live_session.send_audio_chunk(&chunk).await {
                            Ok(()) => false,
                            Err(error_value) => {
                                warn!(
                                    error = %error_value,
                                    "Failed to stream microphone audio frame"
                                );
                                true
                            }
                        },
                        None => true,
                    }
                };

                if send_failed {
                    let cleaned = manager_for_pcm
                        .shutdown_if_generation_current(
                            generation,
                            capture_for_pcm.clone(),
                            playback_for_pcm.clone(),
                            ConversationLifecycle::Failed,
                        )
                        .await;
                    if cleaned {
                        state_for_pcm(CharacterState::Error);
                    }
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

        let manager = self.clone();
        let event_loop_context = ConversationEventLoopContext {
            generation,
            session_id: session_id.clone(),
            capture,
            playback,
            output_sample_rate,
            tool_router,
            state_callback,
            lifecycle_callback,
            transcript_callback,
            speech_bubble_callback,
        };
        tauri::async_runtime::spawn(async move {
            manager
                .run_event_loop(server_ev_rx, event_loop_context)
                .await;
        });

        Ok(session_id)
    }

    async fn run_event_loop(
        &self,
        mut server_ev_rx: mpsc::Receiver<LiveServerEvent>,
        context: ConversationEventLoopContext,
    ) {
        let ConversationEventLoopContext {
            generation,
            session_id,
            capture,
            playback,
            output_sample_rate,
            tool_router,
            state_callback,
            lifecycle_callback,
            transcript_callback,
            speech_bubble_callback,
        } = context;
        info!(session_id = %session_id, "Conversation event loop started");
        let mut terminal_failed = false;

        while let Some(event) = server_ev_rx.recv().await {
            if !self.is_in_conversation.load(Ordering::SeqCst)
                || self.generation.load(Ordering::SeqCst) != generation
            {
                break;
            }

            if self.should_suppress_interrupted_response_event(&event) {
                continue;
            }

            match event {
                LiveServerEvent::Connected => {
                    info!(session_id = %session_id, "Conversation provider connected");
                }
                LiveServerEvent::UserTranscript(text) => {
                    self.output_suppressed.store(false, Ordering::SeqCst);
                    transcript_callback(session_id.clone(), "user".to_string(), text);
                    Self::set_lifecycle(
                        &self.lifecycle,
                        ConversationLifecycle::Responding,
                        Some(&lifecycle_callback),
                    );
                    state_callback(CharacterState::Thinking);
                }
                LiveServerEvent::ModelTranscript(text) => {
                    transcript_callback(session_id.clone(), "moose".to_string(), text.clone());
                    speech_bubble_callback(text);
                }
                LiveServerEvent::AudioData(pcm_bytes) => {
                    Self::set_lifecycle(
                        &self.lifecycle,
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
                        Err(_error_value) => {
                            terminal_failed = true;
                            break;
                        }
                    }
                }
                LiveServerEvent::Interrupted => {
                    playback.flush();
                    self.output_suppressed.store(false, Ordering::SeqCst);
                    state_callback(CharacterState::Interrupted);
                    Self::set_lifecycle(
                        &self.lifecycle,
                        ConversationLifecycle::Listening,
                        Some(&lifecycle_callback),
                    );
                    state_callback(CharacterState::Listening);
                }
                LiveServerEvent::TurnComplete => {
                    if self.is_in_conversation.load(Ordering::SeqCst)
                        && self.generation.load(Ordering::SeqCst) == generation
                    {
                        Self::set_lifecycle(
                            &self.lifecycle,
                            ConversationLifecycle::Listening,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Listening);
                    }
                }
                LiveServerEvent::ToolCall { id, name, args } => {
                    info!(tool_name = %name, "Handling model tool call");
                    let router = tool_router.clone();
                    let live_ref = self.live_session.clone();
                    let generation_ref = self.generation.clone();

                    tauri::async_runtime::spawn(async move {
                        let result = router.dispatch(&name, &args).await.unwrap_or_else(
                            |error_value| serde_json::json!({ "error": error_value }),
                        );

                        if generation_ref.load(Ordering::SeqCst) != generation {
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
                LiveServerEvent::Error(_error_value) => {
                    terminal_failed = true;
                    break;
                }
                LiveServerEvent::Closed => break,
            }
        }

        let final_lifecycle = if terminal_failed {
            ConversationLifecycle::Failed
        } else {
            ConversationLifecycle::Idle
        };
        let cleaned = self
            .shutdown_if_generation_current(generation, capture, playback, final_lifecycle)
            .await;

        if cleaned {
            if terminal_failed {
                error!("Conversation session terminated with a provider/audio error");
                state_callback(CharacterState::Error);
            } else {
                state_callback(CharacterState::Idle);
            }
            info!(session_id = %session_id, "Conversation event loop exited");
        }
    }

    fn should_suppress_interrupted_response_event(&self, event: &LiveServerEvent) -> bool {
        self.output_suppressed.load(Ordering::SeqCst)
            && matches!(
                event,
                LiveServerEvent::ModelTranscript(_)
                    | LiveServerEvent::AudioData(_)
                    | LiveServerEvent::TurnComplete
                    | LiveServerEvent::ToolCall { .. }
            )
    }

    pub async fn barge_in(&self, playback: Arc<AudioPlayback>) -> Result<(), String> {
        let _operation_guard = self.operation_lock.lock().await;
        if !self.is_active() {
            return Ok(());
        }

        playback.flush();
        self.output_suppressed.store(true, Ordering::SeqCst);
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
        self.shutdown_locked(capture, playback, ConversationLifecycle::Idle)
            .await;
    }

    pub async fn shutdown_application(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
    ) {
        let _operation_guard = self.operation_lock.lock().await;
        self.shutdown_locked(capture, playback.clone(), ConversationLifecycle::Idle)
            .await;
        playback.stop();
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
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

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

    struct CountingSession {
        close_count: Arc<AtomicUsize>,
        interrupt_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LiveSession for CountingSession {
        async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), String> {
            Ok(())
        }

        async fn send_tool_response(&mut self, _response: ToolCallResponse) -> Result<(), String> {
            Ok(())
        }

        async fn interrupt(&mut self) -> Result<(), String> {
            self.interrupt_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }

        async fn close(&mut self) -> Result<(), String> {
            self.close_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
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

    fn test_event_loop_context(
        generation: u64,
        session_id: &str,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        state_events: Arc<SyncMutex<Vec<CharacterState>>>,
        lifecycle_callback: LifecycleCallback,
    ) -> ConversationEventLoopContext {
        ConversationEventLoopContext {
            generation,
            session_id: session_id.to_string(),
            capture,
            playback,
            output_sample_rate: 24_000,
            tool_router: test_tool_router(),
            state_callback: Arc::new(move |state| state_events.lock().push(state)),
            lifecycle_callback,
            transcript_callback: Arc::new(|_, _, _| {}),
            speech_bubble_callback: Arc::new(|_| {}),
        }
    }

    #[test]
    fn lifecycle_transition_table_rejects_invalid_jumps() {
        assert!(ConversationLifecycle::Idle.can_transition_to(ConversationLifecycle::Connecting));
        assert!(ConversationLifecycle::Connecting.can_transition_to(ConversationLifecycle::Failed));
        assert!(
            ConversationLifecycle::Responding.can_transition_to(ConversationLifecycle::Listening)
        );
        assert!(!ConversationLifecycle::Idle.can_transition_to(ConversationLifecycle::Responding));
        assert!(!ConversationLifecycle::Failed.can_transition_to(ConversationLifecycle::Responding));
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

    #[tokio::test]
    async fn centralized_shutdown_is_idempotent_and_closes_session_once() {
        let manager = ConversationManager::new();
        let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
        let playback = Arc::new(AudioPlayback::new());
        let (pcm_tx, _pcm_rx) = mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

        let close_count = Arc::new(AtomicUsize::new(0));
        let interrupt_count = Arc::new(AtomicUsize::new(0));
        *manager.live_session.lock().await = Some(Box::new(CountingSession {
            close_count: close_count.clone(),
            interrupt_count,
        }));
        *manager.active_session_id.lock() = Some("test-session".to_string());
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.lifecycle.write() = ConversationLifecycle::Listening;

        manager
            .stop_session(capture.clone(), playback.clone())
            .await;
        manager.stop_session(capture.clone(), playback).await;

        assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
        assert!(!capture.lock().is_active());
        assert!(!manager.is_active());
        assert_eq!(manager.current_session_id(), None);
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    }

    #[tokio::test]
    async fn application_shutdown_is_idempotent_and_closes_backend_resources() {
        let manager = ConversationManager::new();
        let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
        let playback = Arc::new(AudioPlayback::new());
        let (pcm_tx, _pcm_rx) = mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

        let close_count = Arc::new(AtomicUsize::new(0));
        let interrupt_count = Arc::new(AtomicUsize::new(0));
        *manager.live_session.lock().await = Some(Box::new(CountingSession {
            close_count: close_count.clone(),
            interrupt_count,
        }));
        *manager.active_session_id.lock() = Some("shutdown-session".to_string());
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.lifecycle.write() = ConversationLifecycle::Listening;

        manager
            .shutdown_application(capture.clone(), playback.clone())
            .await;
        manager
            .shutdown_application(capture.clone(), playback.clone())
            .await;

        assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
        assert!(!capture.lock().is_active());
        assert!(!manager.is_active());
        assert_eq!(manager.current_session_id(), None);
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
        assert_eq!(playback.diagnostics().sample_rate_hz, None);
        assert!(!playback.is_playing());
    }

    #[tokio::test]
    async fn centralized_shutdown_emits_retained_lifecycle_events() {
        let manager = ConversationManager::new();
        let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
        let playback = Arc::new(AudioPlayback::new());
        let observed = Arc::new(SyncMutex::new(Vec::new()));
        let observed_for_callback = observed.clone();
        *manager.lifecycle_callback.lock() = Some(Arc::new(move |lifecycle| {
            observed_for_callback.lock().push(lifecycle);
        }));
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.active_session_id.lock() = Some("test-session".to_string());
        *manager.lifecycle.write() = ConversationLifecycle::Listening;

        manager.stop_session(capture, playback).await;

        assert_eq!(
            observed.lock().as_slice(),
            &[ConversationLifecycle::Stopping, ConversationLifecycle::Idle]
        );
        assert!(manager.lifecycle_callback.lock().is_none());
    }

    #[tokio::test]
    async fn provider_closed_event_loop_converges_through_centralized_cleanup() {
        let manager = ConversationManager::new();
        let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
        let playback = Arc::new(AudioPlayback::new());
        let (pcm_tx, _pcm_rx) = mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

        let close_count = Arc::new(AtomicUsize::new(0));
        let interrupt_count = Arc::new(AtomicUsize::new(0));
        *manager.live_session.lock().await = Some(Box::new(CountingSession {
            close_count: close_count.clone(),
            interrupt_count,
        }));
        let generation = 11;
        let session_id = "closed-session";
        manager.generation.store(generation, Ordering::SeqCst);
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.active_session_id.lock() = Some(session_id.to_string());
        *manager.lifecycle.write() = ConversationLifecycle::Listening;

        let lifecycle_events = Arc::new(SyncMutex::new(Vec::new()));
        let lifecycle_events_for_callback = lifecycle_events.clone();
        let lifecycle_callback: LifecycleCallback =
            Arc::new(move |lifecycle| lifecycle_events_for_callback.lock().push(lifecycle));
        *manager.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
        let state_events = Arc::new(SyncMutex::new(Vec::new()));
        let context = test_event_loop_context(
            generation,
            session_id,
            capture.clone(),
            playback.clone(),
            state_events.clone(),
            lifecycle_callback,
        );
        let (event_tx, event_rx) = mpsc::channel(1);
        event_tx.send(LiveServerEvent::Closed).await.unwrap();
        drop(event_tx);

        manager.run_event_loop(event_rx, context).await;

        assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
        assert!(!capture.lock().is_active());
        assert!(!manager.is_active());
        assert_eq!(manager.current_session_id(), None);
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
        assert_eq!(state_events.lock().as_slice(), &[CharacterState::Idle]);
        assert_eq!(
            lifecycle_events.lock().as_slice(),
            &[ConversationLifecycle::Stopping, ConversationLifecycle::Idle]
        );
    }

    #[tokio::test]
    async fn provider_error_event_loop_converges_to_failed_cleanup() {
        let manager = ConversationManager::new();
        let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
        let playback = Arc::new(AudioPlayback::new());
        let (pcm_tx, _pcm_rx) = mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();

        let close_count = Arc::new(AtomicUsize::new(0));
        let interrupt_count = Arc::new(AtomicUsize::new(0));
        *manager.live_session.lock().await = Some(Box::new(CountingSession {
            close_count: close_count.clone(),
            interrupt_count,
        }));
        let generation = 12;
        let session_id = "error-session";
        manager.generation.store(generation, Ordering::SeqCst);
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.active_session_id.lock() = Some(session_id.to_string());
        *manager.lifecycle.write() = ConversationLifecycle::Listening;

        let lifecycle_events = Arc::new(SyncMutex::new(Vec::new()));
        let lifecycle_events_for_callback = lifecycle_events.clone();
        let lifecycle_callback: LifecycleCallback =
            Arc::new(move |lifecycle| lifecycle_events_for_callback.lock().push(lifecycle));
        *manager.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
        let state_events = Arc::new(SyncMutex::new(Vec::new()));
        let context = test_event_loop_context(
            generation,
            session_id,
            capture.clone(),
            playback.clone(),
            state_events.clone(),
            lifecycle_callback,
        );
        let (event_tx, event_rx) = mpsc::channel(1);
        event_tx
            .send(LiveServerEvent::Error(
                "injected provider failure".to_string(),
            ))
            .await
            .unwrap();
        drop(event_tx);

        manager.run_event_loop(event_rx, context).await;

        assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
        assert!(!capture.lock().is_active());
        assert!(!manager.is_active());
        assert_eq!(manager.current_session_id(), None);
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
        assert_eq!(state_events.lock().as_slice(), &[CharacterState::Error]);
        assert_eq!(
            lifecycle_events.lock().as_slice(),
            &[
                ConversationLifecycle::Stopping,
                ConversationLifecycle::Failed
            ]
        );
    }

    #[tokio::test]
    async fn stale_generation_cannot_tear_down_newer_session() {
        let manager = ConversationManager::new();
        let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
        let playback = Arc::new(AudioPlayback::new());
        manager.generation.store(8, Ordering::SeqCst);
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.active_session_id.lock() = Some("new-session".to_string());
        *manager.lifecycle.write() = ConversationLifecycle::Listening;

        let cleaned = manager
            .shutdown_if_generation_current(7, capture, playback, ConversationLifecycle::Idle)
            .await;

        assert!(!cleaned);
        assert!(manager.is_active());
        assert_eq!(manager.current_session_id().as_deref(), Some("new-session"));
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Listening);
    }

    #[tokio::test]
    async fn barge_in_suppresses_stale_output_until_next_user_turn_boundary() {
        let manager = ConversationManager::new();
        let playback = Arc::new(AudioPlayback::new());
        let close_count = Arc::new(AtomicUsize::new(0));
        let interrupt_count = Arc::new(AtomicUsize::new(0));
        *manager.live_session.lock().await = Some(Box::new(CountingSession {
            close_count,
            interrupt_count: interrupt_count.clone(),
        }));
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.lifecycle.write() = ConversationLifecycle::Responding;

        manager.barge_in(playback).await.unwrap();

        assert!(manager.output_suppressed.load(Ordering::SeqCst));
        assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Responding);
    }

    #[test]
    fn interrupted_response_suppression_rejects_stale_non_audio_callbacks() {
        let manager = ConversationManager::new();
        manager.output_suppressed.store(true, Ordering::SeqCst);

        assert!(manager.should_suppress_interrupted_response_event(
            &LiveServerEvent::ModelTranscript("stale transcript".to_string())
        ));
        assert!(
            manager.should_suppress_interrupted_response_event(&LiveServerEvent::AudioData(vec![
                1, 2, 3
            ]))
        );
        assert!(manager.should_suppress_interrupted_response_event(&LiveServerEvent::TurnComplete));
        assert!(
            manager.should_suppress_interrupted_response_event(&LiveServerEvent::ToolCall {
                id: "stale-tool".to_string(),
                name: "remember".to_string(),
                args: serde_json::json!({"fact": "stale"}),
            })
        );

        assert!(!manager.should_suppress_interrupted_response_event(
            &LiveServerEvent::UserTranscript("new user turn".to_string())
        ));
        assert!(!manager.should_suppress_interrupted_response_event(&LiveServerEvent::Interrupted));
        assert!(!manager.should_suppress_interrupted_response_event(&LiveServerEvent::Closed));
    }

    #[tokio::test]
    async fn barge_in_waits_for_the_serialized_operation_boundary() {
        let manager = ConversationManager::new();
        let playback = Arc::new(AudioPlayback::new());
        let close_count = Arc::new(AtomicUsize::new(0));
        let interrupt_count = Arc::new(AtomicUsize::new(0));
        *manager.live_session.lock().await = Some(Box::new(CountingSession {
            close_count,
            interrupt_count: interrupt_count.clone(),
        }));
        manager.is_in_conversation.store(true, Ordering::SeqCst);
        *manager.lifecycle.write() = ConversationLifecycle::Responding;

        let operation_guard = manager.operation_lock.lock().await;
        let manager_for_barge = manager.clone();
        let barge_task = tokio::spawn(async move { manager_for_barge.barge_in(playback).await });

        tokio::task::yield_now().await;
        assert!(!barge_task.is_finished());
        assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 0);

        drop(operation_guard);
        barge_task.await.unwrap().unwrap();
        assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
    }
}
