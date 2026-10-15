use parking_lot::RwLock;
use std::sync::Arc;

pub(crate) trait SecretBackend: Send + Sync {
    fn read_google_api_key(&self) -> Result<Option<String>, String>;
    fn write_google_api_key(&self, key: &str) -> Result<(), String>;
    fn delete_google_api_key(&self) -> Result<(), String>;
    fn read_openai_api_key(&self) -> Result<Option<String>, String>;
    fn write_openai_api_key(&self, key: &str) -> Result<(), String>;
    fn delete_openai_api_key(&self) -> Result<(), String>;
}

#[cfg(any(test, not(target_os = "macos")))]
#[derive(Default)]
pub(crate) struct MemorySecretBackend {
    google_api_key: RwLock<Option<String>>,
    openai_api_key: RwLock<Option<String>>,
}

#[cfg(any(test, not(target_os = "macos")))]
impl SecretBackend for MemorySecretBackend {
    fn read_google_api_key(&self) -> Result<Option<String>, String> {
        Ok(self.google_api_key.read().clone())
    }

    fn write_google_api_key(&self, key: &str) -> Result<(), String> {
        *self.google_api_key.write() = Some(key.to_string());
        Ok(())
    }

    fn delete_google_api_key(&self) -> Result<(), String> {
        *self.google_api_key.write() = None;
        Ok(())
    }

    fn read_openai_api_key(&self) -> Result<Option<String>, String> {
        Ok(self.openai_api_key.read().clone())
    }

    fn write_openai_api_key(&self, key: &str) -> Result<(), String> {
        *self.openai_api_key.write() = Some(key.to_string());
        Ok(())
    }

    fn delete_openai_api_key(&self) -> Result<(), String> {
        *self.openai_api_key.write() = None;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SecretStore {
    backend: Arc<dyn SecretBackend>,
    google_api_key: Arc<RwLock<Option<String>>>,
    openai_api_key: Arc<RwLock<Option<String>>>,
}

impl SecretStore {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        let backend: Arc<dyn SecretBackend> =
            Arc::new(crate::secrets::macos_keychain::MacOsKeychainBackend);

        #[cfg(not(target_os = "macos"))]
        let backend: Arc<dyn SecretBackend> = Arc::new(MemorySecretBackend::default());

        Self::with_backend(backend)
    }

    pub(crate) fn with_backend(backend: Arc<dyn SecretBackend>) -> Result<Self, String> {
        let google_key = backend.read_google_api_key()?;
        let openai_key = backend.read_openai_api_key()?;
        Ok(Self {
            backend,
            google_api_key: Arc::new(RwLock::new(google_key)),
            openai_api_key: Arc::new(RwLock::new(openai_key)),
        })
    }

    pub fn set_google_api_key(&self, key: String) -> Result<(), String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return self.clear();
        }

        self.backend.write_google_api_key(trimmed)?;
        let verified = self.backend.read_google_api_key()?;
        if verified.as_deref() != Some(trimmed) {
            return Err("secure credential verification failed".to_string());
        }

