use crate::ai::types::{LiveOutboundDiagnostics, LiveSessionConfig};
use crate::app::settings_policy::settings_runtime_lock;
use crate::app::state::AppState;
#[cfg(test)]
use crate::asr::AsrMode;
use crate::character::prompt::PromptBuilder;
use crate::character::state::{transition_character_state, CharacterState};
use crate::commands::presentation::{clear_speech_bubble, transition_and_emit};
use crate::commands::speech::{invoke_standalone_speech, schedule_standalone_completion};
use crate::conversation::session::{
    ConversationCallbacks, ConversationLifecycle, ConversationStartRequest,
};
use crate::persistence::{Database, MemoryRecord, TranscriptRecord};
use tauri::{Emitter, Runtime, State};
use tracing::{info, warn};

const MAX_TEXT_MESSAGE_CHARS: usize = 16_384;

/// Return only memories permitted to enter model prompts for the captured settings snapshot.
/// This is the production privacy gate shared by conversational and ambient prompt construction.
pub(super) fn model_prompt_memories(state: &AppState, memory_enabled: bool) -> Vec<String> {
    if memory_enabled {
        state.memory.get_memory_strings()
    } else {
        Vec::new()
    }
}

fn build_conversation_system_instruction(state: &AppState, memory_enabled: bool) -> String {
    let memories = model_prompt_memories(state, memory_enabled);
    let character_config = state.behavior_engine.lock().config.clone();
    PromptBuilder::build_system_instruction(&character_config, &memories, None, false)
}

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
    clear_speech_bubble(&app);
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

    let system_instruction =
        build_conversation_system_instruction(state.inner(), settings.memory_enabled);

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

    transition_and_emit(&state.character_state, &app, CharacterState::Idle)
}

#[tauri::command]
pub async fn barge_in<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    state.ambient_scheduler.interrupt();
    state
        .standalone_speech
        .cancel(state.audio_playback.as_ref());
    clear_speech_bubble(&app);

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
pub async fn get_live_outbound_diagnostics(
    state: State<'_, AppState>,
) -> Result<Option<LiveOutboundDiagnostics>, String> {
    Ok(state.conversation_mgr.live_outbound_diagnostics().await)
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
    state.memory.forget_everything()?;
    state.ambient_scheduler.interrupt();
    state
        .behavior_engine
        .lock()
        .cooldowns
        .clear_event_fingerprints();
    crate::desktop::runtime::reset_observation_state();
    Ok(())
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

    let system_instruction =
        build_conversation_system_instruction(state.inner(), settings.memory_enabled);

    let text_model = state.get_text_model();
    let text_res = match text_model
        .generate(crate::ai::types::TextRequest {
            prompt: msg_trimmed,
            system_instruction: Some(system_instruction),
            temperature: Some(0.85),
            max_tokens: Some(1024),
        })
        .await
    {
        Ok(response) => response,
        Err(error_value) => {
            if *state.character_state.read() == CharacterState::Thinking {
                transition_and_emit(&state.character_state, &app, CharacterState::Idle)?;
            }
            return Err(state.secrets.redact(&error_value));
        }
    };

    let reply = text_res.text;
    let _ = app.emit("moose://transcript/moose", &reply);
    if let Err(error_value) = persist_transcript_if_enabled(
        state.db.as_ref(),
        settings.save_transcripts,
        "debug_terminal",
        "moose",
        &reply,
    ) {
        if *state.character_state.read() == CharacterState::Thinking {
            transition_and_emit(&state.character_state, &app, CharacterState::Idle)?;
        }
        return Err(error_value);
    }

    if *state.is_muted.read() {
        if *state.character_state.read() == CharacterState::Thinking {
            transition_and_emit(&state.character_state, &app, CharacterState::Idle)?;
        }
        return Ok(reply);
    }

    let playback = match invoke_standalone_speech(state.inner(), &app, &reply, None).await {
        Ok(playback) => playback,
        Err(error_value) => {
            if *state.character_state.read() == CharacterState::Thinking {
                transition_and_emit(&state.character_state, &app, CharacterState::Idle)?;
            }
            clear_speech_bubble(&app);
            return Err(error_value);
        }
    };
    schedule_standalone_completion(state.character_state.clone(), app.clone(), playback);

    Ok(reply)
}
