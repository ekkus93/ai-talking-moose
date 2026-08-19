use parking_lot::RwLock;
use std::sync::Arc;

pub(crate) trait SecretBackend: Send + Sync {
    fn read_google_api_key(&self) -> Result<Option<String>, String>;
    fn write_google_api_key(&self, key: &str) -> Result<(), String>;
    fn delete_google_api_key(&self) -> Result<(), String>;
}

#[cfg(any(test, not(target_os = "macos")))]
#[derive(Default)]
pub(crate) struct MemorySecretBackend {
    google_api_key: RwLock<Option<String>>,
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
}

#[derive(Clone)]
pub struct SecretStore {
    backend: Arc<dyn SecretBackend>,
    google_api_key: Arc<RwLock<Option<String>>>,
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
        let initial = backend.read_google_api_key()?;
        Ok(Self {
            backend,
            google_api_key: Arc::new(RwLock::new(initial)),
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

    /// Redact any occurrence of the stored key in a log or string.
    pub fn redact(&self, input: &str) -> String {
        self.google_api_key.read().as_deref().map_or_else(
            || input.to_string(),
            |key| crate::secrets::redact_secret(input, key),
        )
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
    fn empty_key_clears_the_backend() {
        let backend = Arc::new(MemorySecretBackend::default());
        let store = SecretStore::with_backend(backend).unwrap();
        store.set_google_api_key("secret".to_string()).unwrap();
        store.set_google_api_key("   ".to_string()).unwrap();
        assert!(!store.has_google_api_key());
    }
}
