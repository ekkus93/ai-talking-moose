use crate::ai::google::{
    validate_live_model, GoogleAuth, GoogleLiveProvider, GoogleModelDescriptor,
    GoogleTtsVoiceDescriptor, GOOGLE_MODELS, GOOGLE_TTS_VOICES,
};
use crate::ai::loopback::{LoopbackReadiness, LoopbackTextModel, LoopbackTextServer};
use crate::ai::traits::RealtimeConversationProvider;
use crate::ai::tts_catalog::{tts_catalog, TtsCatalog};
use crate::ai::types::{LiveSessionConfig, TextProvider};
use crate::app::runtime_preferences::apply_changed_runtime_preferences;
use crate::app::settings_policy::{
    conversation_restart_required, persist_and_apply_settings, settings_runtime_lock,
    validate_app_settings, validate_selected_device,
};
use crate::app::state::{AppSettings, AppState, OnboardingStatus};
use crate::app::wake_word_settings::WakeWordSettings;
use crate::audio::capture::{AudioCapture, AudioCaptureDiagnostics};
use crate::audio::devices::{AudioDeviceInfo, AudioDeviceManager};
use crate::audio::permissions::{
    microphone_permission_state, request_microphone_permission, MicrophonePermissionState,
};
use crate::audio::playback::AudioPlaybackDiagnostics;
use crate::character::idle_banter::normalize_idle_banter_seed_topics;
use crate::character::state::{transition_character_state, CharacterState};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

struct DiagnosticCaptureLease {
    capture: Arc<Mutex<AudioCapture>>,
}

impl DiagnosticCaptureLease {
    fn start(
        capture: Arc<Mutex<AudioCapture>>,
        device_name: Option<String>,
        target_sample_rate: u32,
        pcm_sender: mpsc::Sender<Vec<u8>>,
        level_sender: Option<mpsc::Sender<f32>>,
    ) -> Result<Self, String> {
        capture
            .lock()
            .start(device_name, target_sample_rate, pcm_sender, level_sender)
            .map_err(|error| error.to_string())?;
        Ok(Self { capture })
    }
}

