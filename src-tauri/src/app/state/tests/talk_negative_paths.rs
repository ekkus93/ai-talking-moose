//! ELSV-400: Talk negative paths and selection capture. A failing voice never
//! falls back to a different voice provider, and a conversation provider keeps
//! the selection it was created with even if settings change afterwards.
use super::super::*;
use crate::ai::types::{LiveServerEvent, LiveSessionConfig, ProviderError, ProviderErrorKind};
use crate::test_support::fake_http::{FakeHttp, OK, OLLAMA_OK, OPENAI_OK};
use std::time::Duration;

fn config() -> LiveSessionConfig {
    LiveSessionConfig {
        model: "selected-model".into(),
        voice_name: None,
        system_instruction: None,
        sample_rate_in: 16_000,
        sample_rate_out: 24_000,
        tools: vec![],
    }
}

fn base_settings(ollama: &FakeHttp, tts_provider: TtsProvider) -> AppSettings {
    AppSettings {
        text_provider: TextProvider::Ollama,
        tts_provider,
        asr_mode: AsrMode::WhisperSmall,
        ollama_server_address: ollama.origin(),
        ollama_text_model: "selected-model".into(),
        ..AppSettings::default()
    }
}

/// Connects, sends one text turn, and returns every event up to the first
/// terminal one (`TurnComplete` or `Error`) plus anything that trails it.
async fn run_turn(
    provider: Arc<dyn crate::ai::traits::RealtimeConversationProvider>,
) -> Vec<LiveServerEvent> {
    let (sender, mut events) = tokio::sync::mpsc::channel(16);
    let mut session = provider.connect(config(), sender).await.unwrap();
    assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
    session.send_text_turn("hello moose").await.unwrap();
    let mut seen = Vec::new();
    loop {
        let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("turn must finish")
            .expect("conversation closed early");
        let terminal = matches!(event, LiveServerEvent::TurnComplete | LiveServerEvent::Error(_));
        seen.push(event);
        if terminal {
            break;
        }
    }
    while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), events.recv()).await {
        seen.push(event);
    }
    session.close().await.unwrap();
    seen
}

fn error_of(events: &[LiveServerEvent]) -> &ProviderError {
    events
        .iter()
        .find_map(|e| match e {
            LiveServerEvent::Error(error) => Some(error),
            _ => None,
        })
        .expect("an Error event")
}

#[tokio::test]
async fn google_tts_without_a_saved_key_reports_auth_and_never_uses_local_tts() {
    let ollama = FakeHttp::spawn(OK, OLLAMA_OK);
    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_google_api_key());
    let settings = base_settings(&ollama, TtsProvider::Google);
    let events = run_turn(state.get_conversation_provider_for(&settings)).await;

    // Pinned behavior: the selected text provider runs first (the key check
    // lives in the synthesizer), its reply is shown as the final transcript, and
    // only then does Google TTS fail with Auth. No audio and no completion.
    assert_eq!(ollama.hits().len(), 1);
    assert!(matches!(
        events.as_slice(),
        [LiveServerEvent::ModelTranscript(t), LiveServerEvent::Error(_)]
            if t.text == "Ollama says hi." && t.is_final
    ), "unexpected events");
    let error = error_of(&events);
    assert_eq!(error.kind, ProviderErrorKind::Auth);
    // A Local TTS fallback would surface a Local-TTS Setup/Model error or audio.
    assert!(!events.iter().any(|e| matches!(
        e,
        LiveServerEvent::AudioData(_) | LiveServerEvent::TurnComplete
    )));
}

