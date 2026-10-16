//! ELSV-400: ambient remarks use only the selected non-Local text provider.
//! Success is observed at the generation boundary `process_ambient_event` uses;
//! failure is observed end to end through `process_ambient_event` itself.
use super::*;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextProvider};
use crate::test_support::fake_http::{FakeHttp, LLAMA_OK, OK, OLLAMA_OK, OPENAI_OK, UNAVAILABLE};

const PROMPT: &str = "ambient-prompt-ELSV400";
const ALL: [TextProvider; 3] = [
    TextProvider::Ollama,
    TextProvider::LlamaServer,
    TextProvider::Openai,
];

struct Fakes {
    ollama: FakeHttp,
    llama: FakeHttp,
    openai: FakeHttp,
}

impl Fakes {
    fn new(status: &'static str) -> Self {
        let ok_or = |ok: &'static str| if status == OK { ok } else { "{}" };
        Self {
            ollama: FakeHttp::spawn(status, ok_or(OLLAMA_OK)),
            llama: FakeHttp::spawn(status, ok_or(LLAMA_OK)),
            openai: FakeHttp::spawn(status, ok_or(OPENAI_OK)),
        }
    }

    fn get(&self, provider: TextProvider) -> &FakeHttp {
        match provider {
            TextProvider::Ollama => &self.ollama,
            TextProvider::LlamaServer => &self.llama,
            _ => &self.openai,
        }
    }

    fn state(&self, provider: TextProvider) -> AppState {
        let state = AppState::new_for_tests().unwrap();
        *state.openai_endpoint_override.lock() =
            Some(format!("http://{}/v1/responses", self.openai.address));
        state
            .secrets
            .set_openai_api_key("saved-openai-key".into())
            .unwrap();
        let mut settings = state.settings.write();
        settings.text_provider = provider;
        // Keep provider-routing tests independent of the CI runner clock: a
        // quiet-hours denial returns Ok(None) before the expected HTTP 503.
        settings.quiet_hours_enabled = false;
        settings.ollama_server_address = self.ollama.origin();
        settings.llama_server_address = self.llama.origin();
        settings.ollama_text_model = "selected-model".into();
        settings.llama_server_text_model = "selected-model".into();
        settings.openai_text_model = "gpt-4.1-mini".into();
        drop(settings);
        state
    }

    fn assert_only_contacted(&self, selected: TextProvider) {
        for other in ALL {
            if other != selected {
                assert_eq!(
                    self.get(other).connection_count(),
                    0,
                    "{selected:?} must never contact {other:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn ambient_generation_reaches_only_the_selected_provider() {
    for (provider, path, reply) in [
        (TextProvider::Ollama, "POST /api/chat", "Ollama says hi."),
        (
            TextProvider::LlamaServer,
            "POST /v1/chat/completions",
            "llama-server says hi.",
        ),
        (
            TextProvider::Openai,
            "POST /v1/responses",
            "OpenAI says hi.",
        ),
    ] {
        let fakes = Fakes::new(OK);
        let state = fakes.state(provider);
        let text = generate_ambient_text(&state, PROMPT.to_string())
            .await
            .unwrap();
        assert_eq!(text, reply, "{provider:?}");
        let hits = fakes.get(provider).hits();
        assert_eq!(hits.len(), 1, "{provider:?}");
        assert!(
            hits[0].request_line.starts_with(path),
            "{}",
            hits[0].request_line
        );
        assert!(hits[0].body.contains(PROMPT));
        fakes.assert_only_contacted(provider);
        assert_eq!(state.settings.read().text_provider, provider);
    }
}

#[test]
fn ambient_provider_failure_is_safe_restores_presentation_and_never_falls_back() {
    use tauri::test::{mock_builder, mock_context, noop_assets};
    for provider in ALL {
        let fakes = Fakes::new(UNAVAILABLE);
        let state = fakes.state(provider);
        let settings = state.settings.clone();
        let character_state = state.character_state.clone();
        let app = mock_builder().build(mock_context(noop_assets())).unwrap();
        let handle = app.handle().clone();
        let event = AmbientEvent::new("manual", PROMPT.to_string(), 1.0);
        // Run through the scheduler exactly as production does; it owns the
        // presentation lease that process_ambient_event requires.
        let (task_state, task_handle) = (state.clone(), handle.clone());
        state
            .ambient_scheduler
            .start(move |event| {
                let (state, handle) = (task_state.clone(), task_handle.clone());
                async move { process_ambient_event(event, &state, &handle).await }
            })
            .unwrap();
        let scheduler = state.ambient_scheduler.clone();
        let result = tauri::async_runtime::block_on(scheduler.submit(event));

        let error = result.expect_err("503 must fail the ambient remark");
        assert_eq!(
            error,
            ProviderError::from_kind(ProviderErrorKind::Network).to_string(),
            "{provider:?}"
        );
        assert!(!error.contains(PROMPT) && !error.contains("saved-openai-key"));
        assert_eq!(
            fakes.get(provider).hits().len(),
            1,
            "{provider:?}: no retry"
        );
        fakes.assert_only_contacted(provider);
        assert_eq!(settings.read().text_provider, provider);
        assert!(
            matches!(
                *character_state.read(),
                CharacterState::Idle | CharacterState::Hidden
            ),
            "{provider:?}: presentation must be restored, got {:?}",
            *character_state.read()
        );
    }
}
