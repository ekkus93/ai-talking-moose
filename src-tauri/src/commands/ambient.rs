use crate::ai::types::{TextRequest, TtsRequest};
use crate::app::state::AppState;
use crate::character::ambient::{AmbientEvent, AmbientEventCategory};
use crate::character::behavior::{AmbientPolicyContext, BehaviorEngine};
use crate::character::prompt::PromptBuilder;
use crate::character::state::{transition_character_state, CharacterState};
use tauri::{Emitter, Runtime, State};

fn transition_and_emit<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    target: CharacterState,
) -> Result<(), String> {
    transition_character_state(&state.character_state, target)?;
    let _ = app.emit("moose://state", target);
    Ok(())
}

fn show_character<R: Runtime>(state: &AppState, app: &tauri::AppHandle<R>) -> Result<(), String> {
    let current = *state.character_state.read();
    if matches!(current, CharacterState::Dismissed) {
        transition_and_emit(state, app, CharacterState::Hidden)?;
    }
    if matches!(*state.character_state.read(), CharacterState::Hidden) {
        transition_and_emit(state, app, CharacterState::Appearing)?;
    }
    transition_and_emit(state, app, CharacterState::Idle)
}

async fn speak_standalone(text: &str, state: &AppState) -> Result<(), String> {
    let (voice, rate, pitch, output_device) = {
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
            voice_name: Some(voice),
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

fn ambient_privacy_allowed(state: &AppState, category: AmbientEventCategory) -> bool {
    let settings = state.settings.read();
    match category {
        AmbientEventCategory::Application => settings.active_app_observation,
        AmbientEventCategory::WindowTitle => settings.window_title_observation,
        AmbientEventCategory::Manual
        | AmbientEventCategory::Idle
        | AmbientEventCategory::Power
        | AmbientEventCategory::Wake
        | AmbientEventCategory::System => true,
        AmbientEventCategory::Other => false,
    }
}

fn ambient_policy_context(
    state: &AppState,
    category: AmbientEventCategory,
) -> AmbientPolicyContext {
    AmbientPolicyContext {
        privacy_allowed: ambient_privacy_allowed(state, category),
        muted: *state.is_muted.read(),
        conversation_active: state.conversation_mgr.is_active(),
    }
}

pub(crate) async fn process_ambient_event<R: Runtime>(
    event: AmbientEvent,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    let context = ambient_policy_context(state, event.category);
    let decision = state
        .behavior_engine
        .lock()
        .evaluate_ambient_event(&event, context);
    let _ = app.emit("moose://ambient/decision", &decision);
    if !decision.should_speak {
        return Ok(None);
    }

    if matches!(
        *state.character_state.read(),
        CharacterState::Hidden | CharacterState::Dismissed
    ) {
        show_character(state, app)?;
    }
    transition_and_emit(state, app, CharacterState::Thinking)?;

    let memories = if state.settings.read().memory_enabled {
        state.memory.get_memory_strings()
    } else {
        Vec::new()
    };
    let config = state.behavior_engine.lock().config.clone();
    let prompt = PromptBuilder::build_ambient_prompt(&config, &event.summary, &memories);
    let text = state
        .get_text_model()
        .generate(TextRequest {
            prompt,
            system_instruction: None,
            temperature: Some(0.85),
            max_tokens: Some(60),
        })
        .await
        .map_err(|error_value| state.secrets.redact(&error_value))?
        .text;

    let delivery_context = ambient_policy_context(state, event.category);
    let delivery_decision = state
        .behavior_engine
        .lock()
        .evaluate_ambient_event(&event, delivery_context);
    if !delivery_decision.should_speak {
        let _ = app.emit("moose://ambient/decision", &delivery_decision);
        if *state.character_state.read() == CharacterState::Thinking {
            transition_and_emit(state, app, CharacterState::Idle)?;
        }
        return Ok(None);
    }

    transition_and_emit(state, app, CharacterState::Talking)?;
    let _ = app.emit("moose://speech-bubble", &text);
    speak_standalone(&text, state).await?;
    state
        .behavior_engine
        .lock()
        .record_ambient_delivery(&event);
    Ok(Some(text))
}

#[tauri::command]
pub async fn trigger_ambient_remark(
    event_summary: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    state
        .ambient_scheduler
        .clone()
        .submit(AmbientEvent::new("manual_or_system", event_summary, 0.75))
        .await
}

#[tauri::command]
pub async fn start_conversation<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    state.ambient_scheduler.interrupt();
    crate::commands::conversation::start_conversation(state, app).await
}

#[tauri::command]
pub async fn dismiss_moose<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    state.ambient_scheduler.interrupt();
    crate::commands::character::dismiss_moose(state, app).await
}

#[tauri::command]
pub async fn set_mute<R: Runtime>(
    muted: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    if muted {
        state.ambient_scheduler.interrupt();
    }
    crate::commands::character::set_mute(muted, state, app).await
}

#[tauri::command]
pub async fn barge_in(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state.ambient_scheduler.interrupt();
    let behavior_engine = state.behavior_engine.clone();
    let result = crate::commands::conversation::barge_in(state, app).await;
    if result.is_ok() {
        behavior_engine.lock().cooldowns.record_interruption();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_categories_fail_closed_when_privacy_settings_are_off() {
        let state = AppState::new_for_tests().unwrap();
        assert!(!ambient_privacy_allowed(
            &state,
            AmbientEventCategory::Application
        ));
        assert!(!ambient_privacy_allowed(
            &state,
            AmbientEventCategory::WindowTitle
        ));
        assert!(ambient_privacy_allowed(&state, AmbientEventCategory::Manual));
        assert!(!ambient_privacy_allowed(&state, AmbientEventCategory::Other));

        state.settings.write().active_app_observation = true;
        assert!(ambient_privacy_allowed(
            &state,
            AmbientEventCategory::Application
        ));
        assert!(!ambient_privacy_allowed(
            &state,
            AmbientEventCategory::WindowTitle
        ));
    }

    #[test]
    fn policy_diagnostics_do_not_carry_event_summary() {
        let mut engine = BehaviorEngine::new(
            crate::character::personality::CharacterConfig::default(),
        );
        let event = AmbientEvent::new(
            "active_app_changed",
            "Secret Project Window".to_string(),
            1.0,
        );
        let decision = engine.evaluate_ambient_event(
            &event,
            AmbientPolicyContext {
                privacy_allowed: false,
                ..Default::default()
            },
        );
        let json = serde_json::to_string(&decision).unwrap();
        assert!(!json.contains("Secret Project Window"));
        assert!(json.contains("privacy_denied"));
    }
}
