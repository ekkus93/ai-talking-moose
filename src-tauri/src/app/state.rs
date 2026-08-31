use crate::ai::google::{
    normalize_live_model, normalize_text_model, normalize_tts_model, GoogleAuth,
    GoogleLiveProvider, GoogleSpeechSynthesizer, GoogleTextModel, DEFAULT_LIVE_MODEL,
    DEFAULT_TEXT_MODEL, DEFAULT_TTS_MODEL,
};
use crate::ai::local::{UnavailableLocalTextModel, DEFAULT_LOCAL_TEXT_MODEL_ID};
use crate::ai::traits::{RealtimeConversationProvider, SpeechSynthesizer, TextModel};
use crate::ai::types::TextProvider;
use crate::asr::moonshine::MoonshineModelInstaller;
use crate::asr::AsrMode;
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::audio::speech::StandaloneSpeechController;
use crate::character::ambient::AmbientScheduler;
use crate::character::behavior::BehaviorEngine;
use crate::character::personality::CharacterConfig;
use crate::character::state::CharacterState;
use crate::conversation::session::ConversationManager;
use crate::memory::MemoryManager;
use crate::persistence::sqlite::Database;
#[cfg(test)]
use crate::secrets::MemorySecretBackend;
use crate::secrets::SecretStore;
use crate::tools::builtin::BuiltinTools;
use crate::tools::router::ToolRouter;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

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
    pub text_provider: TextProvider,
    pub live_model: String,
    pub google_text_model: String,
    pub local_text_model: String,
    pub tts_model: String,

    // Privacy
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

pub const CURRENT_SETTINGS_VERSION: u32 = 3;

pub const CURRENT_ONBOARDING_VERSION: u32 = 1;
const ONBOARDING_ACKNOWLEDGED_VERSION_SETTING: &str = "onboarding_acknowledged_version";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnboardingStatus {
    pub current_version: u32,
    pub acknowledged_version: Option<u32>,
    pub needs_acknowledgement: bool,
}

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

            // Keep Google as the staged default until the local installer/runtime/UX is usable.
            // LLM-013 revisits the new-profile default after real CPU model acceptance.
            text_provider: TextProvider::Google,
            live_model: DEFAULT_LIVE_MODEL.to_string(),
            google_text_model: DEFAULT_TEXT_MODEL.to_string(),
            local_text_model: DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
            tts_model: DEFAULT_TTS_MODEL.to_string(),

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
    /// created before an ASR selector or explicit text-provider selector existed.
    /// New profiles default to local Moonshine ASR while legacy profiles without an
    /// ASR selector migrate to Gemini Live audio because that was their only microphone path.
    pub fn from_persisted_json(json: &str) -> Result<(Self, bool), serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let had_asr_mode = value.get("asr_mode").is_some();
        let had_legacy_microphone_permission = value.get("microphone_permission_granted").is_some();
        let had_current_version = value
            .get("settings_version")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(CURRENT_SETTINGS_VERSION));
        let had_enabled_window_title_observation = value
            .get("window_title_observation")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        let had_text_provider = value.get("text_provider").is_some();
        let had_google_text_model = value.get("google_text_model").is_some();
        let had_local_text_model = value.get("local_text_model").is_some();
        let had_legacy_provider = value.get("provider").is_some();
        let legacy_text_model = value
            .get("text_model")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let had_legacy_text_model = legacy_text_model.is_some();

        let mut settings: Self = serde_json::from_value(value)?;
        if !had_asr_mode {
            settings.asr_mode = AsrMode::GeminiLiveAudio;
        }
        if !had_text_provider {
            // Existing installations were Google text users. Never reinterpret an old
            // profile as Local merely because the new selector did not exist yet.
            settings.text_provider = TextProvider::Google;
        }
        if !had_google_text_model {
            if let Some(legacy_text_model) = legacy_text_model {
                settings.google_text_model = legacy_text_model;
            }
        }

        let normalized_live_model = normalize_live_model(&settings.live_model).to_string();
        let normalized_google_text_model =
            normalize_text_model(&settings.google_text_model).to_string();
        let normalized_tts_model = normalize_tts_model(&settings.tts_model).to_string();
        let models_migrated = normalized_live_model != settings.live_model
            || normalized_google_text_model != settings.google_text_model
            || normalized_tts_model != settings.tts_model;
        settings.live_model = normalized_live_model;
        settings.google_text_model = normalized_google_text_model;
        settings.tts_model = normalized_tts_model;

        // Window-title observation is an unsupported V1 compatibility field, not an
        // authoritative preference. Persist it fail-closed even for legacy profiles.
        settings.window_title_observation = false;
        settings.settings_version = CURRENT_SETTINGS_VERSION;

        Ok((
            settings,
            !had_asr_mode
                || had_legacy_microphone_permission
                || !had_current_version
                || !had_text_provider
                || !had_google_text_model
                || !had_local_text_model
                || had_legacy_provider
                || had_legacy_text_model
                || models_migrated
                || had_enabled_window_title_observation,
        ))
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

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub memory: Arc<MemoryManager>,
    pub secrets: Arc<SecretStore>,
    pub character_state: Arc<RwLock<CharacterState>>,
    pub behavior_engine: Arc<Mutex<BehaviorEngine>>,
    pub ambient_scheduler: AmbientScheduler,
    pub audio_capture: Arc<Mutex<AudioCapture>>,
    pub audio_playback: Arc<AudioPlayback>,
    pub standalone_speech: StandaloneSpeechController,
    pub conversation_mgr: Arc<ConversationManager>,
    pub moonshine_installer: Arc<MoonshineModelInstaller>,
    pub tool_router: Arc<ToolRouter>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub is_muted: Arc<RwLock<bool>>,
}

