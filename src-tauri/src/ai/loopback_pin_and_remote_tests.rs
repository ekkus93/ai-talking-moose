//! ELSV-300: the `localhost` -> 127.0.0.1 pin is observable, and a remote address
//! is rejected before any network I/O.
use super::loopback::{LoopbackReadiness, LoopbackTextModel, LoopbackTextServer};
use super::traits::TextModel;
use super::types::{ProviderErrorKind, TextRequest};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const KINDS: [LoopbackTextServer; 2] =
    [LoopbackTextServer::Ollama, LoopbackTextServer::LlamaServer];

fn request() -> TextRequest {
    TextRequest {
        prompt: "hello".into(),
        system_instruction: None,
        temperature: None,
        max_tokens: Some(8),
    }
}

/// With `localhost` configured, the connection must go to 127.0.0.1 even when an
/// IPv6 listener owns the same port. Unpinned system resolution typically prefers
/// `::1`, so this test would see the decoy accept a connection without the pin.
/// Limitation: the pin cannot be forced to matter from test code (the resolver is
/// not injectable), so bind an IPv6 decoy on the same port and require it to stay
/// untouched. This makes the effective-destination assertion explicit instead of
/// silently degrading to ordinary IPv4 reachability.
#[tokio::test]
async fn localhost_connects_to_ipv4_even_when_an_ipv6_listener_owns_the_same_port() {
    for kind in KINDS {
        let v4 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = v4.local_addr().unwrap().port();
        let decoy = TcpListener::bind(format!("[::1]:{port}"))
            .await
            .expect("IPv6 loopback must be available to prove the localhost pin");

        let server = tokio::spawn(async move {
            let (mut stream, _) = v4.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf).await.unwrap();
            let body = match kind {
                LoopbackTextServer::Ollama => r#"{"models":[{"name":"selected-model"}]}"#,
                LoopbackTextServer::LlamaServer => r#"{"data":[{"id":"selected-model"}]}"#,
            };
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        });

        let model = LoopbackTextModel::new(
            kind,
            format!("http://localhost:{port}"),
            "selected-model".into(),
        );
        assert_eq!(model.check_readiness().await, LoopbackReadiness::ModelReady);
        server.await.unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(200), decoy.accept())
                .await
                .is_err(),
            "{kind:?}: localhost must be pinned to 127.0.0.1, never ::1"
        );
    }
}

/// Syntactically valid but non-loopback destinations. 10.0.0.5 and 192.0.2.1
/// (TEST-NET-1) would hang for the full connect timeout if a connection were
/// attempted, so a fast answer proves the address was refused before any I/O.
#[tokio::test]
async fn remote_addresses_are_rejected_before_any_network_io() {
    for address in [
        "http://example.com:11434",
        "http://10.0.0.5:11434",
        "http://192.0.2.1:8080",
        "http://0.0.0.0:11434",
        "http://[2001:db8::1]:11434",
    ] {
        for kind in KINDS {
            let model = LoopbackTextModel::new(kind, address.into(), "selected-model".into());

            let started = Instant::now();
            let readiness =
                tokio::time::timeout(Duration::from_millis(500), model.check_readiness())
                    .await
                    .unwrap_or_else(|_| panic!("{address}: readiness attempted network I/O"));
            assert_eq!(readiness, LoopbackReadiness::Unreachable, "{address}");

            let error = tokio::time::timeout(Duration::from_millis(500), model.generate(request()))
                .await
                .unwrap_or_else(|_| panic!("{address}: generation attempted network I/O"))
                .unwrap_err();
            assert_eq!(error.kind, ProviderErrorKind::Setup, "{address}");
            assert!(
                !error.message.contains(address),
                "{address}: address echoed"
            );
            assert!(started.elapsed() < Duration::from_secs(1));
        }
    }
}
