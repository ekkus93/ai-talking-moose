use super::*;
use crate::audio::capture::AudioCapture;
use crate::test_support::{assert_log_capture_live, capture_logs};
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
fn production_conversation_prompt_obeys_memory_privacy_gate() {
    const PRIVATE_MEMORY: &str = "User prefers local speech recognition";

    let state = AppState::new_for_tests().unwrap();
    state
        .memory
        .remember(PRIVATE_MEMORY, Some("conversation"))
        .unwrap();

    let disabled_prompt = build_conversation_system_instruction(&state, false);
    assert!(
        !disabled_prompt.contains(PRIVATE_MEMORY),
        "memory-disabled production prompt must not contain retained memory"
    );

    let enabled_prompt = build_conversation_system_instruction(&state, true);
    assert!(
        enabled_prompt.contains(PRIVATE_MEMORY),
        "re-enabling memory must restore retained memory to the production prompt"
    );
}

#[test]
fn private_memory_consumed_by_conversation_prompt_never_enters_tracing() {
    const PRIVATE_MEMORY: &str = "PRIVATE_MEMORY_FACT_915db4";
    let state = AppState::new_for_tests().unwrap();
    state
        .memory
        .remember(PRIVATE_MEMORY, Some("privacy-test"))
        .unwrap();

    let (prompt, logs) = capture_logs(|| build_conversation_system_instruction(&state, true));

    assert!(
        prompt.contains(PRIVATE_MEMORY),
        "production memory path must consume the sentinel"
    );
    assert_log_capture_live(&logs);
    assert!(!logs.contains(PRIVATE_MEMORY));
}

#[test]
fn typed_text_routes_selected_local_provider_and_restores_idle_on_failure() {
    let app_state = AppState::new_for_tests().unwrap();
    {
        let mut settings = app_state.settings.write();
        settings.text_provider = crate::ai::types::TextProvider::Local;
        settings.local_text_model = "missing-local-model".to_string();
        settings.save_transcripts = false;
    }
    let character_state = app_state.character_state.clone();
    let playback = app_state.audio_playback.clone();
    let db = app_state.db.clone();

    let app = mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![send_text_message])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let error = get_ipc_response(
        &webview,
        ipc_request(
            "send_text_message",
            json!({ "message": "route this through Local" }),
        ),
    )
    .expect_err("selected Local provider must fail locally for an unknown model");

    assert!(error
        .as_str()
        .unwrap()
        .contains("selected conversation model is unavailable"));
    assert_eq!(*character_state.read(), CharacterState::Idle);
    assert_eq!(playback.queue_length(), 0);
    assert!(db.get_transcripts(10).unwrap().is_empty());
}

#[test]
fn typed_text_routes_selected_google_provider_without_fake_fallback() {
    let app_state = AppState::new_for_tests().unwrap();
    app_state.settings.write().text_provider =
        crate::ai::types::TextProvider::Google;
    assert_eq!(
        app_state.settings.read().text_provider,
        crate::ai::types::TextProvider::Google
    );
    let character_state = app_state.character_state.clone();
    let playback = app_state.audio_playback.clone();

    let app = mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![send_text_message])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let error = get_ipc_response(
        &webview,
        ipc_request(
            "send_text_message",
            json!({ "message": "route this through Google" }),
        ),
    )
    .expect_err("selected Google provider without a key must fail authentication");

    assert!(error.as_str().unwrap().contains("authentication failed"));
    assert_eq!(*character_state.read(), CharacterState::Idle);
    assert_eq!(playback.queue_length(), 0);
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
fn forget_everything_ipc_purges_private_records_but_preserves_preferences_and_credentials() {
    let app_state = AppState::new_for_tests().unwrap();
    app_state
        .memory
        .remember("User likes tea", Some("preference"))
        .unwrap();
    app_state
        .db
        .add_transcript("session", "user", "private transcript")
        .unwrap();
    app_state
        .db
        .set_setting("non_secret_preference", "keep")
        .unwrap();
    app_state
        .secrets
        .set_google_api_key("test-api-key".to_string())
        .unwrap();
    let db = app_state.db.clone();
    let secrets = app_state.secrets.clone();

    let app = mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![forget_everything])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    get_ipc_response(&webview, ipc_request("forget_everything", json!({})))
        .expect("Forget Everything should succeed through IPC");

    assert!(db.get_memories().unwrap().is_empty());
    assert!(db.get_transcripts(10).unwrap().is_empty());
    assert_eq!(
        db.get_setting("non_secret_preference").unwrap(),
        Some("keep".to_string())
    );
    assert!(secrets.has_google_api_key());
}

#[test]
fn transcript_retention_on_writes_records() {
    let db = Database::new_in_memory().unwrap();

    persist_transcript_if_enabled(&db, true, "session", "user", "retained input").unwrap();
    persist_transcript_if_enabled(&db, true, "session", "moose", "retained output").unwrap();

    let transcripts = db.get_transcripts(10).unwrap();
    assert_eq!(transcripts.len(), 2);
}
