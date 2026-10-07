use super::super::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn new_settings_default_to_moonshine_tiny_and_local_text() {
    let settings = AppSettings::default();
    assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
    assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);
    assert!(!settings.wake_word_enabled);
    assert_eq!(settings.wake_word_phrase, DEFAULT_WAKE_PHRASE);
    assert_eq!(settings.text_provider, TextProvider::Local);
    assert_eq!(settings.google_text_model, DEFAULT_TEXT_MODEL);
    assert_eq!(settings.local_text_model, DEFAULT_LOCAL_TEXT_MODEL_ID);
    assert_eq!(settings.tts_provider, TtsProvider::Google);
    assert_eq!(settings.google_tts_model, DEFAULT_TTS_MODEL);
    assert_eq!(settings.google_tts_voice, DEFAULT_TTS_VOICE);
    assert_eq!(settings.local_tts_model, DEFAULT_LOCAL_TTS_MODEL_ID);
    assert_eq!(settings.local_tts_voice, DEFAULT_LOCAL_TTS_VOICE);
    assert_eq!(settings.live_voice, DEFAULT_TTS_VOICE);
    assert!(!settings.active_app_observation);
    assert!(!settings.memory_enabled);
    assert!(!settings.save_transcripts);
    assert!(settings.idle_banter_enabled);
    assert_eq!(settings.idle_banter_initial_delay_minutes, 60);
    assert_eq!(settings.idle_banter_repeat_interval_minutes, 30);
    assert!(!settings.idle_banter_seed_topics.is_empty());
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
    assert!(!settings.wake_word_enabled);
    assert_eq!(settings.wake_word_phrase, DEFAULT_WAKE_PHRASE);
}

#[test]
fn persisted_wake_word_enabled_round_trips_without_migration() {
    let original = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };
    let json = serde_json::to_string(&original).unwrap();

    let (settings, migrated) = AppSettings::from_persisted_json(&json).unwrap();

    assert!(!migrated);
    assert!(settings.wake_word_enabled);
    assert_eq!(settings.wake_word_phrase, DEFAULT_WAKE_PHRASE);
}

#[test]
fn missing_wake_word_fields_default_disabled_and_preserve_unrelated_settings() {
    let mut value = serde_json::to_value(AppSettings {
        asr_mode: AsrMode::MoonshineSmallStreaming,
        tts_provider: TtsProvider::Local,
        local_tts_voice: "Bella".to_string(),
        google_tts_voice: "Kore".to_string(),
        ..Default::default()
    })
    .unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove(WAKE_WORD_ENABLED_FIELD);
    object.remove(WAKE_WORD_PHRASE_FIELD);

    let (settings, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();

    assert!(migrated);
    assert!(!settings.wake_word_enabled);
    assert_eq!(settings.wake_word_phrase, DEFAULT_WAKE_PHRASE);
    assert_eq!(settings.asr_mode, AsrMode::MoonshineSmallStreaming);
    assert_eq!(settings.tts_provider, TtsProvider::Local);
    assert_eq!(settings.local_tts_voice, "Bella");
    assert_eq!(settings.google_tts_voice, "Kore");
}

#[test]
fn wake_word_phrase_normalizes_or_fails_safely() {
    let original = AppSettings {
        wake_word_enabled: true,
        wake_word_phrase: "  hey, moose  ".to_string(),
        ..Default::default()
    };
    let (settings, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&original).unwrap()).unwrap();
    assert!(migrated);
    assert!(settings.wake_word_enabled);
    assert_eq!(settings.wake_word_phrase, DEFAULT_WAKE_PHRASE);

    let invalid = AppSettings {
        wake_word_phrase: "Hey Bruce".to_string(),
        ..Default::default()
    };
    let error =
        AppSettings::from_persisted_json(&serde_json::to_string(&invalid).unwrap()).unwrap_err();
    assert!(matches!(error, PersistedSettingsError::Invalid(_)));
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
    assert!(!settings.wake_word_enabled);
    assert_eq!(settings.wake_word_phrase, DEFAULT_WAKE_PHRASE);

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
fn future_settings_version_fails_closed_without_rewriting_persistence() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_string_lossy().to_string();
    let db = Database::new(&path).unwrap();
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert(
        "settings_version".to_string(),
        serde_json::json!(CURRENT_SETTINGS_VERSION + 1),
    );
    object.insert(
        "future_provider_policy".to_string(),
        serde_json::json!({"private_mode": "future-only"}),
    );
    object.insert("tts_voice".to_string(), serde_json::json!("Puck"));
    let original = serde_json::to_string_pretty(&value).unwrap();
    db.set_setting("app_settings", &original).unwrap();
    drop(db);

    let typed_error = AppSettings::from_persisted_json(&original).unwrap_err();
    assert!(matches!(
        typed_error,
        PersistedSettingsError::FutureVersion {
            found,
            current: CURRENT_SETTINGS_VERSION,
        } if found == u64::from(CURRENT_SETTINGS_VERSION + 1)
    ));

    let secret_store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
    let startup_error = match AppState::new_with_secret_store(Some(&path), secret_store) {
        Ok(_) => panic!("future settings must fail startup closed"),
        Err(error) => error,
    };
    assert!(startup_error.contains("newer application version"));
    assert!(!startup_error.contains("future_provider_policy"));
    assert!(!startup_error.contains("future-only"));

    let reopened = Database::new(&path).unwrap();
    assert_eq!(
        reopened.get_setting("app_settings").unwrap().as_deref(),
        Some(original.as_str()),
        "future-version startup failure must not rewrite or strip persisted JSON"
    );
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

    let secret_store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
    let state = AppState::new_with_secret_store(Some(&path), secret_store).unwrap();

    assert_eq!(state.settings.read().asr_mode, AsrMode::GeminiLiveAudio);
    assert!(state.onboarding_status().unwrap().needs_acknowledgement);
}

