use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
#[ignore = "live Google API test; run explicitly with TALKING_MOOSE_GOOGLE_API_KEY"]
async fn test_gemini_live_asr() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let api_key = std::env::var("TALKING_MOOSE_GOOGLE_API_KEY")
        .expect("set TALKING_MOOSE_GOOGLE_API_KEY to run the live Gemini test");

    let url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}",
        api_key
    );

    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let setup_msg = json!({
        "setup": {
            "model": "models/gemini-2.5-flash-native-audio-latest",
            "generationConfig": {
                "responseModalities": ["AUDIO"]
            }
        }
    });

    write
        .send(Message::Text(setup_msg.to_string()))
        .await
        .unwrap();

    // Read setup complete
    while let Some(Ok(msg)) = read.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
            _ => "".to_string(),
        };
        println!("Setup: {}", text);
        if text.contains("setupComplete") {
            break;
        }
    }

    // Generate 1 second of a test spoken synthetic sine tone at 16kHz mono (16,000 samples)
    let sample_rate = 16000;
    for chunk_idx in 0..10 {
        let mut pcm = Vec::with_capacity(3200);
        for i in 0..1600 {
            let t = (chunk_idx * 1600 + i) as f32 / sample_rate as f32;
            let val = (t * 300.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
            let sample = (val * 32767.0) as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }

        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pcm);
        let chunk_msg = json!({
            "realtimeInput": {
                "mediaChunks": [
                    {
                        "mimeType": "audio/pcm;rate=16000",
                        "data": b64
                    }
                ]
            }
        });

        println!("Sending chunk {}", chunk_idx);
        write
            .send(Message::Text(chunk_msg.to_string()))
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("Finished sending chunks. Waiting for response...");
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(4));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(msg_res) = read.next() => {
                match msg_res {
                    Ok(Message::Text(t)) => println!("Server Text: {}", t),
                    Ok(Message::Binary(b)) => println!("Server Binary: {}", String::from_utf8_lossy(&b)),
                    Ok(other) => println!("Server Other: {:?}", other),
                    Err(e) => {
                        println!("Server Error: {}", e);
                        break;
                    }
                }
            }
            _ = &mut timeout => {
                println!("Timeout reached.");
                break;
            }
        }
    }
}
