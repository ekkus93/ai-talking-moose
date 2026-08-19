#[cfg(target_os = "macos")]
mod macos_keychain;
mod store;

pub use store::SecretStore;
#[cfg(test)]
pub(crate) use store::{MemorySecretBackend, SecretBackend};

const REDACTED_SECRET: &str = "[REDACTED_SECRET]";

/// Replace an exact secret value wherever it appears in an otherwise loggable string.
/// Empty secrets are ignored so a missing credential cannot corrupt every string.
pub fn redact_secret(input: &str, secret: &str) -> String {
    if secret.is_empty() {
        input.to_string()
    } else {
        input.replace(secret, REDACTED_SECRET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "AIzaSySecretTestKey123";

    #[test]
    fn redacts_secret_in_common_log_shapes() {
        for input in [
            format!("key={KEY}"),
            format!("Authorization: Bearer {KEY}"),
            format!(r#"{{"api_key":"{KEY}"}}"#),
            format!("https://example.invalid/path?key={KEY}&x=1"),
        ] {
            let redacted = redact_secret(&input, KEY);
            assert!(!redacted.contains(KEY));
            assert!(redacted.contains(REDACTED_SECRET));
        }
    }

    #[test]
    fn empty_secret_does_not_modify_input() {
        assert_eq!(redact_secret("hello", ""), "hello");
    }
}