        *self.google_api_key.write() = verified;
        Ok(())
    }

    pub fn get_google_api_key(&self) -> Option<String> {
        self.google_api_key.read().clone()
    }

    pub fn has_google_api_key(&self) -> bool {
        self.google_api_key.read().is_some()
    }

    pub fn clear(&self) -> Result<(), String> {
        self.backend.delete_google_api_key()?;
        *self.google_api_key.write() = None;
        Ok(())
    }

    pub fn set_openai_api_key(&self, key: String) -> Result<(), String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return self.clear_openai_api_key();
        }
        if trimmed.len() > 4096 || trimmed.chars().any(char::is_control) {
            return Err("OpenAI API key must be a bounded single-line credential".to_string());
        }
        self.backend.write_openai_api_key(trimmed)?;
        let verified = self.backend.read_openai_api_key()?;
        if verified.as_deref() != Some(trimmed) {
            return Err("secure credential verification failed".to_string());
        }
        *self.openai_api_key.write() = verified;
        Ok(())
    }

    pub fn get_openai_api_key(&self) -> Option<String> {
        self.openai_api_key.read().clone()
    }

    pub fn has_openai_api_key(&self) -> bool {
        self.openai_api_key.read().is_some()
    }

    pub fn clear_openai_api_key(&self) -> Result<(), String> {
        self.backend.delete_openai_api_key()?;
        *self.openai_api_key.write() = None;
        Ok(())
    }

    /// Redact both independent provider credentials from diagnostics and errors.
    pub fn redact(&self, input: &str) -> String {
        // Replace longer credentials first. If one key is a prefix of another,
        // replacing the shorter key first would leave the longer key's suffix
        // visible in diagnostics and prevent its subsequent exact-match redaction.
        let mut keys: Vec<String> = [
            self.google_api_key.read().clone(),
            self.openai_api_key.read().clone(),
        ]
        .into_iter()
        .flatten()
        .filter(|key| !key.is_empty())
        .collect();
        keys.sort_unstable_by_key(|key| std::cmp::Reverse(key.len()));
        let mut redacted = input.to_string();
        for key in keys {
            redacted = crate::secrets::redact_secret(&redacted, &key);
        }
        redacted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_backend_persists_for_store_lifetime_and_redacts() {
        let backend = Arc::new(MemorySecretBackend::default());
        let store = SecretStore::with_backend(backend.clone()).unwrap();
        assert!(!store.has_google_api_key());

        store
            .set_google_api_key("AIzaSySecretTestKey123".to_string())
            .unwrap();
        assert!(store.has_google_api_key());
        assert_eq!(
            store.get_google_api_key().as_deref(),
            Some("AIzaSySecretTestKey123")
        );

        let reloaded = SecretStore::with_backend(backend).unwrap();
        assert_eq!(
            reloaded.get_google_api_key().as_deref(),
            Some("AIzaSySecretTestKey123")
        );
        assert_eq!(
            reloaded.redact("key=AIzaSySecretTestKey123"),
            "key=[REDACTED_SECRET]"
        );

        reloaded.clear().unwrap();
        assert!(!reloaded.has_google_api_key());
    }

    #[test]
    fn provider_keys_have_independent_lifetimes_and_both_are_redacted() {
        let backend = Arc::new(MemorySecretBackend::default());
        let store = SecretStore::with_backend(backend.clone()).unwrap();
        store
            .set_google_api_key("google-test-secret".into())
            .unwrap();
        store
            .set_openai_api_key("sk-openai-test-secret".into())
            .unwrap();
        let reloaded = SecretStore::with_backend(backend).unwrap();
        assert!(reloaded.has_google_api_key());
        assert!(reloaded.has_openai_api_key());
        assert_eq!(
            reloaded.get_openai_api_key().as_deref(),
            Some("sk-openai-test-secret")
        );
        let redacted = reloaded.redact("Google google-test-secret OpenAI sk-openai-test-secret");
        assert!(!redacted.contains("google-test-secret"));
        assert!(!redacted.contains("sk-openai-test-secret"));
        reloaded.clear().unwrap();
        assert!(!reloaded.has_google_api_key());
        assert!(reloaded.has_openai_api_key());
        reloaded
            .set_google_api_key("new-google-secret".into())
            .unwrap();
        reloaded.clear_openai_api_key().unwrap();
        assert!(reloaded.has_google_api_key());
        assert!(!reloaded.has_openai_api_key());
        assert!(reloaded.set_openai_api_key("bad\nsecret".into()).is_err());
        assert!(!reloaded.has_openai_api_key());
    }

    #[test]
    fn overlapping_provider_keys_are_redacted_without_exposing_suffixes() {
        let store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
        store.set_google_api_key("shared-prefix".into()).unwrap();
        store
            .set_openai_api_key("shared-prefix-extended-openai".into())
            .unwrap();

        let message = "Google shared-prefix; OpenAI shared-prefix-extended-openai";
        assert_eq!(
            store.redact(message),
            "Google [REDACTED_SECRET]; OpenAI [REDACTED_SECRET]"
        );
        assert!(!store.redact(message).contains("-extended-openai"));

        // Reverse the overlap: the Google credential is now the longer one.
        store
            .set_google_api_key("short-key-long-google".into())
            .unwrap();
        store.set_openai_api_key("short-key".into()).unwrap();
        assert_eq!(
            store.redact("short-key-long-google and short-key"),
            "[REDACTED_SECRET] and [REDACTED_SECRET]"
        );
    }

    #[test]
    fn empty_key_clears_the_backend() {
        let backend = Arc::new(MemorySecretBackend::default());
        let store = SecretStore::with_backend(backend).unwrap();
        store.set_google_api_key("secret".to_string()).unwrap();
        store.set_google_api_key("   ".to_string()).unwrap();
        assert!(!store.has_google_api_key());
    }
}
