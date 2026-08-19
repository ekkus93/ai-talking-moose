use crate::ai::types::{TextRequest, TtsRequest};
use crate::app::state::AppState;
use crate::character::behavior::BehaviorEngine;
use crate::character::prompt::PromptBuilder;
use crate::character::state::{transition_character_state, CharacterState};
use crate::memory::MemoryManager;
use tauri::{Emitter, Runtime, State};

fn model_prompt_memories(memory_enabled: bool, memory: &MemoryManager) -> Vec<String> {
    if memory_enabled {
        memory.get_memory_strings()
    } else {
        Vec::new()
    }
}

fn transition_and_emit<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    target: CharacterState,
) -> Result<(), String> {
    transition_character_state(&state.character_state, target)?;
    let _ = app.emit("moose://state", target);
    Ok(())
}

async fn speak_standalone(
    text: &str,
    voice_name: Option<String>,
    state: &AppState,
) -> Result<(), String> {
    let (configured_voice, rate, pitch, output_device) = {
        let settings = state.settings.read();
        (
            settings.tts_voice.clone(),
            settings.speaking_rate,
            settings.pitch,
            settings.output_device.clone(),
        )
    };
    let synthesizer = state.get_speech_synthesizer();
    let report = crate::audio::synthesize_and_queue(
        synthesizer.as_ref(),
        state.audio_playback.as_ref(),
        TtsRequest {
            text: text.to_string(),
            voice_name: Some(voice_name.unwrap_or(configured_voice)),
            speaking_rate: Some(rate),
            pitch: Some(pitch),
        },
        output_device,
    )
    .await?;

    if report.dropped_samples > 0 {
        return Err(format!(
            "audio playback queue overflowed and dropped {} samples",
            report.dropped_samples
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn get_character_state(state: State<'_, AppState>) -> Result<CharacterState, String> {
    Ok(*state.character_state.read())
}

#[tauri::command]
pub fn set_character_state<R: Runtime>(
    new_state: CharacterState,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    transition_and_emit(state.inner(), &app, new_state)
}

#[tauri::command]
pub fn show_moose<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    transition_and_emit(state.inner(), &app, CharacterState::Idle)
}

#[tauri::command]
pub fn hide_moose<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    transition_and_emit(state.inner(), &app, CharacterState::Hidden)
}

#[tauri::command]
pub async fn dismiss_moose<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    state.behavior_engine.lock().cooldowns.record_dismissal(now);
    state
        .conversation_mgr
        .stop_session(state.audio_capture.clone(), state.audio_playback.clone())
        .await;
    transition_and_emit(state.inner(), &app, CharacterState::Dismissed)
}

#[tauri::command]
pub async fn set_mute<R: Runtime>(
    muted: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    if muted {
        // Set the privacy gate before awaiting teardown so a racing start request sees
        // muted=true either before or inside the serialized manager startup lock.
        *state.is_muted.write() = true;
        state
            .conversation_mgr
            .stop_session(state.audio_capture.clone(), state.audio_playback.clone())
            .await;
        transition_and_emit(state.inner(), &app, CharacterState::Muted)
    } else {
        *state.is_muted.write() = false;
        // Unmute is deliberately passive: it restores Idle but never starts capture.
        transition_and_emit(state.inner(), &app, CharacterState::Idle)
    }
}

#[tauri::command]
pub fn is_muted(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.is_muted.read())
}

#[tauri::command]
pub async fn audition_voice<R: Runtime>(
    voice_name: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let sample = match voice_name.as_str() {
        "Fenrir" => "Hey pal, I'm Moose. Deep, gravelly, and living in your computer.",
        "Charon" => "I am the Moose. Slow, deadpan, and wondering why you're clicking buttons.",
        "Orus" => "Greetings. Warm and relaxed, right here on your desktop.",
        "Puck" => "Hey there! Look at me, I'm the talking cartoon moose!",
        "Aoede" => "Hello! A bright and cheerful voice for your desktop moose.",
        _ => "Hello from your desktop moose.",
    };

    transition_and_emit(state.inner(), &app, CharacterState::Talking)?;
    let _ = app.emit("moose://speech-bubble", sample);

    speak_standalone(sample, Some(voice_name), state.inner()).await?;
    Ok(sample.to_string())
}

