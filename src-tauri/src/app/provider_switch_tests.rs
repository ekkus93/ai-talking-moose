use super::state::AppState;
use crate::ai::types::{ProviderErrorKind, TextProvider, TextRequest};

fn request(prompt: &str) -> TextRequest {
    TextRequest {
        prompt: prompt.to_string(),
        system_instruction: None,
        temperature: Some(0.2),
        max_tokens: Some(8),
    }
}

#[tokio::test]
async fn text_provider_switch_does_not_retarget_an_already_built_model() {
    let state = AppState::new_for_tests().unwrap();
    {
        let mut settings = state.settings.write();
        settings.text_provider = TextProvider::Local;
        settings.local_text_model = "missing-local-model".to_string();
    }

    // `get_text_model()` captures the provider/model choice for one generation. A settings
    // change can affect the next generation, but it must not retarget work already constructed.
    let captured_local = state.get_text_model();
    state.settings.write().text_provider = TextProvider::Google;

    let captured_error = captured_local
        .generate(request("captured local request"))
        .await
        .expect_err("captured Local request must remain Local after settings switch");
    assert_eq!(captured_error.kind, ProviderErrorKind::Model);

    let next_error = state
        .get_text_model()
        .generate(request("next provider request"))
        .await
        .expect_err("next request must use the newly selected Google provider");
    assert_eq!(next_error.kind, ProviderErrorKind::Auth);
}

#[test]
fn provider_switch_preserves_both_provider_specific_model_choices() {
    let state = AppState::new_for_tests().unwrap();
    {
        let mut settings = state.settings.write();
        settings.google_text_model = "gemini-3.6-flash".to_string();
        settings.local_text_model = "qwen3-0-6b-instruct-q4-k-m".to_string();
        settings.text_provider = TextProvider::Local;
    }

    state.settings.write().text_provider = TextProvider::Google;
    let settings = state.settings.read();
    assert_eq!(settings.google_text_model, "gemini-3.6-flash");
    assert_eq!(
        settings.local_text_model,
        "qwen3-0-6b-instruct-q4-k-m"
    );
}
