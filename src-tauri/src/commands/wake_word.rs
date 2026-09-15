use crate::app::state::AppState;
use crate::audio::wake_word::{WakeTriggerCallback, WakeWordDiagnostics, WakeWordRuntimeState};
use crate::character::state::CharacterState;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{Emitter, Manager, Runtime, State};
use tokio::sync::mpsc;
use tracing::warn;

const WAKE_CAPTURE_QUEUE_CAPACITY: usize = 32;

pub(crate) fn wake_session_end_callback<R: Runtime>(
    state: AppState,
    app: tauri::AppHandle<R>,
) -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(move || {
        let state = state.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = reconcile_wake_runtime(state, app).await {
                warn!(error = %error, "Failed to resume wake-word listening after session end");
            }
        });
    })
}

fn wake_model_root<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<std::path::PathBuf, String> {
    app.path()
        .resource_dir()
        .map(|root| root.join("resources").join("wake_word").join("model"))
        .map_err(|_| "Wake-word resource directory is unavailable.".to_string())
}

async fn handle_wake_trigger<R: Runtime>(state: AppState, app: tauri::AppHandle<R>) {
    let wake_enabled = { state.settings.read().wake_word_enabled };
    if state.conversation_mgr.is_active() || !wake_enabled {
        return;
    }
    let (handoff_tx, handoff_rx) = mpsc::channel(WAKE_CAPTURE_QUEUE_CAPACITY);
    if let Err(error) = state
        .wake_word_runtime
        .begin_command_handoff(handoff_tx, None)
    {
        warn!(kind = ?error.kind, "Wake trigger could not begin command handoff");
        return;
    }
    let end_callback = wake_session_end_callback(state.clone(), app.clone());
    match crate::commands::conversation::start_wake_conversation(
        &state,
        app.clone(),
        handoff_rx,
        end_callback,
    )
    .await
    {
        Ok(_) => state.wake_word_runtime.finish_command_handoff(),
        Err(error) => {
            warn!(error = %error, "Wake-triggered conversation failed to start");
            let _ = state.wake_word_runtime.stop().await;
            state.audio_capture.lock().stop();
            if let Err(resume_error) = reconcile_wake_runtime(state, app).await {
                warn!(error = %resume_error, "Wake-word recovery failed after command start error");
            }
        }
    }
}

pub(crate) fn reconcile_wake_runtime<R: Runtime>(
    state: AppState,
    app: tauri::AppHandle<R>,
) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'static>> {
    Box::pin(async move {
        let settings = state.settings.read().clone();
        if !settings.wake_word_enabled {
            state
                .wake_word_runtime
                .stop()
                .await
                .map_err(|error| error.message)?;
            if !state.conversation_mgr.is_active() {
                state.audio_capture.lock().stop();
            }
            return Ok(());
        }
        let idle = { *state.character_state.read() == CharacterState::Idle };
        if state.conversation_mgr.is_active() || !idle {
            return Ok(());
        }
        let capture_active = { state.audio_capture.lock().is_active() };
        if state.wake_word_runtime.state() == WakeWordRuntimeState::Listening && capture_active {
            return Ok(());
        }

        if state.wake_word_runtime.is_enabled() {
            state
                .wake_word_runtime
                .stop()
                .await
                .map_err(|error| error.message)?;
            state.audio_capture.lock().stop();
        }

        let trigger_state = state.clone();
        let trigger_app = app.clone();
        let callback: WakeTriggerCallback = Arc::new(move || {
            let state = trigger_state.clone();
            let app = trigger_app.clone();
            tauri::async_runtime::spawn(async move {
                handle_wake_trigger(state, app).await;
            });
        });
        state
            .wake_word_runtime
            .start(wake_model_root(&app)?, callback)
            .await
            .map_err(|error| error.message)?;

        let (pcm_tx, mut pcm_rx) = mpsc::channel(WAKE_CAPTURE_QUEUE_CAPACITY);
        let (level_tx, mut level_rx) = mpsc::channel(WAKE_CAPTURE_QUEUE_CAPACITY);
        let capture_result = {
            state
                .audio_capture
                .lock()
                .start(settings.input_device, 16_000, pcm_tx, Some(level_tx))
        };
        if let Err(error) = capture_result {
            let _ = state.wake_word_runtime.stop().await;
            return Err(format!(
                "Failed to start wake-word microphone capture: {error}"
            ));
        }

        let runtime = state.wake_word_runtime.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(chunk) = pcm_rx.recv().await {
                if let Err(error) = runtime.ingest_pcm_bytes(chunk) {
                    warn!(kind = ?error.kind, "Wake-word audio routing failed");
                    break;
                }
            }
        });

        let app_for_level = app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(level) = level_rx.recv().await {
                let _ = app_for_level.emit("moose://audio/input-level", level);
            }
        });
        Ok(())
    })
}

#[tauri::command]
pub fn get_wake_word_diagnostics(
    state: State<'_, AppState>,
) -> Result<WakeWordDiagnostics, String> {
    Ok(state.wake_word_runtime.diagnostics())
}
