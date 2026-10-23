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


/// A selected macOS System Voice must never synthesize or fall back when the
/// independently selected cloud *text* provider has no credentials. This
/// exercises the real macOS adapter selection without invoking audible TTS.
#[tokio::test]
async fn system_voice_never_synthesizes_after_missing_selected_text_credentials() {
    let state = AppState::new_for_tests().unwrap();
    assert!(crate::ai::system_tts::is_supported());
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
                    model: "selected-cloud-text-model".into(),
                    voice_name: Some("system_default".into()),
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: vec![],
                },
                sender,
            )
            .await
            .expect("missing text key must not prevent non-Live session creation");
        assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
        assert_eq!(
            session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
            ProviderErrorKind::Setup,
            "{text_provider:?} must not open Gemini Live"
        );

        session.send_text_turn("private missing-key System Voice turn").await.unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
            .await
            .expect("missing selected text credentials must fail promptly");
        match result {
            Some(LiveServerEvent::Error(error)) => {
                assert_eq!(error.kind, ProviderErrorKind::Auth, "{text_provider:?}");
                assert!(!error.message.contains("private missing-key System Voice turn"));
            }
            _ => panic!("{text_provider:?} must fail closed without speech synthesis"),
        }
        let late =
            tokio::time::timeout(std::time::Duration::from_millis(30), events.recv()).await;
        assert!(
            !matches!(late, Ok(Some(_))),
            "{text_provider:?} must not publish audio or turn completion after Auth"
        );
        session.close().await.unwrap();
    }
}
/// A macOS profile migrating either historical Moonshine mode must retain its
/// independently selected System Voice, text provider, and privacy preferences.
/// This covers persisted data and non-Live routing without invoking audio I/O,
/// a real voice, model installation, or remote text generation.
#[tokio::test]
async fn migrated_moonshine_profile_keeps_macos_system_voice_and_privacy_non_live() {
    let state = AppState::new_for_tests().unwrap();
    assert!(crate::ai::system_tts::is_supported());

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
            let before = AppSettings {
                asr_mode: AsrMode::WhisperSmall,
                text_provider,
                tts_provider: TtsProvider::System,
                system_tts_voice: "system_default".into(),
                memory_enabled: true,
                save_transcripts: false,
                active_app_observation: true,
                ..AppSettings::default()
            };
            let mut persisted = serde_json::to_value(&before).unwrap();
                persisted["asr_mode"] = serde_json::json!(legacy_asr);
                let persisted = serde_json::to_string(&persisted).unwrap();
            let (migrated, changed) = AppSettings::from_persisted_json(&persisted).unwrap();
            assert!(changed, "{legacy_asr:?} should migrate");
            assert_eq!(migrated.asr_mode, AsrMode::WhisperSmall);
            assert_eq!(migrated.text_provider, text_provider);
            assert_eq!(migrated.tts_provider, TtsProvider::System);
            assert_eq!(migrated.system_tts_voice, "system_default");
            assert!(migrated.memory_enabled);
            assert!(!migrated.save_transcripts);
            assert!(migrated.active_app_observation);

            let saved = serde_json::to_string(&migrated).unwrap();
            assert!(!saved.contains("moonshine_"));
            let (restored, migrated_again) = AppSettings::from_persisted_json(&saved).unwrap();
            assert!(!migrated_again, "saved profile must be stable after restart");
            assert_eq!(restored.asr_mode, AsrMode::WhisperSmall);
            assert_eq!(restored.text_provider, text_provider);
            assert_eq!(restored.tts_provider, TtsProvider::System);
            assert_eq!(restored.system_tts_voice, "system_default");
            assert!(restored.memory_enabled);
            assert!(!restored.save_transcripts);
            assert!(restored.active_app_observation);

            let (sender, mut events) = tokio::sync::mpsc::channel(4);
            let mut session = state
                .get_conversation_provider_for(&restored)
                .connect(
                    LiveSessionConfig {
                        model: "selected-model".into(),
                        voice_name: Some(restored.system_tts_voice.clone()),
                        system_instruction: None,
                        sample_rate_in: 16_000,
                        sample_rate_out: 24_000,
                        tools: vec![],
                    },
                    sender,
                )
                .await
                .expect("migrated Mac System Voice must use the non-Live adapter");
            assert!(
                matches!(events.recv().await, Some(LiveServerEvent::Connected)),
                "{legacy_asr:?} {text_provider:?}"
            );
            assert_eq!(
                session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
                ProviderErrorKind::Setup,
                "{legacy_asr:?} {text_provider:?} must not send PCM to Gemini Live"
            );
            assert!(
                events.try_recv().is_err(),
                "rejected PCM must not publish an audio or transcript event"
            );
            session.close().await.unwrap();
        }
    }
}


/// Switching the current profile to Gemini Live cannot mutate an already
/// captured Whisper + macOS System Voice provider into a cloud audio session.
/// A newly selected Gemini Live provider still requires explicit Google auth.
/// Deterministic routing only: no microphone, credentials, or audible TTS.
#[tokio::test]
async fn captured_whisper_system_voice_stays_local_after_switch_to_gemini_live() {
    let state = AppState::new_for_tests().unwrap();
    assert!(crate::ai::system_tts::is_supported());
    assert!(!state.secrets.has_google_api_key());

    let original = AppSettings {
        asr_mode: AsrMode::WhisperSmall,
        text_provider: TextProvider::Ollama,
        tts_provider: TtsProvider::System,
        system_tts_voice: "system_default".into(),
        ..AppSettings::default()
    };
    *state.settings.write() = original.clone();
    let captured = state.get_conversation_provider_for(&original);

    {
        let mut settings = state.settings.write();
        settings.asr_mode = AsrMode::GeminiLiveAudio;
        settings.text_provider = TextProvider::Google;
        settings.tts_provider = TtsProvider::Local;
    }
    assert_eq!(state.settings.read().asr_mode, AsrMode::GeminiLiveAudio);

    let (sender, mut events) = tokio::sync::mpsc::channel(8);
    let mut session = captured
        .connect(
            LiveSessionConfig {
                model: "original-local-model".into(),
                voice_name: Some(original.system_tts_voice.clone()),
                system_instruction: None,
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
                tools: vec![],
            },
            sender,
        )
        .await
        .expect("previously captured Whisper session must stay non-Live");
    assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
    assert_eq!(
        session.send_audio_chunk(&[0, 0]).await.unwrap_err().kind,
        ProviderErrorKind::Setup,
        "an existing local Whisper session must never turn into Gemini Live"
    );
    assert!(events.try_recv().is_err(), "no audio or transcript was emitted");
    session.close().await.unwrap();

    // A freshly selected Gemini Live session must not fall back to Whisper
    // when Google Live credentials are absent.
    let selected_now = state.settings.read().clone();
    let (live_sender, mut live_events) = tokio::sync::mpsc::channel(8);
    let attempt = state
        .get_conversation_provider_for(&selected_now)
        .connect(
            LiveSessionConfig {
                model: "selected-live-model".into(),
                voice_name: None,
                system_instruction: None,
                sample_rate_in: 16_000,
                sample_rate_out: 24_000,
                tools: vec![],
            },
            live_sender,
        )
        .await;
    let error = match attempt {
        Ok(_) => panic!("explicit Live without Google credentials must fail"),
        Err(error) => error,
    };
    assert_eq!(error.kind, ProviderErrorKind::Auth);
    assert!(live_events.try_recv().is_err(), "failed Live startup has no events");
}
