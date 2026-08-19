use crate::ai::types::{TextRequest, TtsRequest};
use crate::app::state::AppState;
use crate::character::behavior::BehaviorEngine;
use crate::character::prompt::PromptBuilder;
use crate::character::state::{transition_character_state, CharacterState};
use crate::memory::MemoryManager;
use tauri::{Emitter, State};

fn model_prompt_memories(memory_enabled: bool, memory: &MemoryManager) -> Vec<String> {
    if memory_enabled {
        memory.get_memory_strings()
    } else {
        Vec::new()
    }
}

fn transition_and_emit(
    state: &AppState,
    app: &tauri::AppHandle,
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
pub fn set_character_state(
    new_state: CharacterState,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    transition_and_emit(state.inner(), &app, new_state)
}

#[tauri::command]
pub fn show_moose(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    transition_and_emit(state.inner(), &app, CharacterState::Idle)
}

#[tauri::command]
pub fn hide_moose(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    transition_and_emit(state.inner(), &app, CharacterState::Hidden)
}

#[tauri::command]
pub async fn dismiss_moose(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
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
pub async fn set_mute(
    muted: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
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
pub async fn audition_voice(
    voice_name: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
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
pub async fn trigger_canned_reaction(
    reaction_type: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
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
pub async fn trigger_ambient_remark(
    event_summary: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
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
    use std::sync::Arc;

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
