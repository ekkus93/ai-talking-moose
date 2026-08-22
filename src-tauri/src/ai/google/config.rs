use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoogleModelCapability {
    LiveAudio,
    TextGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GoogleModelDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub capabilities: &'static [GoogleModelCapability],
}

const LIVE_AUDIO: &[GoogleModelCapability] = &[GoogleModelCapability::LiveAudio];
const TEXT_GENERATION: &[GoogleModelCapability] = &[GoogleModelCapability::TextGeneration];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GoogleProviderConfig {
    pub live_websocket_endpoint: &'static str,
    pub models: &'static [GoogleModelDescriptor],
}

pub const GOOGLE_MODELS: &[GoogleModelDescriptor] = &[
    GoogleModelDescriptor {
        id: "gemini-3.1-flash-live-preview",
        display_name: "Gemini 3.1 Flash Live Preview",
        capabilities: LIVE_AUDIO,
    },
    GoogleModelDescriptor {
        id: "gemini-3.7-flash",
        display_name: "Gemini 3.7 Flash",
        capabilities: TEXT_GENERATION,
    },
    GoogleModelDescriptor {
        id: "gemini-3.6-flash",
        display_name: "Gemini 3.6 Flash",
        capabilities: TEXT_GENERATION,
    },
];

pub const GOOGLE_PROVIDER_CONFIG: GoogleProviderConfig = GoogleProviderConfig {
    live_websocket_endpoint: "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent",
    models: GOOGLE_MODELS,
};

// Compatibility alias for transport code. The endpoint value remains owned by
// the typed provider configuration above, so there is still one source of truth.
pub const LIVE_WEBSOCKET_ENDPOINT: &str = GOOGLE_PROVIDER_CONFIG.live_websocket_endpoint;

pub const DEFAULT_LIVE_MODEL: &str = GOOGLE_MODELS[0].id;
pub const DEFAULT_TEXT_MODEL: &str = GOOGLE_MODELS[1].id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GoogleTtsVoiceDescriptor {
    pub id: &'static str,
    pub style: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GoogleTtsConfig {
    pub models: &'static [&'static str],
    pub voices: &'static [GoogleTtsVoiceDescriptor],
    pub default_model: &'static str,
    pub default_voice: &'static str,
}

/// Standalone TTS stays on the generateContent-compatible 2.5 Flash TTS model for
/// this V1 implementation. The model identifier is centralized here so a future
/// transport/model migration cannot diverge across runtime call sites.
pub const GOOGLE_TTS_MODELS: &[&str] = &["gemini-2.5-flash-preview-tts"];
pub const DEFAULT_TTS_MODEL: &str = GOOGLE_TTS_MODELS[0];
pub const LEGACY_TTS_MODEL: &str = "en-US-Standard-B";

