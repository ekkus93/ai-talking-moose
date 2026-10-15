use super::loopback::{LoopbackTextModel, LoopbackTextServer};
use super::traits::TextModel;
use super::types::TextRequest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn ipv6_loopback_generation_uses_the_exact_selected_endpoint() {
    for (kind, path, payload) in [
        (
            LoopbackTextServer::Ollama,
            "/api/chat",
            r#"{"done":true,"model":"selected-model","message":{"role":"assistant","content":"ipv6 reply"}}"#,
        ),
        (
            LoopbackTextServer::LlamaServer,
            "/v1/chat/completions",
            r#"{"choices":[{"message":{"role":"assistant","content":"ipv6 reply"},"finish_reason":"stop"}]}"#,
        ),
    ] {
        let Ok(listener) = TcpListener::bind("[::1]:0").await else {
            eprintln!("IPv6 loopback unavailable; skipping socket qualification");
            return;
        };
        let address = format!("http://[::1]:{}", listener.local_addr().unwrap().port());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = [0u8; 4096];
            let count = socket.read(&mut bytes).await.unwrap();
            assert!(String::from_utf8_lossy(&bytes[..count])
                .starts_with(&format!("POST {path} HTTP/1.1")));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(kind, address, "selected-model".into());
        let reply = model
            .generate(TextRequest {
                prompt: "test IPv6 generation".into(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(16),
            })
            .await
            .unwrap();
        assert_eq!(reply.text, "ipv6 reply");
        server.await.unwrap();
    }
}
