use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::*;
use crate::app::wake_word_command_handoff::WakeCommandHandoffAudio;
use crate::asr::lifecycle::LocalAsrLifecycle;
use crate::asr::moonshine::MoonshineModelInstaller;
use crate::asr::pipeline::LocalAsrPipeline;
use crate::asr::{AsrEvent, AsrMode};
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::character::state::CharacterState;
use crate::tools::router::ToolRouter;
use parking_lot::{Mutex as SyncMutex, RwLock};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tracing::warn;
use uuid::Uuid;

#[cfg(not(test))]
const PROVIDER_OPERATION_TIMEOUT: Duration = Duration::from_secs(8);
#[cfg(test)]
const PROVIDER_OPERATION_TIMEOUT: Duration = Duration::from_millis(100);

mod event_loop;
mod local_asr;
mod provider_ops;
mod start;

#[cfg(test)]
use local_asr::LocalAsrPreparationTestGate;
use local_asr::{LocalAsrDiagnosticsStore, LocalAsrPreparation};

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
    pub asr_mode: AsrMode,
    pub moonshine_installer: Option<Arc<MoonshineModelInstaller>>,
    pub whisper_installer: Option<Arc<crate::asr::whisper::WhisperModelInstaller>>,
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

#[derive(Debug, Clone)]
struct ConversationStartReservation {
    generation: u64,
    session_id: String,
}

#[derive(Clone)]
pub struct ConversationManager {
    active_session_id: Arc<SyncMutex<Option<String>>>,
    active_asr_mode: Arc<SyncMutex<Option<AsrMode>>>,
    live_session: Arc<AsyncMutex<Option<Box<dyn LiveSession>>>>,
    lifecycle: Arc<RwLock<ConversationLifecycle>>,
    is_in_conversation: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    output_suppressed: Arc<AtomicBool>,
    state_callback: Arc<SyncMutex<Option<StateCallback>>>,
    lifecycle_callback: Arc<SyncMutex<Option<LifecycleCallback>>>,
    transcript_callback: Arc<SyncMutex<Option<TranscriptCallback>>>,
    local_asr: Arc<LocalAsrLifecycle>,
    local_asr_diagnostics: Arc<LocalAsrDiagnosticsStore>,
    operation_lock: Arc<AsyncMutex<()>>,
    #[cfg(test)]
    local_asr_preparation_test_gate: Arc<SyncMutex<Option<Arc<LocalAsrPreparationTestGate>>>>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            active_session_id: Arc::new(SyncMutex::new(None)),
            active_asr_mode: Arc::new(SyncMutex::new(None)),
            live_session: Arc::new(AsyncMutex::new(None)),
            lifecycle: Arc::new(RwLock::new(ConversationLifecycle::Idle)),
            is_in_conversation: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            output_suppressed: Arc::new(AtomicBool::new(false)),
            state_callback: Arc::new(SyncMutex::new(None)),
            lifecycle_callback: Arc::new(SyncMutex::new(None)),
            transcript_callback: Arc::new(SyncMutex::new(None)),
            local_asr: Arc::new(LocalAsrLifecycle::default()),
            local_asr_diagnostics: Arc::new(LocalAsrDiagnosticsStore::default()),
            operation_lock: Arc::new(AsyncMutex::new(())),
            #[cfg(test)]
            local_asr_preparation_test_gate: Arc::new(SyncMutex::new(None)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_in_conversation.load(Ordering::SeqCst)
    }

