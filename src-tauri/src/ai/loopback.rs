//! Network-constrained, backend-only local text-server integrations.
//! Server URLs have one exact loopback origin, no redirects, no proxy, and no
//! cloud fallback even if the local server is offline or responds with errors.
use crate::ai::traits::TextModel;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextRequest, TextResponse};
use crate::app::settings_policy::validate_loopback_server_address;
use async_trait::async_trait;
use reqwest::{redirect::Policy, Client, StatusCode};
use serde::Serialize;
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopbackTextServer {
    Ollama,
    LlamaServer,
}

impl LoopbackTextServer {
    fn chat_path(self) -> &'static str {
        match self {
            Self::Ollama => "/api/chat",
            Self::LlamaServer => "/v1/chat/completions",
        }
    }

    fn models_path(self) -> &'static str {
        match self {
            Self::Ollama => "/api/tags",
            Self::LlamaServer => "/v1/models",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopbackReadiness {
    Unreachable,
    ReachableButModelUnavailable,
    ModelReady,
}

pub struct LoopbackTextModel {
    kind: LoopbackTextServer,
    base_url: String,
    model: String,
    client: Result<Client, ProviderError>,
}

impl LoopbackTextModel {
    pub fn new(kind: LoopbackTextServer, base_url: String, model: String) -> Self {
        let validated = validate_loopback_server_address(&base_url);
        let client = validated
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Setup))
            .and_then(|()| {
                let url = reqwest::Url::parse(&base_url)
                    .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Setup))?;
                let mut builder = Client::builder()
                    .no_proxy()
                    .redirect(Policy::none())
                    .timeout(Duration::from_secs(30))
                    .connect_timeout(Duration::from_secs(4));
                // Force the hostname to a loopback IP even when machine DNS or
                // hosts files would resolve "localhost" to a remote host.
                if url.host_str() == Some("localhost") {
                    let target = SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::LOCALHOST),
                        url.port()
                            .ok_or_else(|| ProviderError::from_kind(ProviderErrorKind::Setup))?,
                    );
                    builder = builder.resolve("localhost", target);
                }
                builder
                    .build()
                    .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Setup))
            });
        Self {
            kind,
            base_url,
            model,
            client,
        }
    }

    fn clean_model(&self) -> Result<(), ProviderError> {
        if self.model.is_empty()
            || self.model.len() > 128
            || self.model.trim() != self.model
            || self.model.chars().any(char::is_control)
        {
            Err(ProviderError::from_kind(ProviderErrorKind::Model))
        } else {
            Ok(())
        }
    }

    fn classify_status(status: StatusCode) -> ProviderError {
        let kind = match status.as_u16() {
            400 | 404 | 422 => ProviderErrorKind::Model,
            401 | 403 => ProviderErrorKind::Auth,
            429 => ProviderErrorKind::Quota,
            408 | 500..=599 => ProviderErrorKind::Network,
            _ => ProviderErrorKind::Setup,
        };
        ProviderError::from_kind(kind)
    }

    async fn bounded_json(response: &mut reqwest::Response) -> Result<Value, ProviderError> {
        let mut bytes = Vec::new();
        while let Some(part) = response
            .chunk()
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?
        {
            if bytes.len().saturating_add(part.len()) > MAX_BODY_BYTES {
                return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
            }
            bytes.extend_from_slice(&part);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Protocol))
    }

    fn text_from_json(&self, value: &Value) -> Result<TextResponse, ProviderError> {
        // An echoed user/tool message is not a generated assistant reply.
        // When the server supplies a model identity, it must be the selected one.
        if value.get("model").is_some() && value["model"].as_str() != Some(self.model.as_str()) {
            return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
        }
        let text = match self.kind {
            LoopbackTextServer::Ollama => {
                if value["message"]["role"].as_str() != Some("assistant") {
                    return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
                }
                value["message"]["content"].as_str()
            }
            LoopbackTextServer::LlamaServer => {
                if value["choices"][0]["message"]["role"].as_str() != Some("assistant") {
                    return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
                }
                value["choices"][0]["message"]["content"].as_str()
            }
        }
        .ok_or_else(|| ProviderError::from_kind(ProviderErrorKind::Protocol))?;
        if text.trim().is_empty() || text.len() > MAX_OUTPUT_BYTES {
            return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
        }
        let finish_reason = match self.kind {
            LoopbackTextServer::Ollama => {
                if value["done"].as_bool() != Some(true)
                    || (value.get("done_reason").is_some()
                        && value["done_reason"].as_str() != Some("stop"))
                {
                    return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
                }
                Some("completed".to_string())
            }
            LoopbackTextServer::LlamaServer => {
                if value["choices"][0]["finish_reason"].as_str() != Some("stop") {
                    return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
                }
                Some("stop".to_string())
            }
        };
        Ok(TextResponse {
            text: text.trim().to_string(),
            finish_reason,
        })
    }

    pub async fn check_readiness(&self) -> LoopbackReadiness {
        let Ok(client) = &self.client else {
            return LoopbackReadiness::Unreachable;
        };
        let result = client
            .get(format!(
                "{}{}",
                self.base_url.trim_end_matches('/'),
                self.kind.models_path()
            ))
            .timeout(Duration::from_secs(4))
            .send()
            .await;
        let Ok(mut response) = result else {
            return LoopbackReadiness::Unreachable;
        };
        if !response.status().is_success() || self.clean_model().is_err() {
            return LoopbackReadiness::ReachableButModelUnavailable;
        }
        let Ok(data) = Self::bounded_json(&mut response).await else {
            return LoopbackReadiness::ReachableButModelUnavailable;
        };
        let models = match self.kind {
            LoopbackTextServer::Ollama => &data["models"],
            LoopbackTextServer::LlamaServer => &data["data"],
        };
        let available = models.as_array().is_some_and(|models| {
            models.iter().any(|entry| {
                let candidate = match self.kind {
                    LoopbackTextServer::Ollama => {
                        entry["name"].as_str().or_else(|| entry["model"].as_str())
                    }
                    LoopbackTextServer::LlamaServer => entry["id"].as_str(),
                };
                candidate == Some(self.model.as_str())
            })
        });
        if available {
            LoopbackReadiness::ModelReady
        } else {
            LoopbackReadiness::ReachableButModelUnavailable
        }
    }
}

