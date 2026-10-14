//! Backend-only OpenAI Responses API text adapter.
//! No implicit retry, credential persistence, service switching or fallback.
use crate::ai::traits::TextModel;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextRequest, TextResponse};
use async_trait::async_trait;
use reqwest::{redirect::Policy, Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

const RESPONSES_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_PROMPT_BYTES: usize = 64 * 1024;

pub struct OpenAiTextModel {
    api_key: String,
    model: String,
    endpoint: String,
    client: Result<Client, ProviderError>,
}

impl OpenAiTextModel {
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_endpoint(api_key, model, RESPONSES_ENDPOINT.to_string())
    }

    fn with_endpoint(api_key: String, model: String, endpoint: String) -> Self {
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(8))
            .build()
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Setup));
        Self {
            api_key,
            model,
            endpoint,
            client,
        }
    }

    fn status_error(status: StatusCode) -> ProviderError {
        let kind = match status.as_u16() {
            401 | 403 => ProviderErrorKind::Auth,
            429 => ProviderErrorKind::Quota,
            400 | 404 | 422 => ProviderErrorKind::Model,
            408 | 500..=599 => ProviderErrorKind::Network,
            _ => ProviderErrorKind::Setup,
        };
        ProviderError::from_kind(kind)
    }

    fn output_text(value: &Value) -> Result<TextResponse, ProviderError> {
        let protocol_error = || ProviderError::from_kind(ProviderErrorKind::Protocol);
        if value["status"].as_str() != Some("completed") {
            return Err(protocol_error());
        }
        let messages = value["output"].as_array().ok_or_else(protocol_error)?;
        let mut text = String::new();
        for message in messages {
            if message["type"].as_str() != Some("message") {
                continue;
            }
            // Never treat a user/tool echo as generated assistant text.
            if message["role"].as_str() != Some("assistant") {
                return Err(protocol_error());
            }
            let parts = message["content"].as_array().ok_or_else(protocol_error)?;
            for part in parts {
                if part["type"].as_str() == Some("output_text") {
                    let segment = part["text"].as_str().ok_or_else(protocol_error)?;
                    if text.len().saturating_add(segment.len()) > MAX_TEXT_BYTES {
                        return Err(protocol_error());
                    }
                    text.push_str(segment);
                }
            }
        }
        if text.trim().is_empty() {
            return Err(protocol_error());
        }
        Ok(TextResponse {
            text: text.trim().to_string(),
            finish_reason: Some("completed".to_string()),
        })
    }
}

