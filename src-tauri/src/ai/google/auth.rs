use crate::ai::types::ProviderError;
use tracing::warn;

pub(crate) const GOOGLE_API_KEY_HEADER: &str = "x-goog-api-key";

pub(crate) fn trace_google_provider_failure(surface: &'static str, error: &ProviderError) {
    warn!(
        provider = "gemini",
        surface = surface,
        error_kind = ?error.kind,
        retryable = error.retryable,
        "Google provider failure"
    );
}

#[derive(Debug, Clone)]
pub struct GoogleAuth {
    pub api_key: String,
}

impl GoogleAuth {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn is_valid(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    pub fn redact(&self, input: &str) -> String {
        crate::secrets::redact_secret(input, &self.api_key)
    }

    pub fn masked_key(&self) -> String {
        if self.api_key.chars().count() <= 8 {
            return "********".to_string();
        }

        let prefix: String = self.api_key.chars().take(4).collect();
        let suffix_chars: Vec<char> = self.api_key.chars().rev().take(4).collect();
        let suffix: String = suffix_chars.into_iter().rev().collect();
        format!("{prefix}...{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{assert_log_capture_live, capture_logs};

    #[test]
    fn masked_key_uses_character_boundaries_for_non_ascii_keys() {
        let auth = GoogleAuth::new("密钥ABCD1234終".to_string());
        assert_eq!(auth.masked_key(), "密钥AB...234終");
    }

    #[test]
    fn google_provider_failure_logging_never_includes_raw_error_detail() {
        const PRIVATE_DETAIL: &str =
            "request failed at https://private.example.invalid/?key=AIzaSyPRIVATE_GOOGLE_KEY_65f2";
        let (_, logs) = capture_logs(|| {
            for surface in ["text", "tts", "live"] {
                let error = ProviderError {
                    kind: crate::ai::types::ProviderErrorKind::Network,
                    message: PRIVATE_DETAIL.to_string(),
                    retryable: true,
                };
                trace_google_provider_failure(surface, &error);
            }
        });

        assert_log_capture_live(&logs);
        assert!(logs.contains("Google provider failure"));
        assert!(logs.contains("Network"));
        assert!(!logs.contains(PRIVATE_DETAIL));
        assert!(!logs.contains("private.example.invalid"));
        assert!(!logs.contains("AIzaSyPRIVATE_GOOGLE_KEY_65f2"));
    }
}
