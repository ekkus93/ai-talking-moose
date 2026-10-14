use super::*;

#[test]
fn response_parsers_are_provider_specific_and_fail_closed() {
    let ollama = LoopbackTextModel::new(
        LoopbackTextServer::Ollama,
        "http://127.0.0.1:11434".into(),
        "local".into(),
    );
    let llama = LoopbackTextModel::new(
        LoopbackTextServer::LlamaServer,
        "http://127.0.0.1:8080".into(),
        "local".into(),
    );
    let native = json!({"done": true, "model": "local", "message": {"role": "assistant", "content": "native reply"}});
    let compatible = json!({"choices":[{"message":{"role":"assistant","content":"compatible reply"},"finish_reason":"stop"}]});
    assert_eq!(ollama.text_from_json(&native).unwrap().text, "native reply");
    assert!(llama.text_from_json(&native).is_err());
    assert_eq!(
        llama.text_from_json(&compatible).unwrap().text,
        "compatible reply"
    );
    assert!(ollama.text_from_json(&compatible).is_err());
    // Some adapters emit an empty tool_calls list on plain text replies.
    assert!(ollama
        .text_from_json(&json!({
            "done": true,
            "message": {"role": "assistant", "content": "text", "tool_calls": []}
        }))
        .is_ok());
    assert!(llama
        .text_from_json(&json!({
            "choices": [{
                "message": {"role": "assistant", "content": "text", "tool_calls": []},
                "finish_reason": "stop"
            }]
        }))
        .is_ok());
    for malformed in [
        json!({"choices": []}),
        json!({"choices": [
            {"message": {"role": "assistant", "content": "first"}, "finish_reason": "stop"},
            {"message": {"role": "assistant", "content": "second"}, "finish_reason": "stop"}
        ]}),
        json!({"choices": [{
            "message": {"role": "assistant", "content": "tool output", "tool_calls": [{"id": "unrequested"}]},
            "finish_reason": "stop"
        }]}),
    ] {
        assert_eq!(
            llama.text_from_json(&malformed).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
    }
    assert_eq!(
        ollama
            .text_from_json(&json!({
                "done": true,
                "message": {
                    "role": "assistant",
                    "content": "tool output",
                    "tool_calls": [{"function": {"name": "unrequested"}}]
                }
            }))
            .unwrap_err()
            .kind,
        ProviderErrorKind::Protocol
    );
    assert!(llama
        .text_from_json(&json!({"choices":[{"message":{"role":"user","content":"echoed prompt"},"finish_reason":"stop"}]}))
        .is_err());
    assert!(llama
        .text_from_json(
            &json!({"choices":[{"message":{"content":"missing role"},"finish_reason":"stop"}]})
        )
        .is_err());
    assert!(ollama
        .text_from_json(
            &json!({"done":false,"message":{"role":"assistant","content":"partial"}})
        )
        .is_err());
    for bad in [
        json!({"done":true,"message":{"role":"user","content":"echoed prompt"}}),
        json!({"done":true,"message":{"content":"missing role"}}),
        json!({"done":true,"model":"other-model","message":{"role":"assistant","content":"wrong model"}}),
    ] {
        assert_eq!(
            ollama.text_from_json(&bad).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
    }
    assert_eq!(
        llama.text_from_json(&json!({"model":"other-model","choices":[{"message":{"role":"assistant","content":"wrong model"},"finish_reason":"stop"}]}))
            .unwrap_err().kind,
        ProviderErrorKind::Protocol
    );
}

#[test]
fn token_budget_truncation_is_not_spoken_as_a_complete_reply() {
    let ollama = LoopbackTextModel::new(
        LoopbackTextServer::Ollama,
        "http://127.0.0.1:11434".into(),
        "local".into(),
    );
    let llama = LoopbackTextModel::new(
        LoopbackTextServer::LlamaServer,
        "http://127.0.0.1:8080".into(),
        "local".into(),
    );
    for reason in ["length", "unload", "error"] {
        let response = json!({
            "done": true,
            "done_reason": reason,
            "model": "local",
            "message": {"role": "assistant", "content": "incomplete sentence"}
        });
        assert_eq!(
            ollama.text_from_json(&response).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
    }
    for reason in ["length", "tool_calls", "content_filter"] {
        let response = json!({
            "choices": [{
                "message": {"role": "assistant", "content": "incomplete sentence"},
                "finish_reason": reason
            }]
        });
        assert_eq!(
            llama.text_from_json(&response).unwrap_err().kind,
            ProviderErrorKind::Protocol
        );
    }
    assert_eq!(
        llama
            .text_from_json(&json!({
                "choices": [{
                    "message": {"role": "assistant", "content": "unfinished"},
                    "finish_reason": null
                }]
            }))
            .unwrap_err()
            .kind,
        ProviderErrorKind::Protocol
    );
    assert!(ollama
        .text_from_json(&json!({
            "done": true,
            "done_reason": "stop",
            "message": {"role": "assistant", "content": "complete"}
        }))
        .is_ok());
}

#[tokio::test]
async fn remote_and_redirect_target_origins_are_rejected() {
    let remote = LoopbackTextModel::new(
        LoopbackTextServer::Ollama,
        "https://example.com:11434".into(),
        "local".into(),
    );
    let err = remote
        .generate(TextRequest {
            prompt: "hello".into(),
            system_instruction: None,
            temperature: None,
            max_tokens: None,
        })
        .await
        .unwrap_err();
    assert_eq!(err.kind, ProviderErrorKind::Setup);
}
#[tokio::test]
async fn invalid_generation_parameters_fail_before_network() {
    let model = LoopbackTextModel::new(
        LoopbackTextServer::Ollama,
        "http://127.0.0.1:1".into(),
        "selected-model".into(),
    );
    for temperature in [f32::NAN, f32::INFINITY, -0.1, 2.1] {
        let result = model
            .generate(TextRequest {
                prompt: "hello".into(),
                system_instruction: None,
                temperature: Some(temperature),
                max_tokens: Some(32),
            })
            .await;
        assert_eq!(result.unwrap_err().kind, ProviderErrorKind::Setup);
    }
    for max_tokens in [0, 4097, u32::MAX] {
        let result = model
            .generate(TextRequest {
                prompt: "hello".into(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(max_tokens),
            })
            .await;
        assert_eq!(result.unwrap_err().kind, ProviderErrorKind::Setup);
    }
    let result = model
        .generate(TextRequest {
            prompt: " ".into(),
            system_instruction: None,
            temperature: None,
            max_tokens: Some(32),
        })
        .await;
    assert_eq!(result.unwrap_err().kind, ProviderErrorKind::Setup);
}

#[tokio::test]
async fn fake_servers_use_provider_specific_paths_and_response_shapes() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    for (kind, path, payload) in [
        (
            LoopbackTextServer::Ollama,
            "/api/chat",
            r#"{"done":true,"model":"selected-model","message":{"role":"assistant","content":"ollama response"}}"#,
        ),
        (
            LoopbackTextServer::LlamaServer,
            "/v1/chat/completions",
            r#"{"choices":[{"message":{"role":"assistant","content":"llama response"},"finish_reason":"stop"}]}"#,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let bytes = stream.read(&mut request).await.unwrap();
            let received = String::from_utf8_lossy(&request[..bytes]);
            assert!(received.starts_with(&format!("POST {path} HTTP/1.1")));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(kind, base_url, "selected-model".into());
        let answer = model
            .generate(TextRequest {
                prompt: "say hello".into(),
                system_instruction: Some("be concise".into()),
                temperature: Some(0.4),
                max_tokens: Some(32),
            })
            .await
            .unwrap();
        assert!(answer.text.ends_with(" response"));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn fake_server_redirect_is_not_followed() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 2048];
        let _ = stream.read(&mut request).await.unwrap();
        stream
            .write_all(
                b"HTTP/1.1 302 Found\r\nLocation: https://example.invalid/collect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
    });
    let model = LoopbackTextModel::new(
        LoopbackTextServer::Ollama,
        base_url,
        "selected-model".into(),
    );
    let error = model
        .generate(TextRequest {
            prompt: "private transcript".into(),
            system_instruction: None,
            temperature: None,
            max_tokens: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Setup);
    assert!(!error.message.contains("private transcript"));
    server.await.unwrap();
}
#[tokio::test]
async fn invalid_server_and_model_do_not_claim_reachability() {
    let model = LoopbackTextModel::new(
        LoopbackTextServer::Ollama,
        "https://example.invalid:11434".into(),
        String::new(),
    );
    assert_eq!(
        model.check_readiness().await,
        LoopbackReadiness::Unreachable
    );
}

#[tokio::test]
async fn fake_ollama_readiness_requires_an_exact_model_match() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    for (catalog, expected) in [
        (
            r#"{"models":[{"name":"different-model"}]}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
        (
            r#"{"models":[{"name":"selected-model"}]}"#,
            LoopbackReadiness::ModelReady,
        ),
        (
            r#"{"models":"malformed"}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count])
                .starts_with("GET /api/tags HTTP/1.1"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{catalog}",
                catalog.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(
            LoopbackTextServer::Ollama,
            base_url,
            "selected-model".into(),
        );
        assert_eq!(model.check_readiness().await, expected);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn fake_llama_server_readiness_requires_exact_selected_model() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    for (catalog, expected) in [
        (
            r#"{"data":[{"id":"another-model"}]}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
        (
            r#"{"data":[{"id":"selected-model"}]}"#,
            LoopbackReadiness::ModelReady,
        ),
        (
            r#"{"data":[{"id":"selected-model:latest"}]}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
        (
            r#"{"data":{"id":"selected-model"}}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count])
                .starts_with("GET /v1/models HTTP/1.1"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{catalog}",
                catalog.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(
            LoopbackTextServer::LlamaServer,
            base_url,
            "selected-model".into(),
        );
        assert_eq!(model.check_readiness().await, expected);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn fake_server_rejects_truncated_or_mismatched_generation_without_retry() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    for (kind, payload) in [
        (
            LoopbackTextServer::Ollama,
            r#"{"done":true,"done_reason":"length","model":"selected-model","message":{"role":"assistant","content":"unfinished"}}"#,
        ),
        (
            LoopbackTextServer::LlamaServer,
            r#"{"choices":[{"message":{"role":"assistant","content":"unfinished"},"finish_reason":"length"}]}"#,
        ),
        (
            LoopbackTextServer::Ollama,
            r#"{"done":true,"model":"unexpected-model","message":{"role":"assistant","content":"wrong model"}}"#,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).starts_with("POST "));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(kind, base_url, "selected-model".into());
        let error = model
            .generate(TextRequest {
                prompt: "private transcript".into(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(32),
            })
            .await
            .unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Protocol);
        assert!(!error.message.contains("private transcript"));
        assert!(!error.message.contains("unfinished"));
        server.await.unwrap();
    }
}
