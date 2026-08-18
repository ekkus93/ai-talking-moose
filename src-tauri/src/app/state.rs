use crate::ai::fake::{FakeConversationProvider, FakeSpeechSynthesizer, FakeTextModel};
use crate::ai::google::{GoogleAuth, GoogleLiveProvider, GoogleSpeechSynthesizer, GoogleTextModel};
use crate::ai::traits::{RealtimeConversationProvider, SpeechSynthesizer, TextModel};
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::character::behavior::BehaviorEngine;
use crate::character::personality::CharacterConfig;
use crate::character::state::CharacterState;
use crate::conversation::session::ConversationManager;
use crate::memory::MemoryManager;
use crate::persistence::sqlite::Database;
use crate::secrets::SecretStore;
use crate::tools::builtin::BuiltinTools;
use crate::tools::router::ToolRouter;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    // General
    pub launch_at_login: bool,
    pub show_in_menu_bar: bool,
    pub always_on_top: bool,
    pub restore_position: bool,

    // Behavior
    pub unsolicited_comments: bool,
    pub talkativeness: f32,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: u8,
    pub quiet_hours_end: u8,
    pub max_comments_per_hour: u32,
    pub hide_delay_seconds: u32,

    // Audio & Voice
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub volume: f32,
    pub tts_voice: String,
    pub speaking_rate: f32,
    pub pitch: f32,

    // AI Configuration
    pub provider: String, // "google" or "fake"
    pub live_model: String,
    pub text_model: String,
    pub tts_model: String,

    // Privacy
    pub microphone_permission_granted: bool,
    pub active_app_observation: bool,
    pub window_title_observation: bool,
    pub memory_enabled: bool,
    pub save_transcripts: bool,

    // Character Personality
    pub dry: f32,
    pub sarcastic: f32,
    pub friendly: f32,
    pub absurd: f32,
    pub helpful: f32,
    pub verbosity: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            show_in_menu_bar: true,
            always_on_top: false,
            restore_position: true,

            unsolicited_comments: true,
            talkativeness: 0.5,
            quiet_hours_enabled: true,
            quiet_hours_start: 22,
            quiet_hours_end: 8,
            max_comments_per_hour: 4,
            hide_delay_seconds: 6,

            input_device: None,
            output_device: None,
            volume: 1.0,
            tts_voice: "Fenrir".to_string(),
            speaking_rate: 0.95,
            pitch: -1.5,

            provider: "google".to_string(),
            live_model: "gemini-2.5-flash-native-audio-latest".to_string(),
            text_model: "gemini-2.5-flash".to_string(),
            tts_model: "en-US-Standard-B".to_string(),

            microphone_permission_granted: true,
            active_app_observation: true,
            window_title_observation: false,
            memory_enabled: true,
            save_transcripts: true,

            dry: 0.85,
            sarcastic: 0.70,
            friendly: 0.55,
            absurd: 0.65,
            helpful: 0.35,
            verbosity: 0.30,
        }
    }
}

pub struct AppState {
    pub db: Arc<Database>,
    pub memory: Arc<MemoryManager>,
    pub secrets: Arc<SecretStore>,
    pub character_state: Arc<RwLock<CharacterState>>,
    pub behavior_engine: Arc<Mutex<BehaviorEngine>>,
    pub audio_capture: Arc<Mutex<AudioCapture>>,
    pub audio_playback: Arc<AudioPlayback>,
    pub conversation_mgr: Arc<ConversationManager>,
    pub tool_router: Arc<ToolRouter>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub is_muted: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new(db_path: Option<&str>) -> Self {
        let db = if let Some(path) = db_path {
            Arc::new(Database::new(path).unwrap_or_else(|_| Database::new_in_memory().unwrap()))
        } else {
            Arc::new(Database::new_in_memory().unwrap())
        };

        let memory = Arc::new(MemoryManager::new(db.clone()));
        let secrets = Arc::new(SecretStore::new());
        let settings = Arc::new(RwLock::new(AppSettings::default()));

        // Load persisted settings from SQLite if present
        if let Ok(Some(json_str)) = db.get_setting("app_settings") {
            if let Ok(loaded) = serde_json::from_str::<AppSettings>(&json_str) {
                *settings.write() = loaded;
            }
        }

        // Load stored API key if present
        if let Ok(Some(key)) = db.get_setting("google_api_key") {
            secrets.set_google_api_key(key);
        }

        let character_config = CharacterConfig::default();
        let behavior_engine = Arc::new(Mutex::new(BehaviorEngine::new(character_config.clone())));
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new()));
        let audio_playback = Arc::new(AudioPlayback::new());
        let conversation_mgr = Arc::new(ConversationManager::new());

        let builtin_tools = Arc::new(BuiltinTools {
            memory_manager: memory.clone(),
            character_config,
            active_app_permitted: true,
        });
        let tool_router = Arc::new(ToolRouter::new(builtin_tools));

        Self {
            db,
            memory,
            secrets,
            character_state: Arc::new(RwLock::new(CharacterState::Idle)),
            behavior_engine,
            audio_capture,
            audio_playback,
            conversation_mgr,
            tool_router,
            settings,
            is_muted: Arc::new(RwLock::new(false)),
        }
    }

    pub fn get_text_model(&self) -> Box<dyn TextModel> {
        let settings = self.settings.read();
        if settings.provider == "fake" || !self.secrets.has_google_api_key() {
            Box::new(FakeTextModel::default())
        } else {
            let key = self.secrets.get_google_api_key().unwrap_or_default();
            Box::new(GoogleTextModel::new(
                GoogleAuth::new(key),
                settings.text_model.clone(),
            ))
        }
    }

    pub fn get_speech_synthesizer(&self) -> Box<dyn SpeechSynthesizer> {
        let settings = self.settings.read();
        if settings.provider == "fake" || !self.secrets.has_google_api_key() {
            Box::new(FakeSpeechSynthesizer)
        } else {
            let key = self.secrets.get_google_api_key().unwrap_or_default();
            Box::new(GoogleSpeechSynthesizer::new(
                GoogleAuth::new(key),
                settings.tts_voice.clone(),
            ))
        }
    }

    pub fn get_live_provider(&self) -> Arc<dyn RealtimeConversationProvider> {
        let settings = self.settings.read();
        if settings.provider == "fake" || !self.secrets.has_google_api_key() {
            Arc::new(FakeConversationProvider)
        } else {
            let key = self.secrets.get_google_api_key().unwrap_or_default();
            Arc::new(GoogleLiveProvider::new(GoogleAuth::new(key)))
        }
    }
}
