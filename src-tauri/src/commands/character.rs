use crate::ai::types::TtsRequest;
use crate::app::state::AppState;
use crate::character::behavior::BehaviorEngine;
use crate::character::state::{transition_character_state, CharacterState};
use tauri::{Emitter, Runtime, State};

pub(crate) const VOICE_AUDITION_SCRIPT: &str = "Hello, I'm Moose. Oh good, another button. Professionally disappointed. Short version: it works. Longer version: I explain things while looking bewildered.";

fn transition_and_emit<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    target: CharacterState,
) -> Result<(), String> {
    transition_character_state(&state.character_state, target)?;
    let _ = app.emit("moose://state", target);
    Ok(())
}

fn cancel_standalone_audio<R: Runtime>(state: &AppState, app: &tauri::AppHandle<R>) {
    state
        .standalone_speech
        .cancel(state.audio_playback.as_ref());
    let _ = app.emit("moose://speech-bubble", "");
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
    let cancellation = state.standalone_speech.begin(state.audio_playback.as_ref());
    let report = crate::audio::speech::synthesize_and_queue_cancellable(
        synthesizer.as_ref(),
        state.audio_playback.as_ref(),
        TtsRequest {
            text: text.to_string(),
            voice_name: Some(voice_name.unwrap_or(configured_voice)),
            speaking_rate: Some(rate),
            pitch: Some(pitch),
        },
        output_device,
        &cancellation,
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
    show_character(state.inner(), &app)
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
    state.ambient_scheduler.interrupt();
    cancel_standalone_audio(state.inner(), &app);
    let now = chrono::Utc::now();
    state.behavior_engine.lock().cooldowns.record_dismissal(now);
    state
        .conversation_mgr
        .stop_session(state.audio_capture.clone(), state.audio_playback.clone())
        .await;
    transition_and_emit(state.inner(), &app, CharacterState::Dismissed)?;
    transition_and_emit(state.inner(), &app, CharacterState::Hidden)
}

#[tauri::command]
pub async fn set_mute<R: Runtime>(
    muted: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    if muted {
        state.ambient_scheduler.interrupt();
        cancel_standalone_audio(state.inner(), &app);
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
pub fn cancel_standalone_speech<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    cancel_standalone_audio(state.inner(), &app);
    if !state.conversation_mgr.is_active()
        && matches!(*state.character_state.read(), CharacterState::Talking)
    {
        transition_and_emit(state.inner(), &app, CharacterState::Idle)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn audition_voice<R: Runtime>(
    voice_name: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    crate::ai::google::validate_tts_voice(&voice_name)?;
    transition_and_emit(state.inner(), &app, CharacterState::Talking)?;
    let _ = app.emit("moose://speech-bubble", VOICE_AUDITION_SCRIPT);

    if let Err(error_value) =
        speak_standalone(VOICE_AUDITION_SCRIPT, Some(voice_name), state.inner()).await
    {
        let _ = app.emit("moose://speech-bubble", "");
        if !state.conversation_mgr.is_active()
            && matches!(*state.character_state.read(), CharacterState::Talking)
        {
            transition_and_emit(state.inner(), &app, CharacterState::Idle)?;
        }
        return Err(error_value);
    }
    Ok(VOICE_AUDITION_SCRIPT.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::capture::AudioCapture;
    use serde_json::json;
    use std::sync::Arc;
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
    use tauri::webview::InvokeRequest;
    use tauri::Listener;

    #[test]
    fn voice_auditions_use_one_fixed_original_moose_corpus() {
        let script = VOICE_AUDITION_SCRIPT;
        assert!(script.contains("Hello, I'm Moose"));
        assert!(script.contains("another button"));
        assert!(script.contains("Professionally disappointed"));
        assert!(script.contains("Short version"));
        assert!(script.contains("Longer version"));
    }

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

    struct InteractionTestFixture {
        app: tauri::App<tauri::test::MockRuntime>,
        webview: tauri::WebviewWindow<tauri::test::MockRuntime>,
        authoritative_state: Arc<parking_lot::RwLock<CharacterState>>,
        capture: Arc<parking_lot::Mutex<AudioCapture>>,
        playback: Arc<crate::audio::playback::AudioPlayback>,
        is_muted: Arc<parking_lot::RwLock<bool>>,
        behavior_engine: Arc<parking_lot::Mutex<BehaviorEngine>>,
    }

    fn interaction_test_app(character_state: CharacterState) -> InteractionTestFixture {
        let mut app_state = AppState::new_for_tests().unwrap();
        app_state.audio_capture = Arc::new(parking_lot::Mutex::new(AudioCapture::new_mock()));
        let authoritative_state = app_state.character_state.clone();
        let capture = app_state.audio_capture.clone();
        let playback = app_state.audio_playback.clone();
        let is_muted = app_state.is_muted.clone();
        let behavior_engine = app_state.behavior_engine.clone();

        let (pcm_tx, _pcm_rx) = tokio::sync::mpsc::channel(1);
        capture.lock().start(None, 16_000, pcm_tx, None).unwrap();
        playback.seed_buffer_for_tests(&[0.25, -0.25, 0.5], 0.5);
        let app = mock_builder()
            .manage(app_state)
            .invoke_handler(tauri::generate_handler![
                dismiss_moose,
                set_character_state,
                set_mute,
                show_moose
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        get_ipc_response(
            &webview,
            ipc_request(
                "set_character_state",
                json!({ "newState": character_state }),
            ),
        )
        .expect("interaction setup state should succeed through IPC");

        InteractionTestFixture {
            app,
            webview,
            authoritative_state,
            capture,
            playback,
            is_muted,
            behavior_engine,
        }
    }

    fn capture_state_events(
        app: &tauri::App<tauri::test::MockRuntime>,
    ) -> Arc<parking_lot::Mutex<Vec<CharacterState>>> {
        let events = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let events_for_listener = events.clone();
        app.listen("moose://state", move |event| {
            let state: CharacterState = serde_json::from_str(event.payload()).unwrap();
            events_for_listener.lock().push(state);
        });
        events
    }

    #[test]
    fn mute_ipc_tears_down_listening_and_talking_and_unmute_stays_passive() {
        for initial_state in [CharacterState::Listening, CharacterState::Talking] {
            let fixture = interaction_test_app(initial_state);
            assert!(fixture.capture.lock().is_active());
            assert!(fixture.playback.is_playing());
            assert!(fixture.playback.queue_length() > 0);

            get_ipc_response(
                &fixture.webview,
                ipc_request("set_mute", json!({ "muted": true })),
            )
            .expect("mute should succeed through IPC");

            assert!(*fixture.is_muted.read());
            assert_eq!(*fixture.authoritative_state.read(), CharacterState::Muted);
            assert!(!fixture.capture.lock().is_active());
            assert!(!fixture.playback.is_playing());
            assert_eq!(fixture.playback.queue_length(), 0);
            assert_eq!(fixture.playback.diagnostics().output_level, 0.0);
            get_ipc_response(
                &fixture.webview,
                ipc_request("set_mute", json!({ "muted": false })),
            )
            .expect("unmute should succeed through IPC");

            assert!(!*fixture.is_muted.read());
            assert_eq!(*fixture.authoritative_state.read(), CharacterState::Idle);
            assert!(!fixture.capture.lock().is_active());
        }
    }

    #[test]
    fn dismiss_ipc_tears_down_listening_and_talking_hides_and_records_cooldown() {
        for initial_state in [CharacterState::Listening, CharacterState::Talking] {
            let fixture = interaction_test_app(initial_state);
            let state_events = capture_state_events(&fixture.app);
            assert!(fixture.capture.lock().is_active());
            assert!(fixture.playback.is_playing());
            assert!(fixture.playback.queue_length() > 0);

            get_ipc_response(&fixture.webview, ipc_request("dismiss_moose", json!({})))
                .expect("dismiss should succeed through IPC");

            assert_eq!(
                state_events.lock().as_slice(),
                &[CharacterState::Dismissed, CharacterState::Hidden]
            );
            assert_eq!(*fixture.authoritative_state.read(), CharacterState::Hidden);
            assert!(!fixture.capture.lock().is_active());
            assert!(!fixture.playback.is_playing());
            assert_eq!(fixture.playback.queue_length(), 0);
            assert_eq!(fixture.playback.diagnostics().output_level, 0.0);
            let dismissal_time = fixture
                .behavior_engine
                .lock()
                .cooldowns
                .last_dismissal_time
                .expect("dismissal should record a cooldown timestamp");
            assert!(!fixture.behavior_engine.lock().cooldowns.can_speak_ambient(
                dismissal_time,
                0,
                10,
                false,
                22,
                8
            ));

            get_ipc_response(&fixture.webview, ipc_request("show_moose", json!({})))
                .expect("explicit user show should reappear through the state machine");
            assert_eq!(*fixture.authoritative_state.read(), CharacterState::Idle);
        }
    }

    #[test]
    fn hidden_character_reappears_through_appearing_before_idle() {
        let app_state = AppState::new_for_tests().unwrap();
        let authoritative_state = app_state.character_state.clone();

        let app = mock_builder()
            .manage(app_state)
            .invoke_handler(tauri::generate_handler![hide_moose, show_moose])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        get_ipc_response(&webview, ipc_request("hide_moose", json!({})))
            .expect("hide should succeed through IPC");
        assert_eq!(*authoritative_state.read(), CharacterState::Hidden);

        let state_events = capture_state_events(&app);
        get_ipc_response(&webview, ipc_request("show_moose", json!({})))
            .expect("show should accept a hidden character");

        assert_eq!(
            state_events.lock().as_slice(),
            &[CharacterState::Appearing, CharacterState::Idle]
        );
        assert_eq!(*authoritative_state.read(), CharacterState::Idle);
    }
}