/// Current Gemini TTS / Live native-audio voice catalog. Live native audio supports
/// the TTS voice set, so one validated voice selector can drive both paths.
pub const GOOGLE_TTS_VOICES: &[GoogleTtsVoiceDescriptor] = &[
    GoogleTtsVoiceDescriptor {
        id: "Zephyr",
        style: "Bright",
    },
    GoogleTtsVoiceDescriptor {
        id: "Puck",
        style: "Upbeat",
    },
    GoogleTtsVoiceDescriptor {
        id: "Charon",
        style: "Informative",
    },
    GoogleTtsVoiceDescriptor {
        id: "Kore",
        style: "Firm",
    },
    GoogleTtsVoiceDescriptor {
        id: "Fenrir",
        style: "Excitable",
    },
    GoogleTtsVoiceDescriptor {
        id: "Leda",
        style: "Youthful",
    },
    GoogleTtsVoiceDescriptor {
        id: "Orus",
        style: "Firm",
    },
    GoogleTtsVoiceDescriptor {
        id: "Aoede",
        style: "Breezy",
    },
    GoogleTtsVoiceDescriptor {
        id: "Callirrhoe",
        style: "Easy-going",
    },
    GoogleTtsVoiceDescriptor {
        id: "Autonoe",
        style: "Bright",
    },
    GoogleTtsVoiceDescriptor {
        id: "Enceladus",
        style: "Breathy",
    },
    GoogleTtsVoiceDescriptor {
        id: "Iapetus",
        style: "Clear",
    },
    GoogleTtsVoiceDescriptor {
        id: "Umbriel",
        style: "Easy-going",
    },
    GoogleTtsVoiceDescriptor {
        id: "Algieba",
        style: "Smooth",
    },
    GoogleTtsVoiceDescriptor {
        id: "Despina",
        style: "Smooth",
    },
    GoogleTtsVoiceDescriptor {
        id: "Erinome",
        style: "Clear",
    },
    GoogleTtsVoiceDescriptor {
        id: "Algenib",
        style: "Gravelly",
    },
    GoogleTtsVoiceDescriptor {
        id: "Rasalgethi",
        style: "Informative",
    },
    GoogleTtsVoiceDescriptor {
        id: "Laomedeia",
        style: "Upbeat",
    },
    GoogleTtsVoiceDescriptor {
        id: "Achernar",
        style: "Soft",
    },
    GoogleTtsVoiceDescriptor {
        id: "Alnilam",
        style: "Firm",
    },
    GoogleTtsVoiceDescriptor {
        id: "Schedar",
        style: "Even",
    },
    GoogleTtsVoiceDescriptor {
        id: "Gacrux",
        style: "Mature",
    },
    GoogleTtsVoiceDescriptor {
        id: "Pulcherrima",
        style: "Forward",
    },
    GoogleTtsVoiceDescriptor {
        id: "Achird",
        style: "Friendly",
    },
    GoogleTtsVoiceDescriptor {
        id: "Zubenelgenubi",
        style: "Casual",
    },
    GoogleTtsVoiceDescriptor {
        id: "Vindemiatrix",
        style: "Gentle",
    },
    GoogleTtsVoiceDescriptor {
        id: "Sadachbia",
        style: "Lively",
    },
    GoogleTtsVoiceDescriptor {
        id: "Sadaltager",
        style: "Knowledgeable",
    },
    GoogleTtsVoiceDescriptor {
        id: "Sulafat",
        style: "Warm",
    },
];
pub const DEFAULT_TTS_VOICE: &str = "Fenrir";

pub const GOOGLE_TTS_CONFIG: GoogleTtsConfig = GoogleTtsConfig {
    models: GOOGLE_TTS_MODELS,
    voices: GOOGLE_TTS_VOICES,
    default_model: DEFAULT_TTS_MODEL,
    default_voice: DEFAULT_TTS_VOICE,
};

pub fn supports_capability(model_id: &str, capability: GoogleModelCapability) -> bool {
    GOOGLE_MODELS
        .iter()
        .any(|model| model.id == model_id && model.capabilities.contains(&capability))
}

pub fn validate_live_model(model_id: &str) -> Result<(), String> {
    if supports_capability(model_id, GoogleModelCapability::LiveAudio) {
        Ok(())
    } else {
        Err(format!(
            "Gemini model '{model_id}' does not support Live audio conversations"
        ))
    }
}

pub fn validate_text_model(model_id: &str) -> Result<(), String> {
    if supports_capability(model_id, GoogleModelCapability::TextGeneration) {
        Ok(())
    } else {
        Err(format!(
            "Gemini model '{model_id}' does not support text generation in this application"
        ))
    }
}

pub fn validate_tts_model(model_id: &str) -> Result<(), String> {
    if GOOGLE_TTS_MODELS.contains(&model_id) || model_id == LEGACY_TTS_MODEL {
        Ok(())
    } else {
        Err(format!("unsupported Gemini TTS model '{model_id}'"))
    }
}

pub fn validate_tts_voice(voice_id: &str) -> Result<(), String> {
    if GOOGLE_TTS_VOICES.iter().any(|voice| voice.id == voice_id) {
        Ok(())
    } else {
        Err(format!("unsupported Gemini TTS voice '{voice_id}'"))
    }
}

pub fn normalize_live_model(model_id: &str) -> &'static str {
    if supports_capability(model_id, GoogleModelCapability::LiveAudio) {
        GOOGLE_MODELS
            .iter()
            .find(|model| model.id == model_id)
            .map_or(DEFAULT_LIVE_MODEL, |model| model.id)
    } else {
        DEFAULT_LIVE_MODEL
    }
}

pub fn normalize_text_model(model_id: &str) -> &'static str {
    if supports_capability(model_id, GoogleModelCapability::TextGeneration) {
        GOOGLE_MODELS
            .iter()
            .find(|model| model.id == model_id)
            .map_or(DEFAULT_TEXT_MODEL, |model| model.id)
    } else {
        DEFAULT_TEXT_MODEL
    }
}

