use crate::ai::google::auth::GoogleAuth;
use crate::ai::google::text::GoogleTextModel;
use crate::ai::traits::TextModel;
use crate::ai::types::TextRequest;
use crate::app::state::{AppSettings, AppState};
use crate::audio::devices::{AudioDeviceInfo, AudioDeviceManager};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub fn update_settings(
    new_settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    *state.settings.write() = new_settings.clone();

    // Persist to SQLite
    if let Ok(json_str) = serde_json::to_string(&new_settings) {
        let _ = state.db.set_setting("app_settings", &json_str);
    }
    Ok(())
}

#[tauri::command]
pub fn set_google_api_key(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    state.secrets.set_google_api_key(api_key.clone());
    let _ = state.db.set_setting("google_api_key", &api_key);
    Ok(())
}

#[tauri::command]
pub fn clear_google_api_key(state: State<'_, AppState>) -> Result<(), String> {
    state.secrets.clear();
    let _ = state.db.set_setting("google_api_key", "");
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
            message: format!("Connection failed: {}", e),
        }),
    }
}

#[tauri::command]
pub fn list_audio_devices() -> Result<(Vec<AudioDeviceInfo>, Vec<AudioDeviceInfo>), String> {
    let inputs = AudioDeviceManager::list_input_devices();
    let outputs = AudioDeviceManager::list_output_devices();
    Ok((inputs, outputs))
}
