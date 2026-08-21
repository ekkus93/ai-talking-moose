use talking_moose_lib::ai::google::{GoogleAuth, GoogleLiveProvider, DEFAULT_LIVE_MODEL};
use talking_moose_lib::ai::{LiveServerEvent, LiveSessionConfig, RealtimeConversationProvider};

fn live_api_enabled(value: Option<&str>) -> bool {
    matches!(value, Some("1") | Some("true") | Some("TRUE"))
}

#[test]
fn live_api_guard_defaults_off() {
    assert!(!live_api_enabled(None));
    assert!(!live_api_enabled(Some("0")));
    assert!(live_api_enabled(Some("1")));
}

#[tokio::test]
#[ignore = "live Google API test; requires explicit opt-in and dedicated API key"]
async fn test_gemini_live_asr() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    assert!(
        live_api_enabled(
            std::env::var("TALKING_MOOSE_ALLOW_LIVE_API")
                .ok()
                .as_deref()
        ),
        "set TALKING_MOOSE_ALLOW_LIVE_API=1 to opt in to the live Gemini test"
    );
    let api_key = std::env::var("TALKING_MOOSE_GOOGLE_API_KEY")
        .expect("set TALKING_MOOSE_GOOGLE_API_KEY to run the live Gemini test");

    let provider = GoogleLiveProvider::new(GoogleAuth::new(api_key));
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(64);
    let mut session = provider
        .connect(
            LiveSessionConfig {
                model: DEFAULT_LIVE_MODEL.to_string(),
                voice_name: Some("Puck".to_string()),
                system_instruction: Some(
                    "This is an opt-in connectivity test. Respond briefly to audible speech."
                        .to_string(),
                ),
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
                tools: vec![],
            },
            event_tx,
        )
        .await
        .expect("Gemini Live setup handshake must complete");

    // The production provider does not return from connect until setupComplete has
    // been received. Confirm that the corresponding provider-neutral event arrives.
    let connected = tokio::time::timeout(tokio::time::Duration::from_secs(2), event_rx.recv())
        .await
        .expect("Connected event timed out")
        .expect("event channel closed unexpectedly");
    assert!(matches!(connected, LiveServerEvent::Connected));

    // Send one second of deterministic synthetic PCM through the same production
    // audio path used by microphone capture. The tone need not transcribe; this test
    // verifies the live protocol path without logging private provider frames.
    let sample_rate = 16_000_u32;
    for chunk_index in 0..10_u32 {
        let mut pcm = Vec::with_capacity(3_200);
        for sample_index in 0..1_600_u32 {
            let frame = chunk_index * 1_600 + sample_index;
            let seconds = frame as f32 / sample_rate as f32;
            let value = (seconds * 300.0 * std::f32::consts::TAU).sin() * 0.5;
            let sample = (value * f32::from(i16::MAX)) as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        session
            .send_audio_chunk(&pcm)
            .await
            .expect("synthetic PCM send failed");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Give Google a short opportunity to reject the stream at protocol level. We
    // intentionally do not require a transcript from a synthetic tone.
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, event_rx.recv()).await {
            Ok(Some(LiveServerEvent::Error(error))) => {
                panic!("Gemini Live returned a structured error: {}", error.message);
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }

    session.close().await.expect("Gemini Live close failed");
}
