use super::super::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

/// Corrupt settings-version values must fail before an ASR migration or SQLite
/// rewrite, without logging attacker-controlled persisted values.
#[test]
fn malformed_settings_versions_fail_closed_and_preserve_sqlite() {
    for invalid in [
        serde_json::Value::Null,
        serde_json::json!(-1),
        serde_json::json!(3.5),
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::json!("synthetic-version-canary-not-for-logs"),
    ] {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value["settings_version"] = invalid.clone();
        value["asr_mode"] = serde_json::json!("moonshine_tiny_streaming");
        value["future_display_preference"] = serde_json::json!({"layout": "compact"});
        let original = value.to_string();

        let db = Database::new(&path).unwrap();
        db.set_setting("app_settings", &original).unwrap();
        drop(db);

        let secrets =
            SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
        let error = match AppState::new_with_secret_store(Some(&path), secrets) {
            Ok(_) => panic!("invalid version {invalid:?} must prevent startup"),
            Err(error) => error,
        };
        assert!(
            error.contains(
                "failed to decode persisted app settings: persisted settings are invalid: unsupported settings version"
            ),
            "invalid version {invalid:?} was not rejected: {error}"
        );
        assert!(!error.contains("synthetic-version-canary-not-for-logs"));

        let reopened = Database::new(&path).unwrap();
        assert_eq!(
            reopened.get_setting("app_settings").unwrap().as_deref(),
            Some(original.as_str()),
            "invalid version must not rewrite persisted settings or migrate ASR"
        );
    }
}
