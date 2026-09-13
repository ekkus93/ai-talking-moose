use talking_moose_lib::ai::local_tts::{
    DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE,
};
use talking_moose_lib::ai::traits::SpeechSynthesizer;
use talking_moose_lib::ai::types::{ProviderErrorKind, TtsProvider, TtsRequest};
use talking_moose_lib::app::state::AppState;
use tokio_util::sync::CancellationToken;

fn request(text: &str) -> TtsRequest {
    TtsRequest {
        text: text.to_string(),
        voice_name: None,
        speaking_rate: Some(1.0),
        pitch: None,
    }
}

#[tokio::test]
async fn app_state_google_tts_helper_fails_auth_without_saved_key() {
    let state = AppState::new(None).unwrap();
    state.settings.write().tts_provider = TtsProvider::Google;

    let error = state
        .get_speech_synthesizer()
        .synthesize(request("private google helper text"))
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Auth);
    assert!(!error.message.contains("private google helper text"));
}

#[tokio::test]
async fn app_state_local_tts_helper_uses_real_local_provider_without_cloud_fallback() {
    let state = AppState::new(None).unwrap();
    state.settings.write().tts_provider = TtsProvider::Local;

    let error = state
        .get_speech_synthesizer()
        .synthesize(TtsRequest {
            text: "private local helper text".to_string(),
            voice_name: Some(DEFAULT_LOCAL_TTS_VOICE.to_string()),
            speaking_rate: Some(1.0),
            pitch: None,
        })
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Setup);
    assert_ne!(
        error.message,
        "Local TTS runtime is not available in this build yet."
    );
    assert!(!error.message.contains("private local helper text"));
}

#[tokio::test]
async fn app_state_local_tts_helper_rejects_unknown_model_before_runtime_use() {
    let state = AppState::new(None).unwrap();
    {
        let mut settings = state.settings.write();
        settings.tts_provider = TtsProvider::Local;
        settings.local_tts_model = "missing-local-model".to_string();
    }

    let error = state
        .get_speech_synthesizer()
        .synthesize(request("private unknown local model text"))
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Model);
    assert!(!error.message.contains("private unknown local model text"));
    assert!(!error.message.contains(DEFAULT_LOCAL_TTS_MODEL_ID));
}

#[tokio::test]
async fn app_state_local_tts_helper_reports_provider_neutral_cancellation() {
    let state = AppState::new(None).unwrap();
    state.settings.write().tts_provider = TtsProvider::Local;
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let error = state
        .get_speech_synthesizer()
        .synthesize_cancellable(request("private cancelled local helper text"), &cancellation)
        .await
        .unwrap_err();

    assert_eq!(error.kind, ProviderErrorKind::Cancelled);
    assert!(!error.retryable);
    assert!(!error.message.contains("private cancelled local helper text"));
}
