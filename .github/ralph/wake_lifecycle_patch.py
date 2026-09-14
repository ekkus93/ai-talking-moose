from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual < count:
        raise SystemExit(f"{path}: expected at least {count} match(es), found {actual}: {old[:100]!r}")
    p.write_text(text.replace(old, new, count))


replace(
    "src-tauri/src/audio/wake_word/runtime.rs",
    "    pub fn suspend_for_talking(&self) {\n",
    """    pub fn finish_command_handoff(&self) {
        let mut inner = self.inner.lock();
        if !inner.enabled {
            return;
        }
        inner.state = WakeWordRuntimeState::Suspended;
        inner.command_sink = None;
        inner.level_sink = None;
        inner.ring.clear();
        inner.suspended_for_talking = false;
    }

    pub fn suspend_for_talking(&self) {
""",
)

replace(
    "src-tauri/src/asr/pipeline.rs",
    "    pub fn diagnostics(&self) -> LocalAsrPipelineDiagnostics {\n",
    """    pub(super) fn enqueue_pre_roll(&self, bytes: Vec<u8>) -> Result<(), AsrError> {
        if !self.is_running() {
            return Err(invalid_state_error(
                "Local ASR inference is not running; wake-word pre-roll was not accepted.",
            ));
        }
        let sender = self.pcm_sender.as_ref().ok_or_else(|| {
            invalid_state_error("Local ASR input is closed; wake-word pre-roll was not accepted.")
        })?;
        sender.try_send(bytes).map_err(|_| {
            invalid_state_error("Local ASR queue could not accept wake-word pre-roll.")
        })
    }

    pub fn diagnostics(&self) -> LocalAsrPipelineDiagnostics {
""",
)

replace(
    "src-tauri/src/conversation/session.rs",
    "type InputLevelCallback = Arc<dyn Fn(f32) + Send + Sync>;\n",
    "type InputLevelCallback = Arc<dyn Fn(f32) + Send + Sync>;\ntype SessionEndCallback = Arc<dyn Fn() + Send + Sync>;\n",
)
replace(
    "src-tauri/src/conversation/session.rs",
    "    pub callbacks: ConversationCallbacks,\n}\n",
    """    pub callbacks: ConversationCallbacks,
    pub wake_handoff_rx: Option<mpsc::Receiver<Vec<u8>>>,
    pub one_shot: bool,
    pub session_end_callback: Option<SessionEndCallback>,
}
""",
)
replace(
    "src-tauri/src/conversation/session.rs",
    "    speech_bubble_callback: SpeechBubbleCallback,\n}\n",
    """    speech_bubble_callback: SpeechBubbleCallback,
    one_shot: bool,
    session_end_callback: Option<SessionEndCallback>,
}
""",
)
replace(
    "src-tauri/src/conversation/session.rs",
    """    async fn begin_shutdown_locked(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        final_lifecycle: ConversationLifecycle,
    ) -> Option<Box<dyn LiveSession>> {
""",
    """    async fn begin_shutdown_locked(
        &self,
        capture: Arc<SyncMutex<AudioCapture>>,
        playback: Arc<AudioPlayback>,
        final_lifecycle: ConversationLifecycle,
        stop_capture: bool,
    ) -> Option<Box<dyn LiveSession>> {
""",
)
replace(
    "src-tauri/src/conversation/session.rs",
    "        capture.lock().stop();\n        playback.flush();\n",
    """        if stop_capture {
            capture.lock().stop();
        }
        playback.flush();
""",
)

p = Path("src-tauri/src/conversation/session.rs")
text = p.read_text()
text = text.replace(
    "            .begin_shutdown_locked(capture, playback, final_lifecycle)\n",
    "            .begin_shutdown_locked(capture, playback, final_lifecycle, true)\n",
)
text = text.replace(
    "            .begin_shutdown_locked(capture, playback, ConversationLifecycle::Idle)\n",
    "            .begin_shutdown_locked(capture, playback, ConversationLifecycle::Idle, true)\n",
)
text = text.replace(
    "            .begin_shutdown_locked(capture, playback.clone(), ConversationLifecycle::Idle)\n",
    "            .begin_shutdown_locked(capture, playback.clone(), ConversationLifecycle::Idle, true)\n",
)
p.write_text(text)

