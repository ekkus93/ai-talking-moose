use crate::ai::google::auth::GoogleAuth;
use crate::ai::google::text::GoogleTextModel;
use crate::ai::traits::TextModel;
use crate::ai::types::TextRequest;
use crate::app::state::{AppSettings, AppState};
use crate::audio::devices::{AudioDeviceInfo, AudioDeviceManager};
use crate::character::behavior::BehaviorEngine;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
}

fn synchronize_behavior_engine(settings: &AppSettings, engine: &mut BehaviorEngine) {
    settings.apply_to_character_config(&mut engine.config);
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub fn update_settings(
    mut new_settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    new_settings.settings_version = crate::app::state::CURRENT_SETTINGS_VERSION;
    *state.settings.write() = new_settings.clone();
    {
        let mut engine = state.behavior_engine.lock();
        synchronize_behavior_engine(&new_settings, &mut engine);
    }

    // Persist to SQLite
    if let Ok(json_str) = serde_json::to_string(&new_settings) {
        let _ = state.db.set_setting("app_settings", &json_str);
    }
    Ok(())
}

#[tauri::command]
pub fn set_google_api_key(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    state.secrets.set_google_api_key(api_key)
}

#[tauri::command]
pub fn clear_google_api_key(state: State<'_, AppState>) -> Result<(), String> {
    state.secrets.clear()?;
    // Defensive cleanup for databases created before secure storage was introduced.
    state
        .db
        .delete_setting("google_api_key")
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn has_google_api_key(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.secrets.has_google_api_key())
}

#[tauri::command]
pub async fn test_ai_connection(
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, String> {
    let key = match state.secrets.get_google_api_key() {
        Some(k) if !k.trim().is_empty() => k,
        _ => {
            return Ok(ConnectionTestResult {
                success: false,
                message: "No Google API Key configured".to_string(),
            });
        }
    };

    let model = GoogleTextModel::new(GoogleAuth::new(key), "gemini-2.5-flash".to_string());
    match model
        .generate(TextRequest {
            prompt: "Ping test. Reply with 'Pong'.".to_string(),
            system_instruction: None,
            temperature: Some(0.1),
            max_tokens: Some(10),
        })
        .await
    {
        Ok(_) => Ok(ConnectionTestResult {
            success: true,
            message: "Successfully connected to Google Gemini API!".to_string(),
        }),
        Err(e) => Ok(ConnectionTestResult {
            success: false,
            message: format!("Connection failed: {}", state.secrets.redact(&e)),
        }),
    }
}

#[tauri::command]
pub fn list_audio_devices() -> Result<(Vec<AudioDeviceInfo>, Vec<AudioDeviceInfo>), String> {
    let inputs = AudioDeviceManager::list_input_devices();
    let outputs = AudioDeviceManager::list_output_devices();
    Ok((inputs, outputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::personality::CharacterConfig;

    #[test]
    fn settings_updates_change_behavior_engine_without_restart() {
        let mut engine = BehaviorEngine::new(CharacterConfig::default());
        let settings = AppSettings {
            unsolicited_comments: false,
            talkativeness: 0.93,
            quiet_hours_enabled: false,
            quiet_hours_start: 3,
            quiet_hours_end: 7,
            max_comments_per_hour: 1,
            ..Default::default()
        };

        synchronize_behavior_engine(&settings, &mut engine);

        assert!(!engine.config.behavior.unsolicited_comments);
        assert_eq!(engine.config.personality.talkativeness, 0.93);
        assert!(!engine.config.behavior.quiet_hours_enabled);
        assert_eq!(engine.config.behavior.quiet_hours_start, 3);
        assert_eq!(engine.config.behavior.quiet_hours_end, 7);
        assert_eq!(engine.config.behavior.max_comments_per_hour, 1);
        assert!(engine
            .evaluate_event("test", "runtime settings applied", 1.0)
            .is_none());
    }
}
