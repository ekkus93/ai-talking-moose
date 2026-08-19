use crate::ai::google::auth::GoogleAuth;
use crate::ai::google::text::GoogleTextModel;
use crate::ai::traits::TextModel;
use crate::ai::types::TextRequest;
use crate::app::state::{AppSettings, AppState};
use crate::audio::capture::AudioCaptureDiagnostics;
use crate::audio::devices::{AudioDeviceInfo, AudioDeviceManager};
use crate::audio::permissions::{
    microphone_permission_state, request_microphone_permission, MicrophonePermissionState,
};
use crate::audio::playback::AudioPlaybackDiagnostics;
use crate::character::behavior::BehaviorEngine;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::State;
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioDiagnostics {
    pub configured_input_device: Option<String>,
    pub configured_output_device: Option<String>,
    pub microphone_permission: MicrophonePermissionState,
    pub capture: AudioCaptureDiagnostics,
    pub playback: AudioPlaybackDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicrophoneTestResult {
    pub peak_level: f32,
    pub diagnostics: AudioDiagnostics,
}

fn synchronize_behavior_engine(settings: &AppSettings, engine: &mut BehaviorEngine) {
    settings.apply_to_character_config(&mut engine.config);
}

fn collect_audio_diagnostics(state: &AppState) -> AudioDiagnostics {
    let settings = state.settings.read();
    let capture = state.audio_capture.lock().diagnostics();
    let playback = state.audio_playback.diagnostics();
    AudioDiagnostics {
        configured_input_device: settings.input_device.clone(),
        configured_output_device: settings.output_device.clone(),
        microphone_permission: microphone_permission_state(),
        capture,
        playback,
    }
}

fn generate_output_test_tone(sample_rate_hz: u32, duration_ms: u32) -> Vec<i16> {
    let sample_count = u64::from(sample_rate_hz) * u64::from(duration_ms) / 1_000;
    let sample_count = usize::try_from(sample_count).unwrap_or(usize::MAX);
    let amplitude = 0.15_f32 * f32::from(i16::MAX);
    let angular_step = std::f32::consts::TAU * 440.0 / sample_rate_hz as f32;
    (0..sample_count)
        .map(|index| (amplitude * (angular_step * index as f32).sin()).round() as i16)
        .collect()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = state.settings.read().clone();
    settings.microphone_permission_granted = microphone_permission_state().is_granted();
    Ok(settings)
}

#[tauri::command]
pub fn update_settings(
    mut new_settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    new_settings.settings_version = crate::app::state::CURRENT_SETTINGS_VERSION;
    new_settings.microphone_permission_granted = microphone_permission_state().is_granted();

    let json_str = serde_json::to_string(&new_settings).map_err(|error| error.to_string())?;
    state
        .db
        .set_setting("app_settings", &json_str)
        .map_err(|error| error.to_string())?;

    *state.settings.write() = new_settings.clone();
    let mut engine = state.behavior_engine.lock();
    synchronize_behavior_engine(&new_settings, &mut engine);
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
    let inputs = AudioDeviceManager::list_input_devices()?;
    let outputs = AudioDeviceManager::list_output_devices()?;
    Ok((inputs, outputs))
}

#[tauri::command]
pub fn get_microphone_permission() -> Result<MicrophonePermissionState, String> {
    Ok(microphone_permission_state())
}

#[tauri::command]
pub async fn request_microphone_access() -> Result<MicrophonePermissionState, String> {
    request_microphone_permission().await
}

#[tauri::command]
pub fn get_audio_diagnostics(state: State<'_, AppState>) -> Result<AudioDiagnostics, String> {
    Ok(collect_audio_diagnostics(&state))
}

#[tauri::command]
pub async fn test_microphone(
    state: State<'_, AppState>,
) -> Result<MicrophoneTestResult, String> {
    if state.conversation_mgr.is_active() {
        return Err("stop the active conversation before testing the microphone".to_string());
    }

    #[cfg(target_os = "macos")]
    if microphone_permission_state() != MicrophonePermissionState::Granted {
        return Err(
            "microphone access is not granted; use Request Microphone Access first".to_string(),
        );
    }

    let input_device = state.settings.read().input_device.clone();
    let (pcm_tx, _pcm_rx) = mpsc::channel(16);
    let (level_tx, mut level_rx) = mpsc::channel(16);
    {
        let mut capture = state.audio_capture.lock();
        capture
            .start(input_device, 16_000, pcm_tx, Some(level_tx))
            .map_err(|error| error.to_string())?;
    }

    let deadline = Instant::now() + Duration::from_millis(750);
    let mut peak_level = 0.0_f32;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, level_rx.recv()).await {
            Ok(Some(level)) => peak_level = peak_level.max(level),
            Ok(None) | Err(_) => break,
        }
    }

    state.audio_capture.lock().stop();
    Ok(MicrophoneTestResult {
        peak_level,
        diagnostics: collect_audio_diagnostics(&state),
    })
}

#[tauri::command]
pub fn test_audio_output(state: State<'_, AppState>) -> Result<AudioDiagnostics, String> {
    if state.conversation_mgr.is_active() {
        return Err("stop the active conversation before testing audio output".to_string());
    }

    const TEST_SAMPLE_RATE_HZ: u32 = 24_000;
    const TEST_DURATION_MS: u32 = 500;
    let output_device = state.settings.read().output_device.clone();
    state
        .audio_playback
        .start(output_device)
        .map_err(|error| error.to_string())?;
    let tone = generate_output_test_tone(TEST_SAMPLE_RATE_HZ, TEST_DURATION_MS);
    state
        .audio_playback
        .enqueue_pcm_i16(&tone, TEST_SAMPLE_RATE_HZ)
        .map_err(|error| error.to_string())?;
    Ok(collect_audio_diagnostics(&state))
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

    #[test]
    fn output_test_tone_has_expected_length_and_is_bounded() {
        let tone = generate_output_test_tone(24_000, 500);
        assert_eq!(tone.len(), 12_000);
        assert!(tone.iter().all(|sample| sample.unsigned_abs() <= 5_000));
    }
}