    pub fn active_asr_mode(&self) -> Option<AsrMode> {
        *self.active_asr_mode.lock()
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

    #[cfg(test)]
    fn set_local_asr_preparation_test_gate(&self, gate: Option<Arc<LocalAsrPreparationTestGate>>) {
        *self.local_asr_preparation_test_gate.lock() = gate;
    }

    pub async fn live_outbound_diagnostics(&self) -> Option<LiveOutboundDiagnostics> {
        self.live_session
            .lock()
            .await
            .as_ref()
            .and_then(|session| session.outbound_diagnostics())
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

    fn reserve_start_locked(
        &self,
        teardown_generation: u64,
        muted: &RwLock<bool>,
        lifecycle_callback: &LifecycleCallback,
    ) -> Result<ConversationStartReservation, String> {
        if self.generation.load(Ordering::SeqCst) != teardown_generation {
            return Err("Conversation start was cancelled".to_string());
        }
        if *muted.read() {
            return Err("Moose is currently muted".to_string());
        }

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let reservation = ConversationStartReservation {
            generation,
            session_id: Uuid::new_v4().to_string(),
        };
        self.output_suppressed.store(false, Ordering::SeqCst);
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Connecting,
            Some(lifecycle_callback),
        );
        Ok(reservation)
    }

    fn start_reservation_is_current(&self, reservation: &ConversationStartReservation) -> bool {
        self.generation.load(Ordering::SeqCst) == reservation.generation
    }

    async fn dispose_cancelled_start(
        local_pipeline: &mut Option<LocalAsrPipeline>,
        mut session: Option<Box<dyn LiveSession>>,
    ) {
        if let Some(ref mut session) = session {
            Self::close_provisional_session(session).await;
        }
        Self::stop_provisional_local_asr(local_pipeline).await;
    }

    async fn begin_shutdown_locked(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        final_lifecycle: ConversationLifecycle,
    ) -> Option<Box<dyn LiveSession>> {
        let lifecycle_callback = self.lifecycle_callback.lock().clone();
        Self::set_lifecycle(
            &self.lifecycle,
            ConversationLifecycle::Stopping,
            lifecycle_callback.as_ref(),
        );

        // Keep the current generation routable while the local-ASR worker drains
        // accepted PCM and delivers its stop-time FinalTranscript. Capture is
        // already stopped, so no new microphone input can enter the pipeline.
        capture.lock().stop();
        let active_asr_mode = *self.active_asr_mode.lock();
        self.stop_local_asr_for_shutdown(active_asr_mode, &capture)
            .await;

        self.generation.fetch_add(1, Ordering::SeqCst);
        self.is_in_conversation.store(false, Ordering::SeqCst);
        self.output_suppressed.store(false, Ordering::SeqCst);
        playback.flush();

        let session = self.live_session.lock().await.take();
        *self.active_session_id.lock() = None;
        *self.active_asr_mode.lock() = None;
        Self::set_lifecycle(
            &self.lifecycle,
            final_lifecycle,
            lifecycle_callback.as_ref(),
        );
        *self.state_callback.lock() = None;
        *self.lifecycle_callback.lock() = None;
        *self.transcript_callback.lock() = None;
        session
    }

    async fn shutdown_if_generation_current(
        &self,
        expected_generation: u64,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        final_lifecycle: ConversationLifecycle,
    ) -> bool {
        let operation_guard = self.operation_lock.lock().await;
        if self.generation.load(Ordering::SeqCst) != expected_generation {
            return false;
        }

        let session = self
            .begin_shutdown_locked(capture, playback, final_lifecycle)
            .await;
        drop(operation_guard);
        Self::close_detached_session(session).await;
        true
    }

    pub async fn barge_in(&self, playback: Arc<AudioPlayback>) -> Result<(), String> {
        let operation_guard = self.operation_lock.lock().await;
        if !self.is_active() {
            return Ok(());
        }

        let generation = self.generation.load(Ordering::SeqCst);
        playback.flush();
        self.output_suppressed.store(true, Ordering::SeqCst);
        drop(operation_guard);

        let mut session_lock = self.live_session.lock().await;
        if !self.is_active() || self.generation.load(Ordering::SeqCst) != generation {
            return Ok(());
        }
        if let Some(ref mut session) = *session_lock {
            Self::bounded_provider_operation(session.interrupt())
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
        let operation_guard = self.operation_lock.lock().await;
        let session = self
            .begin_shutdown_locked(capture, playback, ConversationLifecycle::Idle)
            .await;
        drop(operation_guard);
        Self::close_detached_session(session).await;
    }

    pub async fn shutdown_application(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
    ) {
        let operation_guard = self.operation_lock.lock().await;
        let session = self
            .begin_shutdown_locked(capture, playback.clone(), ConversationLifecycle::Idle)
            .await;
        drop(operation_guard);
        Self::close_detached_session(session).await;
        playback.stop();
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "session/tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "session/diagnostics_tests.rs"]
mod diagnostics_tests;