#[async_trait]
impl TextModel for OpenAiTextModel {
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, ProviderError> {
        let setup = || ProviderError::from_kind(ProviderErrorKind::Setup);
        if self.api_key.is_empty()
            || self.api_key.len() > 4096
            || self.api_key.chars().any(char::is_control)
        {
            return Err(ProviderError::from_kind(ProviderErrorKind::Auth));
        }
        if self.model.is_empty()
            || self.model.len() > 128
            || !self
                .model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._:".contains(&b))
            || request.prompt.trim().is_empty()
            || request.prompt.len() > MAX_PROMPT_BYTES
            || request
                .max_tokens
                .is_some_and(|limit| limit == 0 || limit > 4096)
            || request
                .system_instruction
                .as_ref()
                .is_some_and(|s| s.len() > MAX_PROMPT_BYTES)
        {
            return Err(setup());
        }
        #[cfg(test)]
        if crate::test_support::network_denied() {
            return Err(ProviderError::from_kind(ProviderErrorKind::Network));
        }

        let mut body = json!({
            "model": self.model,
            "input": request.prompt,
            "max_output_tokens": request.max_tokens.unwrap_or(1024),
            "store": false
        });
        if let Some(instructions) = request.system_instruction {
            body["instructions"] = json!(instructions);
        }
        if let Some(temp) = request.temperature {
            if !temp.is_finite() || !(0.0..=2.0).contains(&temp) {
                return Err(setup());
            }
            // Reasoning models may reject temperature, so leave unspecified
            // unless the explicitly selected model is known to accept it.
            if self.model.starts_with("gpt-4.1") || self.model.starts_with("gpt-4o") {
                body["temperature"] = json!(temp);
            }
        }
        let client = self.client.as_ref().map_err(Clone::clone)?;
        let mut response = client
            .post(&self.endpoint)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key),
            )
            .json(&body)
            .send()
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?;
        if !response.status().is_success() {
            return Err(Self::status_error(response.status()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Network))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(ProviderError::from_kind(ProviderErrorKind::Protocol));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Protocol))?;
        Self::output_text(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(prompt: &str) -> TextRequest {
        TextRequest {
            prompt: prompt.into(),
            system_instruction: Some("Stay concise.".into()),
            temperature: Some(0.5),
            max_tokens: Some(40),
        }
    }

    #[test]
    fn parser_extracts_only_bounded_assistant_output() {
        let response = json!({
            "status": "completed",
            "output": [
                {"type": "reasoning", "content": []},
                {"type": "message", "role": "assistant",
                 "content": [{"type":"output_text", "text":"hello "},
                             {"type":"output_text", "text":"world"}]}
            ]
        });
        assert_eq!(
            OpenAiTextModel::output_text(&response).unwrap().text,
            "hello world"
        );
        for bad in [
            json!({"status":"completed", "output":[]}),
            json!({"status":"incomplete", "output":[]}),
            json!({"status":"completed", "output":[{"type":"message", "role":"assistant", "content":[{"type":"output_text"}]}]}),
            json!({"status":"completed", "output":[{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":"x".repeat(MAX_TEXT_BYTES + 1)}]}]}),
            json!({"status":"completed", "output":[{"type":"message", "role":"user", "content":[{"type":"output_text", "text":"echo"}]}]}),
            json!({"status":"completed", "output":[{"type":"message", "content":[{"type":"output_text", "text":"missing role"}]}]}),
        ] {
            assert_eq!(
                OpenAiTextModel::output_text(&bad).unwrap_err().kind,
                ProviderErrorKind::Protocol
            );
        }
    }

    #[test]
    fn response_statuses_are_safe_and_categorized() {
        for (code, kind) in [
            (401, ProviderErrorKind::Auth),
            (403, ProviderErrorKind::Auth),
            (429, ProviderErrorKind::Quota),
            (404, ProviderErrorKind::Model),
            (504, ProviderErrorKind::Network),
        ] {
            assert_eq!(
                OpenAiTextModel::status_error(StatusCode::from_u16(code).unwrap()).kind,
                kind
            );
        }
    }

    #[tokio::test]
    async fn missing_key_fails_before_network_io() {
        let model = OpenAiTextModel::new(String::new(), "gpt-4.1-mini".into());
        let err = model.generate(sample("hello")).await.unwrap_err();
        assert_eq!(err.kind, ProviderErrorKind::Auth);
    }

    #[tokio::test]
    async fn zero_output_token_budget_fails_before_network_io() {
        let model = OpenAiTextModel::new("test-openai-key".into(), "gpt-4.1-mini".into());
        let mut request = sample("hello");
        request.max_tokens = Some(0);
        let err = model.generate(request).await.unwrap_err();
        assert_eq!(err.kind, ProviderErrorKind::Setup);
    }

    #[tokio::test]
    async fn invalid_generation_parameters_fail_before_network_io() {
        let model = OpenAiTextModel::new("dummy".into(), "gpt-4.1-mini".into());
        for prompt in ["", "   "] {
            assert_eq!(
                model.generate(sample(prompt)).await.unwrap_err().kind,
                ProviderErrorKind::Setup
            );
        }
        for temperature in [f32::NAN, f32::INFINITY, -0.1, 2.1] {
            let mut request = sample("hello");
            request.temperature = Some(temperature);
            assert_eq!(
                model.generate(request).await.unwrap_err().kind,
                ProviderErrorKind::Setup
            );
        }
        for budget in [0, 4097, u32::MAX] {
            let mut request = sample("hello");
            request.max_tokens = Some(budget);
            assert_eq!(
                model.generate(request).await.unwrap_err().kind,
                ProviderErrorKind::Setup
            );
        }
        let invalid_model = OpenAiTextModel::new("dummy".into(), "../invalid".into());
        assert_eq!(
            invalid_model
                .generate(sample("hello"))
                .await
                .unwrap_err()
                .kind,
            ProviderErrorKind::Setup
        );
        let mut request = sample("hello");
        request.prompt = "x".repeat(MAX_PROMPT_BYTES + 1);
        assert_eq!(
            model.generate(request).await.unwrap_err().kind,
            ProviderErrorKind::Setup
        );
    }

    #[tokio::test]
    async fn fake_http_responses_validate_shape_auth_and_failure_isolation() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1/responses", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 16 * 1024];
            let read = stream.read(&mut buf).await.unwrap();
            let received = String::from_utf8_lossy(&buf[..read]);
            assert!(received.contains("POST /v1/responses HTTP/1.1"));
            assert!(received.contains("Bearer test-openai-key"));
            // Response payload may arrive in a subsequent TCP chunk.
            let payload = r#"{"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hello from fake"}]}]}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                payload.len(), payload
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        });
        let model = OpenAiTextModel::with_endpoint(
            "test-openai-key".into(),
            "gpt-4.1-mini".into(),
            endpoint,
        );
        let result = model.generate(sample("say hello")).await.unwrap();
        assert_eq!(result.text, "hello from fake");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn fake_http_errors_are_sanitized_and_never_follow_redirects() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        for (status, headers, payload, expected) in [
            (
                "HTTP/1.1 429 Too Many Requests",
                "",
                r#"{"error":{"message":"private transcript test-openai-key"}}"#,
                ProviderErrorKind::Quota,
            ),
            (
                "HTTP/1.1 302 Found",
                "Location: https://example.invalid/collect\r\n",
                "",
                ProviderErrorKind::Setup,
            ),
            (
                "HTTP/1.1 200 OK",
                "",
                r#"{"status":"completed","output":[]}"#,
                ProviderErrorKind::Protocol,
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("http://{}/v1/responses", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 4096];
                let size = stream.read(&mut buf).await.unwrap();
                assert!(String::from_utf8_lossy(&buf[..size])
                    .starts_with("POST /v1/responses HTTP/1.1"));
                let response = format!(
                    "{status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            });
            let model = OpenAiTextModel::with_endpoint(
                "test-openai-key".into(),
                "gpt-4.1-mini".into(),
                endpoint,
            );
            let error = model
                .generate(sample("private transcript"))
                .await
                .unwrap_err();
            assert_eq!(error.kind, expected);
            assert!(!error.message.contains("private transcript"));
            assert!(!error.message.contains("test-openai-key"));
            server.await.unwrap();
        }
    }
}
