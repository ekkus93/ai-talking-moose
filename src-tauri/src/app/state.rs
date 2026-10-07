use crate::ai::google::{
    normalize_live_model, normalize_text_model, normalize_tts_model, normalize_tts_voice,
    GoogleAuth, GoogleLiveProvider, GoogleSpeechSynthesizer, DEFAULT_LIVE_MODEL,
    DEFAULT_TEXT_MODEL, DEFAULT_TTS_MODEL, DEFAULT_TTS_VOICE,
};
use crate::ai::local::{LocalRuntimeManager, DEFAULT_LOCAL_TEXT_MODEL_ID};
use crate::ai::local_tts::{
    LocalSpeechSynthesizer, LocalTtsRuntimeManager, DEFAULT_LOCAL_TTS_MODEL_ID,
    DEFAULT_LOCAL_TTS_VOICE,
};
use crate::ai::traits::{RealtimeConversationProvider, SpeechSynthesizer, TextModel};
use crate::ai::types::{TextProvider, TtsProvider};
use crate::app::wake_word_composition::WakeWordApplicationRuntime;
use crate::app::wake_word_settings::{
    WakeWordSettings, DEFAULT_WAKE_PHRASE, WAKE_WORD_ENABLED_FIELD, WAKE_WORD_PHRASE_FIELD,
};
use crate::app::wake_word_state::NativeWakeListenerController;
use crate::asr::moonshine::MoonshineModelInstaller;
use crate::asr::whisper::WhisperModelInstaller;
use crate::asr::AsrMode;
use crate::audio::capture::AudioCapture;
use crate::audio::playback::AudioPlayback;
use crate::audio::speech::StandaloneSpeechController;
use crate::character::ambient::AmbientScheduler;
use crate::character::behavior::BehaviorEngine;
use crate::character::idle_banter::{
    default_idle_banter_seed_topics, normalize_idle_banter_seed_topics, IdleBanterRuntime,
};
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
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub settings_version: u32,
    pub asr_mode: AsrMode,

    // Wake Word
    pub wake_word_enabled: bool,
    pub wake_word_phrase: String,

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
    pub idle_banter_enabled: bool,
    pub idle_banter_initial_delay_minutes: u32,
    pub idle_banter_repeat_interval_minutes: u32,
    pub idle_banter_seed_topics: Vec<String>,

    // Audio & Voice
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub volume: f32,
    pub tts_provider: TtsProvider,
    pub google_tts_voice: String,
    pub local_tts_voice: String,
    pub live_voice: String,
    pub speaking_rate: f32,
    pub pitch: f32,

    // AI Configuration
    pub text_provider: TextProvider,
    pub live_model: String,
    pub google_text_model: String,
    pub local_text_model: String,
    pub google_tts_model: String,
    pub local_tts_model: String,

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

pub const CURRENT_SETTINGS_VERSION: u32 = 5;