replace(
    "src-tauri/src/conversation/session.rs",
    """            tool_router,
            callbacks,
        } = request;
""",
    """            tool_router,
            callbacks,
            mut wake_handoff_rx,
            one_shot,
            session_end_callback,
        } = request;
""",
)
replace(
    "src-tauri/src/conversation/session.rs",
    """        // Invalidate and detach any prior session while serialized, but close its provider handle
        // only after releasing the global operation lock. If Stop races that close it increments
        // generation, and this start notices the invalidation before doing any new provider I/O.
        let operation_guard = self.operation_lock.lock().await;
        let previous_session = self
            .begin_shutdown_locked(
                capture.clone(),
                playback.clone(),
                ConversationLifecycle::Idle,
            )
            .await;
""",
    """        // Wake-triggered startup keeps the existing wake capture alive while local ASR and
        // the provider initialize. Immediately before command capture starts we stop that one
        // authoritative stream, drain its bounded handoff queue, feed pre-roll, and reopen the
        // same AudioCapture for command ASR. Manual starts retain the existing teardown behavior.
        let preserve_wake_capture = wake_handoff_rx.is_some() && !self.is_active();
        let operation_guard = self.operation_lock.lock().await;
        let previous_session = self
            .begin_shutdown_locked(
                capture.clone(),
                playback.clone(),
                ConversationLifecycle::Idle,
                !preserve_wake_capture,
            )
            .await;
""",
)
replace(
    "src-tauri/src/conversation/session.rs",
    """        let (level_tx, mut level_rx) = mpsc::channel::<f32>(32);
        let mut cloud_pcm_rx = None;
        let capture_result = match asr_mode {
""",
    """        let (level_tx, mut level_rx) = mpsc::channel::<f32>(32);
        let mut cloud_pcm_rx = None;

        let wake_pre_roll = if let Some(receiver) = wake_handoff_rx.as_mut() {
            capture.lock().stop();
            let mut bytes = Vec::new();
            while let Ok(chunk) = receiver.try_recv() {
                bytes.extend_from_slice(&chunk);
            }
            bytes
        } else {
            Vec::new()
        };

        if !wake_pre_roll.is_empty() {
            match asr_mode {
                AsrMode::MoonshineTinyStreaming | AsrMode::MoonshineSmallStreaming => {
                    if let Err(error) = local_pipeline
                        .as_ref()
                        .expect("local Moonshine mode must have a provisional ASR pipeline")
                        .enqueue_pre_roll(wake_pre_roll.clone())
                    {
                        Self::stop_provisional_local_asr(&mut local_pipeline).await;
                        playback.flush();
                        Self::set_lifecycle(
                            &self.lifecycle,
                            ConversationLifecycle::Failed,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Error);
                        drop(operation_guard);
                        Self::close_provisional_session(&mut session).await;
                        return Err(error.message);
                    }
                }
                AsrMode::GeminiLiveAudio => {
                    if let Err(error_value) =
                        Self::bounded_provider_operation(session.send_audio_chunk(&wake_pre_roll)).await
                    {
                        Self::stop_provisional_local_asr(&mut local_pipeline).await;
                        playback.flush();
                        Self::set_lifecycle(
                            &self.lifecycle,
                            ConversationLifecycle::Failed,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Error);
                        drop(operation_guard);
                        Self::close_provisional_session(&mut session).await;
                        return Err(error_value.to_string());
                    }
                }
            }
        }

        let capture_result = match asr_mode {
""",
)
replace(
    "src-tauri/src/conversation/session.rs",
    """            speech_bubble_callback,
        };
""",
    """            speech_bubble_callback,
            one_shot,
            session_end_callback,
        };
""",
)

