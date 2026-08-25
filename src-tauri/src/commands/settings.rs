use crate::ai::google::{
    validate_live_model, GoogleAuth, GoogleLiveProvider, GoogleModelDescriptor,
    GoogleTtsVoiceDescriptor, GOOGLE_MODELS, GOOGLE_TTS_VOICES,
};
use crate::ai::traits::RealtimeConversationProvider;
use crate::ai::types::LiveSessionConfig;
use crate::app::runtime_preferences::apply_changed_runtime_preferences;
use crate::app::settings_policy::{
    conversation_restart_required, persist_and_apply_settings, settings_runtime_lock,
    validate_app_settings, validate_selected_device,
};
use crate::app::state::{AppSettings, AppState};
use crate::audio::capture::AudioCaptureDiagnostics;
use crate::audio::devices::{AudioDeviceInfo, AudioDeviceManager};
use crate::audio::permissions::{
    microphone_permission_state, request_microphone_permission, MicrophonePermissionState,
};
use crate::audio::playback::AudioPlaybackDiagnostics;
use crate::character::state::{transition_character_state, CharacterState};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::{Emitter, Runtime, State};
use tokio::sync::mpsc;
use tracing::warn;

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

fn active_conversation_connection_test_result(is_active: bool) -> Option<ConnectionTestResult> {
    is_active.then(|| ConnectionTestResult {
        success: false,
        message: "Stop the active conversation before testing Gemini connectivity.".to_string(),
    })
}

fn validate_changed_audio_devices(
    previous: &AppSettings,
    next: &AppSettings,
) -> Result<(), String> {
    if previous.input_device != next.input_device && next.input_device.is_some() {
        let available = AudioDeviceManager::list_input_devices()?;
        validate_selected_device(next.input_device.as_deref(), &available, "input")?;
    }
    if previous.output_device != next.output_device && next.output_device.is_some() {
        let available = AudioDeviceManager::list_output_devices()?;
        validate_selected_device(next.output_device.as_deref(), &available, "output")?;
    }
    Ok(())
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
pub fn get_google_models() -> Vec<GoogleModelDescriptor> {
    GOOGLE_MODELS.to_vec()
}

#[tauri::command]
pub fn get_google_tts_voices() -> Vec<GoogleTtsVoiceDescriptor> {
    GOOGLE_TTS_VOICES.to_vec()
}

#[tauri::command]
pub async fn update_settings<R: Runtime>(
    mut new_settings: AppSettings,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    // Serialize the entire settings commit with conversation startup. A provider/ASR/device
    // change therefore cannot race a start that already captured the old configuration.
    let _settings_guard = settings_runtime_lock().lock().await;

    new_settings.settings_version = crate::app::state::CURRENT_SETTINGS_VERSION;
    validate_app_settings(&new_settings)?;

    let previous = state.settings.read().clone();
    validate_changed_audio_devices(&previous, &new_settings)?;
    let restart_required = conversation_restart_required(&previous, &new_settings);

    // Apply reversible OS/window preferences before persistence. If a runtime side effect fails,
    // the persisted settings remain unchanged. If persistence then fails, restore the previous
    // runtime preferences so the settings update remains transactional.
    apply_changed_runtime_preferences(&app, &previous, &new_settings)?;
    if let Err(error) = persist_and_apply_settings(
        state.db.as_ref(),
        state.settings.as_ref(),
        state.behavior_engine.as_ref(),
        &new_settings,
    ) {
        if let Err(rollback_error) =
            apply_changed_runtime_preferences(&app, &new_settings, &previous)
        {
            warn!(error = %rollback_error, "Failed to roll back runtime preferences after settings persistence failure");
        }
        return Err(error);
    }

    if restart_required && state.conversation_mgr.is_active() {
        // Privacy-preserving restart policy: tear down the old graph and require a fresh explicit
        // Start action. Never auto-open the microphone merely because a setting changed,
        // especially when switching to Gemini Live cloud audio.
        state
            .conversation_mgr
            .stop_session(state.audio_capture.clone(), state.audio_playback.clone())
            .await;
        if let Err(error) = transition_character_state(&state.character_state, CharacterState::Idle)
        {
            warn!(error = %error, "Conversation stopped after settings change but character state could not transition to Idle");
        } else {
            let _ = app.emit("moose://state", CharacterState::Idle);
        }
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
    if let Some(blocked) =
        active_conversation_connection_test_result(state.conversation_mgr.is_active())
    {
        return Ok(blocked);
    }

    let key = match state.secrets.get_google_api_key() {
        Some(key) if !key.trim().is_empty() => key,
        _ => {
            return Ok(ConnectionTestResult {
                success: false,
                message: "No Google API Key configured".to_string(),
            });
        }
    };
    let settings = state.settings.read().clone();
    if let Err(message) = validate_live_model(&settings.live_model) {
        return Ok(ConnectionTestResult {
            success: false,
            message,
        });
    }

    let provider = GoogleLiveProvider::new(GoogleAuth::new(key));
    let (event_tx, _event_rx) = mpsc::channel(8);
    let config = LiveSessionConfig {
        model: settings.live_model,
        voice_name: Some(settings.tts_voice),
        system_instruction: Some("Talking Moose connectivity test".to_string()),
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: vec![],
    };
    match provider.connect(config, event_tx).await {
        Ok(mut session) => {
            let _ = session.close().await;
            Ok(ConnectionTestResult {
                success: true,
                message: "Gemini Live setup handshake completed successfully.".to_string(),
            })
        }
        Err(error) => Ok(ConnectionTestResult {
            success: false,
            message: error.message,
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
pub async fn test_microphone(state: State<'_, AppState>) -> Result<MicrophoneTestResult, String> {
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
    use crate::character::ambient::AmbientEvent;
    use crate::character::behavior::{AmbientDecisionReason, AmbientPolicyContext, BehaviorEngine};
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

        settings.apply_to_character_config(&mut engine.config);

        assert!(!engine.config.behavior.unsolicited_comments);
        assert_eq!(engine.config.personality.talkativeness, 0.93);
        assert!(!engine.config.behavior.quiet_hours_enabled);
        assert_eq!(engine.config.behavior.quiet_hours_start, 3);
        assert_eq!(engine.config.behavior.quiet_hours_end, 7);
        assert_eq!(engine.config.behavior.max_comments_per_hour, 1);
        let decision = engine.evaluate_ambient_event(
            &AmbientEvent::new("test", "runtime settings applied".to_string(), 1.0),
            AmbientPolicyContext {
                privacy_allowed: true,
                muted: false,
                conversation_active: false,
            },
        );
        assert!(!decision.should_speak);
        assert_eq!(decision.reason, AmbientDecisionReason::UnsolicitedDisabled);
    }

    #[test]
    fn connection_test_guard_blocks_active_conversation_before_provider_work() {
        let blocked = active_conversation_connection_test_result(true)
            .expect("active conversations must block connectivity tests");
        assert!(!blocked.success);
        assert!(blocked.message.contains("Stop the active conversation"));
        assert!(active_conversation_connection_test_result(false).is_none());
    }

    #[test]
    fn output_test_tone_has_expected_length_and_is_bounded() {
        let tone = generate_output_test_tone(24_000, 500);
        assert_eq!(tone.len(), 12_000);
        assert!(tone.iter().all(|sample| sample.unsigned_abs() <= 5_000));
    }
}
