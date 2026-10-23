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

/// A persisted Moonshine profile must reach the same local Whisper
/// conversation boundary as a fresh profile after its ASR-only migration.
/// No model weights, keys, microphones, text API calls or synthesis are used.
#[tokio::test]
async fn migrated_moonshine_profiles_keep_selected_text_and_tts_non_live() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};

    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_google_api_key());

    for legacy_asr in [
        "moonshine_tiny_streaming", "moonshine_small_streaming",
    ] {
        for text_provider in [
            TextProvider::Local,
            TextProvider::Google,
            TextProvider::Openai,
            TextProvider::Ollama,
            TextProvider::LlamaServer,
        ] {
            for tts_provider in [TtsProvider::Local, TtsProvider::Google] {
                let before = AppSettings {
                    asr_mode: AsrMode::WhisperSmall,
                    text_provider,
                    tts_provider,
                    ..AppSettings::default()
                };
                let mut persisted = serde_json::to_value(&before).unwrap();
                persisted["asr_mode"] = serde_json::json!(legacy_asr);
                let persisted = serde_json::to_string(&persisted).unwrap();
                let (migrated, changed) = AppSettings::from_persisted_json(&persisted).unwrap();
                assert!(changed, "{legacy_asr:?} must migrate");
                assert_eq!(migrated.asr_mode, AsrMode::WhisperSmall);
                assert_eq!(migrated.text_provider, text_provider);
                assert_eq!(migrated.tts_provider, tts_provider);

                let persisted_after = serde_json::to_string(&migrated).unwrap();
                assert!(!persisted_after.contains("moonshine_"));
                let (restored, changed_again) =
                    AppSettings::from_persisted_json(&persisted_after).unwrap();
                assert!(!changed_again, "migration must survive restart unchanged");
                assert_eq!(restored.asr_mode, AsrMode::WhisperSmall);
                assert_eq!(restored.text_provider, text_provider);
                assert_eq!(restored.tts_provider, tts_provider);

                let (sender, mut events) = tokio::sync::mpsc::channel(4);
                let mut session = state
                    .get_conversation_provider_for(&restored)
                    .connect(
                        LiveSessionConfig {
                            model: "selected-model".into(),
                            voice_name: None,
                            system_instruction: None,
                            sample_rate_in: 16_000,
                            sample_rate_out: 24_000,
                            tools: vec![],
                        },
                        sender,
                    )
                    .await
                    .expect("migrated local ASR must not require Gemini Live authentication");
                assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
                assert_eq!(
                    session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
                    ProviderErrorKind::Setup,
                    "{legacy_asr:?} {text_provider:?} {tts_provider:?} must not route PCM to Live"
                );
                session.close().await.unwrap();
            }
        }
    }
}

/// Every installed local ASR architecture must preserve the separate non-Live
/// text/TTS routing boundary. A provider choice never turns microphone PCM
/// into Gemini Live traffic merely because it is an external text provider.
#[tokio::test]
async fn whisper_asr_keeps_all_external_text_and_voice_pairs_non_live() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();
    for asr_mode in [
        AsrMode::WhisperSmall,
    ] {
        for text_provider in [
            TextProvider::Local,
            TextProvider::Google,
            TextProvider::Openai,
            TextProvider::Ollama,
            TextProvider::LlamaServer,
        ] {
            for tts_provider in [TtsProvider::Local, TtsProvider::Google] {
                let settings = AppSettings {
                    asr_mode,
                    text_provider,
                    tts_provider,
                    ..AppSettings::default()
                };
                let provider = state.get_conversation_provider_for(&settings);
                let (sender, mut events) = tokio::sync::mpsc::channel(4);
                let mut session = provider
                    .connect(
                        LiveSessionConfig {
                            model: "selected-model".into(),
                            voice_name: None,
                            system_instruction: None,
                            sample_rate_in: 16_000,
                            sample_rate_out: 24_000,
                            tools: vec![],
                        },
                        sender,
                    )
                    .await
                    .expect("Whisper ASR must route through non-Live Talk");
                assert!(
                    matches!(events.recv().await, Some(LiveServerEvent::Connected)),
                    "{asr_mode:?} {text_provider:?} {tts_provider:?}"
                );
                assert_eq!(
                    session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
                    ProviderErrorKind::Setup,
                    "{asr_mode:?} {text_provider:?} {tts_provider:?} must not open Gemini Live"
                );
                session.close().await.unwrap();
            }
        }
    }
}

