//! ELSV-400: the backend event contract for non-Live Talk. The speech bubble and
//! transcript are rendered from these events by the conversation event loop and
//! the frontend (both need a live window), so this pins what the provider emits:
//! final transcript text, then the audio, then `TurnComplete`, per generation.
use crate::ai::local::LocalConversationProvider;
use crate::ai::traits::{RealtimeConversationProvider, SpeechSynthesizer, TextModel};
use crate::ai::types::{
    AudioStreamData, LiveServerEvent, LiveSessionConfig, ProviderError, TextRequest, TextResponse,
    TtsRequest,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify};

struct EchoModel;

#[async_trait]
impl TextModel for EchoModel {
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, ProviderError> {
        Ok(TextResponse {
            text: format!("Reply to {}.", request.prompt),
            finish_reason: None,
        })
    }
}

/// Returns PCM whose length encodes the utterance, so audio can be attributed to
/// a turn. The first utterance additionally waits on `gate` before returning.
struct MarkedPcmSynth {
    gate: Arc<Notify>,
    block_first: bool,
}

fn pcm_for(text: &str) -> Vec<u8> {
    // 24 kHz mono 16-bit: 2 s per "first" utterance, 1 s otherwise.
    let seconds = if text.contains("first") { 2 } else { 1 };
    vec![7u8; 24_000 * 2 * seconds]
}

#[async_trait]
impl SpeechSynthesizer for MarkedPcmSynth {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        if self.block_first && request.text.contains("first") {
            self.gate.notified().await;
        }
        Ok(AudioStreamData {
            pcm_bytes: pcm_for(&request.text),
            sample_rate: 24_000,
        })
    }
}

fn config() -> LiveSessionConfig {
    LiveSessionConfig {
        model: "m".into(),
        voice_name: None,
        system_instruction: None,
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: vec![],
    }
}

async fn session(
    block_first: bool,
) -> (
    Box<dyn crate::ai::traits::LiveSession>,
    mpsc::Receiver<LiveServerEvent>,
    Arc<Notify>,
) {
    let gate = Arc::new(Notify::new());
    let provider = LocalConversationProvider::new(
        Arc::new(EchoModel),
        Arc::new(MarkedPcmSynth {
            gate: gate.clone(),
            block_first,
        }),
        "voice".into(),
        1.0,
    );
    let (sender, mut events) = mpsc::channel(16);
    let session = provider.connect(config(), sender).await.unwrap();
    assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
    (session, events, gate)
}

async fn next(events: &mut mpsc::Receiver<LiveServerEvent>) -> LiveServerEvent {
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("event expected")
        .expect("channel open")
}

#[tokio::test]
async fn turn_emits_transcript_then_audio_then_turn_complete_without_waiting_for_playback() {
    let (mut session, mut events, _gate) = session(false).await;
    let started = Instant::now();
    session.send_text_turn("hi").await.unwrap();

    match next(&mut events).await {
        LiveServerEvent::ModelTranscript(update) => {
            assert_eq!(update.text, "Reply to hi.");
            assert!(update.is_final, "the bubble text is a single final transcript");
        }
        other => panic!("transcript must come first, got {other:?}"),
    }
    match next(&mut events).await {
        LiveServerEvent::AudioData(pcm) => {
            assert_eq!(pcm, pcm_for("Reply to hi."));
            // Known duration: 24_000 samples/s * 2 bytes/sample.
            assert_eq!(pcm.len() / 2, 24_000);
        }
        other => panic!("audio must follow the transcript, got {other:?}"),
    }
    assert!(matches!(next(&mut events).await, LiveServerEvent::TurnComplete));
    // TurnComplete marks synthesis delivery; it does not wait for playback duration.
    assert!(started.elapsed() < Duration::from_millis(900));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), events.recv())
            .await
            .is_err(),
        "nothing may follow TurnComplete"
    );
}

#[tokio::test]
async fn replaced_turn_never_emits_audio_or_completion_for_the_old_generation() {
    let (mut session, mut events, gate) = session(true).await;
    session.send_text_turn("first").await.unwrap();
    // The old turn publishes its transcript, then blocks inside synthesis.
    assert!(matches!(
        next(&mut events).await,
        LiveServerEvent::ModelTranscript(t) if t.text == "Reply to first."
    ));
    session.send_text_turn("second").await.unwrap();
    // Let the superseded synthesis return; its result must be discarded.
    gate.notify_waiters();

    let mut transcripts = vec![];
    let mut audio = vec![];
    let mut completions = 0;
    loop {
        match tokio::time::timeout(Duration::from_millis(500), events.recv()).await {
            Ok(Some(LiveServerEvent::ModelTranscript(t))) => transcripts.push(t.text),
            Ok(Some(LiveServerEvent::AudioData(pcm))) => audio.push(pcm),
            Ok(Some(LiveServerEvent::TurnComplete)) => completions += 1,
            Ok(Some(other)) => panic!("unexpected event {other:?}"),
            Ok(None) | Err(_) => break,
        }
    }
    assert_eq!(transcripts, vec!["Reply to second.".to_string()]);
    assert_eq!(audio, vec![pcm_for("Reply to second.")]);
    assert_eq!(completions, 1, "exactly one TurnComplete, for the new generation");
}

#[tokio::test]
async fn interrupted_turn_emits_interrupted_and_no_turn_complete() {
    let (mut session, mut events, gate) = session(true).await;
    session.send_text_turn("first").await.unwrap();
    assert!(matches!(next(&mut events).await, LiveServerEvent::ModelTranscript(_)));
    session.interrupt().await.unwrap();
    gate.notify_waiters();

    assert!(matches!(next(&mut events).await, LiveServerEvent::Interrupted));
    assert!(
        tokio::time::timeout(Duration::from_millis(300), events.recv())
            .await
            .is_err(),
        "an interrupted turn must publish neither audio nor TurnComplete"
    );
}
