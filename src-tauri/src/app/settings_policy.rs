use crate::ai::google::{
    validate_live_model, validate_text_model, validate_tts_model, validate_tts_voice,
};
use crate::ai::local::local_model_entry;
use crate::ai::local_tts::{validate_local_tts_model, validate_local_tts_voice};
use crate::app::state::AppSettings;
use crate::audio::devices::AudioDeviceInfo;
use crate::character::behavior::BehaviorEngine;
use crate::character::idle_banter::{
    normalize_idle_banter_seed_topics, IDLE_BANTER_MAX_MINUTES, IDLE_BANTER_MIN_MINUTES,
};
use crate::persistence::sqlite::Database;
use parking_lot::{Mutex, RwLock};
use std::sync::OnceLock;
use tokio::sync::Mutex as AsyncMutex;

static SETTINGS_RUNTIME_LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();

/// Server addresses are data, not arbitrary request URLs. Resolve the effective
/// destination again when constructing each HTTP request and disable redirects.
pub(crate) fn validate_loopback_server_address(address: &str) -> Result<(), String> {
    let invalid = || {
        "Server address must be an HTTP loopback host with an explicit port and no path, credentials, query, or fragment".to_string()
    };
    if address.trim() != address || address.contains('\\') || address.chars().any(char::is_control)
    {
        return Err(invalid());
    }
    let url = reqwest::Url::parse(address).map_err(|_| invalid())?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || url.port().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    // Check the unnormalized spelling too: path traversal and encoded hosts
    // must not be admitted merely because URL parsing normalized them away.
    let canonical = format!(
        "http://{}:{}",
        url.host_str().ok_or_else(invalid)?,
        url.port().ok_or_else(invalid)?
    );
    if url.port() == Some(0) || (address != canonical && address != format!("{canonical}/")) {
        return Err(invalid());
    }
    Ok(())
}

/// Serialize settings commits with conversation startup.
///
/// Settings that change ASR/provider/audio ownership use this same lock at both
/// command boundaries so a start can never activate a stale pre-update graph.
pub(crate) fn settings_runtime_lock() -> &'static AsyncMutex<()> {
    SETTINGS_RUNTIME_LOCK.get_or_init(|| AsyncMutex::new(()))
}

