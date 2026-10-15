use super::loopback::{LoopbackReadiness, LoopbackTextModel, LoopbackTextServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn fake_catalog(
    base_host: &str,
    kind: LoopbackTextServer,
    payload: &'static str,
) -> LoopbackReadiness {
    fake_catalog_for_model(base_host, kind, payload, "selected-model").await
}

async fn fake_catalog_for_model(
    base_host: &str,
    kind: LoopbackTextServer,
    payload: &'static str,
    selected_model: &str,
) -> LoopbackReadiness {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://{base_host}:{port}");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 2048];
        let size = stream.read(&mut buffer).await.unwrap();
        let path = match kind {
            LoopbackTextServer::Ollama => "/api/tags",
            LoopbackTextServer::LlamaServer => "/v1/models",
        };
        assert!(
            String::from_utf8_lossy(&buffer[..size]).starts_with(&format!("GET {path} HTTP/1.1"))
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    let model = LoopbackTextModel::new(kind, address, selected_model.into());
    let readiness = model.check_readiness().await;
    server.await.unwrap();
    readiness
}

#[tokio::test]
async fn llama_server_catalog_requires_exact_selected_model() {
    for (body, expected) in [
        (
            r#"{"data":[{"id":"selected-model"}]}"#,
            LoopbackReadiness::ModelReady,
        ),
        (
            r#"{"data":[{"id":"another-model"}]}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
        (
            r#"{"data":"invalid"}"#,
            LoopbackReadiness::ReachableButModelUnavailable,
        ),
    ] {
        assert_eq!(
            fake_catalog("127.0.0.1", LoopbackTextServer::LlamaServer, body).await,
            expected
        );
    }
}

#[tokio::test]
async fn ollama_untagged_alias_is_ready_but_explicit_tags_and_llama_server_are_strict() {
    let catalog = r#"{"models":[{"name":"selected-model:latest"}]}"#;
    assert_eq!(
        fake_catalog_for_model(
            "127.0.0.1",
            LoopbackTextServer::Ollama,
            catalog,
            "selected-model",
        )
        .await,
        LoopbackReadiness::ModelReady
    );
    assert_eq!(
        fake_catalog_for_model(
            "127.0.0.1",
            LoopbackTextServer::Ollama,
            catalog,
            "selected-model:q4",
        )
        .await,
        LoopbackReadiness::ReachableButModelUnavailable
    );
    assert_eq!(
        fake_catalog_for_model(
            "127.0.0.1",
            LoopbackTextServer::LlamaServer,
            r#"{"data":[{"id":"selected-model:latest"}]}"#,
            "selected-model"
        )
        .await,
        LoopbackReadiness::ReachableButModelUnavailable
    );
}

#[tokio::test]
async fn localhost_catalog_is_pinned_to_ipv4_loopback() {
    assert_eq!(
        fake_catalog(
            "localhost",
            LoopbackTextServer::Ollama,
            r#"{"models":[{"name":"selected-model"}]}"#,
        )
        .await,
        LoopbackReadiness::ModelReady
    );
}

#[tokio::test]
async fn truncated_generation_is_rejected_at_the_http_boundary() {
    use super::traits::TextModel;
    use super::types::{ProviderErrorKind, TextRequest};

    for (kind, path, payload) in [
        (
            LoopbackTextServer::Ollama,
            "/api/chat",
            r#"{"done":true,"done_reason":"length","model":"selected-model","message":{"role":"assistant","content":"unfinished"}}"#,
        ),
        (
            LoopbackTextServer::LlamaServer,
            "/v1/chat/completions",
            r#"{"choices":[{"message":{"role":"assistant","content":"unfinished"},"finish_reason":"length"}]}"#,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 4096];
            let size = stream.read(&mut buffer).await.unwrap();
            assert!(String::from_utf8_lossy(&buffer[..size])
                .starts_with(&format!("POST {path} HTTP/1.1")));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(kind, base_url, "selected-model".into());
        let error = model
            .generate(TextRequest {
                prompt: "Say a complete sentence".into(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(16),
            })
            .await
            .unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Protocol);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn model_catalog_redirect_does_not_follow_a_remote_destination() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 2048];
        let size = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..size]).starts_with("GET /v1/models HTTP/1.1"));
        stream
            .write_all(
                b"HTTP/1.1 302 Found\r\nLocation: https://example.invalid/collect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
    });
    let model = LoopbackTextModel::new(
        LoopbackTextServer::LlamaServer,
        base_url,
        "selected-model".into(),
    );
    assert_eq!(
        model.check_readiness().await,
        LoopbackReadiness::ReachableButModelUnavailable
    );
    server.await.unwrap();
}

#[tokio::test]
async fn ipv6_loopback_model_catalog_uses_the_selected_local_server() {
    for (kind, path, payload) in [
        (
            LoopbackTextServer::Ollama,
            "/api/tags",
            r#"{"models":[{"name":"selected-model"}]}"#,
        ),
        (
            LoopbackTextServer::LlamaServer,
            "/v1/models",
            r#"{"data":[{"id":"selected-model"}]}"#,
        ),
    ] {
        // Some CI hosts disable IPv6 entirely. Do not mistake that host
        // limitation for a provider failure; the IPv4 path is tested above.
        let Ok(listener) = TcpListener::bind("[::1]:0").await else {
            eprintln!("IPv6 loopback unavailable; skipping IPv6 socket qualification");
            return;
        };
        let base_url = format!("http://[::1]:{}", listener.local_addr().unwrap().port());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 2048];
            let size = stream.read(&mut buffer).await.unwrap();
            assert!(String::from_utf8_lossy(&buffer[..size])
                .starts_with(&format!("GET {path} HTTP/1.1")));
            let response = format!(
                "HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{payload}",
                payload.len()
            ).replace("\\r", "\r").replace("\\n", "\n");
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let model = LoopbackTextModel::new(kind, base_url, "selected-model".into());
        assert_eq!(model.check_readiness().await, LoopbackReadiness::ModelReady);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn generation_redirect_is_rejected_without_followup_request() {
    use super::traits::TextModel;
    use super::types::{ProviderErrorKind, TextRequest};
    use std::time::Duration;

    for (kind, path) in [
        (LoopbackTextServer::Ollama, "/api/chat"),
        (LoopbackTextServer::LlamaServer, "/v1/chat/completions"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let redirect_target = format!("{base_url}/unexpected");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 4096];
            let size = stream.read(&mut buffer).await.unwrap();
            assert!(String::from_utf8_lossy(&buffer[..size])
                .starts_with(&format!("POST {path} HTTP/1.1")));
            let response = format!(
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: {redirect_target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(150), listener.accept())
                    .await
                    .is_err(),
                "generation must never follow an HTTP redirect"
            );
        });
        let model = LoopbackTextModel::new(kind, base_url, "selected-model".into());
        let error = model
            .generate(TextRequest {
                prompt: "redirect rejection".into(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(16),
            })
            .await
            .unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Setup);
        server.await.unwrap();
    }
}
