use super::session::{
    ConversationCallbacks, ConversationLifecycle, ConversationManager, ConversationStartRequest,
};
use crate::ai::traits::{LiveSession, RealtimeConversationProvider};
use crate::ai::types::{LiveServerEvent, LiveSessionConfig, ProviderError, ProviderErrorKind};
use crate::asr::AsrMode;
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::character::personality::CharacterConfig;
use crate::memory::MemoryManager;
use crate::persistence::Database;
use crate::tools::builtin::BuiltinTools;
use crate::tools::router::ToolRouter;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

struct PendingSetupProvider {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl RealtimeConversationProvider for PendingSetupProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        _event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        self.entered.notify_one();
        self.release.notified().await;
        Err(ProviderError::from_kind(ProviderErrorKind::Setup))
    }
}

fn tool_router() -> Arc<ToolRouter> {
    let settings = Arc::new(RwLock::new(crate::app::state::AppSettings::default()));
    let memory = Arc::new(MemoryManager::new(Arc::new(
        Database::new_in_memory().expect("in-memory database"),
    )));
    Arc::new(ToolRouter::new(Arc::new(BuiltinTools {
        memory_manager: memory,
        character_config: CharacterConfig::default(),
        settings,
    })))
}

#[tokio::test]
async fn conversation_stays_connecting_until_provider_setup_finishes() {
    let manager = ConversationManager::new();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let capture = Arc::new(Mutex::new(AudioCapture::new_mock()));

    let request = ConversationStartRequest {
        provider: Arc::new(PendingSetupProvider {
            entered: entered.clone(),
            release: release.clone(),
        }),
        config: LiveSessionConfig {
            model: "setup-gated-test-model".to_string(),
            voice_name: None,
            system_instruction: None,
            sample_rate_in: 16_000,
            sample_rate_out: 24_000,
            tools: vec![],
        },
        asr_mode: AsrMode::GeminiLiveAudio,
        moonshine_installer: None,
        capture: capture.clone(),
        input_device: None,
        playback: Arc::new(AudioPlayback::new()),
        output_device: None,
        muted: Arc::new(RwLock::new(false)),
        tool_router: tool_router(),
        callbacks: ConversationCallbacks::new(|_| {}, |_| {}, |_, _, _| {}, |_| {}, |_| {}, |_| {}),
    };

    let manager_for_start = manager.clone();
    let start = tokio::spawn(async move { manager_for_start.start_session(request).await });

    entered.notified().await;
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Connecting);
    assert!(!manager.is_active());
    assert_eq!(manager.current_session_id(), None);
    assert!(!capture.lock().is_active());

    release.notify_one();
    let error = start
        .await
        .expect("startup task")
        .expect_err("setup rejection must fail startup");
    assert!(error.contains("could not be configured"));
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Failed);
    assert!(!manager.is_active());
    assert!(!capture.lock().is_active());
}
