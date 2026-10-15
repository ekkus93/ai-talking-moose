//! ELSV-200: HTTP-level failure classification for the OpenAI adapter against
//! an in-process 127.0.0.1 fake. No live service is contacted.
use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const KEY: &str = "sk-test-openai-key-ELSV200";
const PROMPT: &str = "private-prompt-ELSV200";
const BODY_MARKER: &str = "server-body-marker-ELSV200";

async fn serve_once(
    status_line: &'static str,
    body: String,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let size = stream.read(&mut buf).await.unwrap();
        assert!(String::from_utf8_lossy(&buf[..size]).starts_with("POST /v1/responses HTTP/1.1"));
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    (endpoint, handle)
}

fn request() -> TextRequest {
    TextRequest {
        prompt: PROMPT.into(),
        system_instruction: None,
        temperature: None,
        max_tokens: Some(16),
    }
}

async fn failure_for(status_line: &'static str, body: String) -> ProviderError {
    let (endpoint, server) = serve_once(status_line, body).await;
    let model = OpenAiTextModel::with_endpoint(KEY.into(), "gpt-4.1-mini".into(), endpoint);
    let error = model.generate(request()).await.unwrap_err();
    server.await.unwrap();
    for text in [&error.message, &format!("{error:?}"), &error.to_string()] {
        assert!(!text.contains(PROMPT), "prompt leaked: {text}");
        assert!(!text.contains(KEY), "key leaked: {text}");
        assert!(!text.contains(BODY_MARKER), "server body leaked: {text}");
    }
    error
}

#[tokio::test]
async fn http_400_and_404_map_to_model_category() {
    for status in ["HTTP/1.1 400 Bad Request", "HTTP/1.1 404 Not Found"] {
        let error = failure_for(
            status,
            format!(r#"{{"error":{{"message":"{BODY_MARKER} {PROMPT} {KEY}"}}}}"#),
        )
        .await;
        assert_eq!(error.kind, ProviderErrorKind::Model, "{status}");
    }
}

#[tokio::test]
async fn http_401_maps_to_auth_category() {
    let error = failure_for(
        "HTTP/1.1 401 Unauthorized",
        format!(r#"{{"error":{{"message":"{BODY_MARKER} {PROMPT} {KEY}"}}}}"#),
    )
    .await;
    assert_eq!(error.kind, ProviderErrorKind::Auth);
}

#[tokio::test]
async fn http_200_with_non_json_or_malformed_json_maps_to_protocol() {
    for body in [
        format!("<html>{BODY_MARKER} {PROMPT} {KEY}</html>"),
        format!(r#"{{"status":"completed","output":[ {BODY_MARKER} {PROMPT}"#),
        String::new(),
    ] {
        let error = failure_for("HTTP/1.1 200 OK", body).await;
        assert_eq!(error.kind, ProviderErrorKind::Protocol);
    }
}
