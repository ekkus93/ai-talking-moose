use crate::ai::google::{GoogleAuth, GoogleSpeechSynthesizer};
use crate::ai::local_tts::LocalSpeechSynthesizer;
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{TtsProvider, TtsRequest};
use crate::app::state::AppState;
use crate::audio::playback::AudioPlayback;
use crate::audio::speech::{synthesize_and_queue_cancellable, StandaloneSpeechController};
use crate::character::state::CharacterState;
use crate::commands::presentation::{clear_speech_bubble, transition_and_emit};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Runtime};
use tokio_util::sync::CancellationToken;

pub(crate) const MAX_STANDALONE_PLAYBACK_WAIT: Duration = Duration::from_secs(10);
pub(crate) const NO_PLAYABLE_STANDALONE_AUDIO: &str =
    "synthesized speech contained no playable audio";

pub(crate) struct StandaloneSpeechPlayback {
    duration: Duration,
    cancellation: CancellationToken,
    controller: StandaloneSpeechController,
}

impl StandaloneSpeechPlayback {
    pub(crate) async fn wait_without_cancellation(&self, duration: Duration) -> bool {
        tokio::select! {
            () = tokio::time::sleep(duration) => !self.cancellation.is_cancelled(),
            () = self.cancellation.cancelled() => false,
        }
    }

    pub(crate) async fn completed_without_cancellation(&self) -> bool {
        self.wait_without_cancellation(self.duration).await
    }

    pub(crate) fn with_current<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        self.controller.with_current(&self.cancellation, action)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct StandaloneTtsSettingsSnapshot {
    provider: TtsProvider,
    model_id: String,
    voice_id: String,
    speaking_rate: f32,
    pitch: Option<f32>,
    output_device: Option<String>,
}

impl StandaloneTtsSettingsSnapshot {
    fn request(&self, text: &str) -> TtsRequest {
        TtsRequest {
            text: text.to_string(),
            voice_name: Some(self.voice_id.clone()),
            speaking_rate: Some(self.speaking_rate),
            pitch: self.pitch,
        }
    }
}

fn standalone_tts_snapshot(
    state: &AppState,
    voice_override: Option<String>,
) -> StandaloneTtsSettingsSnapshot {
    let settings = state.settings.read();
    match settings.tts_provider {
        TtsProvider::Google => StandaloneTtsSettingsSnapshot {
            provider: TtsProvider::Google,
            model_id: settings.google_tts_model.clone(),
            voice_id: voice_override.unwrap_or_else(|| settings.google_tts_voice.clone()),
            speaking_rate: settings.speaking_rate,
            pitch: Some(settings.pitch),
            output_device: settings.output_device.clone(),
        },
        TtsProvider::Local => StandaloneTtsSettingsSnapshot {
            provider: TtsProvider::Local,
            model_id: settings.local_tts_model.clone(),
            voice_id: voice_override.unwrap_or_else(|| settings.local_tts_voice.clone()),
            speaking_rate: settings.speaking_rate,
            // Kitten Mini has no truthful model-level pitch control. Keep the persisted Google
            // pitch preference untouched while omitting it from Local synthesis.
            pitch: None,
            output_device: settings.output_device.clone(),
        },
    }
}

fn ensure_snapshot_provider(
    snapshot: &StandaloneTtsSettingsSnapshot,
    required_provider: TtsProvider,
) -> Result<(), String> {
    if snapshot.provider == required_provider {
        Ok(())
    } else {
        Err("The standalone speech provider changed; retry the voice audition.".to_string())
    }
}

fn synthesizer_for_snapshot(
    state: &AppState,
    snapshot: &StandaloneTtsSettingsSnapshot,
) -> Box<dyn SpeechSynthesizer> {
    match snapshot.provider {
        TtsProvider::Google => Box::new(GoogleSpeechSynthesizer::new(
            GoogleAuth::new(state.secrets.get_google_api_key().unwrap_or_default()),
            snapshot.model_id.clone(),
            snapshot.voice_id.clone(),
        )),
        TtsProvider::Local => Box::new(LocalSpeechSynthesizer::new(
            state.local_tts_runtime.clone(),
            snapshot.model_id.clone(),
            snapshot.voice_id.clone(),
        )),
    }
}

async fn synthesize_standalone(
    synthesizer: &dyn SpeechSynthesizer,
    playback: &AudioPlayback,
    controller: &StandaloneSpeechController,
    request: TtsRequest,
    output_device: Option<String>,
) -> Result<StandaloneSpeechPlayback, String> {
    let cancellation = controller.begin(playback);
    let report = synthesize_and_queue_cancellable(
        synthesizer,
        playback,
        request,
        output_device,
        &cancellation,
    )
    .await?;

    if report.dropped_samples > 0 {
        controller.cancel(playback);
        return Err(format!(
            "audio playback queue overflowed and dropped {} samples",
            report.dropped_samples
        ));
    }
    if report.queued_samples == 0 {
        controller.cancel(playback);
        return Err(NO_PLAYABLE_STANDALONE_AUDIO.to_string());
    }

    let Some(output_sample_rate) = playback.output_sample_rate_hz() else {
        controller.cancel(playback);
        return Err("audio playback did not report an output sample rate".to_string());
    };
    let duration =
        Duration::from_secs_f64(report.queued_samples as f64 / f64::from(output_sample_rate))
            .min(MAX_STANDALONE_PLAYBACK_WAIT);

    Ok(StandaloneSpeechPlayback {
        duration,
        cancellation,
        controller: controller.clone(),
    })
}

async fn invoke_standalone_speech_snapshot<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    text: &str,
    snapshot: StandaloneTtsSettingsSnapshot,
) -> Result<StandaloneSpeechPlayback, String> {
    let request = snapshot.request(text);
    let output_device = snapshot.output_device.clone();
    let synthesizer = synthesizer_for_snapshot(state, &snapshot);
    let playback = synthesize_standalone(
        synthesizer.as_ref(),
        state.audio_playback.as_ref(),
        &state.standalone_speech,
        request,
        output_device,
    )
    .await?;

    if let Err(error) = transition_and_emit(&state.character_state, app, CharacterState::Talking) {
        state
            .standalone_speech
            .cancel(state.audio_playback.as_ref());
        return Err(error);
    }
    let _ = app.emit("moose://speech-bubble", text);
    Ok(playback)
}

