use super::*;
use crate::asr::lifecycle::LocalAsrResource;
use crate::asr::pipeline::LocalAsrEventDeliveryTracker;
use crate::conversation::session::local_asr::local_asr_event_callback;

struct ProviderHandoffGate {
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

struct GatedRecordingSession {
    text_turns: Arc<SyncMutex<Vec<String>>>,
    gate: Arc<ProviderHandoffGate>,
}

#[async_trait]
impl LiveSession for GatedRecordingSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        self.text_turns.lock().push(text.to_string());
        self.gate.started.notify_one();
        self.gate.release.notified().await;
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }
}

struct FinalOnStopDispatchResource {
    callback: crate::asr::pipeline::LocalAsrPipelineEventCallback,
    delivery: Arc<LocalAsrEventDeliveryTracker>,
}

#[async_trait]
impl LocalAsrResource for FinalOnStopDispatchResource {
    async fn stop(&mut self) -> Result<(), crate::asr::AsrError> {
        let callback = self.callback.clone();
        let delivery = self.delivery.clone();
        tokio::task::spawn_blocking(move || {
            callback(
                AsrEvent::FinalTranscript {
                    text: "scheduled stop final".to_string(),
                },
                delivery.begin(),
            );
        })
        .await
        .map_err(|error| crate::asr::AsrError {
            kind: crate::asr::AsrErrorKind::Internal,
            message: error.to_string(),
            retryable: false,
        })?;
        self.delivery.wait_until_idle().await;
        Ok(())
    }
}

#[tokio::test]
async fn local_final_does_not_wait_for_global_operation_lock() {
    let manager = ConversationManager::new();
    let generation = 55;
    let session_id = "serialized-moonshine-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, generation).await;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count,
        text_turns: text_turns.clone(),
    }));
    *manager.state_callback.lock() = Some(Arc::new(|_| {}));
    *manager.lifecycle_callback.lock() = Some(Arc::new(|_| {}));
    *manager.transcript_callback.lock() = Some(Arc::new(|_, _, _| {}));

    let operation_guard = manager.operation_lock.lock().await;
    let handled = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        manager.handle_local_asr_event(
            generation,
            session_id,
            AsrEvent::FinalTranscript {
                text: "serialized final".to_string(),
            },
        ),
    )
    .await
    .expect("local final must not wait for the global conversation operation lock")
    .unwrap();

    assert!(handled);
    assert_eq!(
        text_turns.lock().as_slice(),
        &["serialized final".to_string()]
    );
    drop(operation_guard);
}

#[tokio::test]
async fn stop_time_final_is_committed_once_before_session_teardown() {
    let manager = Arc::new(ConversationManager::new());
    let generation = 56;
    let session_id = "stop-time-whisper-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::WhisperSmall);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count,
        text_turns: text_turns.clone(),
    }));
    *manager.state_callback.lock() = Some(Arc::new(|_| {}));
    *manager.lifecycle_callback.lock() = Some(Arc::new(|_| {}));
    *manager.transcript_callback.lock() = Some(Arc::new(|_, _, _| {}));
    manager
        .local_asr_lifecycle()
        .attach(
            generation,
            Box::new(FinalOnStopLocalAsrResource {
                manager: manager.clone(),
                generation,
                session_id: session_id.to_string(),
            }),
        )
        .await
        .unwrap();

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    manager
        .stop_session(capture.clone(), playback.clone())
        .await;
    manager.stop_session(capture, playback).await;

    assert_eq!(text_turns.lock().as_slice(), &["stop-time final"]);
    assert!(manager.generation.load(Ordering::SeqCst) > generation);
    assert!(!manager.is_active());
}

#[tokio::test]
async fn normal_stop_waits_for_the_production_async_transcript_handoff() {
    let manager = Arc::new(ConversationManager::new());
    let generation = 57;
    let session_id = "acknowledged-stop-final-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::WhisperSmall);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;

    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    let gate = Arc::new(ProviderHandoffGate {
        started: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    *manager.live_session.lock().await = Some(Box::new(GatedRecordingSession {
        text_turns: text_turns.clone(),
        gate: gate.clone(),
    }));
    *manager.state_callback.lock() = Some(Arc::new(|_| {}));
    *manager.lifecycle_callback.lock() = Some(Arc::new(|_| {}));
    *manager.transcript_callback.lock() = Some(Arc::new(|_, _, _| {}));

    let capture = Arc::new(SyncMutex::new(AudioCapture::new_mock()));
    let playback = Arc::new(AudioPlayback::new());
    let delivery = Arc::new(LocalAsrEventDeliveryTracker::default());
    let callback = local_asr_event_callback(
        (*manager).clone(),
        generation,
        session_id.to_string(),
        capture.clone(),
        playback.clone(),
        Arc::new(|_| {}),
        Arc::new(|_| {}),
    );
    manager
        .local_asr_lifecycle()
        .attach(
            generation,
            Box::new(FinalOnStopDispatchResource { callback, delivery }),
        )
        .await
        .unwrap();

    let stop_manager = manager.clone();
    let stop = tokio::spawn(async move { stop_manager.stop_session(capture, playback).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), gate.started.notified())
        .await
        .expect("the scheduled final should reach the provider handoff");
    assert_eq!(text_turns.lock().as_slice(), &["scheduled stop final"]);
    assert!(
        !stop.is_finished(),
        "normal Stop must keep the generation and session alive until the final handoff completes"
    );

    gate.release.notify_one();
    stop.await.unwrap();
    assert_eq!(text_turns.lock().as_slice(), &["scheduled stop final"]);
    assert!(!manager.is_active());
}
