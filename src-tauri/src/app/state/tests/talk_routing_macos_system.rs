//! Native System Voices remain non-Live for every local ASR and text-model
//! selection. Real PCM synthesis is independently checked by the Mac workflow.
use super::super::*;
use crate::ai::types::{LiveServerEvent, LiveSessionConfig, ProviderErrorKind};

#[tokio::test]
async fn system_voice_is_non_live_for_all_local_asr_text_pairs() {
    let state = AppState::new_for_tests().unwrap();
    assert!(crate::ai::system_tts::is_supported());
    for asr_mode in [
        AsrMode::WhisperSmall,
        AsrMode::MoonshineTinyStreaming,
        AsrMode::MoonshineSmallStreaming,
    ] {
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
                ..AppSettings::default()
            };
            let (sender, mut events) = tokio::sync::mpsc::channel(4);
            let mut session = state
                .get_conversation_provider_for(&settings)
                .connect(
                    LiveSessionConfig {
                        model: "selected-model".into(),
                        voice_name: Some("system_default".into()),
                        system_instruction: None,
                        sample_rate_in: 16_000,
                        sample_rate_out: 24_000,
                        tools: vec![],
                    },
                    sender,
                )
                .await
                .expect("supported System TTS must connect to non-Live Talk");
            assert!(
                matches!(events.recv().await, Some(LiveServerEvent::Connected)),
                "{asr_mode:?} {text_provider:?}"
            );
            assert_eq!(
                session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
                ProviderErrorKind::Setup,
                "{asr_mode:?} {text_provider:?} must not open Gemini Live"
            );
            session.close().await.unwrap();
        }
    }
}