const LEGACY_GOOGLE_API_KEY_SETTING: &str = "google_api_key";

fn moonshine_model_root(db_path: Option<&str>) -> PathBuf {
    let Some(db_path) = db_path else {
        return std::env::temp_dir()
            .join("talking-moose-ai-tests")
            .join("models")
            .join("moonshine");
    };

    Path::new(db_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("models")
        .join("moonshine")
}

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
        if let Err(error) = secrets.set_google_api_key(trimmed.to_string()) {
            // A transient Keychain failure must not brick application startup. Keep
            // the legacy row as the only known-good copy and retry migration later.
            warn!(error = %error, "Deferring legacy Google API key migration because the secure store is unavailable");
            return Ok(());
        }
        if secrets.get_google_api_key().as_deref() != Some(trimmed) {
            warn!(
                "Deferring legacy Google API key migration because secure credential verification failed"
            );
            return Ok(());
        }
    }

    // Delete plaintext only after the secure store reports a usable credential.
    db.delete_setting(LEGACY_GOOGLE_API_KEY_SETTING)
        .map_err(|error| error.to_string())?;
    Ok(())
}

impl AppState {
    pub fn onboarding_status(&self) -> Result<OnboardingStatus, String> {
        let acknowledged_version = self
            .db
            .get_setting(ONBOARDING_ACKNOWLEDGED_VERSION_SETTING)
            .map_err(|error| error.to_string())?
            .and_then(|value| value.parse::<u32>().ok());
        Ok(OnboardingStatus {
            current_version: CURRENT_ONBOARDING_VERSION,
            acknowledged_version,
            needs_acknowledgement: acknowledged_version != Some(CURRENT_ONBOARDING_VERSION),
        })
    }

    pub fn acknowledge_current_onboarding(&self) -> Result<OnboardingStatus, String> {
        self.db
            .set_setting(
                ONBOARDING_ACKNOWLEDGED_VERSION_SETTING,
                &CURRENT_ONBOARDING_VERSION.to_string(),
            )
            .map_err(|error| error.to_string())?;
        self.onboarding_status()
    }

    pub fn new(db_path: Option<&str>) -> Result<Self, String> {
        Self::new_with_secret_store(db_path, SecretStore::new()?)
    }

