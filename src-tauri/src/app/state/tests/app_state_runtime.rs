use super::super::*;
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
fn record_user_interaction_restarts_idle_banter_from_current_settings() {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().idle_banter_initial_delay_minutes = 5;
    let expected_delay = std::time::Duration::from_secs(5 * 60);
    let reset_started = std::time::Instant::now();

    state.record_user_interaction();

    let reset_finished = std::time::Instant::now();
    let after = state.idle_banter_runtime.lock().next_due_at();
    assert!(after >= reset_started + expected_delay);
    assert!(after <= reset_finished + expected_delay);
}

#[test]
fn cloned_app_state_shares_one_local_tts_runtime_manager() {
    let state = AppState::new_for_tests().unwrap();
    let cloned = state.clone();

    assert!(Arc::ptr_eq(
        &state.local_tts_runtime,
        &cloned.local_tts_runtime
    ));
}

#[tokio::test]
async fn local_tts_selection_fails_closed_when_model_is_not_installed() {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().tts_provider = TtsProvider::Local;

    let error = state
        .get_speech_synthesizer()
        .synthesize(TtsRequest {
            text: "private local utterance".to_string(),
            voice_name: Some(DEFAULT_LOCAL_TTS_VOICE.to_string()),
            speaking_rate: Some(1.0),
            pitch: None,
        })
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Setup);
    assert!(!error.retryable);
    assert_eq!(
        error.message,
        "The selected Local TTS model is not installed and verified."
    );
    assert!(!error.message.contains("private local utterance"));
    assert!(!error.message.contains("conversation"));
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
fn malformed_persisted_settings_abort_startup_instead_of_using_fresh_defaults() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_string_lossy().to_string();
    let db = Database::new(&path).unwrap();
    db.set_setting("app_settings", "{not-valid-json").unwrap();
    drop(db);

    let secret_store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
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

    let result =
        AppState::new_with_secret_store(Some(dir.path().to_string_lossy().as_ref()), secret_store);

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