/// Explicit Gemini Live ASR must use Google Live, regardless of the selected
/// standalone text model or TTS voice. Missing Live credentials must not cause
/// a silent fallback to non-Live Talk or an unavailable System speech adapter.
#[tokio::test]
async fn explicit_live_asr_is_separate_from_every_standalone_provider_pair() {
    use crate::ai::types::ProviderErrorKind;
    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_google_api_key());

    for text_provider in [
        TextProvider::Local,
        TextProvider::Google,
        TextProvider::Openai,
        TextProvider::Ollama,
        TextProvider::LlamaServer,
    ] {
        for tts_provider in [TtsProvider::Local, TtsProvider::Google, TtsProvider::System] {
            let settings = AppSettings {
                asr_mode: AsrMode::GeminiLiveAudio,
                text_provider,
                tts_provider,
                ..AppSettings::default()
            };
            let (sender, mut events) = tokio::sync::mpsc::channel(4);
            let result = state
                .get_conversation_provider_for(&settings)
                .connect(
                    LiveSessionConfig {
                        model: "selected-live-model".into(),
                        voice_name: None,
                        system_instruction: None,
                        sample_rate_in: 16_000,
                        sample_rate_out: 24_000,
                        tools: vec![],
                    },
                    sender,
                )
                .await;
            let error = match result {
                Ok(_) => panic!("Live mode without Google auth cannot start"),
                Err(error) => error,
            };
            assert_eq!(
                error.kind,
                ProviderErrorKind::Auth,
                "{text_provider:?} {tts_provider:?} must remain in Gemini Live"
            );
            assert!(events.try_recv().is_err(), "failed Live connect emitted an event");
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


#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn every_local_asr_rejects_unavailable_system_voice_without_contacting_text_provider() {
    use crate::ai::types::ProviderErrorKind;
    let state = AppState::new_for_tests().unwrap();
    for asr_mode in [
        AsrMode::WhisperSmall,
        AsrMode::WhisperSmall,
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
                Ok(_) => panic!("{asr_mode:?} {text_provider:?} cannot start System TTS"),
                Err(error) => error,
            };
            assert_eq!(error.kind, ProviderErrorKind::Setup);
            assert!(!error.retryable);
            assert!(error.message.contains("no text provider was contacted"));
            assert!(!error.message.contains("private instruction"));
            assert!(
                events.try_recv().is_err(),
                "{asr_mode:?} {text_provider:?} must emit no transcript or audio"
            );
        }
    }
}

#[tokio::test]
async fn missing_openai_key_in_non_live_talk_does_not_fall_back() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_openai_api_key());

    // Either standalone TTS choice must stay unused when the selected
    // OpenAI text provider has no key: no silent Local/Google fallback,
    // synthesized audio, or successful turn completion after Auth.
    for tts_provider in [TtsProvider::Local, TtsProvider::Google] {
        let settings = AppSettings {
            text_provider: TextProvider::Openai,
            tts_provider,
            asr_mode: AsrMode::WhisperSmall,
            ..AppSettings::default()
        };
        let provider = state.get_conversation_provider_for(&settings);
        let (sender, mut events) = tokio::sync::mpsc::channel(8);
        let mut session = provider
            .connect(
                LiveSessionConfig {
                    model: settings.openai_text_model.clone(),
                    voice_name: Some(match tts_provider {
                        TtsProvider::Local => settings.local_tts_voice.clone(),
                        TtsProvider::Google => settings.google_tts_voice.clone(),
                        TtsProvider::System => unreachable!("System is not in this test matrix"),
                    }),
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
        session.send_text_turn("private missing-key turn").await.unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
            .await
            .unwrap();
        match result {
            Some(LiveServerEvent::Error(error)) => {
                assert_eq!(error.kind, ProviderErrorKind::Auth, "{tts_provider:?}");
                assert!(!error.message.contains("private missing-key turn"));
            }
            _ => panic!("missing OpenAI key must fail closed with {tts_provider:?}"),
        }
        let late = tokio::time::timeout(std::time::Duration::from_millis(30), events.recv()).await;
        assert!(
            !matches!(late, Ok(Some(_))),
            "{tts_provider:?} must never receive audio or completion after an auth failure"
        );
        session.close().await.unwrap();
    }
}

/// Missing Google *text* credentials must not be interpreted as permission
/// to open Gemini Live or to synthesize through either standalone TTS choice.
#[tokio::test]
async fn missing_google_text_key_in_whisper_non_live_talk_never_starts_tts() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_google_api_key());

    for tts_provider in [TtsProvider::Local, TtsProvider::Google] {
        let settings = AppSettings {
            text_provider: TextProvider::Google,
            tts_provider,
            asr_mode: AsrMode::WhisperSmall,
            ..AppSettings::default()
        };
        let provider = state.get_conversation_provider_for(&settings);
        let (sender, mut events) = tokio::sync::mpsc::channel(8);
        let mut session = provider
            .connect(
                LiveSessionConfig {
                    model: settings.google_text_model.clone(),
                    voice_name: None,
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: vec![],
                },
                sender,
            )
            .await
            .expect("Whisper local ASR must connect only to the non-Live adapter");
        assert!(matches!(
            events.recv().await,
            Some(LiveServerEvent::Connected)
        ));
        session.send_text_turn("private missing Google key").await.unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
            .await
            .expect("missing key must fail promptly");
        match result {
            Some(LiveServerEvent::Error(error)) => {
                assert_eq!(error.kind, ProviderErrorKind::Auth, "{tts_provider:?}");
                assert!(!error.message.contains("private missing Google key"));
            }
            _ => panic!("Google text without a key must fail closed with {tts_provider:?}"),
        }
        let late = tokio::time::timeout(std::time::Duration::from_millis(30), events.recv()).await;
        assert!(
            !matches!(late, Ok(Some(_))),
            "{tts_provider:?} must not receive audio or completion after an auth failure"
        );
        session.close().await.unwrap();
    }
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

