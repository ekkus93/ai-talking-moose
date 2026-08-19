use crate::ai::types::LiveSessionConfig;
use crate::app::state::AppState;
use crate::character::prompt::PromptBuilder;
use crate::character::state::CharacterState;
use crate::conversation::session::ConversationCallbacks;
use crate::persistence::{MemoryRecord, TranscriptRecord};
use tauri::{Emitter, State};
use tokio::sync::mpsc;
use tracing::info;

#[tauri::command]
pub async fn start_conversation(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    if *state.is_muted.read() {
        return Err("Moose is currently muted".to_string());
    }

    let settings = state.settings.read().clone();
    let provider = state.get_live_provider();
    let playback = state.audio_playback.clone();
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
        sample_rate_in: 16000,
        sample_rate_out: 24000,
    };

    let app_handle_1 = app.clone();
    let app_handle_2 = app.clone();
    let app_handle_3 = app.clone();
    let db_ref = state.db.clone();
    let save_transcripts = settings.save_transcripts;

    // Start playback device if not already running
    let _ = state.audio_playback.start(settings.output_device.clone());

    let session_id = state
        .conversation_mgr
        .start_session(
            provider,
            config,
            playback,
            tool_router,
            ConversationCallbacks::new(
                move |new_state: CharacterState| {
                    let _ = app_handle_1.emit("moose://state", new_state);
                },
                move |role: String, text: String| {
                    let _ = app_handle_2.emit(&format!("moose://transcript/{}", role), &text);
                    if save_transcripts {
                        let _ = db_ref.add_transcript("active_session", &role, &text);
                    }
                },
                move |speech_text: String| {
                    let _ = app_handle_3.emit("moose://speech-bubble", &speech_text);
                },
            ),
        )
        .await?;

    // Start audio microphone capture and pipe to conversation manager
    let (pcm_tx, mut pcm_rx) = mpsc::channel::<Vec<u8>>(32);
    let (level_tx, mut level_rx) = mpsc::channel::<f32>(32);

    let mut capture = state.audio_capture.lock();
    capture.start(settings.input_device.clone(), 16000, pcm_tx, Some(level_tx))?;

    let conv_mgr = state.conversation_mgr.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(chunk) = pcm_rx.recv().await {
            if !conv_mgr.is_active() {
                break;
            }
            conv_mgr.send_audio_frame(&chunk).await;
        }
    });

    let app_level = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(level) = level_rx.recv().await {
            let _ = app_level.emit("moose://audio/input-level", level);
        }
    });

    info!("Conversation session started: {}", session_id);
    Ok(session_id)
}

#[tauri::command]
pub async fn stop_conversation(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.audio_capture.lock().stop();
    state
        .conversation_mgr
        .stop_session(state.audio_playback.clone())
        .await;
    *state.character_state.write() = CharacterState::Idle;
    let _ = app.emit("moose://state", CharacterState::Idle);
    Ok(())
}

#[tauri::command]
pub async fn barge_in(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state
        .conversation_mgr
        .barge_in(state.audio_playback.clone())
        .await;
    *state.character_state.write() = CharacterState::Interrupted;
    let _ = app.emit("moose://state", CharacterState::Interrupted);
    Ok(())
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
    let msg_trimmed = message.trim().to_string();
    if msg_trimmed.is_empty() {
        return Ok(String::new());
    }
    let settings = state.settings.read().clone();

    // 1. Emit user transcript and persist only when retention is enabled.
    let _ = app.emit("moose://transcript/user", &msg_trimmed);
    if settings.save_transcripts {
        let _ = state
            .db
            .add_transcript("debug_terminal", "user", &msg_trimmed);
    }

    // 2. Set character state to thinking
    *state.character_state.write() = CharacterState::Thinking;
    let _ = app.emit("moose://state", CharacterState::Thinking);

    // 3. Build prompt with personality, rules, memories
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
        .unwrap_or_else(|e| crate::ai::types::TextResponse {
            text: format!("Oops! My brain had a hiccup: {}", e),
            finish_reason: None,
        });

    let reply = text_res.text;

    // 4. Update state to Talking, emit speech bubble & transcript, and retain only
    // when the user has explicitly enabled local transcript storage.
    *state.character_state.write() = CharacterState::Talking;
    let _ = app.emit("moose://state", CharacterState::Talking);
    let _ = app.emit("moose://transcript/moose", &reply);
    let _ = app.emit("moose://speech-bubble", &reply);
    if settings.save_transcripts {
        let _ = state
            .db
            .add_transcript("debug_terminal", "moose", &reply);
    }

    // 5. Play speech output via system speech
    if !*state.is_muted.read() {
        crate::audio::play_system_speech(&reply);
    }

    Ok(reply)
}
