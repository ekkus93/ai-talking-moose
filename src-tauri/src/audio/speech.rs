use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::TtsRequest;
use crate::audio::playback::{AudioPlayback, PlaybackEnqueueReport};

/// Synthesize one standalone Moose utterance and queue it through the authoritative
/// Rust/CPAL playback path. Browser speech synthesis and platform speech subprocesses
/// are intentionally not fallback paths here.
pub async fn synthesize_and_queue(
    synthesizer: &dyn SpeechSynthesizer,
    playback: &AudioPlayback,
    request: TtsRequest,
    output_device: Option<String>,
) -> Result<PlaybackEnqueueReport, String> {
    let audio = synthesizer
        .synthesize(request)
        .await
        .map_err(|error| error.message)?;
    playback
        .start(output_device)
        .map_err(|error_value| error_value.to_string())?;
    playback
        .enqueue_pcm_bytes(&audio.pcm_bytes, audio.sample_rate)
        .map_err(|error_value| error_value.to_string())
}