replace(
    "src-tauri/src/conversation/session/event_loop.rs",
    "use std::sync::atomic::Ordering;\n",
    "use std::sync::atomic::Ordering;\nuse std::time::{Duration, Instant};\n",
)
replace(
    "src-tauri/src/conversation/session/event_loop.rs",
    """            speech_bubble_callback,
        } = context;
""",
    """            speech_bubble_callback,
            one_shot,
            session_end_callback,
        } = context;
""",
)
replace(
    "src-tauri/src/conversation/session/event_loop.rs",
    """                LiveServerEvent::AudioData(pcm_bytes) => {
                    Self::set_lifecycle(
""",
    """                LiveServerEvent::AudioData(pcm_bytes) => {
                    if one_shot {
                        capture.lock().stop();
                    }
                    Self::set_lifecycle(
""",
)
replace(
    "src-tauri/src/conversation/session/event_loop.rs",
    """                    if self.is_in_conversation.load(Ordering::SeqCst)
                        && self.generation.load(Ordering::SeqCst) == generation
                    {
                        Self::set_lifecycle(
                            &self.lifecycle,
                            ConversationLifecycle::Listening,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Listening);
                    }
                }
""",
    """                    if one_shot {
                        let deadline = Instant::now() + Duration::from_secs(60);
                        while (playback.queue_length() > 0 || playback.is_playing())
                            && Instant::now() < deadline
                        {
                            tokio::time::sleep(Duration::from_millis(20)).await;
                        }
                        break;
                    }
                    if self.is_in_conversation.load(Ordering::SeqCst)
                        && self.generation.load(Ordering::SeqCst) == generation
                    {
                        Self::set_lifecycle(
                            &self.lifecycle,
                            ConversationLifecycle::Listening,
                            Some(&lifecycle_callback),
                        );
                        state_callback(CharacterState::Listening);
                    }
                }
""",
)
replace(
    "src-tauri/src/conversation/session/event_loop.rs",
    """            info!(session_id = %session_id, "Conversation event loop exited");
        }
""",
    """            info!(session_id = %session_id, "Conversation event loop exited");
            if let Some(callback) = session_end_callback {
                callback();
            }
        }
""",
)

