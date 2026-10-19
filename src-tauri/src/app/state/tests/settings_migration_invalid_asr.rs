use super::super::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

/// Invalid persisted selectors must not silently become the fresh-profile
/// Whisper default or the historical Gemini Live default. Startup must fail
/// before a microphone, installer, or provider session can be started.
#[test]
fn malformed_asr_selector_types_fail_closed_and_leave_sqlite_unchanged() {
    for invalid in [
        serde_json::Value::Null,
        serde_json::json!(17),
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::json!("moonshine_unrecognized"),
    ] {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value["asr_mode"] = invalid.clone();
        value["future_display_preference"] = serde_json::json!({"layout": "compact"});
        let original = value.to_string();

        let db = Database::new(&path).unwrap();
        db.set_setting("app_settings", &original).unwrap();
        drop(db);

        let secrets =
            SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
        let error = match AppState::new_with_secret_store(Some(&path), secrets) {
            Ok(_) => panic!("invalid ASR selector {invalid:?} must prevent startup"),
            Err(error) => error,
        };
        assert!(
            error.contains("failed to decode persisted app settings"),
            "invalid ASR selector {invalid:?} was not rejected: {error}"
        );

        let reopened = Database::new(&path).unwrap();
        assert_eq!(
            reopened.get_setting("app_settings").unwrap().as_deref(),
            Some(original.as_str()),
            "failed migration must not rewrite SQLite or choose a fallback for {invalid:?}"
        );
    }
}
