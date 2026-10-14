//! Validate OS-supplied voice identifiers before presenting a macOS voice catalog.
use crate::ai::system_tts_selection::SYSTEM_DEFAULT_VOICE_ID;
use crate::ai::tts_catalog::TtsVoiceDescriptor;
use std::collections::HashSet;

const MAX_VOICE_NAME_BYTES: usize = 256;
const MAX_VOICE_STYLE_BYTES: usize = 128;

pub fn normalize_installed_voices(voices: Vec<TtsVoiceDescriptor>) -> Vec<TtsVoiceDescriptor> {
    let mut seen = HashSet::new();
    voices
        .into_iter()
        .filter(|voice| {
            !voice.id.is_empty()
                && voice.id != SYSTEM_DEFAULT_VOICE_ID
                && voice.id.len() <= 128
                && voice.id.trim() == voice.id
                && !voice.id.chars().any(char::is_control)
                && !voice.display_name.trim().is_empty()
                && voice.display_name.len() <= MAX_VOICE_NAME_BYTES
                && !voice.display_name.chars().any(char::is_control)
                && voice.style.as_ref().is_none_or(|style| {
                    style.len() <= MAX_VOICE_STYLE_BYTES && !style.chars().any(char::is_control)
                })
                && seen.insert(voice.id.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_invalid_and_reserved_voice_ids() {
        let voice = |id: &str| TtsVoiceDescriptor {
            id: id.into(),
            display_name: "Voice".into(),
            style: None,
        };
        let voices = vec![
            voice("com.apple.voice.one"),
            voice("com.apple.voice.one"),
            voice(SYSTEM_DEFAULT_VOICE_ID),
            voice("bad\nvoice"),
            voice(""),
            voice("com.apple.voice.two"),
        ];
        let normalized = normalize_installed_voices(voices);
        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].id, "com.apple.voice.one");
        assert_eq!(normalized[1].id, "com.apple.voice.two");
        let bad = TtsVoiceDescriptor {
            id: "com.apple.voice.bad".into(),
            display_name: String::new(),
            style: None,
        };
        assert!(normalize_installed_voices(vec![bad]).is_empty());
        let bad_style = TtsVoiceDescriptor {
            id: "com.apple.voice.bad".into(),
            display_name: "Valid".into(),
            style: Some("en\nUS".into()),
        };
        assert!(normalize_installed_voices(vec![bad_style.clone()]).is_empty());
        let valid = TtsVoiceDescriptor {
            id: bad_style.id.clone(),
            display_name: "Valid".into(),
            style: None,
        };
        assert_eq!(normalize_installed_voices(vec![bad_style, valid]).len(), 1);
    }
    #[test]
    fn oversized_metadata_and_padded_identifiers_do_not_poison_later_valid_voices() {
        let voice = |id: &str, display_name: String, style: Option<String>| TtsVoiceDescriptor {
            id: id.into(),
            display_name,
            style,
        };
        let accepted = voice(
            "com.apple.voice.valid",
            "Readable Voice".into(),
            Some("en-US".into()),
        );
        let voices = vec![
            voice(" padded.id", "Padded".into(), None),
            voice("trailing.id ", "Padded".into(), None),
            voice(
                "com.apple.voice.valid",
                "x".repeat(MAX_VOICE_NAME_BYTES + 1),
                None,
            ),
            voice(
                "com.apple.voice.valid",
                "Readable Voice".into(),
                Some("x".repeat(MAX_VOICE_STYLE_BYTES + 1)),
            ),
            accepted.clone(),
            accepted.clone(),
        ];
        assert_eq!(normalize_installed_voices(voices), vec![accepted]);
    }
}
