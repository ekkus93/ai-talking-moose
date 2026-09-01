use super::*;
use crate::ai::types::TextProvider;
use crate::asr::AsrMode;
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
fn local_text_selection_does_not_change_moonshine_voice_preflight() {
    let mut app_state = AppState::new_for_tests().unwrap();
    app_state.audio_capture = Arc::new(parking_lot::Mutex::new(AudioCapture::new_mock()));
    {
        let mut settings = app_state.settings.write();
        settings.text_provider = TextProvider::Local;
        settings.local_text_model = "missing-local-model".to_string();
        settings.asr_mode = AsrMode::MoonshineTinyStreaming;
    }
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
        .expect_err("Local text selection must not bypass Moonshine voice preflight");
    let error = error.as_str().unwrap();

    assert!(error.contains("Moonshine Tiny"));
    assert!(error.contains("No microphone audio was sent"));
    assert!(!error.contains("selected conversation model is unavailable"));
    assert!(!capture.lock().is_active());
}

#[test]
fn local_text_selection_still_uses_google_live_for_cloud_voice() {
    let app_state = AppState::new_for_tests().unwrap();
    {
        let mut settings = app_state.settings.write();
        settings.text_provider = TextProvider::Local;
        settings.local_text_model = "missing-local-model".to_string();
        settings.asr_mode = AsrMode::GeminiLiveAudio;
    }

    let app = mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![start_conversation])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let error = get_ipc_response(&webview, ipc_request("start_conversation", json!({})))
        .expect_err("Gemini Live without a Google key must still fail Google authentication");
    let error = error.as_str().unwrap();

    assert!(error.contains("authentication failed"));
    assert!(!error.contains("selected conversation model is unavailable"));
}
