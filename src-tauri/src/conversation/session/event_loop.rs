use super::{ConversationEventLoopContext, ConversationLifecycle, ConversationManager};
use crate::ai::types::{LiveServerEvent, ToolCallResponse};
use crate::character::state::CharacterState;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

impl ConversationManager {
    pub(super) async fn run_event_loop(
        &self,
        mut server_ev_rx: mpsc::Receiver<LiveServerEvent>,
        context: ConversationEventLoopContext,
    ) {
        let ConversationEventLoopContext {
            generation,
            session_id,
            capture,
            playback,
            output_sample_rate,
            tool_router,
            state_callback,
            lifecycle_callback,
            provider_error_callback,
            transcript_callback,
            speech_bubble_callback,
        } = context;
        info!(session_id = %session_id, "Conversation event loop started");
        let mut terminal_failed = false;

        while let Some(event) = server_ev_rx.recv().await {
            if !self.is_in_conversation.load(Ordering::SeqCst)
                || self.generation.load(Ordering::SeqCst) != generation
            {
                break;
            }

            if self.should_suppress_interrupted_response_event(&event) {
                continue;
            }

            match event {
                LiveServerEvent::Connected => {
                    info!(session_id = %session_id, "Conversation provider connected");
                }
                LiveServerEvent::UserTranscript(update) => {
                    if update.is_final {
                        transcript_callback(
                            session_id.clone(),
                            "user_partial".to_string(),
                            String::new(),
                        );
                        self.accept_user_transcript(
                            &session_id,
                            update.text,
                            &state_callback,
                            &lifecycle_callback,
                            &transcript_callback,
                        );
                    } else {
                        transcript_callback(
                            session_id.clone(),
                            "user_partial".to_string(),
                            update.text,
                        );
                    }
                }
                LiveServerEvent::ModelTranscript(update) => {
                    if update.is_final {
                        transcript_callback(
                            session_id.clone(),
                            "moose_partial".to_string(),
                            String::new(),
                        );
                        transcript_callback(
                            session_id.clone(),
                            "moose".to_string(),
                            update.text.clone(),
                        );
                    } else {
                        transcript_callback(
                            session_id.clone(),
                            "moose_partial".to_string(),
                            update.text.clone(),
                        );
                    }
                    speech_bubble_callback(update.text);
                }
                LiveServerEvent::AudioData(pcm_bytes) => {
                    Self::set_lifecycle(
                        &self.lifecycle,
                        ConversationLifecycle::Responding,
                        Some(&lifecycle_callback),
                    );
                    state_callback(CharacterState::Talking);
                    match playback.enqueue_pcm_bytes(&pcm_bytes, output_sample_rate) {
                        Ok(report) if report.dropped_samples > 0 => {
                            warn!(
                                dropped_samples = report.dropped_samples,
                                "Conversation playback queue overflowed"
                            );
                        }
                        Ok(_) => {}
                        Err(_error_value) => {
                            terminal_failed = true;
                            break;
                        }
                    }
                }
                LiveServerEvent::Interrupted => {
                    playback.flush();
                    transcript_callback(
                        session_id.clone(),
                        "moose_partial".to_string(),
                        String::new(),
                    );
                    speech_bubble_callback(String::new());
                    self.output_suppressed.store(false, Ordering::SeqCst);
                    state_callback(CharacterState::Interrupted);
                    Self::set_lifecycle(
                        &self.lifecycle,
                        ConversationLifecycle::Listening,
                        Some(&lifecycle_callback),
                    );
                    state_callback(CharacterState::Listening);
                }
                LiveServerEvent::TurnComplete => {
                    transcript_callback(
                        session_id.clone(),
                        "user_partial".to_string(),
                        String::new(),
                    );
                    transcript_callback(
                        session_id.clone(),
                        "moose_partial".to_string(),
                        String::new(),
                    );
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
                LiveServerEvent::ToolCall { id, name, args } => {
                    // Tool names are provider-controlled. The router records the registered
                    // name or a fixed `unregistered` label after local lookup, so do not
                    // echo an untrusted identifier into normal logs here.
                    info!("Handling model tool call");
                    let router = tool_router.clone();
                    let live_ref = self.live_session.clone();
                    let generation_ref = self.generation.clone();
                    let provider_error_for_tool = provider_error_callback.clone();

                    tauri::async_runtime::spawn(async move {
                        let result = router.dispatch(&name, &args).await.unwrap_or_else(
                            |error_value| serde_json::json!({ "error": error_value }),
                        );

                        if generation_ref.load(Ordering::SeqCst) != generation {
                            return;
                        }
                        let mut session_lock = live_ref.lock().await;
                        if let Some(ref mut live_session) = *session_lock {
                            if let Err(error_value) = live_session
                                .send_tool_response(ToolCallResponse {
                                    id,
                                    name,
                                    output: result,
                                })
                                .await
                            {
                                if generation_ref.load(Ordering::SeqCst) == generation {
                                    provider_error_for_tool(error_value);
                                }
                            }
                        }
                    });
                }
                LiveServerEvent::Error(error_value) => {
                    provider_error_callback(error_value);
                    terminal_failed = true;
                    break;
                }
                LiveServerEvent::Closed => break,
            }
        }

        let final_lifecycle = if terminal_failed {
            ConversationLifecycle::Failed
        } else {
            ConversationLifecycle::Idle
        };
        let cleaned = self
            .shutdown_if_generation_current(generation, capture, playback, final_lifecycle)
            .await;

        if cleaned {
            if terminal_failed {
                error!("Conversation session terminated with a provider/audio error");
                state_callback(CharacterState::Error);
            } else {
                state_callback(CharacterState::Idle);
            }
            info!(session_id = %session_id, "Conversation event loop exited");
        }
    }

    pub(super) fn should_suppress_interrupted_response_event(
        &self,
        event: &LiveServerEvent,
    ) -> bool {
        self.output_suppressed.load(Ordering::SeqCst)
            && matches!(
                event,
                LiveServerEvent::ModelTranscript(_)
                    | LiveServerEvent::AudioData(_)
                    | LiveServerEvent::TurnComplete
                    | LiveServerEvent::ToolCall { .. }
            )
    }
}

#[cfg(test)]
mod asr_handoff_tests;
