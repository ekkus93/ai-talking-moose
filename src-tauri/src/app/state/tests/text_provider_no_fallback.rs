//! ELSV-300: a failed Ollama / llama-server generation reached through the
//! request-snapshot path never changes the selected provider and never
//! substitutes another text provider.
use super::super::*;
use crate::ai::types::{ProviderErrorKind, TextRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn request() -> TextRequest {
    TextRequest {
        prompt: "private no-fallback prompt".into(),
        system_instruction: None,
        temperature: None,
        max_tokens: Some(8),
    }
}

#[tokio::test]
async fn failed_loopback_generation_keeps_provider_and_does_not_fall_back() {
    for provider in [TextProvider::Ollama, TextProvider::LlamaServer] {
        let state = AppState::new_for_tests().unwrap();
        // No Google or OpenAI credential exists. A silent fallback would
        // surface as an Auth error (or a cloud attempt), never as Network from
        // the one loopback server below.
        assert!(!state.secrets.has_google_api_key() && !state.secrets.has_openai_api_key());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        {
            let mut settings = state.settings.write();
            settings.text_provider = provider;
            settings.ollama_server_address = address.clone();
            settings.llama_server_address = address.clone();
            settings.ollama_text_model = "selected-model".into();
            settings.llama_server_text_model = "selected-model".into();
        }
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 8192];
            let size = stream.read(&mut buf).await.unwrap();
            assert!(String::from_utf8_lossy(&buf[..size]).starts_with("POST /"));
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            let _ = stream.shutdown().await;
            // Exactly one local attempt: no retry, no second provider on this socket.
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(200), listener.accept())
                    .await
                    .is_err()
            );
        });

        let snapshot = state.capture_text_request_settings();
        let model = state.get_text_model_for(&snapshot.settings);
        let error = model.generate(request()).await.unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Network);
        assert!(!error.message.contains("private no-fallback prompt"));
        server.await.unwrap();

        assert_eq!(state.settings.read().text_provider, provider);
        assert_eq!(state.settings_snapshot().text_provider, provider);
        assert_eq!(snapshot.settings.text_provider, provider);

        // A refused connection behaves the same way.
        let refused = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}", l.local_addr().unwrap())
        };
        {
            let mut settings = state.settings.write();
            settings.ollama_server_address = refused.clone();
            settings.llama_server_address = refused;
        }
        let snapshot = state.capture_text_request_settings();
        let error = state
            .get_text_model_for(&snapshot.settings)
            .generate(request())
            .await
            .unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Network);
        assert_eq!(state.settings.read().text_provider, provider);
    }
}
