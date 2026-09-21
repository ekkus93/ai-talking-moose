use super::*;
use crate::app::wake_word::runtime::WakeWordRuntimePhase;
use serde_json::json;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;

fn ipc_request(command: &str) -> InvokeRequest {
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
        body: InvokeBody::Json(json!({})),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

#[test]
fn production_start_failure_resumes_enabled_wake_runtime() {
    let app_state = AppState::new_for_tests().unwrap();
    app_state.settings.write().wake_word_enabled = true;
    app_state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    app_state.wake_word_runtime.mark_loaded().unwrap();
    assert_eq!(
        app_state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );
    let wake_runtime = app_state.wake_word_runtime.clone();

    let app = mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![start_conversation])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    get_ipc_response(&webview, ipc_request("start_conversation"))
        .expect_err("missing Moonshine model should fail the normal production start path");

    assert_eq!(wake_runtime.phase(), WakeWordRuntimePhase::Listening);
}

#[test]
fn production_manual_start_failure_keeps_disabled_wake_runtime_disabled() {
    let app_state = AppState::new_for_tests().unwrap();
    assert!(!app_state.settings.read().wake_word_enabled);
    assert_eq!(
        app_state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
    let wake_runtime = app_state.wake_word_runtime.clone();

    let app = mock_builder()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![start_conversation])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    get_ipc_response(&webview, ipc_request("start_conversation"))
        .expect_err("missing Moonshine model should fail the normal manual start path");

    assert_eq!(wake_runtime.phase(), WakeWordRuntimePhase::Disabled);
}
