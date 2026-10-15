//! ELSV-200/400: a saved OpenAI key + selected OpenAI provider reaches the
//! OpenAI endpoint (a loopback fake injected only under `cfg(test)`), and no
//! other selection ever contacts it.
use super::super::*;
use crate::ai::local::LocalConversationProvider;
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{
    AudioStreamData, LiveServerEvent, LiveSessionConfig, ProviderError, ProviderErrorKind,
    TextRequest, TtsRequest,
};
use async_trait::async_trait;
use std::sync::Mutex as StdMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const REPLY: &str = r#"{"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OpenAI says hi."}]}]}"#;

struct CaptureTts(Arc<StdMutex<Vec<TtsRequest>>>);

#[async_trait]
impl SpeechSynthesizer for CaptureTts {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
        self.0.lock().unwrap().push(request);
        Ok(AudioStreamData {
            pcm_bytes: vec![0, 0],
            sample_rate: 24_000,
        })
    }
}

fn request() -> TextRequest {
    TextRequest {
        prompt: "hello moose".into(),
        system_instruction: None,
        temperature: None,
        max_tokens: Some(16),
    }
}

async fn state_with_fake_openai() -> (AppState, TcpListener) {
    let state = AppState::new_for_tests().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    *state.openai_endpoint_override.lock() = Some(format!(
        "http://{}/v1/responses",
        listener.local_addr().unwrap()
    ));
    state.secrets.set_openai_api_key("saved-openai-key".into()).unwrap();
    (state, listener)
}

/// Serves one request and returns the full raw request text.
fn serve_once(listener: TcpListener) -> tokio::task::JoinHandle<String> {
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut raw = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            raw.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&raw).to_string();
            if let Some(split) = text.find("\r\n\r\n") {
                let length = text[..split]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if raw.len() >= split + 4 + length {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{REPLY}",
            REPLY.len()
        );
        stream.write_all(reply.as_bytes()).await.unwrap();
        let _ = stream.shutdown().await;
        String::from_utf8_lossy(&raw).to_string()
    })
}

#[tokio::test]
async fn saved_key_and_selected_openai_model_reach_the_openai_endpoint() {
    let (state, listener) = state_with_fake_openai().await;
    let settings = AppSettings {
        text_provider: TextProvider::Openai,
        openai_text_model: "gpt-4.1-mini".into(),
        ..AppSettings::default()
    };
    let server = serve_once(listener);
    let response = state
        .get_text_model_for(&settings)
        .generate(request())
        .await
        .unwrap();
    assert_eq!(response.text, "OpenAI says hi.");
    let received = server.await.unwrap();
    assert!(received.starts_with("POST /v1/responses HTTP/1.1"), "{received}");
    assert!(received.contains("Bearer saved-openai-key"));
    assert!(received.contains("\"gpt-4.1-mini\""));
    assert!(received.contains("hello moose"));
}

#[tokio::test]
async fn non_live_talk_speaks_the_selected_openai_reply() {
    let (state, listener) = state_with_fake_openai().await;
    let settings = AppSettings {
        text_provider: TextProvider::Openai,
        tts_provider: TtsProvider::Local,
        asr_mode: AsrMode::WhisperSmall,
        ..AppSettings::default()
    };
    let server = serve_once(listener);
    let spoken = Arc::new(StdMutex::new(Vec::new()));
    let conversation = LocalConversationProvider::new(
        Arc::from(state.get_text_model_for(&settings)),
        Arc::new(CaptureTts(spoken.clone())) as Arc<dyn SpeechSynthesizer>,
        settings.local_tts_voice.clone(),
        settings.speaking_rate,
    );
    let (sender, mut events) = tokio::sync::mpsc::channel(16);
    let mut session = conversation
        .connect(
            LiveSessionConfig {
                model: settings.openai_text_model.clone(),
                voice_name: None,
                system_instruction: None,
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
                tools: vec![],
            },
            sender,
        )
        .await
        .unwrap();
    assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
    session.send_text_turn("hello moose").await.unwrap();
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
            .await
            .expect("turn must complete")
        {
            Some(LiveServerEvent::TurnComplete) => break,
            Some(_) => {}
            None => panic!("conversation closed before the spoken reply completed"),
        }
    }
    let received = server.await.unwrap();
    assert!(received.contains("Bearer saved-openai-key"));
    let spoken = spoken.lock().unwrap();
    assert_eq!(spoken.len(), 1);
    assert_eq!(spoken[0].text, "OpenAI says hi.");
}

#[tokio::test]
async fn other_selections_never_contact_the_openai_endpoint() {
    for provider in [
        TextProvider::Google,
        TextProvider::Ollama,
        TextProvider::LlamaServer,
    ] {
        let (state, listener) = state_with_fake_openai().await;
        // A saved OpenAI key exists, and Ollama/llama-server point at a refused port.
        let refused = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}", l.local_addr().unwrap())
        };
        let settings = AppSettings {
            text_provider: provider,
            ollama_server_address: refused.clone(),
            llama_server_address: refused,
            ..AppSettings::default()
        };
        let error = state
            .get_text_model_for(&settings)
            .generate(request())
            .await
            .unwrap_err();
        assert!(
            matches!(error.kind, ProviderErrorKind::Auth | ProviderErrorKind::Network),
            "{provider:?}: {:?}",
            error.kind
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(300), listener.accept())
                .await
                .is_err(),
            "{provider:?} must not send its request to OpenAI"
        );
    }
}