    #[cfg(test)]
    pub(crate) fn new_for_tests() -> Result<Self, String> {
        let secret_store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default()))?;
        Self::new_with_secret_store(None, secret_store)
    }

    fn new_with_secret_store(
        db_path: Option<&str>,
        secret_store: SecretStore,
    ) -> Result<Self, String> {
        let db =
            if let Some(path) = db_path {
                Arc::new(Database::new(path).map_err(|error| {
                    format!("failed to initialize persistent database: {error}")
                })?)
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
                    let normalized =
                        serde_json::to_string(&loaded).map_err(|error| error.to_string())?;
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
        audio_playback.set_volume(settings.read().volume);
        let standalone_speech = StandaloneSpeechController::new();
        let conversation_mgr = Arc::new(ConversationManager::new());
        let moonshine_installer = Arc::new(
            MoonshineModelInstaller::new(moonshine_model_root(db_path))
                .map_err(|error| error.to_string())?,
        );

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
            ambient_scheduler: AmbientScheduler::new(),
            audio_capture,
            audio_playback,
            standalone_speech,
            conversation_mgr,
            moonshine_installer,
            tool_router,
            settings,
            is_muted: Arc::new(RwLock::new(false)),
        })
    }

    pub fn get_text_model(&self) -> Box<dyn TextModel> {
        let settings = self.settings.read();
        match settings.text_provider {
            TextProvider::Google => {
                let key = self.secrets.get_google_api_key().unwrap_or_default();
                Box::new(GoogleTextModel::new(
                    GoogleAuth::new(key),
                    settings.google_text_model.clone(),
                ))
            }
            TextProvider::Local => Box::new(UnavailableLocalTextModel::new(
                settings.local_text_model.clone(),
            )),
        }
    }

    pub fn get_speech_synthesizer(&self) -> Box<dyn SpeechSynthesizer> {
        let settings = self.settings.read();
        let key = self.secrets.get_google_api_key().unwrap_or_default();
        Box::new(GoogleSpeechSynthesizer::new(
            GoogleAuth::new(key),
            settings.tts_model.clone(),
            settings.tts_voice.clone(),
        ))
    }

    pub fn get_live_provider(&self) -> Arc<dyn RealtimeConversationProvider> {
        let key = self.secrets.get_google_api_key().unwrap_or_default();
        Arc::new(GoogleLiveProvider::new(GoogleAuth::new(key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::{ProviderErrorKind, TextRequest, TtsRequest};
    use crate::secrets::{MemorySecretBackend, SecretBackend};
    use std::sync::Arc;
    use tempfile::{tempdir, NamedTempFile};

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
    fn new_settings_default_to_moonshine_tiny_and_staged_google_text() {
        let settings = AppSettings::default();
        assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
        assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);
        assert_eq!(settings.text_provider, TextProvider::Google);
        assert_eq!(settings.google_text_model, DEFAULT_TEXT_MODEL);
        assert_eq!(settings.local_text_model, DEFAULT_LOCAL_TEXT_MODEL_ID);
        assert!(!settings.active_app_observation);
        assert!(!settings.memory_enabled);
        assert!(!settings.save_transcripts);
    }

    #[test]
    fn onboarding_acknowledgement_is_versioned_and_independent_of_settings() {
        let state = AppState::new_for_tests().unwrap();
        let initial = state.onboarding_status().unwrap();
        assert_eq!(initial.current_version, CURRENT_ONBOARDING_VERSION);
        assert_eq!(initial.acknowledged_version, None);
        assert!(initial.needs_acknowledgement);

        state
            .db
            .set_setting(ONBOARDING_ACKNOWLEDGED_VERSION_SETTING, "0")
            .unwrap();
        assert!(state.onboarding_status().unwrap().needs_acknowledgement);

        let acknowledged = state.acknowledge_current_onboarding().unwrap();
        assert_eq!(
            acknowledged.acknowledged_version,
            Some(CURRENT_ONBOARDING_VERSION)
        );
        assert!(!acknowledged.needs_acknowledgement);
        assert!(!state.settings.read().memory_enabled);
        assert!(!state.settings.read().save_transcripts);
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
    fn version_two_text_settings_migrate_to_google_without_persisting_fake_provider() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert("settings_version".to_string(), serde_json::json!(2));
        object.remove("text_provider");
        object.remove("google_text_model");
        object.remove("local_text_model");
        object.insert("provider".to_string(), serde_json::json!("fake"));
        object.insert(
            "text_model".to_string(),
            serde_json::json!("gemini-3.6-flash"),
        );

        let (settings, migrated) =
            AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert!(migrated);
        assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
        assert_eq!(settings.text_provider, TextProvider::Google);
        assert_eq!(settings.google_text_model, "gemini-3.6-flash");
        assert_eq!(settings.local_text_model, DEFAULT_LOCAL_TEXT_MODEL_ID);

        let normalized = serde_json::to_value(settings).unwrap();
        assert!(normalized.get("provider").is_none());
        assert!(normalized.get("text_model").is_none());
        assert_eq!(normalized["text_provider"], "google");
    }

    #[test]
    fn migrated_text_provider_settings_are_idempotent() {
        let original = AppSettings {
            text_provider: TextProvider::Local,
            google_text_model: "gemini-3.6-flash".to_string(),
            local_text_model: "local-catalog-id".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&original).unwrap();

        let (settings, migrated) = AppSettings::from_persisted_json(&json).unwrap();
        assert!(!migrated);
        assert_eq!(settings.text_provider, TextProvider::Local);
        assert_eq!(settings.google_text_model, "gemini-3.6-flash");
        assert_eq!(settings.local_text_model, "local-catalog-id");
    }

    #[test]
    fn migrated_cloud_audio_profile_still_requires_current_onboarding() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let db = Database::new(&path).unwrap();
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("settings_version");
        object.remove("asr_mode");
        db.set_setting("app_settings", &serde_json::to_string(&value).unwrap())
            .unwrap();
        drop(db);

        let secret_store =
            SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
        let state = AppState::new_with_secret_store(Some(&path), secret_store).unwrap();

        assert_eq!(state.settings.read().asr_mode, AsrMode::GeminiLiveAudio);
        assert!(state.onboarding_status().unwrap().needs_acknowledgement);
    }

    #[test]
    fn legacy_tts_model_is_normalized_to_current_configured_model() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value.as_object_mut().unwrap().insert(
            "tts_model".to_string(),
            serde_json::Value::String("en-US-Standard-B".to_string()),
        );
        let (settings, migrated) =
            AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert!(migrated);
        assert_eq!(settings.tts_model, DEFAULT_TTS_MODEL);
    }

    #[test]
    fn unsupported_window_title_setting_is_normalized_fail_closed() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value.as_object_mut().unwrap().insert(
            "window_title_observation".to_string(),
            serde_json::Value::Bool(true),
        );
        let (settings, migrated) =
            AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert!(migrated);
        assert!(!settings.window_title_observation);
    }

    #[test]
    fn legacy_microphone_permission_cache_is_removed_during_normalization() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value.as_object_mut().unwrap().insert(
            "microphone_permission_granted".to_string(),
            serde_json::Value::Bool(true),
        );
        let json = serde_json::to_string(&value).unwrap();

        let (settings, migrated) = AppSettings::from_persisted_json(&json).unwrap();
        assert!(migrated);
        let normalized = serde_json::to_value(settings).unwrap();
        assert!(normalized.get("microphone_permission_granted").is_none());
    }

    #[test]
    fn frontend_contract_matches_authoritative_rust_defaults_and_catalogs() {
        let contract: serde_json::Value =
            serde_json::from_str(include_str!("../../../src/generated/backendContract.json"))
                .unwrap();

        let contract_settings = contract.get("settings").unwrap();
        let authoritative_settings = serde_json::to_value(AppSettings::default()).unwrap();
        let contract_keys = contract_settings.as_object().unwrap();
        let authoritative_keys = authoritative_settings.as_object().unwrap();
        assert_eq!(contract_keys.len(), authoritative_keys.len());
        assert!(authoritative_keys
            .keys()
            .all(|key| contract_keys.contains_key(key)));

        let decoded_settings: AppSettings =
            serde_json::from_value(contract_settings.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(decoded_settings).unwrap(),
            authoritative_settings
        );
        assert_eq!(
            contract.get("google_models").unwrap(),
            &serde_json::to_value(crate::ai::google::GOOGLE_MODELS).unwrap()
        );
        assert_eq!(
            contract.get("google_tts_voices").unwrap(),
            &serde_json::to_value(crate::ai::google::GOOGLE_TTS_VOICES).unwrap()
        );
    }

    #[test]
    fn current_settings_keep_selected_asr_mode() {
        let original = AppSettings {
            asr_mode: AsrMode::MoonshineSmallStreaming,
            ..Default::default()
        };
        let json = serde_json::to_string(&original).unwrap();

        let (settings, migrated) = AppSettings::from_persisted_json(&json).unwrap();
        assert!(!migrated);
        assert_eq!(settings.asr_mode, AsrMode::MoonshineSmallStreaming);
    }

    #[tokio::test]
    async fn configured_google_text_without_secret_fails_auth_instead_of_using_fake_provider() {
        let state = AppState::new_for_tests().unwrap();
        let error = state
            .get_text_model()
            .generate(TextRequest {
                prompt: "must fail auth".to_string(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(8),
            })
            .await
            .expect_err("configured Google text without a key must fail closed");
        assert_eq!(error.kind, ProviderErrorKind::Auth);
    }

    #[tokio::test]
    async fn configured_local_text_without_runtime_fails_model_without_cloud_fallback() {
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().text_provider = TextProvider::Local;
        state.settings.write().local_text_model = "missing-local-model".to_string();

        let error = state
            .get_text_model()
            .generate(TextRequest {
                prompt: "must stay local".to_string(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(8),
            })
            .await
            .expect_err("unavailable Local text must fail rather than call Google or Fake");
        assert_eq!(error.kind, ProviderErrorKind::Model);
    }

    #[tokio::test]
    async fn configured_google_tts_without_secret_fails_auth_instead_of_using_fake_speech() {
        let state = AppState::new_for_tests().unwrap();
        let error = state
            .get_speech_synthesizer()
            .synthesize(TtsRequest {
                text: "must fail auth".to_string(),
                voice_name: None,
                speaking_rate: None,
                pitch: None,
            })
            .await
            .expect_err("configured Google TTS without a key must fail closed");
        assert_eq!(error.kind, ProviderErrorKind::Auth);
    }

    #[tokio::test]
    async fn configured_google_live_without_secret_fails_auth_instead_of_using_fake_provider() {
        let state = AppState::new_for_tests().unwrap();
        let provider = state.get_live_provider();
        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(1);
        let error = provider
            .connect(
                crate::ai::types::LiveSessionConfig {
                    model: "test-model".to_string(),
                    voice_name: None,
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: vec![],
                },
                event_tx,
            )
            .await
            .err()
            .expect("configured Google provider without a key must fail closed");

        assert_eq!(error.kind, ProviderErrorKind::Auth);
    }

    #[test]
    fn settings_apply_to_live_character_config() {
        let settings = AppSettings {
            dry: 0.11,
            sarcastic: 0.22,
            friendly: 0.33,
            absurd: 0.44,
            helpful: 0.55,
            verbosity: 0.66,
            talkativeness: 0.77,
            unsolicited_comments: false,
            quiet_hours_enabled: false,
            quiet_hours_start: 1,
            quiet_hours_end: 6,
            max_comments_per_hour: 2,
            ..Default::default()
        };

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
        let persisted = AppSettings {
            talkativeness: 0.91,
            unsolicited_comments: false,
            quiet_hours_enabled: false,
            ..Default::default()
        };
        db.set_setting("app_settings", &serde_json::to_string(&persisted).unwrap())
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
    fn persistent_database_init_failure_never_falls_back_to_memory() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(MemorySecretBackend::default());
        let secret_store = SecretStore::with_backend(backend).unwrap();

        let result = AppState::new_with_secret_store(
            Some(dir.path().to_string_lossy().as_ref()),
            secret_store,
        );

        let error = match result {
            Ok(_) => panic!("a persistent database failure must abort startup"),
            Err(error) => error,
        };
        assert!(error.contains("failed to initialize persistent database"));
    }

    #[test]
    fn failing_secure_store_during_legacy_migration_does_not_abort_startup() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let db = Database::new(&path).unwrap();
        db.seed_legacy_setting_for_test(LEGACY_GOOGLE_API_KEY_SETTING, "AIzaSyOnlyCopyMustSurvive")
            .unwrap();
        drop(db);

        let secret_store = SecretStore::with_backend(Arc::new(RejectWriteBackend)).unwrap();
        let state = AppState::new_with_secret_store(Some(&path), secret_store)
            .expect("transient secure-store migration failure must not abort startup");

        assert_eq!(
            state
                .db
                .get_setting(LEGACY_GOOGLE_API_KEY_SETTING)
                .unwrap()
                .as_deref(),
            Some("AIzaSyOnlyCopyMustSurvive")
        );
        assert!(!state.secrets.has_google_api_key());
    }
}