/// On macOS the native System Voice is an independently selected TTS output,
/// not permission to open Gemini Live or forward local microphone PCM.
/// This is a routing-only test: it does not synthesize or audition audio.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn whisper_with_macos_system_voice_keeps_every_text_provider_non_live() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};
    let state = AppState::new_for_tests().unwrap();

    for text_provider in [
        TextProvider::Local,
        TextProvider::Google,
        TextProvider::Openai,
        TextProvider::Ollama,
        TextProvider::LlamaServer,
    ] {
        let settings = AppSettings {
            asr_mode: AsrMode::WhisperSmall,
            text_provider,
            tts_provider: TtsProvider::System,
            ..AppSettings::default()
        };
        let (sender, mut events) = tokio::sync::mpsc::channel(4);
        let mut session = state
            .get_conversation_provider_for(&settings)
            .connect(
                LiveSessionConfig {
                    model: "selected-text-model".into(),
                    voice_name: Some("system_default".into()),
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: vec![],
                },
                sender,
            )
            .await
            .expect("Whisper + macOS System Voice must use non-Live Talk");
        assert!(
            matches!(events.recv().await, Some(LiveServerEvent::Connected)),
            "{text_provider:?} did not open the non-Live adapter"
        );
        assert_eq!(
            session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
            ProviderErrorKind::Setup,
            "{text_provider:?} must not forward Whisper PCM to Gemini Live"
        );
        assert!(
            events.try_recv().is_err(),
            "{text_provider:?} must not emit audio or a successful turn"
        );
        session.close().await.unwrap();
    }
}

/// A missing selected cloud-text key must not invoke native macOS System
/// speech synthesis. This tests routing/events, not audible playback.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn whisper_macos_system_voice_missing_text_key_fails_without_synthesis() {
    use crate::ai::types::{LiveServerEvent, ProviderErrorKind};

    let state = AppState::new_for_tests().unwrap();
    assert!(!state.secrets.has_google_api_key());
    assert!(!state.secrets.has_openai_api_key());

    for text_provider in [TextProvider::Google, TextProvider::Openai] {
        let settings = AppSettings {
            asr_mode: AsrMode::WhisperSmall,
            text_provider,
            tts_provider: TtsProvider::System,
            ..AppSettings::default()
        };
        let (sender, mut events) = tokio::sync::mpsc::channel(8);
        let mut session = state
            .get_conversation_provider_for(&settings)
            .connect(
                LiveSessionConfig {
                    model: "selected-text-model".into(),
                    voice_name: Some("system_default".into()),
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: vec![],
                },
                sender,
            )
            .await
            .expect("Whisper with System voice must use non-Live Talk");
        assert!(matches!(
            events.recv().await,
            Some(LiveServerEvent::Connected)
        ));
        session
            .send_text_turn("private system voice no-key turn")
            .await
            .unwrap();
        let next = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
            .await
            .expect("missing text credentials must fail promptly");
        match next {
            Some(LiveServerEvent::Error(error)) => {
                assert_eq!(error.kind, ProviderErrorKind::Auth, "{text_provider:?}");
                assert!(!error.message.contains("private system voice no-key turn"));
            }
            _ => panic!("{text_provider:?} must fail without TTS or a completed turn"),
        }
        let late = tokio::time::timeout(std::time::Duration::from_millis(30), events.recv()).await;
        assert!(
            !matches!(late, Ok(Some(_))),
            "{text_provider:?} must not synthesize or complete after auth failure"
        );
        session.close().await.unwrap();
    }
}
