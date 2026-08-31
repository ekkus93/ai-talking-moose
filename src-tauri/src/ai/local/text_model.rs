use super::catalog::local_model_entry;
use super::installer::{LocalModelInstallError, LocalModelInstaller};
use super::runtime::types::{
    LocalRuntimeError, LocalRuntimeErrorKind, LocalRuntimeGenerateRequest, LocalRuntimeGeneration,
};
use super::runtime::LocalRuntimeManager;
use crate::ai::traits::TextModel;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextRequest, TextResponse};
use async_trait::async_trait;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const DEFAULT_LOCAL_TEMPERATURE: f32 = 0.8;
const RANDOM_GENERATION_SEED: u32 = u32::MAX;

#[async_trait]
trait LocalGenerationRuntime: Send + Sync {
    async fn generate(
        &self,
        installer: Arc<LocalModelInstaller>,
        request: LocalRuntimeGenerateRequest,
        cancellation: CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError>;
}

#[async_trait]
impl LocalGenerationRuntime for LocalRuntimeManager {
    async fn generate(
        &self,
        installer: Arc<LocalModelInstaller>,
        request: LocalRuntimeGenerateRequest,
        cancellation: CancellationToken,
    ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
        LocalRuntimeManager::generate(self, installer, request, cancellation).await
    }
}

enum InstallerState {
    Ready(Arc<LocalModelInstaller>),
    Unavailable(ProviderError),
}

/// Provider-neutral text model backed by the application-owned local llama.cpp runtime.
///
/// The model keeps system/user content structured until it reaches the runtime. The native
/// llama.cpp layer is responsible for applying the selected GGUF's embedded chat template; this
/// type never invents or concatenates model control tokens.
pub struct LocalTextModel {
    runtime: Arc<dyn LocalGenerationRuntime>,
    installer: InstallerState,
    model_id: String,
}

impl LocalTextModel {
    pub(crate) fn new(
        runtime: Arc<LocalRuntimeManager>,
        installer: Result<Arc<LocalModelInstaller>, LocalModelInstallError>,
        model_id: String,
    ) -> Self {
        Self::from_runtime(runtime, installer, model_id)
    }

    fn from_runtime(
        runtime: Arc<dyn LocalGenerationRuntime>,
        installer: Result<Arc<LocalModelInstaller>, LocalModelInstallError>,
        model_id: String,
    ) -> Self {
        let installer = match installer {
            Ok(installer) => InstallerState::Ready(installer),
            Err(_) => {
                InstallerState::Unavailable(ProviderError::from_kind(ProviderErrorKind::Setup))
            }
        };
        Self {
            runtime,
            installer,
            model_id,
        }
    }

    fn map_runtime_error(error: LocalRuntimeError) -> ProviderError {
        let kind = match error.kind {
            LocalRuntimeErrorKind::ShuttingDown | LocalRuntimeErrorKind::Cancelled => {
                ProviderErrorKind::Closed
            }
            LocalRuntimeErrorKind::UnknownModel
            | LocalRuntimeErrorKind::ModelNotInstalled
            | LocalRuntimeErrorKind::UnsafeArtifact
            | LocalRuntimeErrorKind::ModelLoad
            | LocalRuntimeErrorKind::ModelNotLoaded
            | LocalRuntimeErrorKind::ChatTemplate => ProviderErrorKind::Model,
            LocalRuntimeErrorKind::InvalidRequest | LocalRuntimeErrorKind::PromptTooLong => {
                ProviderErrorKind::Setup
            }
            LocalRuntimeErrorKind::Initialization
            | LocalRuntimeErrorKind::ContextCreation
            | LocalRuntimeErrorKind::Tokenization
            | LocalRuntimeErrorKind::Decode
            | LocalRuntimeErrorKind::OutputDecode
            | LocalRuntimeErrorKind::ModelDelete => ProviderErrorKind::Internal,
        };
        ProviderError::from_kind(kind)
    }

