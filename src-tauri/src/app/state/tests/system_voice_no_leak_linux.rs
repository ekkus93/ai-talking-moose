//! ELSV-400: an unsupported OS System voice cannot leak a transcript to any
//! configured text backend, even if credentials and loopback endpoints exist.
use super::super::*;
use crate::ai::types::{LiveSessionConfig, ProviderErrorKind};
use crate::test_support::fake_http::{FakeHttp, LLAMA_OK, OK, OLLAMA_OK, OPENAI_OK};

#[tokio::test]
async fn unavailable_system_voice_blocks_all_local_asr_and_text_connections() {
    let ollama = FakeHttp::spawn(OK, OLLAMA_OK);
    let llama = FakeHttp::spawn(OK, LLAMA_OK);
    let openai = FakeHttp::spawn(OK, OPENAI_OK);
    let state = AppState::new_for_tests().unwrap();
    *state.openai_endpoint_override.lock() =
        Some(format!("http://{}/v1/responses", openai.address));
    state
        .secrets
        .set_google_api_key("saved-google-test-key".into())
        .unwrap();
    state
        .secrets
        .set_openai_api_key("saved-openai-test-key".into())
        .unwrap();
    for asr_mode in [AsrMode::WhisperSmall] {
        for text_provider in [
            TextProvider::Local,
            TextProvider::Google,
            TextProvider::Openai,
            TextProvider::Ollama,
            TextProvider::LlamaServer,
        ] {
            let settings = AppSettings {
                asr_mode,
                text_provider,
                tts_provider: TtsProvider::System,
                ollama_server_address: ollama.origin(),
                llama_server_address: llama.origin(),
                ..AppSettings::default()
            };
            let (sender, mut events) = tokio::sync::mpsc::channel(4);
            let result = state
                .get_conversation_provider_for(&settings)
                .connect(
                    LiveSessionConfig {
                        model: "selected-model".into(),
                        voice_name: Some("system_default".into()),
                        system_instruction: Some("private instruction".into()),
                        sample_rate_in: 16_000,
                        sample_rate_out: 24_000,
                        tools: vec![],
                    },
                    sender,
                )
                .await;
            let error = match result {
                Ok(_) => panic!("unavailable System TTS must reject before any text request"),
                Err(error) => error,
            };
            assert_eq!(
                error.kind,
                ProviderErrorKind::Setup,
                "{asr_mode:?} {text_provider:?} must fail closed"
            );
            assert!(!error.retryable);
            assert!(!error.message.contains("private instruction"));
            assert!(events.try_recv().is_err(), "failed Talk must not emit events");
        }
    }
    assert_eq!(ollama.connection_count(), 0, "Ollama must not be contacted");
    assert_eq!(llama.connection_count(), 0, "llama-server must not be contacted");
    assert_eq!(openai.connection_count(), 0, "OpenAI must not be contacted");
}
