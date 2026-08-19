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
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tracing::info;

pub fn run() {
    // Install Rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize tracing subscriber
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .try_init();

    info!("Starting Talking Moose AI Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Locate app data directory for SQLite database
            let app_data_dir = app.path().app_data_dir().ok();
            let db_path = if let Some(dir) = app_data_dir {
                std::fs::create_dir_all(&dir).ok();
                Some(dir.join("talking_moose.db").to_string_lossy().to_string())
            } else {
                None
            };

            let app_state = AppState::new(db_path.as_deref()).map_err(std::io::Error::other)?;

            // Set up channels for mouth shapes and output audio levels from playback
            let (mouth_tx, mut mouth_rx) = mpsc::channel(64);
            let (out_lvl_tx, mut out_lvl_rx) = mpsc::channel(64);

            // AudioPlayback setup
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
            update_settings,
            set_google_api_key,
            clear_google_api_key,
            has_google_api_key,
            test_ai_connection,
            list_audio_devices,
            get_character_state,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
