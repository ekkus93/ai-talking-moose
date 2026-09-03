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
fn new_settings_default_to_moonshine_tiny_and_local_text() {
    let settings = AppSettings::default();
    assert_eq!(settings.settings_version, CURRENT_SETTINGS_VERSION);
    assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);
    assert_eq!(settings.text_provider, TextProvider::Local);
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
    state.settings.write().text_provider = TextProvider::Google;
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
async fn configured_local_text_with_unknown_model_fails_without_cloud_fallback() {
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
fn malformed_persisted_settings_abort_startup_instead_of_using_fresh_defaults() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_string_lossy().to_string();
    let db = Database::new(&path).unwrap();
    db.set_setting("app_settings", "{not-valid-json").unwrap();
    drop(db);

    let secret_store =
        SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
    let result = AppState::new_with_secret_store(Some(&path), secret_store);

    let error = match result {
        Ok(_) => panic!("malformed persisted settings must not be replaced by defaults"),
        Err(error) => error,
    };
    assert!(error.contains("failed to decode persisted app settings"));
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
