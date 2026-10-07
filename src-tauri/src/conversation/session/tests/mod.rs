use super::*;
use crate::asr::AsrErrorKind;
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

struct ReadyProvider {
    connect_count: Arc<AtomicUsize>,
}

struct ReadySession {
    _event_sender: mpsc::Sender<LiveServerEvent>,
}

#[async_trait]
impl LiveSession for ReadySession {
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
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }
}

#[async_trait]
impl RealtimeConversationProvider for ReadyProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        self.connect_count.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(Box::new(ReadySession {
            _event_sender: event_sender,
        }))
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
        whisper_installer: None,
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

mod events;
mod generation;
mod lifecycle;
mod local_asr;