/// Authoritative standalone speech invocation for ambient remarks, character
/// reactions/auditions, and text-mode replies. Speech is not surfaced as Talking
/// until synthesis has produced playable audio and the bounded playback queue has
/// accepted the entire utterance.
pub(crate) async fn invoke_standalone_speech<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    text: &str,
    voice_override: Option<String>,
) -> Result<StandaloneSpeechPlayback, String> {
    // Capture provider/model/voice/rate/pitch/output-device under one settings read. Any settings
    // change racing this utterance applies to the next utterance instead of mixing providers.
    let snapshot = standalone_tts_snapshot(state, voice_override);
    invoke_standalone_speech_snapshot(state, app, text, snapshot).await
}

/// Provider-guarded standalone invocation used by voice audition. The provider is not an override:
/// it must still match the one captured from authoritative settings. If settings change while the
/// UI request is in flight, the audition fails closed instead of speaking through a different
/// provider than the label the user clicked.
pub(crate) async fn invoke_standalone_speech_for_provider<R: Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    text: &str,
    provider: TtsProvider,
    voice_override: Option<String>,
) -> Result<StandaloneSpeechPlayback, String> {
    let snapshot = standalone_tts_snapshot(state, voice_override);
    ensure_snapshot_provider(&snapshot, provider)?;
    invoke_standalone_speech_snapshot(state, app, text, snapshot).await
}