#[tauri::command]
pub async fn trigger_canned_reaction<R: Runtime>(
    reaction_type: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    if *state.is_muted.read() {
        return Ok(String::new());
    }

    let text = match reaction_type.as_str() {
        "greeting" => BehaviorEngine::get_canned_greeting(),
        "click" => BehaviorEngine::get_canned_click_reaction(),
        "dismiss" => BehaviorEngine::get_canned_dismiss_reaction(),
        _ => BehaviorEngine::get_canned_error_phrase(),
    };

    transition_and_emit(state.inner(), &app, CharacterState::Talking)?;
    let _ = app.emit("moose://speech-bubble", text);

    speak_standalone(text, None, state.inner()).await?;
    Ok(text.to_string())
}

#[tauri::command]
pub async fn trigger_ambient_remark<R: Runtime>(
    event_summary: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    if *state.is_muted.read() || !state.settings.read().unsolicited_comments {
        return Ok(None);
    }

    let should_speak = {
        let mut engine = state.behavior_engine.lock();
        engine
            .evaluate_event("manual_or_system", &event_summary, 0.75)
            .is_some()
    };

    if !should_speak {
        return Ok(None);
    }

    transition_and_emit(state.inner(), &app, CharacterState::Thinking)?;

    let memory_enabled = state.settings.read().memory_enabled;
    let memories = model_prompt_memories(memory_enabled, state.memory.as_ref());
    let config = state.behavior_engine.lock().config.clone();
    let prompt = PromptBuilder::build_ambient_prompt(&config, &event_summary, &memories);

    let text_model = state.get_text_model();
    let text = text_model
        .generate(TextRequest {
            prompt,
            system_instruction: None,
            temperature: Some(0.85),
            max_tokens: Some(60),
        })
        .await
        .map_err(|error_value| state.secrets.redact(&error_value))?
        .text;

    transition_and_emit(state.inner(), &app, CharacterState::Talking)?;
    let _ = app.emit("moose://speech-bubble", &text);

    speak_standalone(&text, None, state.inner()).await?;
    Ok(Some(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::sqlite::Database;
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
    fn set_character_state_ipc_rejects_invalid_transition_without_mutating_state() {
        let app_state = AppState::new_for_tests().unwrap();
        let authoritative_state = app_state.character_state.clone();

        let app = mock_builder()
            .manage(app_state)
            .invoke_handler(tauri::generate_handler![
                get_character_state,
                hide_moose,
                set_character_state
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        get_ipc_response(&webview, ipc_request("hide_moose", json!({})))
            .expect("Idle -> Hidden setup transition should succeed through IPC");
        assert_eq!(*authoritative_state.read(), CharacterState::Hidden);

        let error = get_ipc_response(
            &webview,
            ipc_request(
                "set_character_state",
                json!({ "newState": CharacterState::Talking }),
            ),
        )
        .expect_err("Hidden -> Talking must be rejected at the IPC command boundary");

        assert_eq!(
            error,
            json!("invalid character state transition: Hidden -> Talking")
        );
        assert_eq!(*authoritative_state.read(), CharacterState::Hidden);

        let reported_state =
            get_ipc_response(&webview, ipc_request("get_character_state", json!({})))
                .expect("authoritative state query should still succeed")
                .deserialize::<CharacterState>()
                .unwrap();
        assert_eq!(reported_state, CharacterState::Hidden);
    }

    #[test]
    fn ambient_prompt_memories_obey_memory_setting_and_restore_on_reenable() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let memory = MemoryManager::new(db);
        memory
            .remember(
                "User prefers local speech recognition",
                Some("conversation"),
            )
            .unwrap();

        assert!(model_prompt_memories(false, &memory).is_empty());
        assert_eq!(
            model_prompt_memories(true, &memory),
            vec!["User prefers local speech recognition".to_string()]
        );
    }
}