#[tokio::test]
async fn local_tts_not_installed_reports_setup_or_model_and_never_uses_google_tts() {
    let ollama = FakeHttp::spawn(OK, OLLAMA_OK);
    let state = AppState::new_for_tests().unwrap();
    // A saved Google key proves the absence of fallback: it would make Google TTS viable.
    state.secrets.set_google_api_key("saved-google-key".into()).unwrap();
    let settings = base_settings(&ollama, TtsProvider::Local);
    let events = run_turn(state.get_conversation_provider_for(&settings)).await;

    assert_eq!(ollama.hits().len(), 1);
    let error = error_of(&events);
    assert!(
        matches!(error.kind, ProviderErrorKind::Setup | ProviderErrorKind::Model),
        "{:?}",
        error.kind
    );
    assert!(!error.message.contains("saved-google-key"));
    assert!(!events.iter().any(|e| matches!(
        e,
        LiveServerEvent::AudioData(_) | LiveServerEvent::TurnComplete
    )), "no audio may come from a fallback voice");
}

#[tokio::test]
async fn created_conversation_provider_keeps_its_original_selection_after_settings_change() {
    let ollama = FakeHttp::spawn(OK, OLLAMA_OK);
    let openai = FakeHttp::spawn(OK, OPENAI_OK);
    let state = AppState::new_for_tests().unwrap();
    *state.openai_endpoint_override.lock() =
        Some(format!("http://{}/v1/responses", openai.address));
    state.secrets.set_openai_api_key("saved-openai-key".into()).unwrap();
    state.settings.write().clone_from(&base_settings(&ollama, TtsProvider::Google));

    let provider = state.get_conversation_provider_for(&state.settings.read().clone());

    // The user flips both selections after the provider was captured.
    {
        let mut settings = state.settings.write();
        settings.text_provider = TextProvider::Openai;
        settings.tts_provider = TtsProvider::Local;
    }
    state.secrets.set_google_api_key("late-google-key".into()).unwrap();

    let events = run_turn(provider).await;

    assert_eq!(ollama.hits().len(), 1, "original text provider is still used");
    assert_eq!(openai.connection_count(), 0, "the new selection must not leak in");
    // Original TTS (Google) was built with the then-empty key, so it still fails Auth;
    // had the new selection (Local) applied, the error would be Setup/Model instead.
    assert_eq!(error_of(&events).kind, ProviderErrorKind::Auth);
    assert_eq!(state.settings.read().text_provider, TextProvider::Openai);
}

#[tokio::test]
async fn whisper_provider_snapshot_stays_non_live_after_switch_to_gemini_audio() {
    let ollama = FakeHttp::spawn(OK, OLLAMA_OK);
    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_google_api_key());
    let settings = base_settings(&ollama, TtsProvider::Google);
    *state.settings.write() = settings.clone();

    // Capture a non-Live Whisper conversation before changing all three
    // independent routing choices. A later Gemini Live selection must not
    // convert its existing microphone channel into a cloud audio session.
    let provider = state.get_conversation_provider_for(&settings);
    {
        let mut current = state.settings.write();
        current.asr_mode = AsrMode::GeminiLiveAudio;
        current.text_provider = TextProvider::Google;
        current.tts_provider = TtsProvider::Local;
    }

    let (sender, mut events) = tokio::sync::mpsc::channel(16);
    let mut session = provider.connect(config(), sender).await.unwrap();
    assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
    assert_eq!(
        session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
        ProviderErrorKind::Setup,
        "existing Whisper session must reject Live PCM after settings change"
    );
    assert!(events.try_recv().is_err(), "rejected PCM must emit no Live event");

    session.send_text_turn("hello moose").await.unwrap();
    let transcript = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("original text provider should respond");
    assert!(matches!(
        transcript,
        Some(LiveServerEvent::ModelTranscript(ref t))
            if t.text == "Ollama says hi." && t.is_final
    ));
    let failure = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("original Google TTS with no key must reject");
    assert!(matches!(
        failure,
        Some(LiveServerEvent::Error(ref err)) if err.kind == ProviderErrorKind::Auth
    ));
    assert_eq!(ollama.hits().len(), 1, "must use the original Ollama provider");
    assert_eq!(state.settings.read().asr_mode, AsrMode::GeminiLiveAudio);
    session.close().await.unwrap();
}
