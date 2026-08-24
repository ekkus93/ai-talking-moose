use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};

struct FailingProvider;

#[async_trait]
impl RealtimeConversationProvider for FailingProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        _event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        Err(ProviderError::from_kind(ProviderErrorKind::Network))
    }
}

struct StallingProvider {
    entered: Arc<AtomicBool>,
}

#[async_trait]
impl RealtimeConversationProvider for StallingProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        _event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        self.entered.store(true, AtomicOrdering::SeqCst);
        std::future::pending().await
    }
}

struct ConnectCountingProvider {
    connect_count: Arc<AtomicUsize>,
}

#[async_trait]
impl RealtimeConversationProvider for ConnectCountingProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        _event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        self.connect_count.fetch_add(1, AtomicOrdering::SeqCst);
        Err(ProviderError::from_kind(ProviderErrorKind::Network))
    }
}

struct CountingSession {
    close_count: Arc<AtomicUsize>,
    interrupt_count: Arc<AtomicUsize>,
}

#[async_trait]
impl LiveSession for CountingSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_text_turn(&mut self, _text: &str) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        self.interrupt_count.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        self.close_count.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(())
    }
}

struct ToolRecordingSession {
    responses: Arc<SyncMutex<Vec<ToolCallResponse>>>,
}

#[async_trait]
impl LiveSession for ToolRecordingSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_text_turn(&mut self, _text: &str) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        self.responses.lock().push(response);
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }
}


struct AudioRecordingSession {
    chunks: Arc<SyncMutex<Vec<Vec<u8>>>>,
}

#[async_trait]
impl LiveSession for AudioRecordingSession {
    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        self.chunks.lock().push(pcm_bytes.to_vec());
        Ok(())
    }

    async fn send_text_turn(&mut self, _text: &str) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }
}

struct StallingSession;

#[async_trait]
impl LiveSession for StallingSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        std::future::pending().await
    }

    async fn send_text_turn(&mut self, _text: &str) -> Result<(), ProviderError> {
        std::future::pending().await
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        std::future::pending().await
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        std::future::pending().await
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        std::future::pending().await
    }
}

struct CountingLocalAsrResource {
    stop_count: Arc<AtomicUsize>,
}

#[async_trait]
impl crate::asr::lifecycle::LocalAsrResource for CountingLocalAsrResource {
    async fn stop(&mut self) -> Result<(), crate::asr::AsrError> {
        self.stop_count.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(())
    }
}

async fn attach_counting_local_asr(
    manager: &ConversationManager,
    generation: u64,
) -> Arc<AtomicUsize> {
    let stop_count = Arc::new(AtomicUsize::new(0));
    manager
        .local_asr_lifecycle()
        .attach(
            generation,
            Box::new(CountingLocalAsrResource {
                stop_count: stop_count.clone(),
            }),
        )
        .await
        .unwrap();
    stop_count
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
            tools: vec![],
        },
        asr_mode: AsrMode::GeminiLiveAudio,
        moonshine_installer: None,
        capture: Arc::new(SyncMutex::new(AudioCapture::new_mock())),
        input_device: None,
        playback: Arc::new(AudioPlayback::new()),
        output_device: None,
        muted: Arc::new(RwLock::new(muted)),
        tool_router: test_tool_router(),
        callbacks: ConversationCallbacks::new(|_| {}, |_| {}, |_, _, _| {}, |_| {}, |_| {}, |_| {}),
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
        provider_error_callback: Arc::new(|_| {}),
        transcript_callback: Arc::new(|_, _, _| {}),
        speech_bubble_callback: Arc::new(|_| {}),
    }
}