impl Drop for DiagnosticCaptureLease {
    fn drop(&mut self) {
        self.capture.lock().stop();
    }
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

fn normalize_settings_update(mut new_settings: AppSettings) -> Result<AppSettings, String> {
    // Never downgrade an unknown newer settings schema from live IPC. Startup
    // migration already fails closed for future versions; writes must agree.
    if new_settings.settings_version > crate::app::state::CURRENT_SETTINGS_VERSION {
        return Err(
            "Settings were written by a newer app version; update this app before changing them."
                .to_string(),
        );
    }
    // IPC sends complete AppSettings with flattened, unrecognized fields. Never
    // let credential-like aliases or nested values reach SQLite or runtime state.
    new_settings.sanitize_future_settings();
    new_settings.settings_version = crate::app::state::CURRENT_SETTINGS_VERSION;
    // Window-title observation remains unsupported in V1. Treat the serialized field
    // as compatibility metadata and never allow a settings write to enable it.
    new_settings.window_title_observation = false;
    new_settings.idle_banter_seed_topics =
        normalize_idle_banter_seed_topics(&new_settings.idle_banter_seed_topics)?;

    let wake = WakeWordSettings::from_app_settings_fields(
        new_settings.wake_word_enabled,
        &new_settings.wake_word_phrase,
    )?;
    new_settings.wake_word_enabled = wake.enabled;
    new_settings.wake_word_phrase = wake.phrase;

    validate_app_settings(&new_settings)?;
    Ok(new_settings)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub fn get_onboarding_status(state: State<'_, AppState>) -> Result<OnboardingStatus, String> {
    state.onboarding_status()
}

#[tauri::command]
pub fn acknowledge_onboarding(state: State<'_, AppState>) -> Result<OnboardingStatus, String> {
    state.acknowledge_current_onboarding()
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
pub fn get_tts_catalog() -> TtsCatalog {
    let mut catalog = tts_catalog();
    // System Voices exist only where native synthesis is implemented (macOS); the
    // static catalog and generated contract stay platform-neutral.
    if let Some(system) = crate::ai::system_tts::provider_descriptor() {
        catalog.providers.push(system);
    }
    catalog
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

    new_settings = normalize_settings_update(new_settings)?;

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

    // Apply gain only after the settings transaction commits so persistence failure
    // cannot leave live playback using a value the runtime settings rejected.
    state.audio_playback.set_volume(new_settings.volume);
    state.record_user_interaction();

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

/// The OpenAI credential is independent of the Google credential and never
/// becomes part of serialized AppSettings, SQLite, or frontend storage.
#[tauri::command]
pub fn set_openai_api_key(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    state.secrets.set_openai_api_key(api_key)
}

#[tauri::command]
pub fn clear_openai_api_key(state: State<'_, AppState>) -> Result<(), String> {
    state.secrets.clear_openai_api_key()?;
    // Clean up any untrusted legacy database value without touching Google.
    state
        .db
        .delete_setting("openai_api_key")
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn has_openai_api_key(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.secrets.has_openai_api_key())
}

/// A connection check never launches a service or downloads model weights.
#[tauri::command]
pub async fn test_local_text_server(
    provider: TextProvider,
    state: State<'_, AppState>,
) -> Result<LoopbackReadiness, String> {
    if state.conversation_mgr.is_active() {
        return Err("Stop the active conversation before checking server readiness".to_string());
    }
    let settings = state.settings_snapshot();
    let model = match provider {
        TextProvider::Ollama => LoopbackTextModel::new(
            LoopbackTextServer::Ollama,
            settings.ollama_server_address,
            settings.ollama_text_model,
        ),
        TextProvider::LlamaServer => LoopbackTextModel::new(
            LoopbackTextServer::LlamaServer,
            settings.llama_server_address,
            settings.llama_server_text_model,
        ),
        _ => {
            return Err("Select Ollama or llama-server for a loopback readiness check".to_string())
        }
    };
    Ok(model.check_readiness().await)
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
        voice_name: Some(settings.live_voice),
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
    let capture_lease = DiagnosticCaptureLease::start(
        state.audio_capture.clone(),
        input_device,
        16_000,
        pcm_tx,
        Some(level_tx),
    )?;

    let deadline = Instant::now() + Duration::from_millis(750);
    let mut peak_level = 0.0_f32;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, level_rx.recv()).await {
            Ok(Some(level)) => peak_level = peak_level.max(level),
            Ok(None) | Err(_) => break,
        }
    }

    drop(capture_lease);
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
    fn live_settings_update_normalizes_wake_phrase_and_preserves_unrelated_fields() {
        let settings = AppSettings {
            wake_word_enabled: true,
            wake_word_phrase: "  hey, moose  ".to_string(),
            asr_mode: crate::asr::AsrMode::GeminiLiveAudio,
            tts_provider: crate::ai::types::TtsProvider::Local,
            talkativeness: 0.73,
            ..Default::default()
        };

        let normalized = normalize_settings_update(settings).unwrap();
        assert!(normalized.wake_word_enabled);
        assert_eq!(normalized.wake_word_phrase, "Hey, Moose");
        assert_eq!(normalized.asr_mode, crate::asr::AsrMode::GeminiLiveAudio);
        assert_eq!(
            normalized.tts_provider,
            crate::ai::types::TtsProvider::Local
        );
        assert_eq!(normalized.talkativeness, 0.73);
    }

    #[test]
    fn live_settings_ipc_rejects_retired_moonshine_without_rewriting_saved_selection() {
        let saved = AppSettings {
            asr_mode: crate::asr::AsrMode::WhisperSmall,
            text_provider: TextProvider::Local,
            save_transcripts: true,
            ..Default::default()
        };
        let persisted_before = serde_json::to_string(&saved).unwrap();
        for retired in ["moonshine_tiny_streaming", "moonshine_small_streaming"] {
            // Exercise the same Serde decoding as a direct/stale Tauri IPC caller.
            let mut incoming = serde_json::to_value(&saved).unwrap();
            incoming["asr_mode"] = serde_json::json!(retired);
            let decoded: AppSettings = serde_json::from_value(incoming).unwrap();
            let error = normalize_settings_update(decoded).unwrap_err();
            assert!(
                error.contains("Moonshine ASR is retired"),
                "{retired}: {error}"
            );
            // Rejected requests cannot reach persist_and_apply_settings.
            let (loaded, migrated) = AppSettings::from_persisted_json(&persisted_before).unwrap();
            assert!(!migrated);
            assert_eq!(loaded.asr_mode, crate::asr::AsrMode::WhisperSmall);
            assert_eq!(loaded.text_provider, TextProvider::Local);
            assert!(loaded.save_transcripts);
        }
    }

    #[test]
    fn live_settings_ipc_accepts_whisper_and_explicit_cloud_on_supported_platforms() {
        for mode in [
            crate::asr::AsrMode::WhisperSmall,
            crate::asr::AsrMode::GeminiLiveAudio,
        ] {
            let updated = normalize_settings_update(AppSettings {
                asr_mode: mode,
                memory_enabled: true,
                ..Default::default()
            })
            .unwrap();
            assert_eq!(updated.asr_mode, mode);
            assert!(updated.memory_enabled);
            let (reloaded, migrated) =
                AppSettings::from_persisted_json(&serde_json::to_string(&updated).unwrap())
                    .unwrap();
            assert!(!migrated);
            assert_eq!(reloaded.asr_mode, mode);
        }
    }

    #[test]
    fn live_settings_update_rejects_unknown_future_schema_before_downgrade() {
        let current = crate::app::state::CURRENT_SETTINGS_VERSION;
        let settings = AppSettings {
            settings_version: current + 1,
            text_provider: TextProvider::Openai,
            ..Default::default()
        };
        let error = normalize_settings_update(settings).unwrap_err();
        assert!(error.contains("newer app version"));

        let old_settings = AppSettings {
            settings_version: current.saturating_sub(1),
            ..Default::default()
        };
        assert_eq!(
            normalize_settings_update(old_settings)
                .unwrap()
                .settings_version,
            current
        );
    }

    #[test]
    fn live_settings_update_scrubs_nested_future_credentials_without_losing_preferences() {
        let mut settings = AppSettings::default();
        settings
            .future_settings
            .insert("openaiKey".into(), serde_json::json!("credential-canary"));
        settings.future_settings.insert(
            "future_display_hint".into(),
            serde_json::json!({
                "enabled": true,
                "theme": "dark",
                "api-key": "credential-canary",
                "children": [
                    {"refreshToken": "credential-canary", "font": "large"},
                    {"extra": {"authHeader": "credential-canary", "mode": "local"}}
                ]
            }),
        );
        let normalized = normalize_settings_update(settings).unwrap();
        let json = serde_json::to_value(&normalized).unwrap();
        assert!(!json.to_string().contains("credential-canary"));
        assert!(json.get("openaiKey").is_none());
        assert_eq!(json["future_display_hint"]["enabled"], true);
        assert_eq!(json["future_display_hint"]["theme"], "dark");
        assert_eq!(json["future_display_hint"]["children"][0]["font"], "large");
        assert_eq!(
            json["future_display_hint"]["children"][1]["extra"]["mode"],
            "local"
        );
        let (reloaded, migrated) = AppSettings::from_persisted_json(&json.to_string()).unwrap();
        assert!(!migrated, "clean IPC settings must not be re-migrated");
        assert_eq!(reloaded.future_settings, normalized.future_settings);
    }

    #[test]
    fn live_settings_update_rejects_invalid_wake_phrase_before_commit() {
        let settings = AppSettings {
            wake_word_enabled: true,
            wake_word_phrase: "Hey Bruce".to_string(),
            ..Default::default()
        };

        let error = normalize_settings_update(settings).unwrap_err();
        assert_eq!(error, "Wake Word V1 only supports the default wake phrase");
    }

    #[test]
    fn rejected_live_wake_update_leaves_previous_settings_restart_loadable() {
        let previous = AppSettings::default();
        let persisted_before = serde_json::to_string(&previous).unwrap();
        let invalid = AppSettings {
            wake_word_enabled: true,
            wake_word_phrase: "Hey Bruce".to_string(),
            ..previous.clone()
        };

        assert!(normalize_settings_update(invalid).is_err());
        let (reloaded, _) = AppSettings::from_persisted_json(&persisted_before).unwrap();
        assert_eq!(reloaded.wake_word_enabled, previous.wake_word_enabled);
        assert_eq!(reloaded.wake_word_phrase, previous.wake_word_phrase);
    }

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

    #[tokio::test]
    async fn cancelling_microphone_diagnostic_stops_capture() {
        let capture = Arc::new(Mutex::new(AudioCapture::new_mock()));
        let task_capture = capture.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();

        let diagnostic = tokio::spawn(async move {
            let (pcm_tx, _pcm_rx) = mpsc::channel(1);
            let lease =
                DiagnosticCaptureLease::start(task_capture, None, 16_000, pcm_tx, None).unwrap();
            started_tx
                .send(())
                .expect("diagnostic owner should report capture startup");
            std::future::pending::<()>().await;
            drop(lease);
        });

        started_rx
            .await
            .expect("diagnostic owner should reach its cancellable wait");
        assert!(capture.lock().is_active());

        diagnostic.abort();
        let join_error = diagnostic
            .await
            .expect_err("aborted diagnostic task should be cancelled");
        assert!(join_error.is_cancelled());
        assert!(!capture.lock().is_active());
    }

    #[test]
    fn output_test_tone_has_expected_length_and_is_bounded() {
        let tone = generate_output_test_tone(24_000, 500);
        assert_eq!(tone.len(), 12_000);
        assert!(tone.iter().all(|sample| sample.unsigned_abs() <= 5_000));
    }
}
