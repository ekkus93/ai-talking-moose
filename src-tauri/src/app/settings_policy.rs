use crate::ai::google::{
    validate_live_model, validate_text_model, validate_tts_model, validate_tts_voice,
};
use crate::app::state::AppSettings;
use crate::audio::devices::AudioDeviceInfo;
use crate::character::behavior::BehaviorEngine;
use crate::persistence::sqlite::Database;
use parking_lot::{Mutex, RwLock};
use std::sync::OnceLock;
use tokio::sync::Mutex as AsyncMutex;

static SETTINGS_RUNTIME_LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();

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

    if !matches!(settings.provider.as_str(), "google" | "fake") {
        return Err("unsupported AI provider".to_string());
    }
    validate_live_model(&settings.live_model)?;
    validate_text_model(&settings.text_model)?;
    validate_tts_voice(&settings.tts_voice)?;
    validate_tts_model(&settings.tts_model)?;
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

    Ok(())
}

pub(crate) fn conversation_restart_required(previous: &AppSettings, next: &AppSettings) -> bool {
    previous.asr_mode != next.asr_mode
        || previous.provider != next.provider
        || previous.live_model != next.live_model
        || previous.input_device != next.input_device
        || previous.output_device != next.output_device
        || previous.tts_voice != next.tts_voice
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

    struct RejectPersistence;

    impl SettingsPersistence for RejectPersistence {
        fn persist(&self, _json: &str) -> Result<(), String> {
            Err("injected persistence failure".to_string())
        }
    }

    #[test]
    fn validation_rejects_invalid_ranges_provider_and_identifiers() {
        let settings = AppSettings {
            talkativeness: f32::NAN,
            ..Default::default()
        };
        assert!(validate_app_settings(&settings).is_err());

        let settings = AppSettings {
            provider: "unexpected".to_string(),
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
        assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);

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
        next.tts_voice = "Puck".to_string();
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.memory_enabled = !previous.memory_enabled;
        assert!(conversation_restart_required(&previous, &next));

        let mut next = previous.clone();
        next.save_transcripts = !previous.save_transcripts;
        assert!(conversation_restart_required(&previous, &next));

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
        assert!(validate_selected_device(None, &available, "input").is_ok());
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