pub(crate) fn validate_app_settings(settings: &AppSettings) -> Result<(), String> {
    fn finite_range(name: &str, value: f32, min: f32, max: f32) -> Result<(), String> {
        if value.is_finite() && (min..=max).contains(&value) {
            Ok(())
        } else {
            Err(format!("{name} must be between {min} and {max}"))
        }
    }

    fn bounded_identifier(name: &str, value: &str, max_bytes: usize) -> Result<(), String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(format!("{name} must not be empty"));
        }
        if trimmed.len() > max_bytes {
            return Err(format!("{name} is too long"));
        }
        Ok(())
    }

    fn optional_device_id(name: &str, value: Option<&str>) -> Result<(), String> {
        if let Some(value) = value {
            bounded_identifier(name, value, 256)?;
        }
        Ok(())
    }

    validate_live_model(&settings.live_model)?;
    validate_text_model(&settings.google_text_model)?;
    bounded_identifier("local text model ID", &settings.local_text_model, 128)?;
    if local_model_entry(&settings.local_text_model).is_none() {
        return Err("unsupported local text model".to_string());
    }
    validate_tts_voice(&settings.google_tts_voice)?;
    validate_tts_voice(&settings.live_voice)?;
    validate_tts_model(&settings.google_tts_model)?;
    validate_local_tts_model(&settings.local_tts_model)?;
    validate_local_tts_voice(&settings.local_tts_voice)?;
    for (name, model_id) in [
        ("OpenAI text model", settings.openai_text_model.as_str()),
        ("Ollama text model", settings.ollama_text_model.as_str()),
        (
            "llama-server text model",
            settings.llama_server_text_model.as_str(),
        ),
        ("system TTS voice", settings.system_tts_voice.as_str()),
    ] {
        bounded_identifier(name, model_id, 128)?;
        if model_id.chars().any(char::is_control) || model_id.trim() != model_id {
            return Err(format!("{name} contains invalid characters"));
        }
    }
    // The native voice catalog excludes directional Unicode formatting that
    // could disguise a saved OS voice identifier in the Control Panel.
    if crate::ai::system_tts_voice_catalog::has_bidi_formatting(&settings.system_tts_voice) {
        return Err("system TTS voice contains unsupported formatting characters".to_string());
    }
    // Keep the settings boundary consistent with OpenAI request validation:
    // a saved model ID must be usable before the user starts a request.
    if !settings
        .openai_text_model
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-._:".contains(&byte))
    {
        return Err("OpenAI text model ID contains unsupported characters".to_string());
    }
    validate_loopback_server_address(&settings.ollama_server_address)?;
    validate_loopback_server_address(&settings.llama_server_address)?;
    optional_device_id("input device ID", settings.input_device.as_deref())?;
    optional_device_id("output device ID", settings.output_device.as_deref())?;

    finite_range("talkativeness", settings.talkativeness, 0.0, 1.0)?;
    finite_range("volume", settings.volume, 0.0, 1.0)?;
    finite_range("speaking rate", settings.speaking_rate, 0.25, 4.0)?;
    finite_range("pitch", settings.pitch, -20.0, 20.0)?;
    for (name, value) in [
        ("dry", settings.dry),
        ("sarcastic", settings.sarcastic),
        ("friendly", settings.friendly),
        ("absurd", settings.absurd),
        ("helpful", settings.helpful),
        ("verbosity", settings.verbosity),
    ] {
        finite_range(name, value, 0.0, 1.0)?;
    }

    if settings.quiet_hours_start > 23 || settings.quiet_hours_end > 23 {
        return Err("quiet-hour values must be between 0 and 23".to_string());
    }
    if !(1..=12).contains(&settings.max_comments_per_hour) {
        return Err("maximum ambient comments per hour must be between 1 and 12".to_string());
    }
    if settings.hide_delay_seconds > 3_600 {
        return Err("hide delay must not exceed 3600 seconds".to_string());
    }
    if !(IDLE_BANTER_MIN_MINUTES..=IDLE_BANTER_MAX_MINUTES)
        .contains(&settings.idle_banter_initial_delay_minutes)
    {
        return Err(format!(
            "Idle Banter initial delay must be between {IDLE_BANTER_MIN_MINUTES} and {IDLE_BANTER_MAX_MINUTES} minutes"
        ));
    }
    if !(IDLE_BANTER_MIN_MINUTES..=IDLE_BANTER_MAX_MINUTES)
        .contains(&settings.idle_banter_repeat_interval_minutes)
    {
        return Err(format!(
            "Idle Banter repeat interval must be between {IDLE_BANTER_MIN_MINUTES} and {IDLE_BANTER_MAX_MINUTES} minutes"
        ));
    }
    normalize_idle_banter_seed_topics(&settings.idle_banter_seed_topics)?;

    Ok(())
}

pub(crate) fn conversation_restart_required(previous: &AppSettings, next: &AppSettings) -> bool {
    previous.asr_mode != next.asr_mode
        || previous.live_model != next.live_model
        || previous.input_device != next.input_device
        || previous.output_device != next.output_device
        || previous.live_voice != next.live_voice
        || previous.memory_enabled != next.memory_enabled
        || previous.save_transcripts != next.save_transcripts
}

pub(crate) fn validate_selected_device(
    selected: Option<&str>,
    available: &[AudioDeviceInfo],
    kind: &str,
) -> Result<(), String> {
    let Some(selected) = selected else {
        return Ok(());
    };
    if available.iter().any(|device| device.id == selected) {
        Ok(())
    } else {
        Err(format!("selected {kind} device is unavailable: {selected}"))
    }
}

trait SettingsPersistence {
    fn persist(&self, json: &str) -> Result<(), String>;
}

impl SettingsPersistence for Database {
    fn persist(&self, json: &str) -> Result<(), String> {
        self.set_setting("app_settings", json)
            .map_err(|error| error.to_string())
    }
}

fn persist_and_apply<P: SettingsPersistence>(
    persistence: &P,
    runtime_settings: &RwLock<AppSettings>,
    behavior_engine: &Mutex<BehaviorEngine>,
    next: &AppSettings,
) -> Result<(), String> {
    let json = serde_json::to_string(next).map_err(|error| error.to_string())?;
    persistence.persist(&json)?;

    *runtime_settings.write() = next.clone();
    next.apply_to_character_config(&mut behavior_engine.lock().config);
    Ok(())
}

