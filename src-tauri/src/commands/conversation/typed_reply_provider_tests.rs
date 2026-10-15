// ELSV-400: a typed message sent through the real `send_text_message` command
// reaches exactly the selected non-Local text provider (and no other), shows the
// provider's reply as the moose transcript, and fails closed with a safe error.
use super::*;
use crate::ai::types::{ProviderError, ProviderErrorKind, TextProvider};
use crate::test_support::fake_http::{
    FakeHttp, LLAMA_OK, OK, OLLAMA_OK, OPENAI_OK, UNAVAILABLE,
};
use serde_json::json;
use std::sync::Mutex as StdMutex;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::Listener;

const PROMPT: &str = "typed-prompt-ELSV400";

fn ipc_request(command: &str, body: serde_json::Value) -> InvokeRequest {
    InvokeRequest {
        cmd: command.to_string(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: if cfg!(any(windows, target_os = "android")) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        }
        .parse()
        .unwrap(),
        body: InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

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

    fn by_provider(&self, provider: TextProvider) -> &FakeHttp {
        match provider {
            TextProvider::Ollama => &self.ollama,
            TextProvider::LlamaServer => &self.llama,
            TextProvider::Openai => &self.openai,
            other => panic!("no fake for {other:?}"),
        }
    }
}

fn configured_state(provider: TextProvider, fakes: &Fakes) -> AppState {
    let state = AppState::new_for_tests().unwrap();
    *state.openai_endpoint_override.lock() =
        Some(format!("http://{}/v1/responses", fakes.openai.address));
    state
        .secrets
        .set_openai_api_key("saved-openai-key".into())
        .unwrap();
    {
        let mut settings = state.settings.write();
        settings.text_provider = provider;
        settings.ollama_server_address = fakes.ollama.origin();
        settings.llama_server_address = fakes.llama.origin();
        settings.ollama_text_model = "selected-model".into();
        settings.llama_server_text_model = "selected-model".into();
        settings.openai_text_model = "gpt-4.1-mini".into();
        settings.save_transcripts = true;
    }
    // Muted so the success path ends at the transcript and never needs a voice.
    *state.is_muted.write() = true;
    state
}

/// Runs the real command on a mock Tauri window; returns the command result and
/// every `moose://transcript/moose` payload emitted.
fn send_typed(state: AppState) -> (Result<serde_json::Value, serde_json::Value>, Vec<String>) {
    let app = mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![send_text_message])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let moose_transcripts = std::sync::Arc::new(StdMutex::new(Vec::new()));
    let sink = moose_transcripts.clone();
    app.listen_any("moose://transcript/moose", move |event| {
        sink.lock().unwrap().push(event.payload().to_string());
    });
    let result = get_ipc_response(&webview, ipc_request("send_text_message", json!({ "message": PROMPT })))
        .map(|body| body.deserialize::<serde_json::Value>().unwrap());
    let seen = moose_transcripts.lock().unwrap().clone();
    (result, seen)
}

const SELECTIONS: [(TextProvider, &str, &str); 3] = [
    (TextProvider::Ollama, "POST /api/chat", "Ollama says hi."),
    (
        TextProvider::LlamaServer,
        "POST /v1/chat/completions",
        "llama-server says hi.",
    ),
    (TextProvider::Openai, "POST /v1/responses", "OpenAI says hi."),
];

#[test]
fn typed_reply_uses_only_the_selected_provider_and_shows_its_reply() {
    for (provider, expected_request, expected_reply) in SELECTIONS {
        let fakes = Fakes::new(OK);
        let state = configured_state(provider, &fakes);
        let db = state.db.clone();
        let settings = state.settings.clone();
        let (result, transcripts) = send_typed(state);

        assert_eq!(result.unwrap(), json!(expected_reply), "{provider:?}");
        let hits = fakes.by_provider(provider).hits();
        assert_eq!(hits.len(), 1, "{provider:?}: exactly one request");
        assert!(hits[0].request_line.starts_with(expected_request), "{provider:?}: {}", hits[0].request_line);
        assert!(hits[0].body.contains(PROMPT), "{provider:?}");
        let model = if provider == TextProvider::Openai { "gpt-4.1-mini" } else { "selected-model" };
        assert!(hits[0].body.contains(model), "{provider:?}: {}", hits[0].body);
        for other in [TextProvider::Ollama, TextProvider::LlamaServer, TextProvider::Openai] {
            if other != provider {
                assert_eq!(
                    fakes.by_provider(other).connection_count(),
                    0,
                    "{provider:?} must never contact {other:?}"
                );
            }
        }
        // The reply is shown as the moose transcript and persisted when retention is on.
        assert_eq!(transcripts, vec![json!(expected_reply).to_string()], "{provider:?}");
        let stored = db.get_transcripts(10).unwrap();
        assert!(stored.iter().any(|t| t.role == "moose" && t.text == expected_reply));
        assert_eq!(settings.read().text_provider, provider);
    }
}

#[test]
fn typed_reply_provider_failure_is_safe_and_never_falls_back() {
    for (provider, _, _) in SELECTIONS {
        let fakes = Fakes::new(UNAVAILABLE);
        let state = configured_state(provider, &fakes);
        let settings = state.settings.clone();
        let character_state = state.character_state.clone();
        let db = state.db.clone();
        let (result, transcripts) = send_typed(state);

        let error = result.expect_err("503 must fail the typed message");
        let error = error.as_str().unwrap().to_string();
        assert_eq!(
            error,
            ProviderError::from_kind(ProviderErrorKind::Network).to_string(),
            "{provider:?}"
        );
        for secret in [PROMPT, "saved-openai-key", "127.0.0.1"] {
            assert!(!error.contains(secret), "{provider:?} leaked {secret}");
        }
        assert_eq!(fakes.by_provider(provider).hits().len(), 1, "{provider:?}: one attempt, no retry");
        for other in [TextProvider::Ollama, TextProvider::LlamaServer, TextProvider::Openai] {
            if other != provider {
                assert_eq!(fakes.by_provider(other).connection_count(), 0, "{provider:?} -> {other:?}");
            }
        }
        assert!(transcripts.is_empty(), "no reply may be shown on failure");
        assert!(!db.get_transcripts(10).unwrap().iter().any(|t| t.role == "moose"));
        assert_eq!(settings.read().text_provider, provider, "failure must not switch providers");
        assert_eq!(*character_state.read(), CharacterState::Idle);
    }
}