#[test]
fn version_three_tts_settings_migrate_once_to_split_google_local_and_live_ownership() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert("settings_version".to_string(), serde_json::json!(3));
    object.remove("tts_provider");
    object.remove("google_tts_model");
    object.remove("google_tts_voice");
    object.remove("local_tts_model");
    object.remove("local_tts_voice");
    object.remove("live_voice");
    object.insert(
        "tts_model".to_string(),
        serde_json::Value::String("en-US-Standard-B".to_string()),
    );
    object.insert(
        "tts_voice".to_string(),
        serde_json::Value::String("Puck".to_string()),
    );

    let (settings, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();
    assert!(migrated);
    assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
    assert_eq!(settings.tts_provider, TtsProvider::Google);
    assert_eq!(settings.google_tts_model, DEFAULT_TTS_MODEL);
    assert_eq!(settings.google_tts_voice, "Puck");
    assert_eq!(settings.live_voice, "Puck");
    assert_eq!(settings.local_tts_model, DEFAULT_LOCAL_TTS_MODEL_ID);
    assert_eq!(settings.local_tts_voice, DEFAULT_LOCAL_TTS_VOICE);

    let normalized = serde_json::to_value(&settings).unwrap();
    assert!(normalized.get("tts_model").is_none());
    assert!(normalized.get("tts_voice").is_none());

    let (round_tripped, migrated_again) =
        AppSettings::from_persisted_json(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert!(!migrated_again);
    assert_eq!(round_tripped.google_tts_voice, "Puck");
    assert_eq!(round_tripped.live_voice, "Puck");
}

#[test]
fn version_four_profiles_migrate_idle_banter_defaults_without_touching_existing_preferences() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert("settings_version".to_string(), serde_json::json!(4));
    object.remove("idle_banter_enabled");
    object.remove("idle_banter_initial_delay_minutes");
    object.remove("idle_banter_repeat_interval_minutes");
    object.remove("idle_banter_seed_topics");
    object.insert("talkativeness".to_string(), serde_json::json!(0.73));
    object.insert("memory_enabled".to_string(), serde_json::json!(true));
    object.insert("quiet_hours_start".to_string(), serde_json::json!(21));
    object.insert("quiet_hours_end".to_string(), serde_json::json!(7));
    object.insert("google_tts_voice".to_string(), serde_json::json!("Kore"));
    object.insert("local_tts_voice".to_string(), serde_json::json!("Luna"));

    let (settings, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();
    assert!(migrated);
    assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
    assert!(settings.idle_banter_enabled);

    assert_eq!(settings.idle_banter_initial_delay_minutes, 60);
    assert_eq!(settings.idle_banter_repeat_interval_minutes, 30);
    assert_eq!(
        settings.idle_banter_seed_topics,
        default_idle_banter_seed_topics()
    );
    assert_eq!(settings.talkativeness, 0.73);
    assert!(settings.memory_enabled);
    assert_eq!(settings.quiet_hours_start, 21);
    assert_eq!(settings.quiet_hours_end, 7);
    assert_eq!(settings.google_tts_voice, "Kore");
    assert_eq!(settings.local_tts_voice, "Luna");

    let (round_tripped, migrated_again) =
        AppSettings::from_persisted_json(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert!(!migrated_again);
    assert_eq!(
        round_tripped.idle_banter_seed_topics,
        settings.idle_banter_seed_topics
    );
}

#[test]
fn legacy_empty_idle_banter_seed_list_repairs_but_current_invalid_list_fails_closed() {
    let mut legacy = serde_json::to_value(AppSettings::default()).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert("settings_version".to_string(), serde_json::json!(4));
    object.insert("idle_banter_seed_topics".to_string(), serde_json::json!([]));
    let (repaired, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&legacy).unwrap()).unwrap();
    assert!(migrated);
    assert_eq!(
        repaired.idle_banter_seed_topics,
        default_idle_banter_seed_topics()
    );

    let mut current = serde_json::to_value(AppSettings::default()).unwrap();
    current
        .as_object_mut()
        .unwrap()
        .insert("idle_banter_seed_topics".to_string(), serde_json::json!([]));
    assert!(matches!(
        AppSettings::from_persisted_json(&serde_json::to_string(&current).unwrap()),
        Err(PersistedSettingsError::Invalid(_))
    ));
}

#[test]
fn current_profile_preserves_persisted_bella_local_tts_voice() {
    let settings = AppSettings {
        local_tts_voice: "Bella".to_string(),
        ..Default::default()
    };

    let (loaded, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&settings).unwrap()).unwrap();

    assert!(!migrated);
    assert_eq!(loaded.local_tts_voice, "Bella");
}

