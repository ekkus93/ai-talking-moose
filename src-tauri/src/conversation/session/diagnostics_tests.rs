use super::*;
use crate::asr::pipeline::LOCAL_ASR_QUEUE_CAPACITY_CHUNKS;
use crate::asr::types::LocalAsrRuntimeDiagnostics;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

#[tokio::test]
async fn shutdown_preserves_last_local_asr_diagnostics_for_selected_mode() {
    struct DiagnosticLocalAsrResource {
        diagnostics: LocalAsrRuntimeDiagnostics,
    }

    #[async_trait]
    impl crate::asr::lifecycle::LocalAsrResource for DiagnosticLocalAsrResource {
        async fn stop(&mut self) -> Result<(), crate::asr::AsrError> {
            Ok(())
        }

        fn diagnostics(&self) -> Option<LocalAsrRuntimeDiagnostics> {
            Some(self.diagnostics.clone())
        }
    }

    let manager = ConversationManager::new();
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    manager
        .local_asr_lifecycle()
        .attach(
            1,
            Box::new(DiagnosticLocalAsrResource {
                diagnostics: LocalAsrRuntimeDiagnostics {
                    input_sample_rate_hz: 16_000,
                    streaming: true,
                    queue_capacity: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
                    first_partial_latency_ms: Some(42),
                    real_time_factor: Some(0.4),
                    ..LocalAsrRuntimeDiagnostics::default()
                },
            }),
        )
        .await
        .unwrap();

    let _ = manager
        .begin_shutdown_locked(
            Arc::new(SyncMutex::new(AudioCapture::new_mock())),
            Arc::new(AudioPlayback::new()),
            ConversationLifecycle::Idle,
        )
        .await;

    let (snapshot, dropped_chunks) = manager
        .last_local_asr_diagnostics(AsrMode::MoonshineTinyStreaming)
        .expect("Tiny diagnostics should survive resource teardown");
    assert_eq!(snapshot.first_partial_latency_ms, Some(42));
    assert_eq!(snapshot.real_time_factor, Some(0.4));
    assert!(!snapshot.streaming);
    assert!(snapshot.metrics_snapshot);
    assert_eq!(dropped_chunks, 0);
    assert!(manager
        .last_local_asr_diagnostics(AsrMode::MoonshineSmallStreaming)
        .is_none());
}

struct StableProvider {
    close_count: Arc<AtomicUsize>,
    interrupt_count: Arc<AtomicUsize>,
}

#[async_trait]
impl RealtimeConversationProvider for StableProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        Ok(Box::new(StableSession {
            _event_sender: event_sender,
            close_count: self.close_count.clone(),
            interrupt_count: self.interrupt_count.clone(),
        }))
    }
}

struct StableSession {
    _event_sender: mpsc::Sender<LiveServerEvent>,
    close_count: Arc<AtomicUsize>,
    interrupt_count: Arc<AtomicUsize>,
}

#[async_trait]
impl LiveSession for StableSession {
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

fn stable_request(
    close_count: Arc<AtomicUsize>,
    interrupt_count: Arc<AtomicUsize>,
    playback: Arc<AudioPlayback>,
) -> ConversationStartRequest {
    ConversationStartRequest {
        provider: Arc::new(StableProvider {
            close_count,
            interrupt_count,
        }),
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
        playback,
        output_device: None,
        muted: Arc::new(RwLock::new(false)),
        tool_router: test_tool_router(),
        callbacks: ConversationCallbacks::new(|_| {}, |_| {}, |_, _, _| {}, |_| {}, |_| {}, |_| {}),
    }
}

#[tokio::test]
async fn audio_output_start_failure_closes_provider_before_microphone_capture() {
    let manager = ConversationManager::new();
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    let request = stable_request(
        close_count.clone(),
        interrupt_count,
        Arc::new(AudioPlayback::new_failing_mock("synthetic output failure")),
    );
    let capture = request.capture.clone();

    let error = manager.start_session(request).await.unwrap_err();

    assert!(error.contains("failed to start audio output"));
    assert!(error.contains("synthetic output failure"));
    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!capture.lock().is_active());
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
}

#[tokio::test]
async fn repeated_start_stop_cycles_are_hardware_free_and_release_each_session() {
    const CYCLES: usize = 16;
    let manager = ConversationManager::new();
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));

    for _ in 0..CYCLES {
        let request = stable_request(
            close_count.clone(),
            interrupt_count.clone(),
            Arc::new(AudioPlayback::new_mock()),
        );
        let capture = request.capture.clone();
        let playback = request.playback.clone();

        manager.start_session(request).await.unwrap();
        assert!(manager.is_active());
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Listening);
        assert!(capture.lock().is_active());
        assert_eq!(playback.output_sample_rate_hz(), Some(24_000));

        manager
            .stop_session(capture.clone(), playback.clone())
            .await;
        assert!(!manager.is_active());
        assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
        assert!(!capture.lock().is_active());
        assert_eq!(playback.queue_length(), 0);
    }

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), CYCLES);
    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 0);
}

#[tokio::test]
async fn concurrent_stop_requests_are_serialized_and_close_provider_once() {
    let manager = ConversationManager::new();
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    let request = stable_request(
        close_count.clone(),
        interrupt_count,
        Arc::new(AudioPlayback::new_mock()),
    );
    let capture = request.capture.clone();
    let playback = request.playback.clone();

    manager.start_session(request).await.unwrap();

    let first_manager = manager.clone();
    let first_capture = capture.clone();
    let first_playback = playback.clone();
    let first = tokio::spawn(async move {
        first_manager
            .stop_session(first_capture, first_playback)
            .await;
    });
    let second_manager = manager.clone();
    let second_capture = capture.clone();
    let second_playback = playback.clone();
    let second = tokio::spawn(async move {
        second_manager
            .stop_session(second_capture, second_playback)
            .await;
    });

    first.await.unwrap();
    second.await.unwrap();

    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
    assert!(!manager.is_active());
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Idle);
    assert!(!capture.lock().is_active());
    assert_eq!(playback.queue_length(), 0);
}

#[tokio::test]
async fn barge_in_flushes_buffered_output_and_interrupts_once() {
    let manager = ConversationManager::new();
    let close_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    let playback = Arc::new(AudioPlayback::new_mock());
    let request = stable_request(
        close_count.clone(),
        interrupt_count.clone(),
        playback.clone(),
    );
    let capture = request.capture.clone();

    manager.start_session(request).await.unwrap();
    playback.seed_buffer_for_tests(&[0.5; 256], 0.7);
    assert_eq!(playback.queue_length(), 256);
    assert!(playback.is_playing());

    manager.barge_in(playback.clone()).await.unwrap();

    assert_eq!(playback.queue_length(), 0);
    assert!(!playback.is_playing());
    assert_eq!(playback.diagnostics().output_level, 0.0);
    assert_eq!(interrupt_count.load(AtomicOrdering::SeqCst), 1);

    manager.stop_session(capture, playback).await;
    assert_eq!(close_count.load(AtomicOrdering::SeqCst), 1);
}
