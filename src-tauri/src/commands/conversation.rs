use crate::ai::types::{LiveSessionConfig, TtsRequest};
use crate::app::settings_policy::settings_runtime_lock;
use crate::app::state::AppState;
#[cfg(test)]
use crate::asr::AsrMode;
use crate::character::prompt::PromptBuilder;
use crate::character::state::{transition_character_state, CharacterState};
use crate::conversation::session::{
    ConversationCallbacks, ConversationLifecycle, ConversationStartRequest,
};
use crate::persistence::{Database, MemoryRecord, TranscriptRecord};
use tauri::{Emitter, Runtime, State};
use tracing::{info, warn};

const MAX_TEXT_MESSAGE_CHARS: usize = 16_384;

fn normalize_text_message(message: String) -> Result<Option<String>, String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_TEXT_MESSAGE_CHARS {
        return Err(format!(
            "text message exceeds the {MAX_TEXT_MESSAGE_CHARS}-character limit"
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn persist_transcript_if_enabled(
    db: &Database,
    enabled: bool,
    session_id: &str,
    role: &str,
    text: &str,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }

    db.add_transcript(session_id, role, text)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn is_final_transcript_role(role: &str) -> bool {
    matches!(role, "user" | "moose")
}

fn transition_and_emit<R: Runtime>(
    character_state: &parking_lot::RwLock<CharacterState>,
    app: &tauri::AppHandle<R>,
    target: CharacterState,
) -> Result<(), String> {
    transition_character_state(character_state, target)?;
    let _ = app.emit("moose://state", target);
    Ok(())
}

fn prepare_character_for_conversation<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let current = *state.character_state.read();
    if current.can_transition_to(&CharacterState::Listening) {
        return Ok(());
    }
    transition_and_emit(&state.character_state, app, CharacterState::Idle)
}

#[tauri::command]
pub async fn start_conversation<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    state.ambient_scheduler.interrupt();
    state
        .standalone_speech
        .cancel(state.audio_playback.as_ref());
    let _ = app.emit("moose://speech-bubble", "");
    if *state.is_muted.read() {
        return Err("Moose is currently muted".to_string());
    }

    // Prevent a settings update from committing a new ASR/provider/device selection while this
    // start request is still constructing or activating the old graph.
    let _settings_guard = settings_runtime_lock().lock().await;
    let settings = state.settings.read().clone();
    prepare_character_for_conversation(state.inner(), &app)?;
    let provider = state.get_live_provider();
    let tool_router = state.tool_router.clone();

    let memories = if settings.memory_enabled {
        state.memory.get_memory_strings()
    } else {
        vec![]
    };

    let character_config = state.behavior_engine.lock().config.clone();
    let system_instruction =
        PromptBuilder::build_system_instruction(&character_config, &memories, None, false);

    let config = LiveSessionConfig {
        model: settings.live_model.clone(),
        voice_name: Some(settings.tts_voice.clone()),
        system_instruction: Some(system_instruction),
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: tool_router.get_declarations(),
    };

    let character_state = state.character_state.clone();
    let app_state = app.clone();
    let app_lifecycle = app.clone();
    let app_provider_error = app.clone();
    let app_transcript = app.clone();
    let app_bubble = app.clone();
    let app_level = app.clone();
    let db_ref = state.db.clone();
    let save_transcripts = settings.save_transcripts;

    let request = ConversationStartRequest {
        provider,
        config,
        asr_mode: settings.asr_mode,
        moonshine_installer: Some(state.moonshine_installer.clone()),
        capture: state.audio_capture.clone(),
        input_device: settings.input_device.clone(),
        playback: state.audio_playback.clone(),
        output_device: settings.output_device.clone(),
        muted: state.is_muted.clone(),
        tool_router,
        callbacks: ConversationCallbacks::new(
            move |new_state: CharacterState| {
                if let Err(error_value) = transition_character_state(&character_state, new_state) {
                    warn!(error = %error_value, ?new_state, "Rejected conversation character transition");
                    return;
                }
                let _ = app_state.emit("moose://state", new_state);
            },
            move |lifecycle: ConversationLifecycle| {
                let _ = app_lifecycle.emit("moose://conversation/lifecycle", lifecycle);
            },
            move |session_id: String, role: String, text: String| {
                let _ = app_transcript.emit(&format!("moose://transcript/{role}"), &text);
                if is_final_transcript_role(&role) {
                    if let Err(error_value) = persist_transcript_if_enabled(
                        db_ref.as_ref(),
                        save_transcripts,
                        &session_id,
                        &role,
                        &text,
                    ) {
                        warn!(error = %error_value, "Failed to persist retained transcript");
                    }
                }
            },
            move |speech_text: String| {
                let _ = app_bubble.emit("moose://speech-bubble", &speech_text);
            },
            move |level: f32| {
                let _ = app_level.emit("moose://audio/input-level", level);
            },
            move |provider_error| {
                let _ = app_provider_error.emit("moose://conversation/error", provider_error);
            },
        ),
    };

    let session_id = state.conversation_mgr.start_session(request).await?;

    info!(session_id = %session_id, "Conversation session started");
    Ok(session_id)
}

#[tauri::command]
pub async fn stop_conversation(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state
        .conversation_mgr
        .stop_session(state.audio_capture.clone(), state.audio_playback.clone())
        .await;

    let target = if *state.is_muted.read() {
        CharacterState::Muted
    } else {
        CharacterState::Idle
    };
    transition_and_emit(&state.character_state, &app, target)
}

#[tauri::command]
pub async fn barge_in(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state.ambient_scheduler.interrupt();
    state
        .standalone_speech
        .cancel(state.audio_playback.as_ref());
    let _ = app.emit("moose://speech-bubble", "");

    let conversation_active = state.conversation_mgr.is_active();
    state
        .conversation_mgr
        .barge_in(state.audio_playback.clone())
        .await?;

    if *state.character_state.read() == CharacterState::Talking {
        let target = if conversation_active {
            CharacterState::Interrupted
        } else {
            CharacterState::Idle
        };
        transition_and_emit(&state.character_state, &app, target)?;
    }
    state.behavior_engine.lock().cooldowns.record_interruption();
    Ok(())
}

#[tauri::command]
pub fn get_conversation_lifecycle(
    state: State<'_, AppState>,
) -> Result<ConversationLifecycle, String> {
    Ok(state.conversation_mgr.lifecycle())
}

#[tauri::command]
pub fn get_memories(state: State<'_, AppState>) -> Result<Vec<MemoryRecord>, String> {
    state.memory.get_all_memories()
}

#[tauri::command]
pub fn delete_memory(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    state.memory.forget(id)
}

#[tauri::command]
pub fn forget_everything(state: State<'_, AppState>) -> Result<(), String> {
    state.memory.forget_everything()
}

#[tauri::command]
pub fn get_transcripts(
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<TranscriptRecord>, String> {
    state.db.get_transcripts(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_text_message(
    message: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let Some(msg_trimmed) = normalize_text_message(message)? else {
        return Ok(String::new());
    };
    let settings = state.settings.read().clone();

    let _ = app.emit("moose://transcript/user", &msg_trimmed);
    persist_transcript_if_enabled(
        state.db.as_ref(),
        settings.save_transcripts,
        "debug_terminal",
        "user",
        &msg_trimmed,
    )?;

    transition_and_emit(&state.character_state, &app, CharacterState::Thinking)?;

    let memories = if settings.memory_enabled {
        state.memory.get_memory_strings()
    } else {
        vec![]
    };
    let character_config = state.behavior_engine.lock().config.clone();
    let system_instruction =
        PromptBuilder::build_system_instruction(&character_config, &memories, None, false);

    let text_model = state.get_text_model();
    let text_res = text_model
        .generate(crate::ai::types::TextRequest {
            prompt: msg_trimmed,
            system_instruction: Some(system_instruction),
            temperature: Some(0.85),
            max_tokens: Some(1024),
        })
        .await
        .map_err(|error_value| state.secrets.redact(&error_value))?;

    let reply = text_res.text;

    transition_and_emit(&state.character_state, &app, CharacterState::Talking)?;
    let _ = app.emit("moose://transcript/moose", &reply);
    let _ = app.emit("moose://speech-bubble", &reply);
    persist_transcript_if_enabled(
        state.db.as_ref(),
        settings.save_transcripts,
        "debug_terminal",
        "moose",
        &reply,
    )?;

    if !*state.is_muted.read() {
        let synthesizer = state.get_speech_synthesizer();
        let cancellation = state.standalone_speech.begin(state.audio_playback.as_ref());
        let report = crate::audio::speech::synthesize_and_queue_cancellable(
            synthesizer.as_ref(),
            state.audio_playback.as_ref(),
            TtsRequest {
                text: reply.clone(),
                voice_name: Some(settings.tts_voice.clone()),
                speaking_rate: Some(settings.speaking_rate),
                pitch: Some(settings.pitch),
            },
            settings.output_device.clone(),
            &cancellation,
        )
        .await?;
        if report.dropped_samples > 0 {
            return Err(format!(
                "audio playback queue overflowed and dropped {} samples",
                report.dropped_samples
            ));
        }
    }

    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::capture::AudioCapture;
    use serde_json::json;
    use std::sync::Arc;
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
    use tauri::webview::InvokeRequest;

    fn ipc_request(command: &str, body: serde_json::Value) -> InvokeRequest {
        InvokeRequest {
            cmd: command.to_string(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(any(windows, target_os = "android")) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        }
    }

    #[test]
    fn text_message_admission_is_trimmed_and_hard_bounded() {
        assert_eq!(
            normalize_text_message("  hello moose  ".to_string()).unwrap(),
            Some("hello moose".to_string())
        );
        assert_eq!(normalize_text_message("   ".to_string()).unwrap(), None);

        let oversized = "x".repeat(MAX_TEXT_MESSAGE_CHARS + 1);
        let error = normalize_text_message(oversized).unwrap_err();
        assert!(error.contains("character limit"));
    }

    #[test]
    fn transcript_retention_off_writes_no_records() {
        let db = Database::new_in_memory().unwrap();

        persist_transcript_if_enabled(&db, false, "session", "user", "private input").unwrap();
        persist_transcript_if_enabled(&db, false, "session", "moose", "private output").unwrap();

        assert!(db.get_transcripts(10).unwrap().is_empty());
    }

    #[test]
    fn only_final_live_transcript_roles_are_persistable() {
        assert!(is_final_transcript_role("user"));
        assert!(is_final_transcript_role("moose"));
        assert!(!is_final_transcript_role("user_partial"));
        assert!(!is_final_transcript_role("moose_partial"));
    }

    #[test]
    fn missing_moonshine_tiny_model_fails_before_microphone_capture() {
        let mut app_state = AppState::new_for_tests().unwrap();
        app_state.audio_capture = Arc::new(parking_lot::Mutex::new(AudioCapture::new_mock()));
        let capture = app_state.audio_capture.clone();

        let app = mock_builder()
            .manage(app_state)
            .invoke_handler(tauri::generate_handler![start_conversation])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let error = get_ipc_response(&webview, ipc_request("start_conversation", json!({})))
            .expect_err("missing Tiny model must fail closed before microphone capture");

        assert!(error
            .as_str()
            .unwrap()
            .contains("No microphone audio was sent"));
        assert!(!capture.lock().is_active());
    }

    #[test]
    fn missing_moonshine_small_model_fails_before_microphone_capture() {
        let mut app_state = AppState::new_for_tests().unwrap();
        app_state.audio_capture = Arc::new(parking_lot::Mutex::new(AudioCapture::new_mock()));
        app_state.settings.write().asr_mode = AsrMode::MoonshineSmallStreaming;
        let capture = app_state.audio_capture.clone();

        let app = mock_builder()
            .manage(app_state)
            .invoke_handler(tauri::generate_handler![start_conversation])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let error = get_ipc_response(&webview, ipc_request("start_conversation", json!({})))
            .expect_err("missing Small model must fail closed before microphone capture");

        let error = error.as_str().unwrap();
        assert!(error.contains("Moonshine Small"));
        assert!(error.contains("No microphone audio was sent"));
        assert!(!capture.lock().is_active());
    }

    #[test]
    fn ambient_barge_in_flushes_standalone_audio_and_returns_to_idle() {
        let app_state = AppState::new_for_tests().unwrap();
        let playback = app_state.audio_playback.clone();
        let character_state = app_state.character_state.clone();
        playback.seed_buffer_for_tests(&[0.25, -0.25, 0.5], 0.5);
        *character_state.write() = CharacterState::Talking;

        let app = mock_builder()
            .manage(app_state)
            .invoke_handler(tauri::generate_handler![barge_in])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        get_ipc_response(&webview, ipc_request("barge_in", json!({})))
            .expect("ambient-only barge-in should succeed");

        assert!(!playback.is_playing());
        assert_eq!(playback.queue_length(), 0);
        assert_eq!(playback.diagnostics().output_level, 0.0);
        assert_eq!(*character_state.read(), CharacterState::Idle);
    }

    #[test]
    fn transcript_retention_on_writes_records() {
        let db = Database::new_in_memory().unwrap();

        persist_transcript_if_enabled(&db, true, "session", "user", "retained input").unwrap();
        persist_transcript_if_enabled(&db, true, "session", "moose", "retained output").unwrap();

        let transcripts = db.get_transcripts(10).unwrap();
        assert_eq!(transcripts.len(), 2);
    }
}