pub(crate) fn schedule_standalone_completion<R: Runtime>(
    character_state: Arc<parking_lot::RwLock<CharacterState>>,
    app: tauri::AppHandle<R>,
    playback: StandaloneSpeechPlayback,
) {
    tauri::async_runtime::spawn(async move {
        if !playback.completed_without_cancellation().await {
            return;
        }
        let _ = playback.with_current(|| {
            clear_speech_bubble(&app);
            if *character_state.read() == CharacterState::Talking {
                let _ = transition_and_emit(character_state.as_ref(), &app, CharacterState::Idle);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::local_tts::{DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE};
    use crate::ai::types::{AudioStreamData, ProviderError};
    use async_trait::async_trait;

    struct EmptySpeechSynthesizer;

    #[async_trait]
    impl SpeechSynthesizer for EmptySpeechSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
            Ok(AudioStreamData {
                pcm_bytes: Vec::new(),
                sample_rate: 24_000,
            })
        }
    }

    #[test]
    fn standalone_snapshot_uses_local_identity_and_never_applies_google_pitch() {
        let state = AppState::new_for_tests().unwrap();
        {
            let mut settings = state.settings.write();
            settings.tts_provider = TtsProvider::Local;
            settings.google_tts_voice = "Puck".to_string();
            settings.local_tts_model = DEFAULT_LOCAL_TTS_MODEL_ID.to_string();
            settings.local_tts_voice = DEFAULT_LOCAL_TTS_VOICE.to_string();
            settings.speaking_rate = 1.25;
            settings.pitch = -7.0;
            settings.output_device = Some("local-output".to_string());
        }

        let snapshot = standalone_tts_snapshot(&state, Some("Kiki".to_string()));
        assert_eq!(snapshot.provider, TtsProvider::Local);
        assert_eq!(snapshot.model_id, DEFAULT_LOCAL_TTS_MODEL_ID);
        assert_eq!(snapshot.voice_id, "Kiki");
        assert_eq!(snapshot.speaking_rate, 1.25);
        assert_eq!(snapshot.pitch, None);
        assert_eq!(snapshot.output_device.as_deref(), Some("local-output"));
    }

    #[test]
    fn standalone_snapshot_is_immutable_across_racing_settings_changes() {
        let state = AppState::new_for_tests().unwrap();
        {
            let mut settings = state.settings.write();
            settings.tts_provider = TtsProvider::Local;
            settings.local_tts_model = DEFAULT_LOCAL_TTS_MODEL_ID.to_string();
            settings.local_tts_voice = "Luna".to_string();
            settings.speaking_rate = 0.9;
            settings.output_device = Some("before".to_string());
        }
        let snapshot = standalone_tts_snapshot(&state, None);

        {
            let mut settings = state.settings.write();
            settings.tts_provider = TtsProvider::Google;
            settings.google_tts_model = "changed-google-model".to_string();
            settings.google_tts_voice = "Puck".to_string();
            settings.local_tts_voice = "Leo".to_string();
            settings.speaking_rate = 1.8;
            settings.output_device = Some("after".to_string());
        }

        assert_eq!(snapshot.provider, TtsProvider::Local);
        assert_eq!(snapshot.model_id, DEFAULT_LOCAL_TTS_MODEL_ID);
        assert_eq!(snapshot.voice_id, "Luna");
        assert_eq!(snapshot.speaking_rate, 0.9);
        assert_eq!(snapshot.output_device.as_deref(), Some("before"));
    }

    #[test]
    fn provider_guard_fails_closed_when_audition_races_provider_switch() {
        let state = AppState::new_for_tests().unwrap();
        state.settings.write().tts_provider = TtsProvider::Local;
        let snapshot = standalone_tts_snapshot(&state, Some("Bella".to_string()));

        let error = ensure_snapshot_provider(&snapshot, TtsProvider::Google)
            .expect_err("Google-labelled audition must not run through Local");
        assert!(error.contains("provider changed"));
        assert!(ensure_snapshot_provider(&snapshot, TtsProvider::Local).is_ok());
    }

    #[test]
    fn google_snapshot_preserves_google_pitch_and_does_not_read_local_voice() {
        let state = AppState::new_for_tests().unwrap();
        {
            let mut settings = state.settings.write();
            settings.tts_provider = TtsProvider::Google;
            settings.google_tts_voice = "Puck".to_string();
            settings.local_tts_voice = "Luna".to_string();
            settings.pitch = -2.5;
        }

        let snapshot = standalone_tts_snapshot(&state, None);
        assert_eq!(snapshot.provider, TtsProvider::Google);
        assert_eq!(snapshot.voice_id, "Puck");
        assert_eq!(snapshot.pitch, Some(-2.5));
    }

    #[tokio::test]
    async fn zero_playable_audio_fails_identically_for_every_standalone_speech_path() {
        for (path, voice_override) in [
            ("ambient", None),
            ("character", Some("Puck".to_string())),
            ("conversation", None),
        ] {
            let playback = AudioPlayback::new_mock();
            let controller = StandaloneSpeechController::new();
            let request = TtsRequest {
                text: format!("{path} zero-audio probe"),
                voice_name: voice_override,
                speaking_rate: Some(1.0),
                pitch: Some(0.0),
            };

            let error = match synthesize_standalone(
                &EmptySpeechSynthesizer,
                &playback,
                &controller,
                request,
                None,
            )
            .await
            {
                Ok(_) => panic!("every standalone speech path must reject zero playable audio"),
                Err(error) => error,
            };

            assert_eq!(error, NO_PLAYABLE_STANDALONE_AUDIO, "{path}");
            assert_eq!(playback.queue_length(), 0, "{path}");
            assert!(!playback.is_playing(), "{path}");
        }
    }

    #[tokio::test]
    async fn cancelled_completion_never_claims_the_newer_utterance_finished() {
        let controller = StandaloneSpeechController::new();
        let audio_playback = AudioPlayback::new_mock();
        let current = controller.begin(&audio_playback);
        let playback = StandaloneSpeechPlayback {
            duration: Duration::from_millis(25),
            cancellation: current,
            controller: controller.clone(),
        };

        let _newer_utterance = controller.begin(&audio_playback);

        assert!(!playback.completed_without_cancellation().await);
        assert!(playback.with_current(|| ()).is_none());
    }

    #[test]
    fn standalone_playback_wait_is_hard_bounded() {
        assert!(MAX_STANDALONE_PLAYBACK_WAIT <= Duration::from_secs(10));
    }
}
