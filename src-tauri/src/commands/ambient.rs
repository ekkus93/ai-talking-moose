use crate::ai::types::{TextRequest, TtsRequest};
use crate::app::state::AppState;
use crate::character::ambient::{AmbientEvent, AmbientEventCategory};
use crate::character::behavior::AmbientPolicyContext;
#[cfg(test)]
use crate::character::behavior::BehaviorEngine;
use crate::character::prompt::PromptBuilder;
use crate::character::state::{transition_character_state, CharacterState};
use std::time::Duration;
use tauri::{Emitter, Runtime, State};

const MAX_AMBIENT_OUTPUT_CHARS: usize = 320;
const MAX_AMBIENT_PLAYBACK_WAIT: Duration = Duration::from_secs(10);
const AMBIENT_IDLE_AFTER_SPEECH: Duration = Duration::from_millis(750);

fn transition_and_emit<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    target: CharacterState,
) -> Result<(), String> {
    transition_character_state(&state.character_state, target)?;
    let _ = app.emit("moose://state", target);
    Ok(())
}

/// Ambient appearance is presentation-state-only. It intentionally never shows,
/// raises, or focuses the native window, so an unsolicited remark cannot steal focus.
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

fn bound_ambient_output(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_AMBIENT_OUTPUT_CHARS).collect())
}

async fn speak_standalone(text: &str, state: &AppState) -> Result<Duration, String> {
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

    if report.queued_samples == 0 {
        return Err("synthesized ambient speech contained no playable audio".to_string());
    }

    let sample_rate = state
        .audio_playback
        .output_sample_rate_hz()
        .unwrap_or(24_000);
    let duration = Duration::from_secs_f64(report.queued_samples as f64 / f64::from(sample_rate));
    Ok(duration.min(MAX_AMBIENT_PLAYBACK_WAIT))
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

fn clear_ambient_bubble<R: Runtime>(app: &tauri::AppHandle<R>) {
    let _ = app.emit("moose://speech-bubble", "");
}

fn restore_after_ambient_failure<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    appeared_for_ambient: bool,
) -> Result<(), String> {
    if matches!(
        *state.character_state.read(),
        CharacterState::Thinking | CharacterState::Talking
    ) {
        transition_and_emit(state, app, CharacterState::Idle)?;
    }
    clear_ambient_bubble(app);
    if appeared_for_ambient && *state.character_state.read() == CharacterState::Idle {
        transition_and_emit(state, app, CharacterState::Hidden)?;
    }
    Ok(())
}

async fn complete_ambient_appearance<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    playback_duration: Duration,
    appeared_for_ambient: bool,
) -> Result<(), String> {
    tokio::time::sleep(playback_duration).await;
    if *state.character_state.read() == CharacterState::Talking {
        transition_and_emit(state, app, CharacterState::Idle)?;
    }
    clear_ambient_bubble(app);

    if appeared_for_ambient && *state.character_state.read() == CharacterState::Idle {
        tokio::time::sleep(AMBIENT_IDLE_AFTER_SPEECH).await;
        if *state.character_state.read() == CharacterState::Idle {
            transition_and_emit(state, app, CharacterState::Hidden)?;
        }
    }
    Ok(())
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

    let initial_state = *state.character_state.read();
    if !matches!(
        initial_state,
        CharacterState::Hidden | CharacterState::Dismissed | CharacterState::Idle
    ) {
        return Ok(None);
    }
    let appeared_for_ambient = matches!(
        initial_state,
        CharacterState::Hidden | CharacterState::Dismissed
    );
    if appeared_for_ambient {
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
    let generated = match state
        .get_text_model()
        .generate(TextRequest {
            prompt,
            system_instruction: None,
            temperature: Some(0.85),
            max_tokens: Some(60),
        })
        .await
    {
        Ok(response) => response.text,
        Err(error_value) => {
            restore_after_ambient_failure(state, app, appeared_for_ambient)?;
            return Err(state.secrets.redact(&error_value));
        }
    };
    let Some(text) = bound_ambient_output(&generated) else {
        restore_after_ambient_failure(state, app, appeared_for_ambient)?;
        return Ok(None);
    };

    // V1 deliberately has no model classifier. This second deterministic local gate
    // is authoritative and protects against settings/privacy changes during generation.
    let delivery_context = ambient_policy_context(state, event.category);
    let delivery_decision = state
        .behavior_engine
        .lock()
        .evaluate_ambient_event(&event, delivery_context);
    if !delivery_decision.should_speak {
        let _ = app.emit("moose://ambient/decision", &delivery_decision);
        restore_after_ambient_failure(state, app, appeared_for_ambient)?;
        return Ok(None);
    }

    let playback_duration = match speak_standalone(&text, state).await {
        Ok(duration) => duration,
        Err(error_value) => {
            restore_after_ambient_failure(state, app, appeared_for_ambient)?;
            return Err(error_value);
        }
    };

    // Do not surface a generated remark until TTS was successfully queued. Provider
    // failure therefore never invents or displays a fallback ambient comment.
    transition_and_emit(state, app, CharacterState::Talking)?;
    let _ = app.emit("moose://speech-bubble", &text);
    state.behavior_engine.lock().record_ambient_delivery(&event);
    complete_ambient_appearance(state, app, playback_duration, appeared_for_ambient).await?;
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
        assert!(ambient_privacy_allowed(
            &state,
            AmbientEventCategory::Manual
        ));
        assert!(!ambient_privacy_allowed(
            &state,
            AmbientEventCategory::Other
        ));

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
        let mut engine =
            BehaviorEngine::new(crate::character::personality::CharacterConfig::default());
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

    #[test]
    fn ambient_output_is_locally_bounded_and_empty_output_is_dropped() {
        let oversized = format!("Moose:{}", "x".repeat(MAX_AMBIENT_OUTPUT_CHARS * 4));
        let bounded = bound_ambient_output(&oversized).unwrap();
        assert_eq!(bounded.chars().count(), MAX_AMBIENT_OUTPUT_CHARS);
        assert!(bound_ambient_output("   \n\t").is_none());
    }

    #[test]
    fn ambient_lifecycle_waits_are_hard_bounded() {
        assert!(MAX_AMBIENT_PLAYBACK_WAIT <= Duration::from_secs(10));
        assert!(AMBIENT_IDLE_AFTER_SPEECH <= Duration::from_secs(1));
    }
}
