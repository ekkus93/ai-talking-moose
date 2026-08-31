pub mod catalog;
pub mod installer;

use crate::ai::traits::TextModel;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextRequest, TextResponse};
use async_trait::async_trait;

pub use catalog::{
    local_model_entry, validate_local_model_catalog, LocalModelCatalogEntry,
    LocalModelTemplateHint, DEFAULT_LOCAL_TEXT_MODEL_ID, LOCAL_MODEL_CATALOG,
};
pub use installer::{
    global_local_model_installer, initialize_global_local_model_installer, LocalModelDescriptor,
    LocalModelDiagnostics, LocalModelInstallError, LocalModelInstallErrorKind,
    LocalModelInstallOutcome, LocalModelInstallProgress, LocalModelInstallProgressCallback,
    LocalModelInstallState, LocalModelInstaller,
};

/// Fail-closed Local text provider used while the native runtime/model installer is not yet wired.
///
/// This is intentionally not a fake response generator. Selecting Local before P5-P6 are complete
/// returns a stable model error and never substitutes Google or Fake output.
pub struct UnavailableLocalTextModel {
    model_id: String,
}

impl UnavailableLocalTextModel {
    pub fn new(model_id: String) -> Self {
        Self { model_id }
    }

    #[cfg(test)]
    fn model_id(&self) -> &str {
        &self.model_id
    }
}

#[async_trait]
impl TextModel for UnavailableLocalTextModel {
    async fn generate(&self, _request: TextRequest) -> Result<TextResponse, ProviderError> {
        let _ = &self.model_id;
        Err(ProviderError::from_kind(ProviderErrorKind::Model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_local_model_fails_closed_without_manufactured_text() {
        let model = UnavailableLocalTextModel::new(DEFAULT_LOCAL_TEXT_MODEL_ID.to_string());
        assert_eq!(model.model_id(), DEFAULT_LOCAL_TEXT_MODEL_ID);

        let error = model
            .generate(TextRequest {
                prompt: "local prompt sentinel".to_string(),
                system_instruction: Some("local system sentinel".to_string()),
                temperature: Some(0.8),
                max_tokens: Some(32),
            })
            .await
            .expect_err("unavailable local runtime must fail rather than manufacture output");
        assert_eq!(error.kind, ProviderErrorKind::Model);
        assert!(!error.retryable);
    }
}
