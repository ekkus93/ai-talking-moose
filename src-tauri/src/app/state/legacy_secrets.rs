use crate::persistence::sqlite::Database;
use crate::secrets::SecretStore;
use tracing::warn;

pub(super) const LEGACY_GOOGLE_API_KEY_SETTING: &str = "google_api_key";

pub(super) fn migrate_legacy_google_api_key(
    db: &Database,
    secrets: &SecretStore,
) -> Result<(), String> {
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