core = Path("src-tauri/src/commands/conversation/core.rs")
text = core.read_text()
text = text.replace(
    'use tauri::{Emitter, Runtime, State};\n',
    'use std::sync::Arc;\nuse tauri::{Emitter, Runtime, State};\nuse tokio::sync::mpsc;\n',
)
start = text.index("#[tauri::command]\npub async fn start_conversation")
end = text.index("#[tauri::command]\npub async fn stop_conversation", start)
replacement = r'''async fn start_conversation_impl<R: Runtime>(
    state: &AppState,
    app: tauri::AppHandle<R>,
    wake_handoff_rx: Option<mpsc::Receiver<Vec<u8>>>,
    one_shot: bool,
    session_end_callback: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Result<String, String> {
    state.record_user_interaction();
    state.standalone_speech.cancel(state.audio_playback.as_ref());
    clear_speech_bubble(&app);
    if *state.is_muted.read() {
        return Err("Moose is currently muted".to_string());
    }

    if wake_handoff_rx.is_none() && state.wake_word_runtime.is_enabled() {
        state
            .wake_word_runtime
            .stop()
            .await
            .map_err(|error| error.message)?;
        state.audio_capture.lock().stop();
    }

    let _settings_guard = settings_runtime_lock().lock().await;
    let settings = state.settings.read().clone();
    state.ambient_scheduler.claim_foreground_presentation();
    prepare_character_for_conversation(state, &app)?;
    let provider = state.get_live_provider();
    let tool_router = state.tool_router.clone();

    let system_instruction = build_conversation_system_instruction(state, settings.memory_enabled);
    let config = LiveSessionConfig {
        model: settings.live_model.clone(),
        voice_name: Some(settings.live_voice.clone()),
        system_instruction: Some(system_instruction),
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: tool_router.get_declarations(),
    };

    let character_state = state.character_state.clone();
    let app_state = app.clone();
    let app_lifecycle = app.clone();
    let app_provider_error = app.clone();
    let app_transcript = app.clone();
    let app_bubble = app.clone();
    let app_level = app.clone();
    let db_ref = state.db.clone();
    let save_transcripts = settings.save_transcripts;

    let request = ConversationStartRequest {
        provider,
        config,
        asr_mode: settings.asr_mode,
        moonshine_installer: Some(state.moonshine_installer.clone()),
        capture: state.audio_capture.clone(),
        input_device: settings.input_device.clone(),
        playback: state.audio_playback.clone(),
        output_device: settings.output_device.clone(),
        muted: state.is_muted.clone(),
        tool_router,
        callbacks: ConversationCallbacks::new(
            move |new_state: CharacterState| {
                if let Err(error_value) = transition_character_state(&character_state, new_state) {
                    warn!(error = %error_value, ?new_state, "Rejected conversation character transition");
                    return;
                }
                let _ = app_state.emit("moose://state", new_state);
            },
            move |lifecycle: ConversationLifecycle| {
                let _ = app_lifecycle.emit("moose://conversation/lifecycle", lifecycle);
            },
            move |session_id: String, role: String, text: String| {
                let _ = app_transcript.emit(&format!("moose://transcript/{role}"), &text);
                if is_final_transcript_role(&role) {
                    if let Err(error_value) = persist_transcript_if_enabled(
                        db_ref.as_ref(),
                        save_transcripts,
                        &session_id,
                        &role,
                        &text,
                    ) {
                        warn!(error = %error_value, "Failed to persist retained transcript");
                    }
                }
            },
            move |speech_text: String| {
                let _ = app_bubble.emit("moose://speech-bubble", &speech_text);
            },
            move |level: f32| {
                let _ = app_level.emit("moose://audio/input-level", level);
            },
            move |provider_error| {
                let _ = app_provider_error.emit("moose://conversation/error", provider_error);
            },
        ),
        wake_handoff_rx,
        one_shot,
        session_end_callback,
    };

    let session_id = state.conversation_mgr.start_session(request).await?;
    info!(session_id = %session_id, one_shot, "Conversation session started");
    Ok(session_id)
}

#[tauri::command]
pub async fn start_conversation<R: Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let end_callback = if state.settings.read().wake_word_enabled {
        Some(crate::commands::wake_word::wake_session_end_callback(
            state.inner().clone(),
            app.clone(),
        ))
    } else {
        None
    };
    start_conversation_impl(state.inner(), app, None, false, end_callback).await
}

pub(crate) async fn start_wake_conversation<R: Runtime>(
    state: &AppState,
    app: tauri::AppHandle<R>,
    wake_handoff_rx: mpsc::Receiver<Vec<u8>>,
    session_end_callback: Arc<dyn Fn() + Send + Sync>,
) -> Result<String, String> {
    start_conversation_impl(
        state,
        app,
        Some(wake_handoff_rx),
        true,
        Some(session_end_callback),
    )
    .await
}

'''
core.write_text(text[:start] + replacement + text[end:])

replace(
    "src-tauri/src/commands/conversation/core.rs",
    """    state.ambient_scheduler.claim_foreground_presentation();
    transition_and_emit(&state.character_state, &app, CharacterState::Idle)
}
""",
    """    state.ambient_scheduler.claim_foreground_presentation();
    transition_and_emit(&state.character_state, &app, CharacterState::Idle)?;
    crate::commands::wake_word::reconcile_wake_runtime(state.inner(), app).await
}
""",
)

