use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::TtsRequest;
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

fn tts_request(
    state: &AppState,
    text: &str,
    voice_override: Option<String>,
) -> (TtsRequest, Option<String>) {
    let settings = state.settings.read();
    (
        TtsRequest {
            text: text.to_string(),
            voice_name: Some(voice_override.unwrap_or_else(|| settings.tts_voice.clone())),
            speaking_rate: Some(settings.speaking_rate),
            pitch: Some(settings.pitch),
        },
        settings.output_device.clone(),
    )
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
    let (request, output_device) = tts_request(state, text, voice_override);
    let synthesizer = state.get_speech_synthesizer();
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
