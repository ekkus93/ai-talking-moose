pub mod ai;
pub mod app;
pub mod asr;
pub mod audio;
pub mod character;
pub mod commands;
pub mod conversation;
pub mod desktop;
pub mod memory;
pub mod persistence;
pub mod secrets;
pub mod tools;

use app::state::AppState;
use commands::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tracing::info;

pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .try_init();

    info!("Starting Talking Moose AI Application");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().ok();
            let db_path = if let Some(dir) = app_data_dir {
                std::fs::create_dir_all(&dir).ok();
                Some(dir.join("talking_moose.db").to_string_lossy().to_string())
            } else {
                None
            };

            let app_state = AppState::new(db_path.as_deref()).map_err(std::io::Error::other)?;

            let (mouth_tx, mut mouth_rx) = mpsc::channel(64);
            let (out_lvl_tx, mut out_lvl_rx) = mpsc::channel(64);

            app_state.audio_playback.set_mouth_sender(mouth_tx);
            app_state.audio_playback.set_output_level_sender(out_lvl_tx);

            let app_handle_mouth = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(mouth) = mouth_rx.recv().await {
                    let _ = app_handle_mouth.emit("moose://mouth", mouth);
                }
            });

            let app_handle_lvl = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(lvl) = out_lvl_rx.recv().await {
                    let _ = app_handle_lvl.emit("moose://audio/output-level", lvl);
                }
            });

            app.manage(app_state);
            info!("Talking Moose AI backend initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_google_models,
            update_settings,
            get_asr_models,
            get_asr_diagnostics,
            install_asr_model,
            delete_asr_model,
            set_google_api_key,
            clear_google_api_key,
            has_google_api_key,
            test_ai_connection,
            list_audio_devices,
            get_microphone_permission,
            request_microphone_access,
            get_audio_diagnostics,
            test_microphone,
            test_audio_output,
            get_character_state,
            get_conversation_lifecycle,
            set_character_state,
            show_moose,
            hide_moose,
            dismiss_moose,
            set_mute,
            is_muted,
            trigger_canned_reaction,
            trigger_ambient_remark,
            audition_voice,
            start_conversation,
            stop_conversation,
            barge_in,
            get_memories,
            delete_memory,
            forget_everything,
            get_transcripts,
            send_text_message,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    let shutdown_started = Arc::new(AtomicBool::new(false));
    app.run(move |app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if shutdown_started.swap(true, Ordering::SeqCst) {
                return;
            }

            // Keep the event loop alive long enough to close microphone/session resources.
            // AppHandle::exit below triggers a second ExitRequested event, which is allowed
            // through because shutdown_started is already true.
            api.prevent_exit();
            let handle = app_handle.clone();
            let exit_code = code.unwrap_or(0);
            tauri::async_runtime::spawn(async move {
                let resources = handle.try_state::<AppState>().map(|state| {
                    (
                        state.conversation_mgr.clone(),
                        state.audio_capture.clone(),
                        state.audio_playback.clone(),
                    )
                });

                if let Some((conversation_mgr, audio_capture, audio_playback)) = resources {
                    conversation_mgr
                        .shutdown_application(audio_capture, audio_playback)
                        .await;
                }

                handle.exit(exit_code);
            });
        }
    });
}
