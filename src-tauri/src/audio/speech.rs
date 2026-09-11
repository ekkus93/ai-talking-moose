use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{ProviderErrorKind, TtsRequest};
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

    /// Run a synchronous completion action only while `token` still owns the
    /// authoritative standalone utterance slot. Holding the slot lock across the
    /// action makes the ownership check atomic with respect to `begin`/`cancel`,
    /// preventing stale completion work from clearing a newer utterance's UI state.
    pub fn with_current<T>(
        &self,
        token: &CancellationToken,
        action: impl FnOnce() -> T,
    ) -> Option<T> {
        let current = self.current.lock();
        if current.is_cancelled() || &*current != token {
            return None;
        }
        Some(action())
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
    let audio = synthesizer
        .synthesize_cancellable(request, cancellation)
        .await
        .map_err(|error| {
            if error.kind == ProviderErrorKind::Cancelled || cancellation.is_cancelled() {
                STANDALONE_SPEECH_CANCELLED.to_string()
            } else {
                error.message
            }
        })?;

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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct NeverSynthesizer;

    struct OversizedSynthesizer;

    struct ErrorSynthesizer {
        kind: ProviderErrorKind,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl SpeechSynthesizer for OversizedSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
            let samples = 24_000 * (crate::audio::playback::MAX_QUEUED_PLAYBACK_SECONDS + 2);
            Ok(AudioStreamData {
                pcm_bytes: vec![0_u8; samples * 2],
                sample_rate: 24_000,
            })
        }
    }

    #[async_trait]
    impl SpeechSynthesizer for NeverSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
            std::future::pending().await
        }
    }

    #[async_trait]
    impl SpeechSynthesizer for ErrorSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(ProviderError::from_kind(self.kind))
        }
    }

    #[tokio::test]
    async fn standalone_speech_queue_overload_is_bounded_and_reported() {
        let playback = AudioPlayback::new_mock();
        let report = synthesize_and_queue(
            &OversizedSynthesizer,
            &playback,
            TtsRequest {
                text: "oversized speech".to_string(),
                voice_name: Some("Fenrir".to_string()),
                speaking_rate: Some(1.0),
                pitch: Some(0.0),
            },
            None,
        )
        .await
        .unwrap();

        assert_eq!(report.queued_samples, playback.max_queued_samples());
        assert!(report.dropped_samples > 0);
        assert_eq!(playback.queue_length(), playback.max_queued_samples());
        assert_eq!(
            playback.dropped_samples(),
            u64::try_from(report.dropped_samples).unwrap()
        );

        StandaloneSpeechController::new().cancel(&playback);
        assert_eq!(playback.queue_length(), 0);
        assert!(!playback.is_playing());
    }

    #[tokio::test]
    async fn provider_failures_propagate_once_without_fallback_or_audio() {
        for kind in [
            ProviderErrorKind::Setup,
            ProviderErrorKind::Model,
            ProviderErrorKind::Internal,
            ProviderErrorKind::Auth,
            ProviderErrorKind::Network,
            ProviderErrorKind::Protocol,
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let synthesizer = ErrorSynthesizer {
                kind,
                calls: calls.clone(),
            };
            let playback = AudioPlayback::new_mock();
            let expected = ProviderError::from_kind(kind).message;
            let error = synthesize_and_queue(
                &synthesizer,
                &playback,
                TtsRequest {
                    text: "provider failure must never fallback".to_string(),
                    voice_name: None,
                    speaking_rate: Some(1.0),
                    pitch: None,
                },
                None,
            )
            .await
            .expect_err("provider failure must propagate instead of trying another provider");

            assert_eq!(error, expected, "{kind:?}");
            assert_eq!(calls.load(Ordering::SeqCst), 1, "{kind:?}");
            assert!(!playback.is_playing(), "{kind:?}");
            assert_eq!(playback.queue_length(), 0, "{kind:?}");
        }
    }

    #[tokio::test]
    async fn explicit_cancellation_reaches_provider_contract_and_flushes_playback() {
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
