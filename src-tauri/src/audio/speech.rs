use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::TtsRequest;
use crate::audio::playback::{AudioPlayback, PlaybackEnqueueReport};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub const STANDALONE_SPEECH_CANCELLED: &str = "standalone speech cancelled";

#[derive(Clone)]
pub struct StandaloneSpeechController {
    current: Arc<Mutex<CancellationToken>>,
}

impl StandaloneSpeechController {
    pub fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(CancellationToken::new())),
        }
    }

    /// Begin one authoritative standalone utterance. Starting a new utterance
    /// cancels any older synthesis request and immediately removes older queued
    /// standalone audio so stale speech cannot resume later.
    pub fn begin(&self, playback: &AudioPlayback) -> CancellationToken {
        let mut current = self.current.lock();
        current.cancel();
        playback.flush();
        let next = CancellationToken::new();
        *current = next.clone();
        next
    }

    /// Cancel in-flight synthesis and already-queued standalone playback.
    pub fn cancel(&self, playback: &AudioPlayback) {
        self.current.lock().cancel();
        playback.flush();
    }
}

impl Default for StandaloneSpeechController {
    fn default() -> Self {
        Self::new()
    }
}

/// Synthesize one standalone Moose utterance and queue it through the authoritative
/// Rust/CPAL playback path. Browser speech synthesis and platform speech subprocesses
/// are intentionally not fallback paths here.
pub async fn synthesize_and_queue(
    synthesizer: &dyn SpeechSynthesizer,
    playback: &AudioPlayback,
    request: TtsRequest,
    output_device: Option<String>,
) -> Result<PlaybackEnqueueReport, String> {
    let cancellation = CancellationToken::new();
    synthesize_and_queue_cancellable(synthesizer, playback, request, output_device, &cancellation)
        .await
}

pub async fn synthesize_and_queue_cancellable(
    synthesizer: &dyn SpeechSynthesizer,
    playback: &AudioPlayback,
    request: TtsRequest,
    output_device: Option<String>,
    cancellation: &CancellationToken,
) -> Result<PlaybackEnqueueReport, String> {
    let audio = tokio::select! {
        () = cancellation.cancelled() => {
            return Err(STANDALONE_SPEECH_CANCELLED.to_string());
        }
        result = synthesizer.synthesize(request) => {
            result.map_err(|error| error.message)?
        }
    };

    if cancellation.is_cancelled() {
        return Err(STANDALONE_SPEECH_CANCELLED.to_string());
    }

    playback
        .start(output_device)
        .map_err(|error_value| error_value.to_string())?;
    if cancellation.is_cancelled() {
        playback.flush();
        return Err(STANDALONE_SPEECH_CANCELLED.to_string());
    }

    let report = playback
        .enqueue_pcm_bytes(&audio.pcm_bytes, audio.sample_rate)
        .map_err(|error_value| error_value.to_string())?;
    if cancellation.is_cancelled() {
        playback.flush();
        return Err(STANDALONE_SPEECH_CANCELLED.to_string());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::{AudioStreamData, ProviderError};
    use async_trait::async_trait;
    use std::time::Duration;

    struct NeverSynthesizer;

    #[async_trait]
    impl SpeechSynthesizer for NeverSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
            std::future::pending().await
        }
    }

    #[tokio::test]
    async fn explicit_cancellation_aborts_in_flight_synthesis_and_flushes_playback() {
        let synthesizer = NeverSynthesizer;
        let playback = AudioPlayback::new_mock();
        let controller = StandaloneSpeechController::new();
        let cancellation = controller.begin(&playback);
        playback.seed_buffer_for_tests(&[0.25, -0.25, 0.5], 0.5);

        let future = synthesize_and_queue_cancellable(
            &synthesizer,
            &playback,
            TtsRequest {
                text: "This request should be cancelled.".to_string(),
                voice_name: Some("Fenrir".to_string()),
                speaking_rate: Some(1.0),
                pitch: Some(0.0),
            },
            None,
            &cancellation,
        );
        tokio::pin!(future);

        tokio::select! {
            result = &mut future => panic!("synthesis unexpectedly completed: {result:?}"),
            () = tokio::time::sleep(Duration::from_millis(10)) => {
                controller.cancel(&playback);
            }
        }

        let error = future
            .await
            .expect_err("cancelled synthesis must fail closed");
        assert_eq!(error, STANDALONE_SPEECH_CANCELLED);
        assert!(!playback.is_playing());
        assert_eq!(playback.queue_length(), 0);
        assert_eq!(playback.diagnostics().output_level, 0.0);
    }
}
