use crate::ai::types::TextRequest;
use crate::app::request_snapshot::TextRequestSettingsSnapshot;
use crate::app::state::{AppSettings, AppState};
use crate::character::ambient::{AmbientEvent, AmbientEventCategory};
use crate::character::behavior::AmbientPolicyContext;
#[cfg(test)]
use crate::character::behavior::BehaviorEngine;
use crate::character::prompt::PromptBuilder;
use crate::character::state::CharacterState;
use crate::commands::conversation::model_prompt_memories;
use crate::commands::presentation::{clear_speech_bubble, show_character, transition_and_emit};
use crate::commands::speech::{invoke_standalone_speech, StandaloneSpeechPlayback};
use std::time::Duration;
use tauri::{Emitter, Runtime, State};

const MAX_AMBIENT_OUTPUT_CHARS: usize = 320;

fn bound_ambient_output(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_AMBIENT_OUTPUT_CHARS).collect())
}

fn build_ambient_model_prompt_for(
    state: &AppState,
    snapshot: &TextRequestSettingsSnapshot,
    event_summary: &str,
) -> String {
    let memories = model_prompt_memories(state, snapshot.settings.memory_enabled);
    PromptBuilder::build_ambient_prompt(&snapshot.character_config, event_summary, &memories)
}

#[cfg(test)]
fn build_ambient_model_prompt(state: &AppState, event_summary: &str) -> String {
    let snapshot = state.capture_text_request_settings();
    build_ambient_model_prompt_for(state, &snapshot, event_summary)
}

fn ambient_text_request(prompt: String) -> TextRequest {
    TextRequest {
        prompt,
        system_instruction: None,
        temperature: Some(0.85),
        max_tokens: Some(60),
    }
}

async fn generate_ambient_text_for(
    state: &AppState,
    settings: &AppSettings,
    prompt: String,
) -> Result<String, crate::ai::types::ProviderError> {
    state
        .get_text_model_for(settings)
        .generate(ambient_text_request(prompt))
        .await
        .map(|response| response.text)
}

#[cfg(test)]
async fn generate_ambient_text(
    state: &AppState,
    prompt: String,
) -> Result<String, crate::ai::types::ProviderError> {
    let settings = state.settings_snapshot();
    generate_ambient_text_for(state, &settings, prompt).await
}

fn ambient_privacy_allowed_for(settings: &AppSettings, category: AmbientEventCategory) -> bool {
    match category {
        AmbientEventCategory::Application => settings.active_app_observation,
        // Window-title observation remains deliberately unsupported in V1.
        AmbientEventCategory::WindowTitle => false,
        AmbientEventCategory::Manual
        | AmbientEventCategory::Idle
        | AmbientEventCategory::Power
        | AmbientEventCategory::Wake
        | AmbientEventCategory::System => true,
        AmbientEventCategory::Other => false,
    }
}

#[cfg(test)]
fn ambient_privacy_allowed(state: &AppState, category: AmbientEventCategory) -> bool {
    let settings = state.settings_snapshot();
    ambient_privacy_allowed_for(&settings, category)
}

fn ambient_policy_context_for(
    state: &AppState,
    settings: &AppSettings,
    category: AmbientEventCategory,
) -> AmbientPolicyContext {
    AmbientPolicyContext {
        privacy_allowed: ambient_privacy_allowed_for(settings, category),
        muted: *state.is_muted.read(),
        conversation_active: state.conversation_mgr.is_active(),
    }
}

fn ambient_policy_context(
    state: &AppState,
    category: AmbientEventCategory,
) -> AmbientPolicyContext {
    let settings = state.settings_snapshot();
    ambient_policy_context_for(state, &settings, category)
}

fn configured_ambient_hide_delay(state: &AppState) -> Duration {
    Duration::from_secs(u64::from(state.settings.read().hide_delay_seconds))
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
        transition_and_emit(&state.character_state, app, CharacterState::Idle)?;
    }
    clear_speech_bubble(app);
    if appeared_for_ambient && *state.character_state.read() == CharacterState::Idle {
        transition_and_emit(&state.character_state, app, CharacterState::Hidden)?;
    }
    Ok(())
}

