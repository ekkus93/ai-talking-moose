use super::super::*;
use crate::ai::types::LiveSessionConfig;

#[tokio::test]
async fn google_text_with_local_asr_does_not_open_gemini_live() {
    let state = AppState::new_for_tests().unwrap();
    let settings = AppSettings {
        text_provider: TextProvider::Google,
        asr_mode: AsrMode::WhisperSmall,
        tts_provider: TtsProvider::Local,
        ..AppSettings::default()
    };
    let provider = state.get_conversation_provider_for(&settings);
    let (sender, _receiver) = tokio::sync::mpsc::channel(4);
    let session = provider
        .connect(
            LiveSessionConfig {
                model: settings.google_text_model.clone(),
                voice_name: Some(settings.local_tts_voice.clone()),
                system_instruction: None,
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
                tools: vec![],
            },
            sender,
        )
        .await;
    assert!(
        session.is_ok(),
        "local ASR must not attempt Google Live authentication"
    );
}
