use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::*;
use crate::asr::lifecycle::LocalAsrLifecycle;
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::character::state::CharacterState;
use crate::tools::router::ToolRouter;
use parking_lot::{Mutex as SyncMutex, RwLock};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tracing::warn;
use uuid::Uuid;

mod event_loop;

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
type ProviderErrorCallback = Arc<dyn Fn(ProviderError) + Send + Sync>;
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
    provider_error: ProviderErrorCallback,
    transcript: TranscriptCallback,
    speech_bubble: SpeechBubbleCallback,
    input_level: InputLevelCallback,
}

impl ConversationCallbacks {
    pub fn new<S, L, T, B, I, E>(
        state: S,
        lifecycle: L,
        transcript: T,
        speech_bubble: B,
        input_level: I,
        provider_error: E,
    ) -> Self
    where
        S: Fn(CharacterState) + Send + Sync + 'static,
        L: Fn(ConversationLifecycle) + Send + Sync + 'static,
        T: Fn(String, String, String) + Send + Sync + 'static,
        B: Fn(String) + Send + Sync + 'static,
        I: Fn(f32) + Send + Sync + 'static,
        E: Fn(ProviderError) + Send + Sync + 'static,
    {
        Self {
            state: Arc::new(state),
            lifecycle: Arc::new(lifecycle),
            provider_error: Arc::new(provider_error),
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
    provider_error_callback: ProviderErrorCallback,
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
    local_asr: Arc<LocalAsrLifecycle>,
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
            local_asr: Arc::new(LocalAsrLifecycle::default()),
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

    pub fn local_asr_lifecycle(&self) -> Arc<LocalAsrLifecycle> {
        self.local_asr.clone()
    }

    pub async fn local_asr_callback_is_current(&self, generation: u64) -> bool {
        self.is_active()
            && self.generation.load(Ordering::SeqCst) == generation
            && self.local_asr.accepts_callback(generation).await
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
            warn!(
                kind = ?error_value.kind,
                "Failed to close provisional conversation session"
            );
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
        if let Err(error_value) = self.local_asr.stop_and_clear().await {
            warn!(
                kind = ?error_value.kind,
                "Local ASR resource teardown reported an error"
            );
        }

        let mut session_lock = self.live_session.lock().await;
        if let Some(mut session) = session_lock.take() {
            if let Err(error_value) = session.close().await {
                warn!(
                    kind = ?error_value.kind,
                    "Failed to close conversation session"
                );
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
            provider_error: provider_error_callback,
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
                provider_error_callback(error_value.clone());
                state_callback(CharacterState::Error);
                return Err(error_value.to_string());
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
        let provider_error_for_pcm = provider_error_callback.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(chunk) = pcm_rx.recv().await {
                if !manager_for_pcm.is_in_conversation.load(Ordering::SeqCst)
                    || manager_for_pcm.generation.load(Ordering::SeqCst) != generation
                {
                    break;
                }

                let send_error = {
                    let mut session_lock = manager_for_pcm.live_session.lock().await;
                    match session_lock.as_mut() {
                        Some(live_session) => live_session.send_audio_chunk(&chunk).await.err(),
                        None => Some(ProviderError::from_kind(ProviderErrorKind::Closed)),
                    }
                };

                if let Some(error_value) = send_error {
                    let cleaned = manager_for_pcm
                        .shutdown_if_generation_current(
                            generation,
                            capture_for_pcm.clone(),
                            playback_for_pcm.clone(),
                            ConversationLifecycle::Failed,
                        )
                        .await;
                    if cleaned {
                        warn!(
                            kind = ?error_value.kind,
                            "Failed to stream microphone audio frame"
                        );
                        provider_error_for_pcm(error_value);
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
            provider_error_callback,
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

    pub async fn barge_in(&self, playback: Arc<AudioPlayback>) -> Result<(), String> {
        let _operation_guard = self.operation_lock.lock().await;
        if !self.is_active() {
            return Ok(());
        }

        playback.flush();
        self.output_suppressed.store(true, Ordering::SeqCst);
        let mut session_lock = self.live_session.lock().await;
        if let Some(ref mut session) = *session_lock {
            session
                .interrupt()
                .await
                .map_err(|error| error.to_string())?;
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
mod tests;
