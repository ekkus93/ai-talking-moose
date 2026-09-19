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

    /// Cancel one utterance only if `token` still owns the standalone slot.
    /// This is used by ambient cancellation so stale cleanup can never cancel a
    /// newer foreground utterance that took ownership after the ambient request.
    pub fn cancel_if_current(&self, playback: &AudioPlayback, token: &CancellationToken) -> bool {
        let current = self.current.lock();
        if current.is_cancelled() || &*current != token {
            return false;
        }
        current.cancel();
        playback.flush();
        true
    }

    /// Return whether `token` still owns the authoritative standalone slot,
    /// regardless of whether that current token has been cancelled. Terminal
    /// cleanup uses this to distinguish cancellation of the current utterance
    /// from cleanup of an utterance superseded by a newer one.
    pub fn owns_slot(&self, token: &CancellationToken) -> bool {
        let current = self.current.lock();
        &*current == token
    }

    pub fn is_current(&self, token: &CancellationToken) -> bool {
        let current = self.current.lock();
        !current.is_cancelled() && &*current == token
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
    playback
        .enqueue(&audio.samples, audio.sample_rate_hz)
        .map_err(|error_value| error_value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::traits::SpeechSynthesizer;
    use crate::ai::types::{AudioBuffer, ProviderError};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct NeverSynthesizer;

    #[async_trait]
    impl SpeechSynthesizer for NeverSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioBuffer, ProviderError> {
            futures::future::pending().await
        }

        async fn synthesize_cancellable(
            &self,
            _request: TtsRequest,
            cancellation: &CancellationToken,
        ) -> Result<AudioBuffer, ProviderError> {
            cancellation.cancelled().await;
            Err(ProviderError::cancelled("cancelled"))
        }
    }

    struct CountingFailSynthesizer {
        calls: Arc<AtomicUsize>,
        error: ProviderError,
    }

    #[async_trait]
    impl SpeechSynthesizer for CountingFailSynthesizer {
        async fn synthesize(&self, _request: TtsRequest) -> Result<AudioBuffer, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(self.error.clone())
        }
    }

    #[tokio::test]
    async fn standalone_speech_starts_playback_and_queues_audio() {
        struct StaticSynthesizer;

        #[async_trait]
        impl SpeechSynthesizer for StaticSynthesizer {
            async fn synthesize(&self, _request: TtsRequest) -> Result<AudioBuffer, ProviderError> {
                Ok(AudioBuffer {
                    samples: vec![0.25, -0.25, 0.5],
                    sample_rate_hz: 24_000,
                })
            }
        }

        let playback = AudioPlayback::new_mock();
        let report = synthesize_and_queue(
            &StaticSynthesizer,
            &playback,
            TtsRequest {
                text: "Hello".to_string(),
                voice_name: None,
                speaking_rate: None,
                pitch: None,
            },
            None,
        )
        .await
        .unwrap();

        assert!(playback.is_playing());
        assert_eq!(report.queued_samples, 3);
        assert_eq!(report.source_sample_rate_hz, 24_000);
    }

    #[tokio::test]
    async fn provider_failures_propagate_without_fallback() {
        for (kind, expected) in [
            (ProviderErrorKind::Auth, "auth failure"),
            (ProviderErrorKind::RateLimit, "rate limit"),
            (ProviderErrorKind::Network, "network failure"),
            (ProviderErrorKind::InvalidRequest, "invalid request"),
            (ProviderErrorKind::Other, "provider failure"),
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let synthesizer = CountingFailSynthesizer {
                calls: calls.clone(),
                error: ProviderError::new(kind, expected),
            };
            let playback = AudioPlayback::new_mock();
            let error = synthesize_and_queue(
                &synthesizer,
                &playback,
                TtsRequest {
                    text: "No fallback".to_string(),
                    voice_name: None,
                    speaking_rate: None,
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

    #[test]
    fn ownership_aware_cancellation_never_cancels_newer_foreground_speech() {
        let playback = AudioPlayback::new_mock();
        let controller = StandaloneSpeechController::new();
        let ambient = controller.begin(&playback);
        let foreground = controller.begin(&playback);

        assert!(!controller.cancel_if_current(&playback, &ambient));
        assert!(controller.is_current(&foreground));
        assert!(!foreground.is_cancelled());

        assert!(controller.cancel_if_current(&playback, &foreground));
        assert!(foreground.is_cancelled());
    }

    #[test]
    fn slot_ownership_survives_current_cancellation_but_not_supersession() {
        let playback = AudioPlayback::new_mock();
        let controller = StandaloneSpeechController::new();
        let first = controller.begin(&playback);

        assert!(controller.is_current(&first));
        assert!(controller.owns_slot(&first));

        assert!(controller.cancel_if_current(&playback, &first));
        assert!(!controller.is_current(&first));
        assert!(controller.owns_slot(&first));

        let second = controller.begin(&playback);
        assert!(!controller.is_current(&first));
        assert!(!controller.owns_slot(&first));
        assert!(controller.is_current(&second));
        assert!(controller.owns_slot(&second));
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