Path("src-tauri/src/commands/wake_word.rs").write_text(r'''use crate::app::state::AppState;
use crate::audio::wake_word::{WakeTriggerCallback, WakeWordDiagnostics, WakeWordRuntimeState};
use crate::character::state::CharacterState;
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
            if let Err(error) = reconcile_wake_runtime(&state, app).await {
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
    if state.conversation_mgr.is_active() || !state.settings.read().wake_word_enabled {
        return;
    }
    let (handoff_tx, handoff_rx) = mpsc::channel(WAKE_CAPTURE_QUEUE_CAPACITY);
    if let Err(error) = state.wake_word_runtime.begin_command_handoff(handoff_tx, None) {
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
            if let Err(resume_error) = reconcile_wake_runtime(&state, app).await {
                warn!(error = %resume_error, "Wake-word recovery failed after command start error");
            }
        }
    }
}

pub(crate) async fn reconcile_wake_runtime<R: Runtime>(
    state: &AppState,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
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
    if state.conversation_mgr.is_active() || *state.character_state.read() != CharacterState::Idle {
        return Ok(());
    }
    if state.wake_word_runtime.state() == WakeWordRuntimeState::Listening
        && state.audio_capture.lock().is_active()
    {
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
    if let Err(error) = state.audio_capture.lock().start(
        settings.input_device,
        16_000,
        pcm_tx,
        Some(level_tx),
    ) {
        let _ = state.wake_word_runtime.stop().await;
        return Err(format!("Failed to start wake-word microphone capture: {error}"));
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
}

#[tauri::command]
pub fn get_wake_word_diagnostics(state: State<'_, AppState>) -> Result<WakeWordDiagnostics, String> {
    Ok(state.wake_word_runtime.diagnostics())
}
''')

replace(
    "src-tauri/src/commands/mod.rs",
    "pub mod tool_diagnostics;\n",
    "pub mod tool_diagnostics;\npub mod wake_word;\n",
)
replace(
    "src-tauri/src/commands/mod.rs",
    "pub use tool_diagnostics::*;\n",
    "pub use tool_diagnostics::*;\npub use wake_word::*;\n",
)

replace(
    "src-tauri/src/commands/settings.rs",
    """    }

    Ok(())
}

#[tauri::command]
pub fn set_google_api_key""",
    """    }

    crate::commands::wake_word::reconcile_wake_runtime(state.inner(), app).await?;
    Ok(())
}

#[tauri::command]
pub fn set_google_api_key""",
)

replace(
    "src-tauri/src/lib.rs",
    "            app.manage(app_state);\n            app::tray::install(app, tray_visible)?;\n",
    """            app.manage(app_state.clone());
            if startup_settings.wake_word_enabled {
                let wake_state = app_state.clone();
                let wake_app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) =
                        commands::wake_word::reconcile_wake_runtime(&wake_state, wake_app).await
                    {
                        warn!(error = %error, "Persisted wake-word mode could not start");
                    }
                });
            }
            app::tray::install(app, tray_visible)?;
""",
)
replace(
    "src-tauri/src/lib.rs",
    "            get_audio_diagnostics,\n",
    "            get_audio_diagnostics,\n            get_wake_word_diagnostics,\n",
)
replace(
    "src-tauri/src/lib.rs",
    """                        state.local_tts_runtime.clone(),
                        state.conversation_mgr.clone(),
""",
    """                        state.local_tts_runtime.clone(),
                        state.wake_word_runtime.clone(),
                        state.conversation_mgr.clone(),
""",
)
replace(
    "src-tauri/src/lib.rs",
    """                    local_tts_runtime,
                    conversation_mgr,
                    audio_capture,
                    audio_playback,
                )) = resources
""",
    """                    local_tts_runtime,
                    wake_word_runtime,
                    conversation_mgr,
                    audio_capture,
                    audio_playback,
                )) = resources
""",
)
replace(
    "src-tauri/src/lib.rs",
    """                    conversation_mgr
                        .shutdown_application(audio_capture, audio_playback)
                        .await;
""",
    """                    if let Err(error) = wake_word_runtime.stop().await {
                        warn!(kind = ?error.kind, "Failed to stop wake-word runtime during shutdown");
                    }
                    conversation_mgr
                        .shutdown_application(audio_capture, audio_playback)
                        .await;
""",
)
