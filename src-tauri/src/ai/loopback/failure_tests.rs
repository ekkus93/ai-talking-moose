//! ELSV-300: failure, bounds, cancellation and sanitization behavior of the
//! Ollama / llama-server adapter against in-process 127.0.0.1 fakes only.
use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PROMPT: &str = "private-prompt-ELSV300";
const BODY_MARKER: &str = "server-body-marker-ELSV300";

const KINDS: [LoopbackTextServer; 2] =
    [LoopbackTextServer::Ollama, LoopbackTextServer::LlamaServer];

fn request() -> TextRequest {
    TextRequest {
        prompt: PROMPT.into(),
        system_instruction: None,
        temperature: None,
        max_tokens: Some(16),
    }
}

fn ok_payload(kind: LoopbackTextServer) -> &'static str {
    match kind {
        LoopbackTextServer::Ollama => {
            r#"{"done":true,"model":"selected-model","message":{"role":"assistant","content":"local reply"}}"#
        }
        LoopbackTextServer::LlamaServer => {
            r#"{"choices":[{"message":{"role":"assistant","content":"local reply"},"finish_reason":"stop"}]}"#
        }
    }
}

fn assert_clean(error: &ProviderError) {
    for text in [&error.message, &format!("{error:?}"), &error.to_string()] {
        for forbidden in [
            PROMPT,
            BODY_MARKER,
            "127.0.0.1",
            "localhost",
            "userinfo",
            "secret-pass",
        ] {
            assert!(!text.contains(forbidden), "{forbidden} leaked: {text}");
        }
    }
}

/// Serve exactly one HTTP response; a second connection fails the task.
async fn serve_one(
    kind: LoopbackTextServer,
    path: &'static str,
    method: &'static str,
    status_line: &'static str,
    body: String,
) -> (String, tokio::task::JoinHandle<()>) {
    let _ = kind;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 8192];
        let size = stream.read(&mut buf).await.unwrap();
        assert!(
            String::from_utf8_lossy(&buf[..size]).starts_with(&format!("{method} {path} HTTP/1.1"))
        );
        let head = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        // The client may legitimately hang up early on oversized bodies.
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(body.as_bytes()).await;
        let _ = stream.shutdown().await;
        assert!(
            tokio::time::timeout(Duration::from_millis(150), listener.accept())
                .await
                .is_err(),
            "adapter must not retry"
        );
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

fn chat_path(kind: LoopbackTextServer) -> &'static str {
    kind.chat_path()
}

fn refused_address() -> String {
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);
    format!("http://127.0.0.1:{port}")
}

fn short_timeout_client(timeout: Duration) -> Result<Client, ProviderError> {
    Ok(Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(timeout)
        .build()
        .unwrap())
}

