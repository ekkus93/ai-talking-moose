use parking_lot::{Mutex, RwLock};
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
    // Serialize credential writes, verification reads, and cache updates across clones.
    mutation_lock: Arc<Mutex<()>>,
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
            mutation_lock: Arc::new(Mutex::new(())),
            google_api_key: Arc::new(RwLock::new(google_key)),
            openai_api_key: Arc::new(RwLock::new(openai_key)),
        })
    }

    pub fn set_google_api_key(&self, key: String) -> Result<(), String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return self.clear();
        }

        let _guard = self.mutation_lock.lock();
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
        let _guard = self.mutation_lock.lock();
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
        let _guard = self.mutation_lock.lock();
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
        let _guard = self.mutation_lock.lock();
        self.backend.delete_openai_api_key()?;
        *self.openai_api_key.write() = None;
        Ok(())
    }

    /// Redact every occurrence of both provider credentials against the original
    /// input, including overlapping values. Sequential replacement can expose
    /// part of a credential that overlaps another credential.
    pub fn redact(&self, input: &str) -> String {
        let keys: Vec<String> = [
            self.google_api_key.read().clone(),
            self.openai_api_key.read().clone(),
        ]
        .into_iter()
        .flatten()
        .filter(|key| !key.is_empty())
        .collect();

        // Scan UTF-8 character boundaries so overlapping matches of the same
        // credential are not skipped, unlike str::match_indices.
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for (start, _) in input.char_indices() {
            for key in &keys {
                if input[start..].starts_with(key) {
                    spans.push((start, start + key.len()));
                }
            }
        }
        spans.sort_unstable_by_key(|(start, _)| *start);

        // Coalesce overlapping spans before emitting any replacement text.
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in spans {
            if let Some((_, previous_end)) = merged.last_mut() {
                if start < *previous_end {
                    *previous_end = (*previous_end).max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }

        let mut redacted = String::with_capacity(input.len());
        let mut cursor = 0;
        for (start, end) in merged {
            redacted.push_str(&input[cursor..start]);
            redacted.push_str(crate::secrets::REDACTED_SECRET);
            cursor = end;
        }
        redacted.push_str(&input[cursor..]);
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
    fn partial_overlap_redacts_union_without_leaking_either_suffix() {
        let store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
        store.set_google_api_key("shared-ABCD".into()).unwrap();
        store.set_openai_api_key("ABCD-tail".into()).unwrap();
        assert_eq!(
            store.redact("combined shared-ABCD-tail separate shared-ABCD and ABCD-tail"),
            "combined [REDACTED_SECRET] separate [REDACTED_SECRET] and [REDACTED_SECRET]"
        );
        assert_eq!(store.redact("unrelated text"), "unrelated text");
    }

    #[test]
    fn overlapping_repetitions_and_utf8_boundaries_remain_fully_hidden() {
        let store = SecretStore::with_backend(Arc::new(MemorySecretBackend::default())).unwrap();
        store.set_google_api_key("abab".into()).unwrap();
        store.set_openai_api_key("baba".into()).unwrap();
        assert_eq!(store.redact("xababay"), "x[REDACTED_SECRET]y");

        store.set_google_api_key("café!".into()).unwrap();
        store.set_openai_api_key("é!xyz".into()).unwrap();
        assert_eq!(
            store.redact("prefix café!xyz suffix"),
            "prefix [REDACTED_SECRET] suffix"
        );
    }

    #[test]
    fn simultaneous_secret_updates_keep_backend_and_cache_consistent() {
        use std::sync::Barrier;

        let backend = Arc::new(MemorySecretBackend::default());
        let store = SecretStore::with_backend(backend.clone()).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        std::thread::scope(|scope| {
            for worker in 0..2 {
                let store = store.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    for iteration in 0..100 {
                        store
                            .set_openai_api_key(format!("openai-{worker}-{iteration}"))
                            .unwrap();
                        store
                            .set_google_api_key(format!("google-{worker}-{iteration}"))
                            .unwrap();
                    }
                });
            }
            barrier.wait();
        });
        assert!(Arc::ptr_eq(
            &store.mutation_lock,
            &store.clone().mutation_lock
        ));
        assert_eq!(*backend.openai_api_key.read(), store.get_openai_api_key());
        assert_eq!(*backend.google_api_key.read(), store.get_google_api_key());
        store.clear_openai_api_key().unwrap();
        assert!(!store.has_openai_api_key());
        assert!(store.has_google_api_key());
    }

    #[test]
    fn simultaneous_secret_clears_and_replacements_remain_consistent() {
        use std::sync::Barrier;

        let backend = Arc::new(MemorySecretBackend::default());
        let store = SecretStore::with_backend(backend.clone()).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        std::thread::scope(|scope| {
            for worker in 0..2 {
                let store = store.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    for iteration in 0..100 {
                        if (worker + iteration) % 2 == 0 {
                            store.clear_openai_api_key().unwrap();
                        } else {
                            store
                                .set_openai_api_key("session-key".into())
                                .unwrap();
                        }
                    }
                });
            }
            barrier.wait();
        });
        assert_eq!(*backend.openai_api_key.read(), store.get_openai_api_key());
        assert!(!store.has_google_api_key());
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
