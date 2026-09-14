mod engine;
mod runtime;

pub use engine::{
    verify_model_artifacts, SherpaKwsEngine, WakeWordEngine, WakeWordError, WakeWordErrorKind,
    DEFAULT_KEYWORDS_SCORE, DEFAULT_KEYWORDS_THRESHOLD, WAKE_CHANNELS, WAKE_INFERENCE_THREADS,
    WAKE_SAMPLE_RATE_HZ,
};
pub use runtime::{
    WakeTriggerCallback, WakeWordDiagnostics, WakeWordRuntimeManager, WakeWordRuntimeState,
};

pub const DEFAULT_WAKE_PHRASE: &str = "Hey, Moose";
pub const CANONICAL_WAKE_KEYWORD: &str = "HEY MOOSE";
pub const WAKE_RING_BUFFER_SECONDS: u32 = 2;

pub fn normalize_wake_phrase(value: &str) -> Option<String> {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character.is_whitespace() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase();
    (normalized == CANONICAL_WAKE_KEYWORD).then(|| DEFAULT_WAKE_PHRASE.to_string())
}

#[cfg(test)]
mod phrase_tests {
    use super::*;

    #[test]
    fn v1_phrase_normalization_is_fixed_and_deterministic() {
        for value in ["Hey, Moose", "hey moose", "HEY MOOSE", " Hey   Moose "] {
            assert_eq!(
                normalize_wake_phrase(value).as_deref(),
                Some(DEFAULT_WAKE_PHRASE)
            );
        }
        assert_eq!(normalize_wake_phrase("Hey Bruce"), None);
        assert_eq!(normalize_wake_phrase("Moose"), None);
    }
}