#[test]
fn missing_local_tts_voice_defaults_to_current_luna_without_forced_voice_migration() {
    assert_eq!(DEFAULT_LOCAL_TTS_VOICE, "Luna");
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value.as_object_mut().unwrap().remove("local_tts_voice");

    let (loaded, migrated) =
        AppSettings::from_persisted_json(&serde_json::to_string(&value).unwrap()).unwrap();

    assert!(migrated);
    assert_eq!(loaded.local_tts_voice, DEFAULT_LOCAL_TTS_VOICE);
}

#[test]
fn split_tts_voice_settings_round_trip_independently() {
    let original = AppSettings {
        google_tts_voice: "Kore".to_string(),
        local_tts_voice: "Luna".to_string(),
        live_voice: "Puck".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_string(&original).unwrap();
    let (loaded, migrated) = AppSettings::from_persisted_json(&json).unwrap();
    assert!(!migrated);
    assert_eq!(loaded.google_tts_voice, "Kore");
    assert_eq!(loaded.local_tts_voice, "Luna");
    assert_eq!(loaded.live_voice, "Puck");
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
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../src/generated/backendContract.json"
    ))
    .unwrap();

    let contract_settings = contract.get("settings").unwrap();
    let authoritative_settings = serde_json::to_value(AppSettings::default()).unwrap();
    let contract_keys = contract_settings.as_object().unwrap();
    let authoritative_keys = authoritative_settings.as_object().unwrap();
    assert_eq!(contract_keys.len(), authoritative_keys.len());
    assert!(authoritative_keys
        .keys()
        .all(|key| contract_keys.contains_key(key)));

    let decoded_settings: AppSettings = serde_json::from_value(contract_settings.clone()).unwrap();
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
