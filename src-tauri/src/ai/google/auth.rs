use tracing::warn;

pub(crate) const GOOGLE_API_KEY_HEADER: &str = "x-goog-api-key";

pub(crate) fn trace_google_transport_error(surface: &'static str, error: &str, api_key: &str) {
    let safe_error = crate::secrets::redact_secret(error, api_key);
    warn!(
        provider = "gemini",
        surface = surface,
        error = %safe_error,
        "Google provider transport failure"
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
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

    impl CapturedLogs {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for CapturedLogs {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for CapturedLogs {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn masked_key_uses_character_boundaries_for_non_ascii_keys() {
        let auth = GoogleAuth::new("密钥ABCD1234終".to_string());
        assert_eq!(auth.masked_key(), "密钥AB...234終");
    }

    #[test]
    fn google_transport_tracing_redacts_raw_key_for_every_surface() {
        const KEY: &str = "AIzaSyPRIVATE_GOOGLE_KEY_65f2";
        let captured = CapturedLogs::default();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(captured.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            for surface in ["text", "tts", "live"] {
                trace_google_transport_error(
                    surface,
                    &format!("request failed at https://example.invalid/?key={KEY}"),
                    KEY,
                );
            }
        });

        let logs = captured.text();
        assert!(logs.contains("Google provider transport failure"));
        assert!(logs.contains("[REDACTED_SECRET]"));
        assert!(!logs.contains(KEY));
    }
}
