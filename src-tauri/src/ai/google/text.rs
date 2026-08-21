use crate::ai::google::auth::GoogleAuth;
use crate::ai::traits::TextModel;
use crate::ai::types::{TextRequest, TextResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::{error, info};

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

    async fn try_generate_with_model(
        &self,
        model: &str,
        request: &TextRequest,
    ) -> Result<TextResponse, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.auth.api_key
        );

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

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                format!(
                    "HTTP request to Gemini failed: {}",
                    self.auth.redact(&e.to_string())
                )
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            error!("Gemini API error with model {}: {}", model, status);
            return Err(format!("Gemini API returned error code {}", status));
        }

        let json_val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response JSON: {}", e))?;

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
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, String> {
        if !self.auth.is_valid() {
            return Err("Google API key is not configured".to_string());
        }

        self.try_generate_with_model(&self.model_name, &request)
            .await
    }
}
