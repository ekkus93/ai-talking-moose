use crate::ai::google::{GoogleTtsVoiceDescriptor, GOOGLE_TTS_MODELS, GOOGLE_TTS_VOICES};
use crate::ai::local_tts::{LOCAL_TTS_MODEL_IDS, LOCAL_TTS_VOICE_IDS};
use crate::ai::types::TtsProvider;
use serde::{Deserialize, Serialize};

pub const TTS_SAMPLE_RATE_HZ: u32 = 24_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsModelDescriptor {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsVoiceDescriptor {
    pub id: String,
    pub display_name: String,
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsProviderDescriptor {
    pub id: TtsProvider,
    pub display_name: String,
    pub is_local: bool,
    pub models: Vec<TtsModelDescriptor>,
    pub voices: Vec<TtsVoiceDescriptor>,
    pub sample_rate_hz: u32,
    pub supports_speaking_rate: bool,
    pub supports_pitch: bool,
    pub install_required: bool,
    pub license_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiLiveVoiceCatalog {
    pub display_name: String,
    pub is_local: bool,
    pub sample_rate_hz: u32,
    pub voices: Vec<TtsVoiceDescriptor>,
    pub license_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsCatalog {
    pub providers: Vec<TtsProviderDescriptor>,
    pub gemini_live: GeminiLiveVoiceCatalog,
}

fn google_voice(voice: &GoogleTtsVoiceDescriptor) -> TtsVoiceDescriptor {
    TtsVoiceDescriptor {
        id: voice.id.to_string(),
        display_name: voice.id.to_string(),
        style: Some(voice.style.to_string()),
    }
}

fn google_voices() -> Vec<TtsVoiceDescriptor> {
    GOOGLE_TTS_VOICES.iter().map(google_voice).collect()
}

fn local_voices() -> Vec<TtsVoiceDescriptor> {
    LOCAL_TTS_VOICE_IDS
        .iter()
        .map(|voice| TtsVoiceDescriptor {
            id: (*voice).to_string(),
            display_name: (*voice).to_string(),
            style: None,
        })
        .collect()
}

pub fn tts_catalog() -> TtsCatalog {
    let google_voices = google_voices();
    TtsCatalog {
        providers: vec![
            TtsProviderDescriptor {
                id: TtsProvider::Google,
                display_name: "Google Gemini TTS".to_string(),
                is_local: false,
                models: GOOGLE_TTS_MODELS
                    .iter()
                    .map(|model| TtsModelDescriptor {
                        id: (*model).to_string(),
                        display_name: "Gemini 2.5 Flash Preview TTS".to_string(),
                    })
                    .collect(),
                voices: google_voices.clone(),
                sample_rate_hz: TTS_SAMPLE_RATE_HZ,
                supports_speaking_rate: true,
                supports_pitch: true,
                install_required: false,
                license_summary:
                    "Google-hosted service; no TTS model/runtime artifacts are distributed by the application."
                        .to_string(),
            },
            TtsProviderDescriptor {
                id: TtsProvider::Local,
                display_name: "KittenTTS Mini 0.8".to_string(),
                is_local: true,
                models: LOCAL_TTS_MODEL_IDS
                    .iter()
                    .map(|model| TtsModelDescriptor {
                        id: (*model).to_string(),
                        display_name: "KittenTTS Mini 0.8".to_string(),
                    })
                    .collect(),
                voices: local_voices(),
                sample_rate_hz: TTS_SAMPLE_RATE_HZ,
                supports_speaking_rate: true,
                supports_pitch: false,
                install_required: true,
                license_summary: "KittenTTS Mini 0.8: Apache-2.0; Microsoft ONNX Runtime 1.23.2: MIT; selected Rust/G2P dependency graph: permissive-only per the accepted P0 license gate."
                    .to_string(),
            },
        ],
        gemini_live: GeminiLiveVoiceCatalog {
            display_name: "Google Gemini Live".to_string(),
            is_local: false,
            sample_rate_hz: TTS_SAMPLE_RATE_HZ,
            voices: google_voices,
            license_summary:
                "Google-hosted service; no Gemini Live voice model artifacts are distributed by the application."
                    .to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::local_tts::{DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE};

    fn provider(catalog: &TtsCatalog, id: TtsProvider) -> &TtsProviderDescriptor {
        catalog
            .providers
            .iter()
            .find(|provider| provider.id == id)
            .expect("catalog must contain provider")
    }

    #[test]
    fn catalog_exposes_exact_google_and_local_providers() {
        let catalog = tts_catalog();
        assert_eq!(catalog.providers.len(), 2);
        assert_eq!(catalog.providers[0].id, TtsProvider::Google);
        assert_eq!(catalog.providers[1].id, TtsProvider::Local);
    }

    #[test]
    fn local_metadata_is_truthful_to_the_accepted_p0_stack() {
        let catalog = tts_catalog();
        let local = provider(&catalog, TtsProvider::Local);
        assert!(local.is_local);
        assert!(local.install_required);
        assert!(local.supports_speaking_rate);
        assert!(!local.supports_pitch);
        assert_eq!(local.sample_rate_hz, 24_000);
        assert_eq!(local.models.len(), 1);
        assert_eq!(local.models[0].id, DEFAULT_LOCAL_TTS_MODEL_ID);
        assert_eq!(local.voices.len(), 8);
        assert!(local
            .voices
            .iter()
            .any(|voice| voice.id == DEFAULT_LOCAL_TTS_VOICE));
        assert!(local.license_summary.contains("Apache-2.0"));
        assert!(local.license_summary.contains("ONNX Runtime 1.23.2"));
    }

    #[test]
    fn google_and_live_catalogs_report_cloud_24khz_voice_metadata() {
        let catalog = tts_catalog();
        let google = provider(&catalog, TtsProvider::Google);
        assert!(!google.is_local);
        assert!(!google.install_required);
        assert!(google.supports_speaking_rate);
        assert!(google.supports_pitch);
        assert_eq!(google.sample_rate_hz, 24_000);
        assert_eq!(google.models.len(), 1);
        assert_eq!(google.voices.len(), 30);
        assert!(!catalog.gemini_live.is_local);
        assert_eq!(catalog.gemini_live.sample_rate_hz, 24_000);
        assert_eq!(catalog.gemini_live.voices, google.voices);
    }
}
