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
#[cfg(test)]
pub(crate) mod test_support;

use app::state::AppState;
use app::window_position::{
    clamp_window_position, load_window_position, persist_window_position,
    schedule_window_position_persist, DisplayBounds, WindowPosition,
};
use commands::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub fn moonshine_native_smoke_check() -> Result<i32, String> {
    asr::moonshine::native_runtime_smoke_check().map_err(|error| error.message)
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
            let app_data_dir = app.path().app_data_dir().ok();
            let db_path = if let Some(dir) = app_data_dir {
                std::fs::create_dir_all(&dir).ok();
                Some(dir.join("talking_moose.db").to_string_lossy().to_string())
            } else {
                None
            };

            let app_state = AppState::new(db_path.as_deref()).map_err(std::io::Error::other)?;
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
            desktop::runtime::start(app_state.settings.clone(), ambient_scheduler)
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

            let tray_visible = app_state.settings.read().show_in_menu_bar;
            app.manage(app_state);
            app::tray::install(app, tray_visible)?;
            info!("Talking Moose AI backend initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_onboarding_status,
            acknowledge_onboarding,
            get_google_models,
            get_google_tts_voices,
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
            get_live_outbound_diagnostics,
            set_character_state,
            show_moose,
            hide_moose,
            commands::character::dismiss_moose,
            commands::character::set_mute,
            is_muted,
            trigger_canned_reaction,
            commands::ambient::trigger_ambient_remark,
            audition_voice,
            commands::character::cancel_standalone_speech,
            commands::conversation::start_conversation,
            stop_conversation,
            commands::conversation::barge_in,
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
                desktop::runtime::stop().await;
                let ambient_scheduler = handle
                    .try_state::<AppState>()
                    .map(|state| state.ambient_scheduler.clone());
                if let Some(scheduler) = ambient_scheduler {
                    scheduler.stop().await;
                }

                if let (Some(state), Some(window)) =
                    (handle.try_state::<AppState>(), handle.get_webview_window("main"))
                {
                    if let Ok(position) = window.outer_position() {
                        if let Err(error) = persist_window_position(
                            state.db.as_ref(),
                            WindowPosition {
                                x: position.x,
                                y: position.y,
                            },
                        ) {
                            warn!(error = %error, "Failed to flush Moose window position during shutdown");
                        }
                    }
                }

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
