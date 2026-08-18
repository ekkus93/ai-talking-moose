use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct SecretStore {
    google_api_key: Arc<RwLock<Option<String>>>,
}

impl SecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_google_api_key(&self, key: String) {
        let trimmed = key.trim().to_string();
        let mut write = self.google_api_key.write();
        if trimmed.is_empty() {
            *write = None;
        } else {
            *write = Some(trimmed);
        }
    }

    pub fn get_google_api_key(&self) -> Option<String> {
        self.google_api_key.read().clone()
    }

    pub fn has_google_api_key(&self) -> bool {
        self.google_api_key.read().is_some()
    }

    pub fn clear(&self) {
        *self.google_api_key.write() = None;
    }

    /// Redact any occurrence of the stored key in a log or string
    pub fn redact(&self, input: &str) -> String {
        if let Some(ref key) = *self.google_api_key.read() {
            if !key.is_empty() {
                return input.replace(key, "[REDACTED_API_KEY]");
            }
        }
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_store_and_redaction() {
        let store = SecretStore::new();
        assert!(!store.has_google_api_key());

        store.set_google_api_key("AIzaSySecretTestKey123".to_string());
        assert!(store.has_google_api_key());
        assert_eq!(
            store.get_google_api_key(),
            Some("AIzaSySecretTestKey123".to_string())
        );

        let log_msg = "Error connecting with key AIzaSySecretTestKey123: 403 Forbidden";
        let redacted = store.redact(log_msg);
        assert_eq!(
            redacted,
            "Error connecting with key [REDACTED_API_KEY]: 403 Forbidden"
        );

        store.clear();
        assert!(!store.has_google_api_key());
    }
}
