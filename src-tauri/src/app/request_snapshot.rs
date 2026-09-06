use crate::ai::google::{GoogleAuth, GoogleTextModel};
use crate::ai::local::{global_local_model_installer, LocalTextModel};
use crate::ai::traits::TextModel;
use crate::ai::types::TextProvider;
use crate::app::state::{AppSettings, AppState};
use crate::character::personality::CharacterConfig;

/// Immutable settings and settings-derived character configuration captured once
/// for one text-generation request.
#[derive(Clone)]
pub(crate) struct TextRequestSettingsSnapshot {
    pub(crate) settings: AppSettings,
    pub(crate) character_config: CharacterConfig,
}

impl AppState {
    pub(crate) fn settings_snapshot(&self) -> AppSettings {
        self.settings.read().clone()
    }

    pub(crate) fn capture_text_request_settings(&self) -> TextRequestSettingsSnapshot {
        let settings = self.settings_snapshot();
        let mut character_config = self.behavior_engine.lock().config.clone();
        settings.apply_to_character_config(&mut character_config);
        TextRequestSettingsSnapshot {
            settings,
            character_config,
        }
    }

    pub(crate) fn get_text_model_for(&self, settings: &AppSettings) -> Box<dyn TextModel> {
        match settings.text_provider {
            TextProvider::Google => {
                let key = self.secrets.get_google_api_key().unwrap_or_default();
                Box::new(GoogleTextModel::new(
                    GoogleAuth::new(key),
                    settings.google_text_model.clone(),
                ))
            }
            TextProvider::Local => Box::new(LocalTextModel::new(
                self.local_llm_runtime.clone(),
                global_local_model_installer(),
                settings.local_text_model.clone(),
            )),
        }
    }
}