    fn runtime_request(
        &self,
        request: TextRequest,
    ) -> Result<LocalRuntimeGenerateRequest, ProviderError> {
        let entry = local_model_entry(&self.model_id)
            .ok_or_else(|| ProviderError::from_kind(ProviderErrorKind::Model))?;
        let max_output_tokens = request
            .max_tokens
            .unwrap_or(entry.recommended_max_output)
            .min(entry.recommended_max_output);
        Ok(LocalRuntimeGenerateRequest {
            model_id: self.model_id.clone(),
            system_instruction: request.system_instruction,
            prompt: request.prompt,
            temperature: request.temperature.unwrap_or(DEFAULT_LOCAL_TEMPERATURE),
            max_output_tokens,
            seed: RANDOM_GENERATION_SEED,
        })
    }
}

#[async_trait]
impl TextModel for LocalTextModel {
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, ProviderError> {
        let runtime_request = self.runtime_request(request)?;
        let installer = match &self.installer {
            InstallerState::Ready(installer) => installer.clone(),
            InstallerState::Unavailable(error) => return Err(error.clone()),
        };
        let generation = self
            .runtime
            .generate(installer, runtime_request, CancellationToken::new())
            .await
            .map_err(Self::map_runtime_error)?;
        Ok(TextResponse {
            text: generation.text,
            finish_reason: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use tempfile::tempdir;

    struct FakeRuntime {
        requests: Mutex<Vec<LocalRuntimeGenerateRequest>>,
        result: Result<LocalRuntimeGeneration, LocalRuntimeError>,
    }

    #[async_trait]
    impl LocalGenerationRuntime for FakeRuntime {
        async fn generate(
            &self,
            _installer: Arc<LocalModelInstaller>,
            request: LocalRuntimeGenerateRequest,
            _cancellation: CancellationToken,
        ) -> Result<LocalRuntimeGeneration, LocalRuntimeError> {
            self.requests.lock().push(request);
            self.result.clone()
        }
    }

    fn generation(text: &str) -> LocalRuntimeGeneration {
        LocalRuntimeGeneration {
            text: text.to_string(),
            prompt_tokens: 7,
            output_tokens: 3,
            duration_ms: 4,
            tokens_per_second: Some(750.0),
        }
    }

    fn runtime_with_result(
        result: Result<LocalRuntimeGeneration, LocalRuntimeError>,
    ) -> Arc<FakeRuntime> {
        Arc::new(FakeRuntime {
            requests: Mutex::new(Vec::new()),
            result,
        })
    }

    #[tokio::test]
    async fn maps_provider_neutral_request_into_bounded_local_runtime_request() {
        let dir = tempdir().unwrap();
        let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
        let runtime = runtime_with_result(Ok(generation("moose reply")));
        let model = LocalTextModel::from_runtime(
            runtime.clone(),
            Ok(installer),
            super::super::DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
        );

        let response = model
            .generate(TextRequest {
                prompt: "user prompt".to_string(),
                system_instruction: Some("system prompt".to_string()),
                temperature: Some(0.55),
                max_tokens: Some(32),
            })
            .await
            .unwrap();

        assert_eq!(response.text, "moose reply");
        assert_eq!(response.finish_reason, None);
        let requests = runtime.requests.lock();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.model_id, super::super::DEFAULT_LOCAL_TEXT_MODEL_ID);
        assert_eq!(request.system_instruction.as_deref(), Some("system prompt"));
        assert_eq!(request.prompt, "user prompt");
        assert_eq!(request.temperature, 0.55);
        assert_eq!(request.max_output_tokens, 32);
        assert_eq!(request.seed, RANDOM_GENERATION_SEED);
    }

    #[tokio::test]
    async fn defaults_temperature_and_caps_output_to_catalog_bound() {
        let dir = tempdir().unwrap();
        let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
        let runtime = runtime_with_result(Ok(generation("bounded")));
        let model = LocalTextModel::from_runtime(
            runtime.clone(),
            Ok(installer),
            super::super::DEFAULT_LOCAL_TEXT_MODEL_ID.to_string(),
        );
        let entry = local_model_entry(super::super::DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();

        model
            .generate(TextRequest {
                prompt: "bounded request".to_string(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(entry.recommended_max_output + 500),
            })
            .await
            .unwrap();

        let requests = runtime.requests.lock();
        assert_eq!(requests[0].temperature, DEFAULT_LOCAL_TEMPERATURE);
        assert_eq!(requests[0].max_output_tokens, entry.recommended_max_output);
    }

    #[tokio::test]
    async fn unknown_model_fails_before_runtime_and_never_falls_back() {
        let dir = tempdir().unwrap();
        let installer = Arc::new(LocalModelInstaller::new(dir.path().to_path_buf()).unwrap());
        let runtime = runtime_with_result(Ok(generation("must not be returned")));
        let model = LocalTextModel::from_runtime(
            runtime.clone(),
            Ok(installer),
            "missing-local-model".to_string(),
        );

        let error = model
            .generate(TextRequest {
                prompt: "stay local".to_string(),
                system_instruction: None,
                temperature: None,
                max_tokens: Some(8),
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Model);
        assert!(runtime.requests.lock().is_empty());
    }

    #[test]
    fn runtime_errors_map_to_stable_safe_provider_categories() {
        let cases = [
            (
                LocalRuntimeErrorKind::ModelNotInstalled,
                ProviderErrorKind::Model,
            ),
            (
                LocalRuntimeErrorKind::ChatTemplate,
                ProviderErrorKind::Model,
            ),
            (
                LocalRuntimeErrorKind::PromptTooLong,
                ProviderErrorKind::Setup,
            ),
            (LocalRuntimeErrorKind::Decode, ProviderErrorKind::Internal),
            (LocalRuntimeErrorKind::Cancelled, ProviderErrorKind::Closed),
        ];
        for (runtime_kind, provider_kind) in cases {
            let mapped = LocalTextModel::map_runtime_error(LocalRuntimeError {
                kind: runtime_kind,
                message: "sensitive runtime detail must not escape",
            });
            assert_eq!(mapped.kind, provider_kind);
            assert!(!mapped.message.contains("sensitive runtime detail"));
        }
    }
}
