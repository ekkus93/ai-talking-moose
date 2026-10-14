use super::loopback::{LoopbackReadiness, LoopbackTextModel, LoopbackTextServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn fake_catalog(
    base_host: &str,
    kind: LoopbackTextServer,
    payload: &'static str,
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
    let model = LoopbackTextModel::new(kind, address, "selected-model".into());
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