async fn complete_ambient_appearance<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    playback: &StandaloneSpeechPlayback,
    appeared_for_ambient: bool,
) -> Result<(), String> {
    if !playback.completed_without_cancellation().await {
        return Ok(());
    }
    let Some(result) = playback.with_current(|| {
        if *state.character_state.read() == CharacterState::Talking {
            transition_and_emit(&state.character_state, app, CharacterState::Idle)?;
        }
        clear_speech_bubble(app);
        Ok::<(), String>(())
    }) else {
        return Ok(());
    };
    result?;

    if appeared_for_ambient && *state.character_state.read() == CharacterState::Idle {
        if !playback
            .wait_without_cancellation(configured_ambient_hide_delay(state))
            .await
        {
            return Ok(());
        }
        if let Some(result) = playback.with_current(|| {
            if *state.character_state.read() == CharacterState::Idle {
                transition_and_emit(&state.character_state, app, CharacterState::Hidden)?;
            }
            Ok::<(), String>(())
        }) {
            result?;
        }
    }
    Ok(())
}

pub(crate) async fn process_ambient_event<R: Runtime>(
    event: AmbientEvent,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    let request_snapshot = state.capture_text_request_settings();
    let context = ambient_policy_context_for(state, &request_snapshot.settings, event.category);
    let decision = state
        .behavior_engine
        .lock()
        .evaluate_ambient_event_with_config(&event, context, &request_snapshot.character_config);
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
        show_character(&state.character_state, app)?;
    }
    transition_and_emit(&state.character_state, app, CharacterState::Thinking)?;

    let prompt = build_ambient_model_prompt_for(state, &request_snapshot, &event.summary);
    let generated = match generate_ambient_text_for(state, &request_snapshot.settings, prompt).await
    {
        Ok(text) => text,
        Err(error_value) => {
            restore_after_ambient_failure(state, app, appeared_for_ambient)?;
            return Err(crate::ai::types::ProviderError::from_kind(error_value.kind).to_string());
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

    let playback = match invoke_standalone_speech(state, app, &text, None).await {
        Ok(playback) => playback,
        Err(error_value) => {
            restore_after_ambient_failure(state, app, appeared_for_ambient)?;
            return Err(error_value);
        }
    };

    // The shared standalone-speech helper does not surface Talking/bubble state until
    // synthesis produced playable audio and the bounded queue accepted the utterance.
    state.behavior_engine.lock().record_ambient_delivery(&event);
    complete_ambient_appearance(state, app, &playback, appeared_for_ambient).await?;
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
    use crate::ai::types::{ProviderErrorKind, TextProvider};

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

        {
            let mut settings = state.settings.write();
            settings.active_app_observation = true;
            settings.window_title_observation = true;
        }
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
    fn ambient_request_preserves_short_generation_policy() {
        let request = ambient_text_request("ambient prompt".to_string());
        assert_eq!(request.prompt, "ambient prompt");
        assert_eq!(request.system_instruction, None);
        assert_eq!(request.temperature, Some(0.85));
        assert_eq!(request.max_tokens, Some(60));
    }

    #[tokio::test]
    async fn ambient_generation_routes_through_selected_local_text_provider() {
        let state = AppState::new_for_tests().unwrap();
        {
            let mut settings = state.settings.write();
            settings.text_provider = TextProvider::Local;
            settings.local_text_model = "missing-local-model".to_string();
        }

        let error = generate_ambient_text(&state, "ambient local prompt".to_string())
            .await
            .expect_err("selected Local provider must fail locally rather than call Google");
        assert_eq!(error.kind, ProviderErrorKind::Model);
    }

    #[tokio::test]
    async fn ambient_generation_routes_through_selected_google_text_provider() {
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().text_provider = TextProvider::Google;
        assert_eq!(state.settings.read().text_provider, TextProvider::Google);

        let error = generate_ambient_text(&state, "ambient google prompt".to_string())
            .await
            .expect_err("selected Google provider without a key must fail authentication");
        assert_eq!(error.kind, ProviderErrorKind::Auth);
    }

    #[test]
    fn production_ambient_prompt_obeys_memory_privacy_gate() {
        const PRIVATE_MEMORY: &str = "User prefers local speech recognition";

        let state = AppState::new_for_tests().unwrap();
        state
            .memory
            .remember(PRIVATE_MEMORY, Some("conversation"))
            .unwrap();

        state.settings.write().memory_enabled = false;
        let disabled_prompt = build_ambient_model_prompt(&state, "A harmless ambient event");
        assert!(
            !disabled_prompt.contains(PRIVATE_MEMORY),
            "memory-disabled ambient prompt must not contain retained memory"
        );

        state.settings.write().memory_enabled = true;
        let enabled_prompt = build_ambient_model_prompt(&state, "A harmless ambient event");
        assert!(
            enabled_prompt.contains(PRIVATE_MEMORY),
            "re-enabling memory must restore retained memory to the ambient prompt"
        );
    }

    #[tokio::test]
    async fn ambient_request_uses_one_snapshot_across_concurrent_settings_change() {
        const PRIVATE_MEMORY: &str = "LLMR_P4_AMBIENT_PRIVATE_MEMORY";
        let state = AppState::new_for_tests().unwrap();
        state
            .memory
            .remember(PRIVATE_MEMORY, Some("p4-ambient-snapshot"))
            .unwrap();
        {
            let mut settings = state.settings.write();
            settings.text_provider = TextProvider::Local;
            settings.local_text_model = "missing-local-model-a".to_string();
            settings.memory_enabled = true;
            settings.active_app_observation = true;
            settings.talkativeness = 1.0;
        }

        let captured = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let resume = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let request_state = state.clone();
        let request_captured = captured.clone();
        let request_resume = resume.clone();

        let in_flight = tokio::spawn(async move {
            let snapshot = request_state.capture_text_request_settings();
            request_captured.wait().await;
            request_resume.wait().await;
            let privacy_allowed =
                ambient_privacy_allowed_for(&snapshot.settings, AmbientEventCategory::Application);
            let prompt =
                build_ambient_model_prompt_for(&request_state, &snapshot, "application changed");
            let error =
                generate_ambient_text_for(&request_state, &snapshot.settings, prompt.clone())
                    .await
                    .expect_err("captured ambient request should remain Local");
            (snapshot, privacy_allowed, prompt, error.kind)
        });

        captured.wait().await;
        {
            let mut settings = state.settings.write();
            settings.text_provider = TextProvider::Google;
            settings.google_text_model = "gemini-3.6-flash".to_string();
            settings.memory_enabled = false;
            settings.active_app_observation = false;
            settings.talkativeness = 0.0;
        }
        resume.wait().await;

        let (captured_snapshot, privacy_allowed, captured_prompt, error_kind) =
            in_flight.await.unwrap();
        assert_eq!(
            captured_snapshot.settings.text_provider,
            TextProvider::Local
        );
        assert!(captured_snapshot.settings.memory_enabled);
        assert!(privacy_allowed);
        assert_eq!(
            captured_snapshot.character_config.personality.talkativeness,
            1.0
        );
        assert!(captured_prompt.contains(PRIVATE_MEMORY));
        assert_eq!(error_kind, ProviderErrorKind::Model);

        let next_snapshot = state.capture_text_request_settings();
        assert_eq!(next_snapshot.settings.text_provider, TextProvider::Google);
        assert!(!next_snapshot.settings.memory_enabled);
        assert!(!ambient_privacy_allowed_for(
            &next_snapshot.settings,
            AmbientEventCategory::Application
        ));
        assert_eq!(
            next_snapshot.character_config.personality.talkativeness,
            0.0
        );
        let next_prompt =
            build_ambient_model_prompt_for(&state, &next_snapshot, "application changed");
        assert!(!next_prompt.contains(PRIVATE_MEMORY));
        let next_error = generate_ambient_text_for(&state, &next_snapshot.settings, next_prompt)
            .await
            .expect_err("next ambient Google request without key must fail auth");
        assert_eq!(next_error.kind, ProviderErrorKind::Auth);

        let delivery_context = ambient_policy_context(&state, AmbientEventCategory::Application);
        assert!(!delivery_context.privacy_allowed);
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
    fn ambient_lifecycle_playback_wait_is_hard_bounded() {
        assert!(crate::commands::speech::MAX_STANDALONE_PLAYBACK_WAIT <= Duration::from_secs(10));
    }

    #[test]
    fn configured_hide_delay_reads_live_app_setting() {
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().hide_delay_seconds = 17;
        assert_eq!(
            configured_ambient_hide_delay(&state),
            Duration::from_secs(17)
        );
    }
}
