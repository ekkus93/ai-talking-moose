use talking_moose_lib::ai::fake::FakeConversationProvider;
use talking_moose_lib::ai::traits::RealtimeConversationProvider;
use talking_moose_lib::ai::types::*;
use talking_moose_lib::app::state::AppState;
use talking_moose_lib::character::cooldown::CooldownTracker;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_app_state_and_database_persistence() {
    let state = AppState::new(None).unwrap();

    // Test settings persistence
    let mut settings = state.settings.read().clone();
    settings.talkativeness = 0.95;
    *state.settings.write() = settings;

    // Test memory addition
    let mem_id = state
        .memory
        .remember("User loves Rust", Some("general"))
        .unwrap();
    let memories = state.memory.get_all_memories().unwrap();
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].id, mem_id);

    // Test Forget Everything
    state.memory.forget_everything().unwrap();
    assert_eq!(state.memory.get_all_memories().unwrap().len(), 0);
}

#[tokio::test]
async fn test_fake_live_conversation_flow() {
    let provider = FakeConversationProvider;
    let (tx, mut rx) = mpsc::channel(32);

    let mut session = provider
        .connect(
            LiveSessionConfig {
                model: "fake-live".to_string(),
                voice_name: None,
                system_instruction: Some("Be a moose".to_string()),
                sample_rate_in: 16000,
                sample_rate_out: 24000,
                tools: vec![],
            },
            tx,
        )
        .await
        .unwrap();

    // Verify initial connection
    let event1 = rx.recv().await.unwrap();
    match event1 {
        LiveServerEvent::Connected => {}
        _ => panic!("Expected Connected event"),
    }

    // Send audio chunk
    session.send_audio_chunk(&vec![0u8; 320]).await.unwrap();

    // Verify user transcript event
    let event2 = rx.recv().await.unwrap();
    match event2 {
        LiveServerEvent::UserTranscript(t) => assert!(t.text.contains("Hello")),
        _ => panic!("Expected UserTranscript event"),
    }

    // Verify model transcript event
    let event3 = rx.recv().await.unwrap();
    match event3 {
        LiveServerEvent::ModelTranscript(t) => assert!(t.text.contains("Greetings")),
        _ => panic!("Expected ModelTranscript event"),
    }

    // Interruption
    session.interrupt().await.unwrap();
}

#[test]
fn test_cooldown_and_annoyance_stress() {
    let mut tracker = CooldownTracker::new();
    let now = chrono::Utc::now();

    // 4 ambient speeches in an hour
    for _ in 0..4 {
        tracker.record_speech(now);
    }

    // Should be blocked by hourly limit (max 4)
    assert!(!tracker.can_speak_ambient(now, 0, 4, false, 22, 8));

    // Fast forward 2 hours
    let later = now + chrono::Duration::hours(2);
    assert!(tracker.can_speak_ambient(later, 0, 4, false, 22, 8));
}