pub(crate) fn persist_and_apply_settings(
    db: &Database,
    runtime_settings: &RwLock<AppSettings>,
    behavior_engine: &Mutex<BehaviorEngine>,
    next: &AppSettings,
) -> Result<(), String> {
    persist_and_apply(db, runtime_settings, behavior_engine, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::AsrMode;
    use crate::character::personality::CharacterConfig;
    use std::collections::BTreeSet;

    struct RejectPersistence;

    impl SettingsPersistence for RejectPersistence {
        fn persist(&self, _json: &str) -> Result<(), String> {
            Err("injected persistence failure".to_string())
        }
    }

    #[test]
    fn validation_rejects_invalid_ranges_and_identifiers() {
        let settings = AppSettings {
            talkativeness: f32::NAN,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            local_text_model: "   ".to_string(),
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            local_text_model: "../arbitrary.gguf".to_string(),
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            local_text_model: "unknown-model".to_string(),
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            quiet_hours_start: 24,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            max_comments_per_hour: 0,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            input_device: Some("   ".to_string()),
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            idle_banter_initial_delay_minutes: 4,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            idle_banter_initial_delay_minutes: 1_441,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            idle_banter_repeat_interval_minutes: 4,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            idle_banter_repeat_interval_minutes: 1_441,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            idle_banter_seed_topics: vec!["Moose".into(), "  moose ".into()],
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());
    }

    #[test]
    fn saved_system_voice_rejects_directional_spoofing_at_settings_boundary() {
        let mut settings = AppSettings::default();
        assert!(validate_app_settings(&settings).is_ok());
        settings.system_tts_voice = "com.apple.voice.accepted".into();
        assert!(validate_app_settings(&settings).is_ok());
        for id in [
            "com.apple.voice.\u{202e}spoof",
            "com.apple.voice.\u{2066}spoof\u{2069}",
            "com.apple.voice.\u{200f}spoof",
        ] {
            settings.system_tts_voice = id.into();
            assert!(validate_app_settings(&settings).is_err());
        }
    }

    #[test]
    fn unknown_fields_and_missing_legacy_fields_migrate_deterministically() {
        let mut current = serde_json::to_value(AppSettings::default()).unwrap();
        current.as_object_mut().unwrap().insert(
            "future_additive_setting".to_string(),
            serde_json::json!({"enabled": true}),
        );
        let (settings, migrated) =
            AppSettings::from_persisted_json(&serde_json::to_string(&current).unwrap()).unwrap();
        assert!(!migrated);
        assert_eq!(settings.asr_mode, AppSettings::default().asr_mode);

        let mut legacy = serde_json::to_value(AppSettings::default()).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.remove("settings_version");
        object.remove("asr_mode");
        object.remove("save_transcripts");
        let (settings, migrated) =
            AppSettings::from_persisted_json(&serde_json::to_string(&legacy).unwrap()).unwrap();
        assert!(migrated);
        assert_eq!(settings.asr_mode, AsrMode::GeminiLiveAudio);
        assert!(!settings.save_transcripts);
    }

    #[test]
    fn restart_policy_covers_captured_conversation_configuration() {
        let previous = AppSettings::default();

        let mut next = previous.clone();
        next.asr_mode = AsrMode::GeminiLiveAudio;
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.input_device = Some("External Mic".to_string());
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.live_voice = "Puck".to_string();
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.google_tts_voice = "Puck".to_string();
        assert!(!conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.local_tts_voice = "Leo".to_string();
        assert!(validate_local_tts_voice(&previous.local_tts_voice).is_ok());
        assert!(validate_local_tts_voice(&next.local_tts_voice).is_ok());
        assert_ne!(previous.local_tts_voice, next.local_tts_voice);
        assert!(!conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.memory_enabled = !previous.memory_enabled;
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.save_transcripts = !previous.save_transcripts;
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.text_provider = crate::ai::types::TextProvider::Local;
        assert!(!conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.talkativeness = 0.75;
        assert!(!conversation_restart_required(&previous, &next));
    }

    #[test]
    fn selected_device_must_come_from_current_enumeration() {
        let available = vec![AudioDeviceInfo {
            id: "Built-in Mic".to_string(),
            name: "Built-in Mic".to_string(),
            is_default: true,
        }];
        assert!(validate_selected_device(Some("Built-in Mic"), &available, "input").is_ok());
        assert!(validate_selected_device(Some("Missing Mic"), &available, "input").is_err());
        assert!(validate_selected_device(Some("Missing Output"), &available, "output").is_err());
        assert!(validate_selected_device(None, &available, "input").is_ok());
    }

    #[test]
    fn provider_addresses_and_model_ids_are_validated_at_backend_boundary() {
        for address in [
            "http://localhost:11434",
            "http://[::1]:8080",
            "http://127.0.0.1:1234/",
        ] {
            assert!(
                validate_loopback_server_address(address).is_ok(),
                "{address}"
            );
        }
        for address in [
            "https://localhost:11434",
            "http://example.com:11434",
            "http://localhost",
            "http://localhost:0",
            "http://user@localhost:9000",
            "http://localhost:9000/v1",
            "http://localhost:9000/a/..",
            "http://localhost:9000/?key=x",
            "http://localhost:9000/#x",
            "http://127.0.0.2:11434",
            "http://0.0.0.0:11434",
            "http://localhost:65536",
            "http://LOCALHOST:11434",
            "http://localhost:11434/%2e%2e",
            "http://[::ffff:127.0.0.1]:11434",
            " http://localhost:9000",
        ] {
            assert!(
                validate_loopback_server_address(address).is_err(),
                "{address}"
            );
        }
        let mut settings = AppSettings {
            openai_text_model: String::new(),
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());
        settings.openai_text_model = "gpt-4.1 mini".into();
        assert!(validate_app_settings(&settings).is_err());
        settings.openai_text_model = "gpt-4.1/mini".into();
        assert!(validate_app_settings(&settings).is_err());
        settings.openai_text_model = "gpt-4.1-mini".into();
        settings.ollama_text_model = "bad\nmodel".into();
        assert!(validate_app_settings(&settings).is_err());
        settings.ollama_text_model = "example:latest".into();
        settings.ollama_server_address = "https://elsewhere.example".into();
        assert!(validate_app_settings(&settings).is_err());
        settings.ollama_server_address = "http://localhost:11434".into();
        assert!(validate_app_settings(&settings).is_ok());
    }

    #[test]
    fn external_provider_settings_round_trip_without_erasing_other_provider_choices() {
        use crate::ai::types::{TextProvider, TtsProvider};

        let mut settings = AppSettings {
            openai_text_model: "gpt-4.1-mini".into(),
            ollama_text_model: "local-model:latest".into(),
            llama_server_text_model: "loaded-model".into(),
            system_tts_voice: "system_default".into(),
            ..Default::default()
        };
        for text_provider in [
            TextProvider::Local,
            TextProvider::Google,
            TextProvider::Openai,
            TextProvider::Ollama,
            TextProvider::LlamaServer,
        ] {
            for tts_provider in [TtsProvider::Local, TtsProvider::Google, TtsProvider::System] {
                settings.text_provider = text_provider;
                settings.tts_provider = tts_provider;
                let json = serde_json::to_string(&settings).unwrap();
                let (decoded, migrated) = AppSettings::from_persisted_json(&json).unwrap();
                assert!(!migrated);
                assert_eq!(decoded.text_provider, text_provider);
                assert_eq!(decoded.tts_provider, tts_provider);
                assert_eq!(decoded.openai_text_model, settings.openai_text_model);
                assert_eq!(decoded.ollama_text_model, settings.ollama_text_model);
                assert_eq!(
                    decoded.llama_server_text_model,
                    settings.llama_server_text_model
                );
                assert_eq!(decoded.system_tts_voice, settings.system_tts_voice);
            }
        }
    }

    #[test]
    fn persisted_settings_are_classified_and_user_preferences_have_runtime_consumers() {
        let user_consumers = [
            ("asr_mode", "conversation start selects local or Gemini ASR"),
            ("launch_at_login", "startup runtime preferences"),
            ("show_in_menu_bar", "startup runtime preferences"),
            ("always_on_top", "window runtime preferences"),
            ("restore_position", "startup window restoration"),
            ("unsolicited_comments", "behavior engine ambient policy"),
            ("talkativeness", "behavior engine personality"),
            ("quiet_hours_enabled", "behavior engine ambient policy"),
            ("quiet_hours_start", "behavior engine ambient policy"),
            ("quiet_hours_end", "behavior engine ambient policy"),
            ("max_comments_per_hour", "behavior engine ambient budget"),
            ("hide_delay_seconds", "ambient post-speech hide delay"),
            ("idle_banter_enabled", "Idle Banter dedicated feature gate"),
            (
                "idle_banter_initial_delay_minutes",
                "Idle Banter initial inactivity scheduler",
            ),
            (
                "idle_banter_repeat_interval_minutes",
                "Idle Banter repeat scheduler",
            ),
            (
                "idle_banter_seed_topics",
                "Idle Banter creative-direction prompt selection",
            ),
            ("input_device", "conversation capture and microphone test"),
            ("output_device", "conversation and standalone playback"),
            ("volume", "shared CPAL playback gain"),
            ("tts_provider", "standalone TTS provider selection"),
            ("google_tts_voice", "Google standalone speech selection"),
            ("local_tts_voice", "Local standalone speech selection"),
            ("system_tts_voice", "macOS system voice selection"),
            ("live_voice", "Gemini Live session voice selection"),
            ("speaking_rate", "standalone TTS request"),
            ("pitch", "standalone TTS request"),
            ("text_provider", "text model provider selection"),
            ("live_model", "Gemini Live session configuration"),
            ("google_text_model", "Google text model construction"),
            ("local_text_model", "Local model catalog selection"),
            ("openai_text_model", "OpenAI model choice"),
            ("ollama_text_model", "Ollama model choice"),
            ("ollama_server_address", "Ollama server selection"),
            ("llama_server_text_model", "llama-server model choice"),
            ("llama_server_address", "llama-server server selection"),
            ("google_tts_model", "Google speech synthesizer construction"),
            ("local_tts_model", "Local speech model selection"),
            ("active_app_observation", "desktop observation privacy gate"),
            ("memory_enabled", "prompt memory privacy gate"),
            ("save_transcripts", "transcript retention policy"),
            ("dry", "character personality prompt"),
            ("sarcastic", "character personality prompt"),
            ("friendly", "character personality prompt"),
            ("absurd", "character personality prompt"),
            ("helpful", "character personality prompt"),
            ("verbosity", "character personality prompt"),
        ];
        let metadata = [
            ("settings_version", "schema and migration metadata"),
            (
                "window_title_observation",
                "V1 compatibility field normalized fail-closed to false",
            ),
            (
                "wake_word_enabled",
                "Wake Word V1 persisted feature gate; runtime consumer lands with wake lifecycle integration",
            ),
            (
                "wake_word_phrase",
                "Wake Word V1 canonical phrase configuration validated fail-closed before runtime integration",
            ),
        ];

        let serialized = serde_json::to_value(AppSettings::default()).unwrap();
        let actual = serialized
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let classified = user_consumers
            .iter()
            .chain(metadata.iter())
            .map(|(field, _)| (*field).to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(actual, classified);
        assert!(user_consumers
            .iter()
            .all(|(_, consumer)| !consumer.is_empty()));
        assert!(metadata
            .iter()
            .all(|(_, classification)| !classification.is_empty()));
        assert!(serialized.get("provider").is_none());
        assert!(serialized.get("text_model").is_none());
        assert!(serialized.get("tts_model").is_none());
        assert!(serialized.get("tts_voice").is_none());
        assert!(serialized.get("microphone_permission_granted").is_none());
    }

    #[test]
    fn failed_persistence_leaves_runtime_settings_and_behavior_unchanged() {
        let runtime_settings = RwLock::new(AppSettings::default());
        let behavior = Mutex::new(BehaviorEngine::new(CharacterConfig::default()));
        let before = runtime_settings.read().clone();
        let before_talkativeness = behavior.lock().config.personality.talkativeness;
        let next = AppSettings {
            talkativeness: 0.91,
            ..before.clone()
        };

        assert!(
            persist_and_apply(&RejectPersistence, &runtime_settings, &behavior, &next).is_err()
        );
        assert_eq!(runtime_settings.read().talkativeness, before.talkativeness);
        assert_eq!(
            behavior.lock().config.personality.talkativeness,
            before_talkativeness
        );
    }
}
