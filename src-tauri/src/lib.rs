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
#[cfg(test)]
pub(crate) mod test_support;
pub mod tools;
pub(crate) mod wake_word_policy;

use app::state::AppState;
use app::window_position::{
    clamp_window_position, load_window_position, persist_window_position,
    schedule_window_position_persist, DisplayBounds, WindowPosition,
};
use commands::*;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tracing::{info, warn};

const LOCAL_LLM_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const LOCAL_TTS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub fn moonshine_native_smoke_check() -> Result<i32, String> {
    asr::moonshine::native_runtime_smoke_check().map_err(|error| error.message)
}

fn persistent_database_path(app_data_dir: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to create application data directory {}: {error}",
                app_data_dir.display()
            ),
        )
    })?;
    Ok(app_data_dir.join("talking_moose.db"))
}

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
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let tauri::WindowEvent::Moved(position) = event else {
                return;
            };
            let Some(state) = window.app_handle().try_state::<AppState>() else {
                return;
            };

            schedule_window_position_persist(
                state.db.clone(),
                WindowPosition {
                    x: position.x,
                    y: position.y,
                },
            );
        })
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|error| {
                io::Error::other(format!(
                    "failed to resolve application data directory: {error}"
                ))
            })?;
            ai::local::initialize_global_local_model_installer(
                app_data_dir.join("models").join("llm"),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
            ai::local_tts::storage::initialize_global_local_tts_storage(
                app_data_dir.join("models").join("tts"),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
            let db_path = persistent_database_path(&app_data_dir)?;
            let db_path = db_path.to_str().ok_or_else(|| {
                io::Error::other("application database path is not valid UTF-8")
            })?;

            let app_state = AppState::new(Some(db_path)).map_err(io::Error::other)?;
            let startup_settings = app_state.settings.read().clone();
            if let Err(error) = app::runtime_preferences::apply_startup_runtime_preferences(
                app.handle(),
                &startup_settings,
            ) {
                warn!(error = %error, "Failed to apply persisted runtime preferences during startup");
            }
            let ambient_scheduler = app_state.ambient_scheduler.clone();
            let ambient_state = app_state.clone();
            let ambient_app = app.handle().clone();
            ambient_scheduler
                .start(move |event| {
                    let state = ambient_state.clone();
                    let app = ambient_app.clone();
                    async move {
                        commands::ambient::process_ambient_event(event, &state, &app).await
                    }
                })
                .map_err(std::io::Error::other)?;
            desktop::runtime::start(
                app_state.settings.clone(),
                ambient_scheduler,
                app_state.idle_banter_runtime.clone(),
            )
            .map_err(std::io::Error::other)?;

            if app_state.settings.read().restore_position {
                if let Some(window) = app.get_webview_window("main") {
                    match load_window_position(app_state.db.as_ref()) {
                        Ok(Some(saved)) => match (window.outer_size(), window.available_monitors()) {
                            (Ok(window_size), Ok(monitors)) => {
                                let displays = monitors
                                    .iter()
                                    .map(|monitor| {
                                        let work_area = monitor.work_area();
                                        DisplayBounds {
                                            x: work_area.position.x,
                                            y: work_area.position.y,
                                            width: work_area.size.width,
                                            height: work_area.size.height,
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                if let Some(restored) = clamp_window_position(
                                    saved,
                                    window_size.width,
                                    window_size.height,
                                    &displays,
                                ) {
                                    if let Err(error) = window.set_position(
                                        tauri::PhysicalPosition::new(restored.x, restored.y),
                                    ) {
                                        warn!(error = %error, "Failed to restore Moose window position");
                                    }
                                }
                            }
                            (Err(error), _) => {
                                warn!(error = %error, "Could not read Moose window size for restore");
                            }
                            (_, Err(error)) => {
                                warn!(error = %error, "Could not enumerate displays for Moose window restore");
                            }
                        },
                        Ok(None) => {}
                        Err(error) => {
                            warn!(error = %error, "Ignoring invalid stored Moose window position");
                        }
                    }
                }
            }

            let (mouth_tx, mut mouth_rx) = mpsc::channel(64);
            let (out_lvl_tx, mut out_lvl_rx) = mpsc::channel(64);

            app_state.audio_playback.set_mouth_sender(mouth_tx);
            app_state.audio_playback.set_output_level_sender(out_lvl_tx);

            let mouth_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(mouth) = mouth_rx.recv().await {
                    let _ = mouth_app.emit("mouth", mouth);
                }
            });

            let level_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(level) = out_lvl_rx.recv().await {
                    let _ = level_app.emit("output-level", level);
                }
            });

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            test_ai_connection,
            list_local_models,
            get_local_model_install_status,
            install_local_model,
            cancel_local_model_install,
            remove_local_model,
            test_local_model,
            get_local_tts_model_install_status,
            install_local_tts_model,
            cancel_local_tts_model_install,
            remove_local_tts_model,
            test_local_tts_model,
            speak_local_tts_sample,
            speak_google_tts_sample,
            speak_gemini_tts_sample,
            get_asr_model_install_status,
            install_asr_model,
            cancel_asr_model_install,
            remove_asr_model,
            test_asr_model,
            start_listening,
            stop_listening,
            cancel_interaction,
            get_app_state,
            get_conversation_history,
            clear_conversation_history,
            get_memory_status,
            list_memories,
            forget_memory,
            get_window_position,
            set_window_position,
            reset_window_position,
            get_desktop_status,
            get_ambient_status,
            trigger_ambient_now,
            get_ambient_history,
            clear_ambient_history,
            get_idle_banter_status,
            trigger_idle_banter_now,
            get_idle_banter_history,
            clear_idle_banter_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let app_handle = app.handle().clone();
    let shutdown_started = Arc::new(AtomicBool::new(false));
    let shutdown_started_for_events = shutdown_started.clone();
    app.run(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !shutdown_started_for_events.swap(true, Ordering::AcqRel) {
                api.prevent_exit();
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    shutdown_runtime(&app_handle).await;
                    app_handle.exit(0);
                });
            }
        }
    });
}

async fn shutdown_runtime(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.ambient_scheduler.shutdown().await;
        state.idle_banter_runtime.shutdown().await;
    }

    if let Err(error) = ai::local::shutdown_global_local_model_runtime(LOCAL_LLM_SHUTDOWN_TIMEOUT).await
    {
        warn!(error = %error, "Failed to shut down local LLM runtime cleanly");
    }
    if let Err(error) =
        ai::local_tts::shutdown_global_local_tts_runtime(LOCAL_TTS_SHUTDOWN_TIMEOUT).await
    {
        warn!(error = %error, "Failed to shut down local TTS runtime cleanly");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn persistent_database_path_creates_parent_and_returns_expected_file() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested").join("app-data");
        let db = persistent_database_path(&nested).unwrap();
        assert!(nested.is_dir());
        assert_eq!(db, nested.join("talking_moose.db"));
    }
}
