use serde::{Deserialize, Serialize};

pub const LIVE_WEBSOCKET_ENDPOINT: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";
pub const DEFAULT_LIVE_MODEL: &str = "gemini-3.1-flash-live-preview";
pub const DEFAULT_TEXT_MODEL: &str = "gemini-3.7-flash";

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

pub const GOOGLE_MODELS: &[GoogleModelDescriptor] = &[
    GoogleModelDescriptor {
        id: DEFAULT_LIVE_MODEL,
        display_name: "Gemini 3.1 Flash Live Preview",
        capabilities: LIVE_AUDIO,
    },
    GoogleModelDescriptor {
        id: DEFAULT_TEXT_MODEL,
        display_name: "Gemini 3.7 Flash",
        capabilities: TEXT_GENERATION,
    },
    GoogleModelDescriptor {
        id: "gemini-3.6-flash",
        display_name: "Gemini 3.6 Flash",
        capabilities: TEXT_GENERATION,
    },
];

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
}
