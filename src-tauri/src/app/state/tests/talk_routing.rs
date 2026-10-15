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

#[tokio::test]
async fn all_text_providers_with_local_asr_connect_without_gemini_live() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();
    for text_provider in [
        TextProvider::Local,
        TextProvider::Google,
        TextProvider::Openai,
        TextProvider::Ollama,
        TextProvider::LlamaServer,
    ] {
        for tts_provider in [TtsProvider::Local, TtsProvider::Google] {
            let settings = AppSettings {
                text_provider,
                tts_provider,
                asr_mode: AsrMode::WhisperSmall,
                ..AppSettings::default()
            };
            let provider = state.get_conversation_provider_for(&settings);
            let (sender, mut events) = tokio::sync::mpsc::channel(4);
            let mut session = provider
                .connect(
                    LiveSessionConfig {
                        model: settings.local_text_model.clone(),
                        voice_name: Some(settings.local_tts_voice.clone()),
                        system_instruction: None,
                        sample_rate_in: 16_000,
                        sample_rate_out: 24_000,
                        tools: vec![],
                    },
                    sender,
                )
                .await
                .expect("local ASR must create the non-Live adapter without cloud authentication");
            assert!(matches!(
                events.recv().await,
                Some(LiveServerEvent::Connected)
            ));
            assert_eq!(
                session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
                ProviderErrorKind::Setup,
                "non-Live Talk must not expose a Gemini Live microphone path"
            );
            session.close().await.unwrap();
        }
    }
}

// Native System synthesis exists on macOS, so this fail-closed path applies elsewhere.
#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn unavailable_system_tts_rejects_before_any_selected_text_provider_starts() {
    use crate::ai::types::ProviderErrorKind;
    let state = AppState::new_for_tests().unwrap();
    for text_provider in [
        TextProvider::Local,
        TextProvider::Google,
        TextProvider::Openai,
        TextProvider::Ollama,
        TextProvider::LlamaServer,
    ] {
        let settings = AppSettings {
            text_provider,
            tts_provider: TtsProvider::System,
            asr_mode: AsrMode::WhisperSmall,
            ..AppSettings::default()
        };
        let provider = state.get_conversation_provider_for(&settings);
        let (sender, mut events) = tokio::sync::mpsc::channel(4);
        let result = provider
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
            Ok(_) => panic!("unavailable System TTS must reject before Talk starts"),
            Err(error) => error,
        };
        assert_eq!(error.kind, ProviderErrorKind::Setup);
        assert!(!error.retryable);
        assert!(error.message.contains("no text provider was contacted"));
        assert!(!error.message.contains("private instruction"));
        assert!(
            events.recv().await.is_none(),
            "failed connection must publish no transcript or audio"
        );
    }
}

#[tokio::test]
async fn missing_openai_key_in_non_live_talk_does_not_fall_back() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();
    let settings = AppSettings {
        text_provider: TextProvider::Openai,
        tts_provider: TtsProvider::Local,
        asr_mode: AsrMode::WhisperSmall,
        ..AppSettings::default()
    };
    let provider = state.get_conversation_provider_for(&settings);
    let (sender, mut events) = tokio::sync::mpsc::channel(8);
    let mut session = provider
        .connect(
            LiveSessionConfig {
                model: settings.openai_text_model.clone(),
                voice_name: Some(settings.local_tts_voice.clone()),
                system_instruction: None,
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
                tools: vec![],
            },
            sender,
        )
        .await
        .unwrap();
    assert!(matches!(
        events.recv().await,
        Some(LiveServerEvent::Connected)
    ));
    session.send_text_turn("Hello").await.unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
        .await
        .unwrap();
    match result {
        Some(LiveServerEvent::Error(error)) => assert_eq!(error.kind, ProviderErrorKind::Auth),
        _ => panic!("missing OpenAI key must report auth failure without another provider"),
    }
    session.close().await.unwrap();
}

#[tokio::test]
async fn invalid_loopback_server_cannot_fall_back_to_cloud_or_any_standalone_voice() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();
    for text_provider in [TextProvider::Ollama, TextProvider::LlamaServer] {
        for tts_provider in [TtsProvider::Local, TtsProvider::Google] {
            let settings = AppSettings {
                text_provider,
                tts_provider,
                asr_mode: AsrMode::WhisperSmall,
                ollama_server_address: "http://example.invalid:11434".into(),
                llama_server_address: "http://example.invalid:8080".into(),
                ..AppSettings::default()
            };
            let provider = state.get_conversation_provider_for(&settings);
            let (sender, mut events) = tokio::sync::mpsc::channel(8);
            let mut session = provider
                .connect(
                    LiveSessionConfig {
                        model: "configured-model".into(),
                        voice_name: None,
                        system_instruction: None,
                        sample_rate_in: 16_000,
                        sample_rate_out: 24_000,
                        tools: vec![],
                    },
                    sender,
                )
                .await
                .unwrap();
            assert!(matches!(
                events.recv().await,
                Some(LiveServerEvent::Connected)
            ));
            session.send_text_turn("Private text").await.unwrap();
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
                    .await
                    .unwrap();
            match result {
                Some(LiveServerEvent::Error(error)) => {
                    assert_eq!(error.kind, ProviderErrorKind::Setup);
                    assert!(!error.message.contains("Private text"));
                }
                _ => panic!("invalid loopback destination must fail before speech synthesis"),
            }
            // No AudioData or TurnComplete follows the selected text failure.
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(20), events.recv())
                    .await
                    .is_err()
            );
            session.close().await.unwrap();
        }
    }
}

// On macOS this would synthesize for real; the macOS acceptance workflow covers it.
#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn selected_system_voice_audition_fails_closed_until_native_adapter_is_available() {
    use crate::ai::types::{ProviderErrorKind, TtsRequest};
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().tts_provider = TtsProvider::System;
    let error = state
        .get_speech_synthesizer()
        .synthesize(TtsRequest {
            text: "audition".into(),
            voice_name: Some("system_default".into()),
            speaking_rate: None,
            pitch: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Setup);
    assert!(!error.retryable);
    assert!(error.message.contains("no other voice provider"));
    assert!(!error.message.contains("audition"));
}
