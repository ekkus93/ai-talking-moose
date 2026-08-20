use super::*;

#[tokio::test]
async fn local_final_transcript_becomes_one_attributed_gemini_text_turn() {
    let manager = ConversationManager::new();
    let generation = 51;
    let session_id = "moonshine-session";
    manager.generation.store(generation, Ordering::SeqCst);
    manager.is_in_conversation.store(true, Ordering::SeqCst);
    manager.output_suppressed.store(true, Ordering::SeqCst);
    *manager.active_session_id.lock() = Some(session_id.to_string());
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    *manager.lifecycle.write() = ConversationLifecycle::Listening;
    let _stop_count = attach_counting_local_asr(&manager, generation).await;

    let audio_upload_count = Arc::new(AtomicUsize::new(0));
    let text_turns = Arc::new(SyncMutex::new(Vec::new()));
    *manager.live_session.lock().await = Some(Box::new(RecordingSession {
        audio_upload_count: audio_upload_count.clone(),
        text_turns: text_turns.clone(),
    }));

    let states = Arc::new(SyncMutex::new(Vec::<CharacterState>::new()));
    let states_for_callback = states.clone();
    *manager.state_callback.lock() = Some(Arc::new(move |state| {
        states_for_callback.lock().push(state);
    }));
    let lifecycle_events = Arc::new(SyncMutex::new(Vec::<ConversationLifecycle>::new()));
    let lifecycle_events_for_callback = lifecycle_events.clone();
    *manager.lifecycle_callback.lock() = Some(Arc::new(move |lifecycle| {
        lifecycle_events_for_callback.lock().push(lifecycle);
    }));
    let transcripts = Arc::new(SyncMutex::new(Vec::<(String, String, String)>::new()));
    let transcripts_for_callback = transcripts.clone();
    *manager.transcript_callback.lock() = Some(Arc::new(move |session, role, text| {
        transcripts_for_callback.lock().push((session, role, text));
    }));

    assert!(!manager
        .handle_local_asr_event(
            generation,
            session_id,
            AsrEvent::PartialTranscript {
                text: "hello moo".to_string(),
            },
        )
        .await
        .unwrap());
    assert!(manager
        .handle_local_asr_event(
            generation,
            session_id,
            AsrEvent::FinalTranscript {
                text: "  hello moose  ".to_string(),
            },
        )
        .await
        .unwrap());
    assert!(!manager
        .handle_local_asr_event(
            generation,
            session_id,
            AsrEvent::SpeechEnded { monotonic_ms: None },
        )
        .await
        .unwrap());

    assert_eq!(text_turns.lock().as_slice(), &["hello moose".to_string()]);
    assert_eq!(
        transcripts.lock().as_slice(),
        &[(
            session_id.to_string(),
            "user".to_string(),
            "hello moose".to_string(),
        )]
    );
    assert_eq!(audio_upload_count.load(AtomicOrdering::SeqCst), 0);
    assert!(!manager.output_suppressed.load(Ordering::SeqCst));
    assert_eq!(manager.lifecycle(), ConversationLifecycle::Responding);
    assert_eq!(
        lifecycle_events.lock().as_slice(),
        &[ConversationLifecycle::Responding]
    );
    assert_eq!(states.lock().as_slice(), &[CharacterState::Thinking]);
    assert!(!manager
        .should_suppress_interrupted_response_event(&LiveServerEvent::AudioData(vec![1, 2, 3, 4])));
}
