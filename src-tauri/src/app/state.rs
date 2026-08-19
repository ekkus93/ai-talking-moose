use crate::ai::fake::{FakeConversationProvider, FakeSpeechSynthesizer, FakeTextModel};
use crate::ai::google::{GoogleAuth, GoogleLiveProvider, GoogleSpeechSynthesizer, GoogleTextModel};
use crate::ai::traits::{RealtimeConversationProvider, SpeechSynthesizer, TextModel};
use crate::asr::AsrMode;
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
#[serde(default)]
pub struct AppSettings {
    pub settings_version: u32,
    pub asr_mode: AsrMode,

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

pub const CURRENT_SETTINGS_VERSION: u32 = 1;

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            settings_version: CURRENT_SETTINGS_VERSION,
            asr_mode: AsrMode::MoonshineTinyStreaming,

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
            active_app_observation: false,
            window_title_observation: false,
            memory_enabled: false,
            save_transcripts: false,

            dry: 0.85,
            sarcastic: 0.70,
            friendly: 0.55,
            absurd: 0.65,
            helpful: 0.35,
            verbosity: 0.30,
        }
    }
}

impl AppSettings {
    /// Deserialize persisted settings while preserving the behavior of installations
    /// created before an ASR selector existed. New profiles default to local
    /// Moonshine Tiny Streaming, while legacy profiles migrate to Gemini Live audio
    /// because that was the only microphone-recognition path they previously used.
    pub fn from_persisted_json(json: &str) -> Result<(Self, bool), serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let had_asr_mode = value.get("asr_mode").is_some();
        let had_current_version = value
            .get("settings_version")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(CURRENT_SETTINGS_VERSION));

        let mut settings: Self = serde_json::from_value(value)?;
        if !had_asr_mode {
            settings.asr_mode = AsrMode::GeminiLiveAudio;
        }
        settings.settings_version = CURRENT_SETTINGS_VERSION;

        Ok((settings, !had_asr_mode || !had_current_version))
    }

    /// Apply user-editable behavior/personality settings to the live character config.
    /// This keeps persisted/frontend settings and the Rust behavior/prompt policy in sync.
    pub fn apply_to_character_config(&self, config: &mut CharacterConfig) {
        config.personality.dry = self.dry;
        config.personality.sarcastic = self.sarcastic;
        config.personality.friendly = self.friendly;
        config.personality.absurd = self.absurd;
        config.personality.helpful = self.helpful;
        config.personality.verbosity = self.verbosity;
        config.personality.talkativeness = self.talkativeness;

        config.behavior.unsolicited_comments = self.unsolicited_comments;
        config.behavior.quiet_hours_enabled = self.quiet_hours_enabled;
        config.behavior.quiet_hours_start = self.quiet_hours_start;
        config.behavior.quiet_hours_end = self.quiet_hours_end;
        config.behavior.max_comments_per_hour = self.max_comments_per_hour;
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

const LEGACY_GOOGLE_API_KEY_SETTING: &str = "google_api_key";

fn migrate_legacy_google_api_key(db: &Database, secrets: &SecretStore) -> Result<(), String> {
    let Some(legacy_key) = db
        .get_setting(LEGACY_GOOGLE_API_KEY_SETTING)
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };

    let trimmed = legacy_key.trim();
    if trimmed.is_empty() {
        db.delete_setting(LEGACY_GOOGLE_API_KEY_SETTING)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    if !secrets.has_google_api_key() {
        secrets.set_google_api_key(trimmed.to_string())?;
        if secrets.get_google_api_key().as_deref() != Some(trimmed) {
            return Err("secure credential verification failed during migration".to_string());
        }
    }

    // Delete plaintext only after the secure store reports a usable credential.
    db.delete_setting(LEGACY_GOOGLE_API_KEY_SETTING)
        .map_err(|error| error.to_string())?;
    Ok(())
}

impl AppState {
    pub fn new(db_path: Option<&str>) -> Result<Self, String> {
        Self::new_with_secret_store(db_path, SecretStore::new()?)
    }

    fn new_with_secret_store(
        db_path: Option<&str>,
        secret_store: SecretStore,
    ) -> Result<Self, String> {
        let db = if let Some(path) = db_path {
            Arc::new(Database::new(path).unwrap_or_else(|_| Database::new_in_memory().unwrap()))
        } else {
            Arc::new(Database::new_in_memory().map_err(|error| error.to_string())?)
        };

        let memory = Arc::new(MemoryManager::new(db.clone()));
        let secrets = Arc::new(secret_store);
        migrate_legacy_google_api_key(&db, &secrets)?;

        let settings = Arc::new(RwLock::new(AppSettings::default()));

        // Load persisted settings from SQLite if present. Persist a normalized copy
        // when a version migration occurs so future starts do not repeat it.
        if let Ok(Some(json_str)) = db.get_setting("app_settings") {
            if let Ok((loaded, migrated)) = AppSettings::from_persisted_json(&json_str) {
                *settings.write() = loaded.clone();
                if migrated {
                    let normalized = serde_json::to_string(&loaded)
                        .map_err(|error| error.to_string())?;
                    db.set_setting("app_settings", &normalized)
                        .map_err(|error| error.to_string())?;
                }
            }
        }

        let mut character_config = CharacterConfig::default();
        settings
            .read()
            .apply_to_character_config(&mut character_config);
        let behavior_engine = Arc::new(Mutex::new(BehaviorEngine::new(character_config.clone())));
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new()));
        let audio_playback = Arc::new(AudioPlayback::new());
        let conversation_mgr = Arc::new(ConversationManager::new());

        let builtin_tools = Arc::new(BuiltinTools {
            memory_manager: memory.clone(),
            character_config,
            settings: settings.clone(),
        });
        let tool_router = Arc::new(ToolRouter::new(builtin_tools));

        Ok(Self {
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
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{MemorySecretBackend, SecretBackend};
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    #[derive(Default)]
    struct RejectWriteBackend;

    impl SecretBackend for RejectWriteBackend {
        fn read_google_api_key(&self) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn write_google_api_key(&self, _key: &str) -> Result<(), String> {
            Err("injected secure-store write failure".to_string())
        }

        fn delete_google_api_key(&self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn new_settings_default_to_moonshine_tiny() {
        let settings = AppSettings::default();
        assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
        assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);
        assert!(!settings.active_app_observation);
        assert!(!settings.memory_enabled);
        assert!(!settings.save_transcripts);
    }

    #[test]
    fn legacy_settings_migrate_to_gemini_live_audio() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("settings_version");
        object.remove("asr_mode");
        let json = serde_json::to_string(&value).unwrap();

        let (settings, migrated) = AppSettings::from_persisted_json(&json).unwrap();
        assert!(migrated);
        assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
        assert_eq!(settings.asr_mode, AsrMode::GeminiLiveAudio);
    }

    #[test]
    fn current_settings_keep_selected_asr_mode() {
        let mut original = AppSettings::default();
        original.asr_mode = AsrMode::MoonshineSmallStreaming;
        let json = serde_json::to_string(&original).unwrap();

        let (settings, migrated) = AppSettings::from_persisted_json(&json).unwrap();
        assert!(!migrated);
        assert_eq!(settings.asr_mode, AsrMode::MoonshineSmallStreaming);
    }

    #[test]
    fn settings_apply_to_live_character_config() {
        let mut settings = AppSettings::default();
        settings.dry = 0.11;
        settings.sarcastic = 0.22;
        settings.friendly = 0.33;
        settings.absurd = 0.44;
        settings.helpful = 0.55;
        settings.verbosity = 0.66;
        settings.talkativeness = 0.77;
        settings.unsolicited_comments = false;
        settings.quiet_hours_enabled = false;
        settings.quiet_hours_start = 1;
        settings.quiet_hours_end = 6;
        settings.max_comments_per_hour = 2;

        let mut config = CharacterConfig::default();
        settings.apply_to_character_config(&mut config);

        assert_eq!(config.personality.dry, 0.11);
        assert_eq!(config.personality.sarcastic, 0.22);
        assert_eq!(config.personality.friendly, 0.33);
        assert_eq!(config.personality.absurd, 0.44);
        assert_eq!(config.personality.helpful, 0.55);
        assert_eq!(config.personality.verbosity, 0.66);
        assert_eq!(config.personality.talkativeness, 0.77);
        assert!(!config.behavior.unsolicited_comments);
        assert!(!config.behavior.quiet_hours_enabled);
        assert_eq!(config.behavior.quiet_hours_start, 1);
        assert_eq!(config.behavior.quiet_hours_end, 6);
        assert_eq!(config.behavior.max_comments_per_hour, 2);
    }

    #[test]
    fn persisted_settings_seed_behavior_engine_on_startup() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let db = Database::new(&path).unwrap();
        let mut persisted = AppSettings::default();
        persisted.talkativeness = 0.91;
        persisted.unsolicited_comments = false;
        persisted.quiet_hours_enabled = false;
        db.set_setting(
            "app_settings",
            &serde_json::to_string(&persisted).unwrap(),
        )
        .unwrap();
        drop(db);

        let backend = Arc::new(MemorySecretBackend::default());
        let secret_store = SecretStore::with_backend(backend).unwrap();
        let state = AppState::new_with_secret_store(Some(&path), secret_store).unwrap();
        let engine = state.behavior_engine.lock();

        assert_eq!(engine.config.personality.talkativeness, 0.91);
        assert!(!engine.config.behavior.unsolicited_comments);
        assert!(!engine.config.behavior.quiet_hours_enabled);
    }

    #[test]
    fn legacy_plaintext_google_key_moves_to_secure_backend_and_is_deleted() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let db = Database::new(&path).unwrap();
        db.seed_legacy_setting_for_test(
            LEGACY_GOOGLE_API_KEY_SETTING,
            "AIzaSyLegacyMigrationTestKey",
        )
        .unwrap();
        drop(db);

        let backend = Arc::new(MemorySecretBackend::default());
        let secret_store = SecretStore::with_backend(backend).unwrap();
        let state = AppState::new_with_secret_store(Some(&path), secret_store).unwrap();

        assert!(state.secrets.has_google_api_key());
        assert_eq!(
            state.secrets.get_google_api_key().as_deref(),
            Some("AIzaSyLegacyMigrationTestKey")
        );
        assert_eq!(
            state.db.get_setting(LEGACY_GOOGLE_API_KEY_SETTING).unwrap(),
            None
        );
        drop(state);

        let reopened = Database::new(&path).unwrap();
        assert_eq!(
            reopened.get_setting(LEGACY_GOOGLE_API_KEY_SETTING).unwrap(),
            None
        );
    }

    #[test]
    fn failed_secure_migration_preserves_legacy_plaintext_row() {
        let db = Database::new_in_memory().unwrap();
        db.seed_legacy_setting_for_test(
            LEGACY_GOOGLE_API_KEY_SETTING,
            "AIzaSyOnlyCopyMustSurvive",
        )
        .unwrap();

        let secret_store = SecretStore::with_backend(Arc::new(RejectWriteBackend)).unwrap();
        let result = migrate_legacy_google_api_key(&db, &secret_store);

        assert!(result.is_err());
        assert_eq!(
            db.get_setting(LEGACY_GOOGLE_API_KEY_SETTING)
                .unwrap()
                .as_deref(),
            Some("AIzaSyOnlyCopyMustSurvive")
        );
        assert!(!secret_store.has_google_api_key());
    }
}