#[test]
fn lifecycle_transition_table_rejects_invalid_jumps() {
    assert!(ConversationLifecycle::Idle.can_transition_to(ConversationLifecycle::Connecting));
    assert!(ConversationLifecycle::Connecting.can_transition_to(ConversationLifecycle::Failed));
    assert!(ConversationLifecycle::Responding.can_transition_to(ConversationLifecycle::Listening));
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
async fn missing_tiny_model_fails_before_provider_connect_or_microphone_capture() {
    let manager = ConversationManager::new();
    let connect_count = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineTinyStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    request.moonshine_installer =
        Some(Arc::new(MoonshineModelInstaller::new(temp.path()).unwrap()));
    let capture = request.capture.clone();

    let error = manager
        .start_session(request)
        .await
        .expect_err("missing Tiny model must fail before provider or microphone startup");

    assert!(error.contains("not installed"));
    assert!(error.contains("No microphone audio was sent"));
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!capture.lock().is_active());
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn missing_small_model_fails_before_provider_connect_or_microphone_capture() {
    let manager = ConversationManager::new();
    let connect_count = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineSmallStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    request.moonshine_installer =
        Some(Arc::new(MoonshineModelInstaller::new(temp.path()).unwrap()));
    let capture = request.capture.clone();

    let error = manager
        .start_session(request)
        .await
        .expect_err("missing Small model must fail before provider or microphone startup");

    assert!(error.contains("Moonshine Small"));
    assert!(error.contains("not installed"));
    assert!(error.contains("No microphone audio was sent"));
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!capture.lock().is_active());
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn corrupt_tiny_model_fails_before_provider_connect_or_microphone_capture() {
    let manager = ConversationManager::new();
    let connect_count = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let installer = Arc::new(MoonshineModelInstaller::new(temp.path()).unwrap());
    let corrupt_model =
        installer.model_path(crate::asr::moonshine::MoonshineModelArchitecture::TinyStreaming);
    std::fs::create_dir_all(&corrupt_model).unwrap();
    std::fs::write(
        corrupt_model.join("corrupt-placeholder"),
        b"not a verified model",
    )
    .unwrap();

    let mut request = test_request(false);
    request.asr_mode = AsrMode::MoonshineTinyStreaming;
    request.provider = Arc::new(ConnectCountingProvider {
        connect_count: connect_count.clone(),
    });
    request.moonshine_installer = Some(installer);
    let capture = request.capture.clone();

    let error = manager
        .start_session(request)
        .await
        .expect_err("corrupt Tiny model must fail before provider or microphone startup");

    assert!(error.contains("corrupt") || error.contains("incomplete"));
    assert!(error.contains("No microphone audio was sent"));
    assert_eq!(connect_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!capture.lock().is_active());
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
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
    let local_asr_stop_count = attach_counting_local_asr(&manager, 0).await;

    manager
        .stop_session(capture.clone(), playback.clone())
        .await;
    manager.stop_session(capture.clone(), playback).await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
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
    let local_asr_stop_count = attach_counting_local_asr(&manager, 0).await;

    manager
        .shutdown_application(capture.clone(), playback.clone())
        .await;
    manager
        .shutdown_application(capture.clone(), playback.clone())
        .await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
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
    let local_asr_stop_count = attach_counting_local_asr(&manager, generation).await;

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
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
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
    let local_asr_stop_count = attach_counting_local_asr(&manager, generation).await;

    let lifecycle_events = Arc::new(SyncMutex::new(Vec::new()));
    let lifecycle_events_for_callback = lifecycle_events.clone();
    let lifecycle_callback: LifecycleCallback =
        Arc::new(move |lifecycle| lifecycle_events_for_callback.lock().push(lifecycle));
    *manager.lifecycle_callback.lock() = Some(lifecycle_callback.clone());
    let state_events = Arc::new(SyncMutex::new(Vec::new()));
    let mut context = test_event_loop_context(
        generation,
        session_id,
        capture.clone(),
        playback.clone(),
        state_events.clone(),
        lifecycle_callback,
    );
    let provider_errors = Arc::new(SyncMutex::new(Vec::new()));
    let provider_errors_for_callback = provider_errors.clone();
    context.provider_error_callback = Arc::new(move |error| {
        provider_errors_for_callback.lock().push(error);
    });
    let (event_tx, event_rx) = mpsc::channel(1);
    event_tx
        .send(LiveServerEvent::Error(ProviderError::from_kind(
            ProviderErrorKind::Protocol,
        )))
        .await
        .unwrap();
    drop(event_tx);

    manager.run_event_loop(event_rx, context).await;

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
    assert_eq!(state_events.lock().as_slice(), &[CharacterState::Error]);
    assert_eq!(provider_errors.lock().len(), 1);
    assert_eq!(provider_errors.lock()[0].kind, ProviderErrorKind::Protocol);
    assert_eq!(
        lifecycle_events.lock().as_slice(),
        &[
            ConversationLifecycle::Stopping,
            ConversationLifecycle::Failed
        ]
    );
}

#[tokio::test]
async fn starting_a_new_provider_tears_down_previous_local_asr_before_connect() {
    let manager = ConversationManager::new();
    manager.generation.store(30, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("old-session".to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let local_asr_stop_count = attach_counting_local_asr(&manager, 30).await;

    let result = manager.start_session(test_request(false)).await;

    assert!(result.is_err());
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.local_asr_lifecycle().is_active().await);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn barge_in_keeps_current_local_asr_active() {
    let manager = ConversationManager::new();
    let playback = Arc::new(AudioPlayback::new());
    let generation = 35;
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("local-barge-session".to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;

    let local_asr_stop_count = attach_counting_local_asr(&manager, generation).await;
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    *manager.live_session.lock().await = Some(Box::new(CountingSession {
        close_count,
        interrupt_count: interrupt_count.clone(),
    }));

    manager.barge_in(playback).await.unwrap();

    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 0);
    assert!(manager.local_asr_lifecycle().is_active().await);
    assert!(manager.local_asr_callback_is_current(generation).await);
}

#[tokio::test]
async fn stale_local_asr_callback_cannot_mutate_newer_generation() {
    let manager = ConversationManager::new();
    manager.generation.store(40, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, 40).await;

    assert!(manager.local_asr_callback_is_current(40).await);
    manager.generation.store(41, Ordering::SeqCst);
    assert!(!manager.local_asr_callback_is_current(40).await);
    assert!(!manager.local_asr_callback_is_current(41).await);
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
    manager.generation.store(21, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;
    let local_asr_stop_count = attach_counting_local_asr(&manager, 21).await;

    manager.barge_in(playback).await.unwrap();

    assert!(manager.output_suppressed.load(Ordering::SeqCst));
    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(local_asr_stop_count.load(AtomicOrdering::SeqCst), 0);
    assert!(manager.local_asr_callback_is_current(21).await);
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Responding);
}

#[tokio::test]
async fn tool_call_event_dispatches_through_router_and_preserves_call_identity() {
    let manager = ConversationManager::new();
    let generation = 60;
    let session_id = "tool-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let responses = Arc::new(SyncMutex::new(Vec::<ToolCallResponse>::new()));
    *manager.live_session.lock().await = Some(Box::new(ToolRecordingSession {
        responses: responses.clone(),
    }));

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let states = Arc::new(SyncMutex::new(Vec::new()));
    let lifecycle_callback: LifecycleCallback = Arc::new(|_| {});
    let context = test_event_loop_context(
        generation,
        session_id,
        capture,
        playback,
        states,
        lifecycle_callback,
    );
    let (event_tx, event_rx) = mpsc::channel(4);
    let manager_for_loop = manager.clone();
    let loop_task = tokio::spawn(async move {
        manager_for_loop.run_event_loop(event_rx, context).await;
    });

    event_tx
        .send(LiveServerEvent::ToolCall {
            id: "call-42".to_string(),
            name: "get_current_time".to_string(),
            args: serde_json::json!({}),
        })
        .await
        .unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if !responses.lock().is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("tool response was not sent");

    let response = responses.lock()[0].clone();
    assert_eq!(response.id, "call-42");
    assert_eq!(response.name, "get_current_time");
    assert!(response.output.get("time").is_some());

    event_tx.send(LiveServerEvent::Closed).await.unwrap();
    drop(event_tx);
    loop_task.await.unwrap();
}

#[test]
fn interrupted_response_suppression_rejects_stale_non_audio_callbacks() {
    let manager = ConversationManager::new();
    manager.output_suppressed.store(true, Ordering::SeqCst);

    assert!(
        manager.should_suppress_interrupted_response_event(&LiveServerEvent::ModelTranscript(
            TranscriptUpdate {
                text: "stale transcript".to_string(),
                is_final: false,
            }
        ))
    );
    assert!(manager
        .should_suppress_interrupted_response_event(&LiveServerEvent::AudioData(vec![1, 2, 3])));
    assert!(manager.should_suppress_interrupted_response_event(&LiveServerEvent::TurnComplete));
    assert!(
        manager.should_suppress_interrupted_response_event(&LiveServerEvent::ToolCall {
            id: "stale-tool".to_string(),
            name: "remember".to_string(),
            args: serde_json::json!({"fact": "stale"}),
        })
    );

    assert!(
        !manager.should_suppress_interrupted_response_event(&LiveServerEvent::UserTranscript(
            TranscriptUpdate {
                text: "new user turn".to_string(),
                is_final: false,
            }
        ))
    );
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

#[tokio::test]
async fn stale_generation_tool_response_cannot_enter_replacement_session() {
    let manager = ConversationManager::new();
    let responses = Arc::new(SyncMutex::new(Vec::<ToolCallResponse>::new()));
    *manager.live_session.lock().await = Some(Box::new(ToolRecordingSession {
        responses: responses.clone(),
    }));
    manager.generation.store(71, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some("replacement-session".to_string());

    let accepted = manager
        .send_tool_response_if_current(
            70,
            "stale-session",
            ToolCallResponse {
                id: "stale-call".to_string(),
                name: "get_current_time".to_string(),
                output: serde_json::json!({"time": "stale"}),
            },
        )
        .await
        .unwrap();

    assert!(!accepted);
    assert!(responses.lock().is_empty());
}

#[tokio::test]
async fn stale_generation_microphone_chunk_cannot_enter_replacement_session() {
    let manager = ConversationManager::new();
    let chunks = Arc::new(SyncMutex::new(Vec::<Vec<u8>>::new()));
    *manager.live_session.lock().await = Some(Box::new(AudioRecordingSession {
        chunks: chunks.clone(),
    }));
    manager.generation.store(81, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_asr_mode.lock() = Some(AsrMode::GeminiLiveAudio);

    let accepted = manager
        .forward_microphone_chunk(80, AsrMode::GeminiLiveAudio, &[1, 2, 3, 4])
        .await
        .unwrap();

    assert!(!accepted);
    assert!(chunks.lock().is_empty());
}

#[tokio::test]
async fn stalled_provider_send_cannot_block_stop_indefinitely() {
    let manager = ConversationManager::new();
    *manager.live_session.lock().await = Some(Box::new(StallingSession));
    manager.generation.store(91, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_asr_mode.lock() = Some(AsrMode::GeminiLiveAudio);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let manager_for_send = manager.clone();
    let send_task = tokio::spawn(async move {
        manager_for_send
            .forward_microphone_chunk(91, AsrMode::GeminiLiveAudio, &[7, 8])
            .await
    });
    tokio::task::yield_now().await;

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        manager.stop_session(capture, playback),
    )
    .await
    .expect("bounded provider I/O must allow Stop to complete");

    let send_error = send_task
        .await
        .unwrap()
        .expect_err("stalled send must reach the provider timeout");
    assert_eq!(send_error.kind, ProviderErrorKind::Network);
    assert!(!manager.is_active());
}

#[tokio::test]
async fn stalled_provider_interrupt_is_bounded() {
    let manager = ConversationManager::new();
    *manager.live_session.lock().await = Some(Box::new(StallingSession));
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.lifecycle.write() = ConversationLifecycle::Responding;

    let error = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        manager.barge_in(Arc::new(AudioPlayback::new())),
    )
    .await
    .expect("bounded provider I/O must allow barge-in to return")
    .expect_err("stalled interrupt must report a timeout");

    assert!(error.contains("operation timeout"));
}
#[tokio::test]
async fn stalled_provider_connect_does_not_hold_operation_lock_against_stop() {
    let manager = ConversationManager::new();
    let entered = Arc::new(AtomicBool::new(false));
    let mut request = test_request(false);
    request.provider = Arc::new(StallingProvider {
        entered: entered.clone(),
    });
    let capture = request.capture.clone();
    let playback = request.playback.clone();

    let manager_for_start = manager.clone();
    let start_task = tokio::spawn(async move { manager_for_start.start_session(request).await });
    tokio::time::timeout(std::time::Duration::from_millis(250), async {
        while !entered.load(AtomicOrdering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("provider connect was not entered");

    tokio::time::timeout(
        std::time::Duration::from_millis(250),
        manager.stop_session(capture, playback),
    )
    .await
    .expect("Stop must not wait for a stalled provider connect");

    let start_error = tokio::time::timeout(std::time::Duration::from_millis(500), start_task)
        .await
        .expect("bounded connect must finish")
        .unwrap()
        .expect_err("the invalidated start must not activate");
    assert_eq!(start_error, "Conversation start was cancelled");
}