#[tokio::test]
async fn stalled_server_times_out_as_network_error_without_retry_or_leak() {
    for kind in KINDS {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 8192];
            let size = stream.read(&mut buf).await.unwrap();
            assert!(String::from_utf8_lossy(&buf[..size])
                .starts_with(&format!("POST {} HTTP/1.1", chat_path(kind))));
            tokio::time::sleep(Duration::from_millis(400)).await;
            assert!(
                tokio::time::timeout(Duration::from_millis(100), listener.accept())
                    .await
                    .is_err(),
                "timeout must not trigger a second request"
            );
            drop(stream);
        });
        // The production deadline is a fixed 30s (not injectable), so a test
        // client with the same policy but a short deadline stands in.
        let mut model = LoopbackTextModel::new(kind, base, "selected-model".into());
        model.client = short_timeout_client(Duration::from_millis(100));
        let error = model.generate(request()).await.unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Network);
        assert_clean(&error);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn response_over_64kib_is_rejected_over_http_for_both_servers() {
    for kind in KINDS {
        let padding = BODY_MARKER.repeat(MAX_BODY_BYTES / BODY_MARKER.len() + 8);
        let body = format!(r#"{{"pad":"{padding}"}}"#);
        assert!(body.len() > MAX_BODY_BYTES);
        let (base, server) =
            serve_one(kind, chat_path(kind), "POST", "HTTP/1.1 200 OK", body).await;
        let model = LoopbackTextModel::new(kind, base, "selected-model".into());
        let error = model.generate(request()).await.unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Protocol);
        assert_clean(&error);
        server.await.unwrap();

        // Readiness applies the same cap and fails closed.
        let padding = BODY_MARKER.repeat(MAX_BODY_BYTES / BODY_MARKER.len() + 8);
        let body = format!(r#"{{"pad":"{padding}"}}"#);
        let (base, server) =
            serve_one(kind, kind.models_path(), "GET", "HTTP/1.1 200 OK", body).await;
        let model = LoopbackTextModel::new(kind, base, "selected-model".into());
        assert_eq!(
            model.check_readiness().await,
            LoopbackReadiness::ReachableButModelUnavailable
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn refused_connection_is_unreachable_and_generation_is_a_sanitized_network_error() {
    for kind in KINDS {
        let model = LoopbackTextModel::new(kind, refused_address(), "selected-model".into());
        assert_eq!(
            model.check_readiness().await,
            LoopbackReadiness::Unreachable
        );
        let error = model.generate(request()).await.unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Network);
        assert_clean(&error);
    }
}

#[tokio::test]
async fn http_500_readiness_is_distinct_from_a_refused_connection() {
    for kind in KINDS {
        let (base, server) = serve_one(
            kind,
            kind.models_path(),
            "GET",
            "HTTP/1.1 500 Internal Server Error",
            BODY_MARKER.into(),
        )
        .await;
        let model = LoopbackTextModel::new(kind, base, "selected-model".into());
        // Current behavior: a server that answers (even with 500) is
        // "reachable", unlike a refused connection which is Unreachable.
        assert_eq!(
            model.check_readiness().await,
            LoopbackReadiness::ReachableButModelUnavailable
        );
        server.await.unwrap();
        let refused = LoopbackTextModel::new(kind, refused_address(), "selected-model".into());
        assert_eq!(
            refused.check_readiness().await,
            LoopbackReadiness::Unreachable
        );
    }
}

#[tokio::test]
async fn generation_through_localhost_reaches_the_pinned_ipv4_loopback_server() {
    for kind in KINDS {
        let (base, server) = serve_one(
            kind,
            chat_path(kind),
            "POST",
            "HTTP/1.1 200 OK",
            ok_payload(kind).into(),
        )
        .await;
        let localhost = base.replace("127.0.0.1", "localhost");
        assert!(localhost.contains("localhost"));
        let model = LoopbackTextModel::new(kind, localhost, "selected-model".into());
        assert_eq!(model.generate(request()).await.unwrap().text, "local reply");
        server.await.unwrap();
    }
}

#[tokio::test]
async fn error_variants_never_contain_prompt_body_or_address_userinfo() {
    for kind in KINDS {
        for (status, expected) in [
            ("HTTP/1.1 400 Bad Request", ProviderErrorKind::Model),
            ("HTTP/1.1 404 Not Found", ProviderErrorKind::Model),
            ("HTTP/1.1 401 Unauthorized", ProviderErrorKind::Auth),
            ("HTTP/1.1 403 Forbidden", ProviderErrorKind::Auth),
            (
                "HTTP/1.1 500 Internal Server Error",
                ProviderErrorKind::Network,
            ),
        ] {
            let (base, server) = serve_one(
                kind,
                chat_path(kind),
                "POST",
                status,
                format!(r#"{{"error":"{BODY_MARKER} {PROMPT}"}}"#),
            )
            .await;
            let model = LoopbackTextModel::new(kind, base, "selected-model".into());
            let error = model.generate(request()).await.unwrap_err();
            assert_eq!(error.kind, expected, "{status}");
            assert_clean(&error);
            server.await.unwrap();
        }
    }

    // Credentials embedded in the address are rejected as Setup before any I/O
    // and are never echoed back.
    for kind in KINDS {
        let model = LoopbackTextModel::new(
            kind,
            "http://userinfo:secret-pass@127.0.0.1:9".into(),
            "selected-model".into(),
        );
        let error = model.generate(request()).await.unwrap_err();
        assert_eq!(error.kind, ProviderErrorKind::Setup);
        assert_clean(&error);
        assert_eq!(
            model.check_readiness().await,
            LoopbackReadiness::Unreachable
        );
    }
}

#[tokio::test]
async fn dropping_generate_mid_request_closes_the_connection_without_retry() {
    for kind in KINDS {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 8192];
            let size = stream.read(&mut buf).await.unwrap();
            assert!(size > 0);
            // After the client future is dropped the socket must reach EOF.
            let mut rest = Vec::new();
            let eof = tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => rest.extend_from_slice(&buf[..n]),
                    }
                }
            })
            .await;
            assert!(
                eof.is_ok(),
                "connection was not released after cancellation"
            );
            assert!(
                tokio::time::timeout(Duration::from_millis(200), listener.accept())
                    .await
                    .is_err(),
                "cancellation must not trigger a retry"
            );
        });
        let model = LoopbackTextModel::new(kind, base, "selected-model".into());
        let outcome =
            tokio::time::timeout(Duration::from_millis(250), model.generate(request())).await;
        assert!(
            outcome.is_err(),
            "server never answers; future must be pending"
        );
        server.await.unwrap();
    }
}
