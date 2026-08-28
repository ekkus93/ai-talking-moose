use crate::ai::google::auth::{trace_google_provider_failure, GoogleAuth, GOOGLE_API_KEY_HEADER};
use crate::ai::traits::TextModel;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextRequest, TextResponse};
use async_trait::async_trait;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::json;
use tracing::info;

pub struct GoogleTextModel {
    auth: GoogleAuth,
    model_name: String,
    client: Client,
}

impl GoogleTextModel {
    pub fn new(auth: GoogleAuth, model_name: String) -> Self {
        Self {
            auth,
            model_name,
            client: Client::new(),
        }
    }

    fn safe_error(kind: ProviderErrorKind) -> ProviderError {
        ProviderError::from_kind(kind)
    }

    fn classify_status(status: StatusCode) -> ProviderError {
        match status.as_u16() {
            401 | 403 => Self::safe_error(ProviderErrorKind::Auth),
            429 => Self::safe_error(ProviderErrorKind::Quota),
            404 => Self::safe_error(ProviderErrorKind::Model),
            400..=499 => Self::safe_error(ProviderErrorKind::Setup),
            _ => Self::safe_error(ProviderErrorKind::Network),
        }
    }

    fn generation_url(model: &str) -> String {
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent")
    }

    fn generation_request(&self, model: &str, body: &serde_json::Value) -> RequestBuilder {
        self.client
            .post(Self::generation_url(model))
            .header(GOOGLE_API_KEY_HEADER, &self.auth.api_key)
            .json(body)
    }

    async fn try_generate_with_model(
        &self,
        model: &str,
        request: &TextRequest,
    ) -> Result<TextResponse, ProviderError> {
        let mut generation_config = json!({
            "maxOutputTokens": request.max_tokens.unwrap_or(1024)
        });
        // Current Gemini 3.x models reject the legacy sampling controls. Keep
        // compatibility only for explicitly constructed older model IDs.
        if !model.starts_with("gemini-3.") {
            generation_config["temperature"] = json!(request.temperature.unwrap_or(0.8));
        }

        let mut body = json!({
            "contents": [
                {
                    "parts": [
                        { "text": &request.prompt }
                    ]
                }
            ],
            "generationConfig": generation_config
        });

        if let Some(ref sys) = request.system_instruction {
            body["systemInstruction"] = json!({
                "parts": [
                    { "text": sys }
                ]
            });
        }

        #[cfg(test)]
        if crate::test_support::network_denied() {
            return Err(Self::safe_error(ProviderErrorKind::Network));
        }

        let resp = self
            .generation_request(model, &body)
            .send()
            .await
            .map_err(|_| {
                let error = Self::safe_error(ProviderErrorKind::Network);
                trace_google_provider_failure("text", &error);
                error
            })?;

        if !resp.status().is_success() {
            let error = Self::classify_status(resp.status());
            trace_google_provider_failure("text", &error);
            return Err(error);
        }

        let json_val: serde_json::Value = resp.json().await.map_err(|_| {
            let error = Self::safe_error(ProviderErrorKind::Protocol);
            trace_google_provider_failure("text", &error);
            error
        })?;

        let text = json_val["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        let finish_reason = json_val["candidates"][0]["finishReason"]
            .as_str()
            .map(|s| s.to_string());

        info!("Gemini generated text successfully with model {}", model);
        Ok(TextResponse {
            text,
            finish_reason,
        })
    }
}

#[async_trait]
impl TextModel for GoogleTextModel {
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, ProviderError> {
        if !self.auth.is_valid() {
            return Err(Self::safe_error(ProviderErrorKind::Auth));
        }

        self.try_generate_with_model(&self.model_name, &request)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn network_denial_harness_blocks_text_before_http_send() {
        let _guard = crate::test_support::deny_network_for_scope();
        let model = GoogleTextModel::new(
            GoogleAuth::new("valid-test-key".to_string()),
            "gemini-3.6-flash".to_string(),
        );
        let error = model
            .generate(TextRequest {
                prompt: "network must be denied".to_string(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(8),
            })
            .await
            .expect_err("test network denial must stop text generation before HTTP I/O");
        assert_eq!(error.kind, ProviderErrorKind::Network);
        assert_eq!(error, ProviderError::from_kind(ProviderErrorKind::Network));
    }

    #[test]
    fn status_mapping_uses_fixed_provider_categories() {
        assert_eq!(
            GoogleTextModel::classify_status(StatusCode::UNAUTHORIZED).kind,
            ProviderErrorKind::Auth
        );
        assert_eq!(
            GoogleTextModel::classify_status(StatusCode::TOO_MANY_REQUESTS).kind,
            ProviderErrorKind::Quota
        );
        assert_eq!(
            GoogleTextModel::classify_status(StatusCode::NOT_FOUND).kind,
            ProviderErrorKind::Model
        );
        assert_eq!(
            GoogleTextModel::classify_status(StatusCode::BAD_REQUEST).kind,
            ProviderErrorKind::Setup
        );
        assert_eq!(
            GoogleTextModel::classify_status(StatusCode::INTERNAL_SERVER_ERROR).kind,
            ProviderErrorKind::Network
        );
    }

    #[test]
    fn generation_request_uses_api_key_header_and_secret_free_url() {
        const KEY: &str = "AIzaSyTEXT_HEADER_ONLY_73c1";
        let model = GoogleTextModel::new(
            GoogleAuth::new(KEY.to_string()),
            "gemini-3.6-flash".to_string(),
        );
        let request = model
            .generation_request("gemini-3.6-flash", &json!({"contents": []}))
            .build()
            .unwrap();

        assert!(request.url().query().is_none());
        assert!(!request.url().as_str().contains(KEY));
        assert_eq!(
            request
                .headers()
                .get(GOOGLE_API_KEY_HEADER)
                .unwrap()
                .to_str()
                .unwrap(),
            KEY
        );
    }
}