pub fn normalize_tts_model(model_id: &str) -> &'static str {
    if model_id == LEGACY_TTS_MODEL {
        return DEFAULT_TTS_MODEL;
    }

    GOOGLE_TTS_MODELS
        .iter()
        .copied()
        .find(|model| *model == model_id)
        .unwrap_or(DEFAULT_TTS_MODEL)
}

pub fn normalize_tts_voice(voice_id: &str) -> &'static str {
    GOOGLE_TTS_VOICES
        .iter()
        .find(|voice| voice.id == voice_id)
        .map_or(DEFAULT_TTS_VOICE, |voice| voice.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_defaults_have_the_expected_capabilities() {
        assert!(supports_capability(
            DEFAULT_LIVE_MODEL,
            GoogleModelCapability::LiveAudio
        ));
        assert!(supports_capability(
            DEFAULT_TEXT_MODEL,
            GoogleModelCapability::TextGeneration
        ));
        assert!(!supports_capability(
            DEFAULT_TEXT_MODEL,
            GoogleModelCapability::LiveAudio
        ));
        assert!(!supports_capability(
            DEFAULT_LIVE_MODEL,
            GoogleModelCapability::TextGeneration
        ));
    }

    #[test]
    fn stale_or_wrong_capability_models_normalize_to_current_defaults() {
        assert_eq!(
            normalize_live_model("gemini-2.5-flash-native-audio-latest"),
            DEFAULT_LIVE_MODEL
        );
        assert_eq!(normalize_live_model(DEFAULT_TEXT_MODEL), DEFAULT_LIVE_MODEL);
        assert_eq!(normalize_text_model("gemini-2.5-flash"), DEFAULT_TEXT_MODEL);
        assert_eq!(normalize_text_model(DEFAULT_LIVE_MODEL), DEFAULT_TEXT_MODEL);
    }

    #[test]
    fn tts_catalog_has_a_valid_default_and_migrates_legacy_model_ids() {
        validate_tts_model(DEFAULT_TTS_MODEL).unwrap();
        validate_tts_model(LEGACY_TTS_MODEL).unwrap();
        validate_tts_voice(DEFAULT_TTS_VOICE).unwrap();
        assert_eq!(DEFAULT_TTS_VOICE, "Fenrir");
        assert_eq!(
            GOOGLE_TTS_VOICES
                .iter()
                .find(|voice| voice.id == DEFAULT_TTS_VOICE)
                .unwrap()
                .style,
            "Excitable"
        );
        assert_eq!(normalize_tts_model(LEGACY_TTS_MODEL), DEFAULT_TTS_MODEL);
        assert_eq!(normalize_tts_voice("not-a-real-voice"), DEFAULT_TTS_VOICE);
    }

    #[test]
    fn model_catalog_serializes_typed_capabilities() {
        let value = serde_json::to_value(GOOGLE_MODELS).unwrap();
        assert_eq!(value[0]["id"], DEFAULT_LIVE_MODEL);
        assert_eq!(value[0]["capabilities"][0], "live_audio");
        assert_eq!(value[1]["id"], DEFAULT_TEXT_MODEL);
        assert_eq!(value[1]["capabilities"][0], "text_generation");
    }

    #[test]
    fn provider_config_serializes_current_live_endpoint_and_model_catalog() {
        let value = serde_json::to_value(GOOGLE_PROVIDER_CONFIG).unwrap();
        assert_eq!(
            value["live_websocket_endpoint"],
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent"
        );
        assert_eq!(
            LIVE_WEBSOCKET_ENDPOINT,
            GOOGLE_PROVIDER_CONFIG.live_websocket_endpoint
        );
        assert_eq!(value["models"][0]["id"], DEFAULT_LIVE_MODEL);
        assert_eq!(value["models"][1]["id"], DEFAULT_TEXT_MODEL);
    }

    #[test]
    fn tts_config_serializes_model_and_voice_catalog() {
        let value = serde_json::to_value(GOOGLE_TTS_CONFIG).unwrap();
        assert_eq!(value["default_model"], DEFAULT_TTS_MODEL);
        assert_eq!(value["default_voice"], DEFAULT_TTS_VOICE);
        assert_eq!(value["voices"].as_array().unwrap().len(), 30);
    }
}