#[async_trait]
impl TextModel for LoopbackTextModel {
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, ProviderError> {
        self.clean_model()?;
        if request.prompt.trim().is_empty()
            || request.prompt.len() > MAX_BODY_BYTES
            || request
                .system_instruction
                .as_ref()
                .is_some_and(|text| text.len() > MAX_BODY_BYTES)
            || request
                .temperature
                .is_some_and(|value| !value.is_finite() || !(0.0..=2.0).contains(&value))
            || request
                .max_tokens
                .is_some_and(|limit| limit == 0 || limit > 4096)
        {
            return Err(ProviderError::from_kind(ProviderErrorKind::Setup));
        }
        #[cfg(test)]
        if crate::test_support::network_denied() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Network));
        }
        let client = self.client.as_ref().map_err(Clone::clone)?;
        let mut messages = Vec::new();
        if let Some(instruction) = request.system_instruction {
            messages.push(json!({"role":"system", "content":instruction}));
        }
        messages.push(json!({"role":"user", "content":request.prompt}));
        let body = match self.kind {
            LoopbackTextServer::Ollama => json!({
                "model": self.model,
                "messages": messages,
                "stream": false,
                "options": {
                    "temperature": request.temperature.unwrap_or(0.8).clamp(0.0, 2.0),
                    "num_predict": request.max_tokens.unwrap_or(1024)
                }
            }),
            LoopbackTextServer::LlamaServer => json!({
                "model": self.model,
                "messages": messages,
                "stream": false,
                "temperature": request.temperature.unwrap_or(0.8).clamp(0.0, 2.0),
                "max_tokens": request.max_tokens.unwrap_or(1024)
            }),
        };
        let mut response = client
            .post(format!(
                "{}{}",
                self.base_url.trim_end_matches('/'),
                self.kind.chat_path()
            ))
            .json(&body)
            .send()
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?;
        if !response.status().is_success() {
            return Err(Self::classify_status(response.status()));
        }
        let output = Self::bounded_json(&mut response).await?;
        self.text_from_json(&output)
    }
}

#[cfg(test)]
mod tests {
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

}
