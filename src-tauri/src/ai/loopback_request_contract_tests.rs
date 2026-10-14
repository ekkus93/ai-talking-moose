use super::loopback::{LoopbackTextModel, LoopbackTextServer};
use super::traits::TextModel;
use super::types::TextRequest;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn local_chat_adapters_send_exact_provider_specific_json() {
    for (kind, path) in [
        (LoopbackTextServer::Ollama, "/api/chat"),
        (LoopbackTextServer::LlamaServer, "/v1/chat/completions"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            let mut chunk = [0u8; 4096];
            let header_end = loop {
                let count = socket.read(&mut chunk).await.unwrap();
                assert!(count > 0, "headers were truncated");
                received.extend_from_slice(&chunk[..count]);
                if let Some(offset) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                    break offset;
                }
            };
            let headers = String::from_utf8_lossy(&received[..header_end]);
            assert!(headers.starts_with(&format!("POST {path} HTTP/1.1")));
            let body_size: usize = headers
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
                .and_then(|line| line.split_once(':'))
                .and_then(|(_, value)| value.trim().parse().ok())
                .expect("request has a content length");
            while received.len() < header_end + 4 + body_size {
                let count = socket.read(&mut chunk).await.unwrap();
                assert!(count > 0, "body was truncated");
                received.extend_from_slice(&chunk[..count]);
            }
            let body: Value =
                serde_json::from_slice(&received[header_end + 4..header_end + 4 + body_size])
                    .unwrap();
            assert_eq!(body["model"], "selected-model");
            assert_eq!(body["stream"], false);
            assert_eq!(body["messages"][0]["role"], "system");
            assert_eq!(body["messages"][0]["content"], "Keep replies short.");
            assert_eq!(body["messages"][1]["role"], "user");
            assert_eq!(body["messages"][1]["content"], "hello");
            assert_eq!(body["messages"].as_array().unwrap().len(), 2);
            match kind {
                LoopbackTextServer::Ollama => {
                    assert!((body["options"]["temperature"].as_f64().unwrap() - 0.4).abs() < 1e-6);
                    assert_eq!(body["options"]["num_predict"], 24);
                    assert!(body.get("max_tokens").is_none());
                }
                LoopbackTextServer::LlamaServer => {
                    assert!((body["temperature"].as_f64().unwrap() - 0.4).abs() < 1e-6);
                    assert_eq!(body["max_tokens"], 24);
                    assert!(body.get("options").is_none());
                }
            }
            let payload = match kind {
                LoopbackTextServer::Ollama => {
                    r#"{"done":true,"model":"selected-model","message":{"role":"assistant","content":"done"}}"#
                }
                LoopbackTextServer::LlamaServer => {
                    r#"{"choices":[{"message":{"role":"assistant","content":"done"},"finish_reason":"stop"}]}"#
                }
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(kind, base_url, "selected-model".into());
        let answer = model
            .generate(TextRequest {
                prompt: "hello".into(),
                system_instruction: Some("Keep replies short.".into()),
                temperature: Some(0.4),
                max_tokens: Some(24),
            })
            .await
            .unwrap();
        assert_eq!(answer.text, "done");
        server.await.unwrap();
    }
}

#[tokio::test]
async fn ollama_untagged_generation_accepts_latest_identity() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let count = socket.read(&mut buf).await.unwrap();
        assert!(String::from_utf8_lossy(&buf[..count]).starts_with("POST /api/chat HTTP/1.1"));
        let payload = r#"{"done":true,"model":"llama3.2:latest","message":{"role":"assistant","content":"alias works"}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    let model = LoopbackTextModel::new(LoopbackTextServer::Ollama, base_url, "llama3.2".into());
    let answer = model
        .generate(TextRequest {
            prompt: "say hello".into(),
            system_instruction: None,
            temperature: None,
            max_tokens: Some(32),
        })
        .await
        .unwrap();
    assert_eq!(answer.text, "alias works");
    server.await.unwrap();
}
