//! Validate OS-supplied voice identifiers before presenting a macOS voice catalog.
use crate::ai::system_tts_selection::SYSTEM_DEFAULT_VOICE_ID;
use crate::ai::tts_catalog::TtsVoiceDescriptor;
use std::collections::HashSet;

pub fn normalize_installed_voices(voices: Vec<TtsVoiceDescriptor>) -> Vec<TtsVoiceDescriptor> {
    let mut seen = HashSet::new();
    voices
        .into_iter()
        .filter(|voice| {
            !voice.id.is_empty()
                && voice.id != SYSTEM_DEFAULT_VOICE_ID
                && voice.id.len() <= 128
                && !voice.id.chars().any(char::is_control)
                && !voice.display_name.trim().is_empty()
                && !voice.display_name.chars().any(char::is_control)
                && voice
                    .style
                    .as_ref()
                    .is_none_or(|style| !style.chars().any(char::is_control))
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
    }
}