#[derive(Debug, Error)]
pub enum PersistedSettingsError {
    #[error(
        "persisted settings were written by newer application version {found}; this build supports settings version {current}"
    )]
    FutureVersion { found: u64, current: u32 },
    #[error("persisted settings JSON is invalid: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("persisted settings are invalid: {0}")]
    Invalid(String),
}

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

            wake_word_enabled: false,
            wake_word_phrase: DEFAULT_WAKE_PHRASE.to_string(),

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
            idle_banter_enabled: true,
            idle_banter_initial_delay_minutes: 60,
            idle_banter_repeat_interval_minutes: 30,
            idle_banter_seed_topics: default_idle_banter_seed_topics(),

            input_device: None,
            output_device: None,
            volume: 1.0,
            tts_provider: TtsProvider::Google,
            google_tts_voice: DEFAULT_TTS_VOICE.to_string(),
            local_tts_voice: DEFAULT_LOCAL_TTS_VOICE.to_string(),
            live_voice: DEFAULT_TTS_VOICE.to_string(),
            speaking_rate: 0.95,
            pitch: -1.5,

            // P12 real-CPU acceptance selected Local text as the new-profile default.
            // Existing pre-selector profiles still migrate explicitly to Google below.
            text_provider: TextProvider::Local,
            live_model: DEFAULT_LIVE_MODEL.to_string(),
            google_text_model: DEFAULT_TEXT_MODEL.to_string(),
            local_text_model: DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
            google_tts_model: DEFAULT_TTS_MODEL.to_string(),
            local_tts_model: DEFAULT_LOCAL_TTS_MODEL_ID.to_string(),

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
    pub fn from_persisted_json(json: &str) -> Result<(Self, bool), PersistedSettingsError> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        if let Some(persisted_version) = value
            .get("settings_version")
            .and_then(serde_json::Value::as_u64)
        {
            if persisted_version > u64::from(CURRENT_SETTINGS_VERSION) {
                return Err(PersistedSettingsError::FutureVersion {
                    found: persisted_version,
                    current: CURRENT_SETTINGS_VERSION,
                });
            }
        }
        let had_asr_mode = value.get("asr_mode").is_some();
        let had_wake_word_enabled = value.get(WAKE_WORD_ENABLED_FIELD).is_some();
        let had_wake_word_phrase = value.get(WAKE_WORD_PHRASE_FIELD).is_some();
        let wake_word_settings = WakeWordSettings::from_persisted_app_settings(&value)
            .map_err(|error| PersistedSettingsError::Invalid(error.to_string()))?;
        let had_legacy_microphone_permission = value.get("microphone_permission_granted").is_some();
        let had_current_version = value
            .get("settings_version")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(CURRENT_SETTINGS_VERSION));
        let had_idle_banter_enabled = value.get("idle_banter_enabled").is_some();
        let had_idle_banter_initial_delay =
            value.get("idle_banter_initial_delay_minutes").is_some();
        let had_idle_banter_repeat_interval =
            value.get("idle_banter_repeat_interval_minutes").is_some();
        let had_idle_banter_seed_topics = value.get("idle_banter_seed_topics").is_some();
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
        let had_tts_provider = value.get("tts_provider").is_some();
        let had_google_tts_model = value.get("google_tts_model").is_some();
        let had_google_tts_voice = value.get("google_tts_voice").is_some();
        let had_local_tts_model = value.get("local_tts_model").is_some();
        let had_local_tts_voice = value.get("local_tts_voice").is_some();
        let had_live_voice = value.get("live_voice").is_some();
        let legacy_tts_model = value
            .get("tts_model")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let had_legacy_tts_model = legacy_tts_model.is_some();
        let legacy_tts_voice = value
            .get("tts_voice")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let had_legacy_tts_voice = legacy_tts_voice.is_some();

        let mut settings: Self = serde_json::from_value(value)?;
        let wake_word_migrated = !had_wake_word_enabled
            || !had_wake_word_phrase
            || settings.wake_word_enabled != wake_word_settings.enabled
            || settings.wake_word_phrase != wake_word_settings.phrase;
        settings.wake_word_enabled = wake_word_settings.enabled;
        settings.wake_word_phrase = wake_word_settings.phrase;
        let legacy_idle_banter = !had_current_version
            || !had_idle_banter_enabled
            || !had_idle_banter_initial_delay
            || !had_idle_banter_repeat_interval
            || !had_idle_banter_seed_topics;
        let idle_banter_seed_repaired =
            match normalize_idle_banter_seed_topics(&settings.idle_banter_seed_topics) {
                Ok(normalized) => {
                    let changed = normalized != settings.idle_banter_seed_topics;
                    settings.idle_banter_seed_topics = normalized;
                    changed
                }
                Err(_) if legacy_idle_banter => {
                    settings.idle_banter_seed_topics = default_idle_banter_seed_topics();
                    true
                }
                Err(error) => return Err(PersistedSettingsError::Invalid(error)),
            };
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
        if !had_tts_provider {
            // Existing profiles predate Local TTS and therefore used Google standalone TTS.
            settings.tts_provider = TtsProvider::Google;
        }
        if !had_google_tts_model {
            if let Some(legacy_tts_model) = legacy_tts_model {
                settings.google_tts_model = legacy_tts_model;
            }
        }
        if !had_google_tts_voice {
            if let Some(legacy_tts_voice) = legacy_tts_voice.as_ref() {
                settings.google_tts_voice = legacy_tts_voice.clone();
            }
        }
        if !had_live_voice {
            if let Some(legacy_tts_voice) = legacy_tts_voice.as_ref() {
                settings.live_voice = legacy_tts_voice.clone();
            }
        }

        let normalized_live_model = normalize_live_model(&settings.live_model).to_string();
        let normalized_google_text_model =
            normalize_text_model(&settings.google_text_model).to_string();
        let normalized_google_tts_model =
            normalize_tts_model(&settings.google_tts_model).to_string();
        let normalized_google_tts_voice =
            normalize_tts_voice(&settings.google_tts_voice).to_string();
        let normalized_live_voice = normalize_tts_voice(&settings.live_voice).to_string();
        let models_migrated = normalized_live_model != settings.live_model
            || normalized_google_text_model != settings.google_text_model
            || normalized_google_tts_model != settings.google_tts_model
            || normalized_google_tts_voice != settings.google_tts_voice
            || normalized_live_voice != settings.live_voice;
        settings.live_model = normalized_live_model;
        settings.google_text_model = normalized_google_text_model;
        settings.google_tts_model = normalized_google_tts_model;
        settings.google_tts_voice = normalized_google_tts_voice;
        settings.live_voice = normalized_live_voice;

        // Window-title observation is an unsupported V1 compatibility field, not an
        // authoritative preference. Persist it fail-closed even for legacy profiles.
        settings.window_title_observation = false;
        settings.settings_version = CURRENT_SETTINGS_VERSION;

        Ok((
            settings,
            !had_asr_mode
                || had_legacy_microphone_permission
                || !had_current_version
                || wake_word_migrated
                || !had_text_provider
                || !had_google_text_model
                || !had_local_text_model
                || !had_tts_provider
                || !had_google_tts_model
                || !had_google_tts_voice
                || !had_local_tts_model
                || !had_local_tts_voice
                || !had_live_voice
                || had_legacy_provider
                || had_legacy_text_model
                || had_legacy_tts_model
                || had_legacy_tts_voice
                || models_migrated
                || had_enabled_window_title_observation
                || legacy_idle_banter
                || idle_banter_seed_repaired,
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
        config.behavior.idle_banter_enabled = self.idle_banter_enabled;
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
    pub idle_banter_runtime: Arc<Mutex<IdleBanterRuntime>>,
    pub audio_capture: Arc<Mutex<AudioCapture>>,
    pub wake_word_runtime: WakeWordApplicationRuntime,
    pub(crate) wake_listener_controller: NativeWakeListenerController,
    pub audio_playback: Arc<AudioPlayback>,
    pub standalone_speech: StandaloneSpeechController,
    pub conversation_mgr: Arc<ConversationManager>,
    pub moonshine_installer: Arc<MoonshineModelInstaller>,
    pub whisper_installer: Arc<crate::asr::whisper::WhisperModelInstaller>,
    pub(crate) local_llm_runtime: Arc<LocalRuntimeManager>,
    pub(crate) local_tts_runtime: Arc<LocalTtsRuntimeManager>,
    pub tool_router: Arc<ToolRouter>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub is_muted: Arc<RwLock<bool>>,
}

mod legacy_secrets;
mod model_paths;
use legacy_secrets::migrate_legacy_google_api_key;
#[cfg(test)]
use legacy_secrets::LEGACY_GOOGLE_API_KEY_SETTING;
use model_paths::{moonshine_model_root, whisper_model_root};

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
        // when a version migration occurs so future starts do not repeat it. A read or
        // decode failure must abort startup rather than silently substituting fresh defaults.
        if let Some(json_str) = db
            .get_setting("app_settings")
            .map_err(|error| format!("failed to read persisted app settings: {error}"))?
        {
            let (loaded, migrated) = AppSettings::from_persisted_json(&json_str)
                .map_err(|error| format!("failed to decode persisted app settings: {error}"))?;
            *settings.write() = loaded.clone();
            if migrated {
                let normalized =
                    serde_json::to_string(&loaded).map_err(|error| error.to_string())?;
                db.set_setting("app_settings", &normalized)
                    .map_err(|error| error.to_string())?;
            }
        }

        let idle_banter_runtime = Arc::new(Mutex::new(IdleBanterRuntime::new(&settings.read())));

        let mut character_config = CharacterConfig::default();
        settings
            .read()
            .apply_to_character_config(&mut character_config);
        let behavior_engine = Arc::new(Mutex::new(BehaviorEngine::new(character_config.clone())));
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new()));
        let wake_word_runtime = WakeWordApplicationRuntime::from_settings(&settings.read())
            .map_err(|error| error.to_string())?;
        let wake_listener_controller = NativeWakeListenerController::default();
        let audio_playback = Arc::new(AudioPlayback::new());
        audio_playback.set_volume(settings.read().volume);
        let standalone_speech = StandaloneSpeechController::new();
        let conversation_mgr = Arc::new(ConversationManager::new());
        let moonshine_installer = Arc::new(
            MoonshineModelInstaller::new(moonshine_model_root(db_path))
                .map_err(|error| error.to_string())?,
        );
        let whisper_installer = Arc::new(
            WhisperModelInstaller::new(whisper_model_root(db_path))
                .map_err(|error| error.to_string())?,
        );
        let local_llm_runtime = Arc::new(LocalRuntimeManager::new());
        let local_tts_runtime = Arc::new(LocalTtsRuntimeManager::new());

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
            idle_banter_runtime,
            audio_capture,
            wake_word_runtime,
            wake_listener_controller,
            audio_playback,
            standalone_speech,
            conversation_mgr,
            moonshine_installer,
            whisper_installer,
            local_llm_runtime,
            local_tts_runtime,
            tool_router,
            settings,
            is_muted: Arc::new(RwLock::new(false)),
        })
    }

    pub fn record_user_interaction(&self) {
        self.ambient_scheduler.interrupt();
        let settings = self.settings.read().clone();
        self.idle_banter_runtime
            .lock()
            .record_user_interaction(&settings);
    }

    pub fn reset_idle_banter_after_wake(&self) {
        self.ambient_scheduler.interrupt();
        let settings = self.settings.read().clone();
        self.idle_banter_runtime.lock().reset_after_wake(&settings);
    }

    pub fn get_text_model(&self) -> Box<dyn TextModel> {
        let settings = self.settings_snapshot();
        self.get_text_model_for(&settings)
    }

    pub fn get_speech_synthesizer(&self) -> Box<dyn SpeechSynthesizer> {
        let settings = self.settings.read();
        match settings.tts_provider {
            TtsProvider::Google => {
                let key = self.secrets.get_google_api_key().unwrap_or_default();
                Box::new(GoogleSpeechSynthesizer::new(
                    GoogleAuth::new(key),
                    settings.google_tts_model.clone(),
                    settings.google_tts_voice.clone(),
                ))
            }
            TtsProvider::Local => Box::new(LocalSpeechSynthesizer::new(
                self.local_tts_runtime.clone(),
                settings.local_tts_model.clone(),
                settings.local_tts_voice.clone(),
            )),
        }
    }

    pub fn get_live_provider(&self) -> Arc<dyn RealtimeConversationProvider> {
        let key = self.secrets.get_google_api_key().unwrap_or_default();
        Arc::new(GoogleLiveProvider::new(GoogleAuth::new(key)))
    }
}

#[cfg(test)]
#[path = "state/tests/mod.rs"]
mod tests;
