use std::sync::Arc;
use std::time::Instant;

use crate::asr::pipeline::PipelineEngine;
use crate::asr::transcript_state::StreamingTranscriptUpdate;
use crate::asr::types::{AsrError, AsrErrorKind};
use crate::asr::whisper::installer::{map_install_error, WhisperModelInstaller};
use crate::asr::whisper::{
    path_to_cstring, NativeWhisperApi, WhisperApi, WhisperModel, WhisperSegment,
};
use tracing::debug;

/// Partial-inference cadence for an active utterance.
///
/// 4,800 samples at 16 kHz = 300 ms. This controls how often Whisper may refresh
/// a local partial; it is explicitly not an utterance/finality boundary.
pub const WHISPER_PARTIAL_INTERVAL_SAMPLES: usize = 4_800;

/// Consecutive local silence required to finalize an active utterance.
///
/// 8,000 samples at 16 kHz = 500 ms.
pub const WHISPER_ENDPOINT_SILENCE_SAMPLES: usize = 8_000;

/// Hard bound for one utterance before deterministic forced finalization.
///
/// 480,000 samples at 16 kHz = 30 seconds.
pub const WHISPER_MAX_UTTERANCE_SAMPLES: usize = 480_000;

/// Simple local RMS gate used only for endpointing. No PCM leaves the process.
pub const WHISPER_SPEECH_RMS_THRESHOLD: f32 = 0.008;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferenceAction {
    None,
    Partial,
    Final,
}

#[derive(Debug)]
struct WhisperUtteranceState {
    segment_id: u64,
    active: bool,
    last_partial_at_samples: usize,
    trailing_silence_samples: usize,
}

impl Default for WhisperUtteranceState {
    fn default() -> Self {
        Self {
            segment_id: 0,
            active: false,
            last_partial_at_samples: 0,
            trailing_silence_samples: 0,
        }
    }
}

impl WhisperUtteranceState {
    fn observe(&mut self, pcm: &[f32], total_samples: usize) -> InferenceAction {
        let has_speech = chunk_has_speech(pcm);
        if !self.active {
            if !has_speech {
                return InferenceAction::None;
            }
            self.active = true;
            self.last_partial_at_samples = 0;
            self.trailing_silence_samples = 0;
        }

        if has_speech {
            self.trailing_silence_samples = 0;
        } else {
            self.trailing_silence_samples = self
                .trailing_silence_samples
                .saturating_add(pcm.len());
        }

        if total_samples >= WHISPER_MAX_UTTERANCE_SAMPLES
            || self.trailing_silence_samples >= WHISPER_ENDPOINT_SILENCE_SAMPLES
        {
            return InferenceAction::Final;
        }

        if total_samples.saturating_sub(self.last_partial_at_samples)
            >= WHISPER_PARTIAL_INTERVAL_SAMPLES
        {
            self.last_partial_at_samples = total_samples;
            return InferenceAction::Partial;
        }

        InferenceAction::None
    }

    fn finish(&mut self) {
        self.active = false;
        self.last_partial_at_samples = 0;
        self.trailing_silence_samples = 0;
        self.segment_id = self.segment_id.saturating_add(1);
    }
}

/// Whisper utterance engine.
///
/// Audio is accumulated for one bounded logical utterance. Whisper inference may
/// refresh partial text on the configured cadence, but the same segment_id
/// remains active until local silence, maximum duration, or explicit stop.
pub struct WhisperEngine {
    api: Box<dyn WhisperApi>,
    model: WhisperModel,
    audio_buffer: Vec<f32>,
    utterance: WhisperUtteranceState,
    stopped: bool,
}

/// Open the Whisper engine from a verified model.
///
/// Missing/corrupt/runtime/load failures retain distinct public error kinds.
pub fn open(installer: Arc<WhisperModelInstaller>) -> Result<WhisperEngine, AsrError> {
    if !cfg!(whisper_native_linked) {
        return Err(AsrError {
            kind: AsrErrorKind::RuntimeUnavailable,
            message: crate::asr::whisper::manifest::WHISPER_RUNTIME_UNBUILT_MESSAGE.to_string(),
            retryable: false,
        });
    }

    let lease = match installer.acquire_verified_model_lease() {
        Ok(Some(lease)) => lease,
        Ok(None) => {
            return Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: "The Whisper Small model is not installed. Install it in Settings before starting local speech recognition.".to_string(),
                retryable: false,
            });
        }
        Err(error) => return Err(map_install_error(error)),
    };

    let api: Box<dyn WhisperApi> = Box::new(NativeWhisperApi);
    let model_path = lease.model_path().to_path_buf();
    let c_model_path = path_to_cstring(&model_path).map_err(|error| AsrError {
        kind: AsrErrorKind::ModelLoadFailed,
        message: error.message,
        retryable: false,
    })?;
    let model = api.load_model(&c_model_path).map_err(|error| AsrError {
        kind: AsrErrorKind::ModelLoadFailed,
        message: error.message,
        retryable: false,
    })?;

    debug!(
        "whisper_model_loaded {model_path}",
        model_path = model_path.display(),
    );

    Ok(WhisperEngine {
        api,
        model,
        audio_buffer: Vec::with_capacity(WHISPER_MAX_UTTERANCE_SAMPLES.min(32_000)),
        utterance: WhisperUtteranceState::default(),
        stopped: false,
    })
}

impl PipelineEngine for WhisperEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        16_000
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        if self.stopped {
            return Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: "Whisper Small local ASR received audio after it was stopped.".to_string(),
                retryable: false,
            });
        }
        if pcm.is_empty() {
            return Ok(Vec::new());
        }
        if pcm.iter().any(|sample| !sample.is_finite()) {
            return Err(AsrError {
                kind: AsrErrorKind::AudioInput,
                message: "Whisper Small local ASR received invalid PCM samples.".to_string(),
                retryable: true,
            });
        }

        let was_active = self.utterance.active;
        let has_speech = chunk_has_speech(pcm);
        if !was_active && !has_speech {
            return Ok(Vec::new());
        }

        self.audio_buffer.extend_from_slice(pcm);
        let action = self.utterance.observe(pcm, self.audio_buffer.len());
        match action {
            InferenceAction::None => Ok(Vec::new()),
            InferenceAction::Partial => self.transcribe_current(false),
            InferenceAction::Final => self.transcribe_current(true),
        }
    }

    fn stop(&mut self) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        if self.stopped {
            return Ok(Vec::new());
        }
        self.stopped = true;

        if !self.utterance.active || self.audio_buffer.is_empty() {
            self.audio_buffer.clear();
            return Ok(Vec::new());
        }

        self.transcribe_current(true)
    }
}

impl Drop for WhisperEngine {
    fn drop(&mut self) {
        // The worker owns graceful finalization and event delivery. Drop must
        // never perform hidden inference whose transcript cannot be forwarded.
        self.stopped = true;
        self.audio_buffer.clear();
    }
}

impl WhisperEngine {
    fn transcribe_current(
        &mut self,
        finalize: bool,
    ) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        if self.audio_buffer.is_empty() {
            if finalize {
                self.utterance.finish();
            }
            return Ok(Vec::new());
        }

        let inference_started = Instant::now();
        let result = self.api.transcribe(&self.model, &self.audio_buffer);
        let latency_ms =
            u32::try_from(inference_started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let segment_id = self.utterance.segment_id;

        if finalize {
            self.audio_buffer.clear();
            self.utterance.finish();
        }

        let transcript = result.map_err(|error| AsrError {
            kind: AsrErrorKind::Inference,
            message: error.message,
            retryable: true,
        })?;
        let text = transcript_text(&transcript.segments);

        if finalize {
            return Ok(vec![StreamingTranscriptUpdate::Final {
                segment_id,
                text,
                latency_ms,
            }]);
        }

        if text.trim().is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![StreamingTranscriptUpdate::Partial {
                segment_id,
                text,
                latency_ms,
            }])
        }
    }
}

fn chunk_has_speech(pcm: &[f32]) -> bool {
    if pcm.is_empty() {
        return false;
    }
    let sum_squares = pcm
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let rms = (sum_squares / pcm.len() as f64).sqrt() as f32;
    rms >= WHISPER_SPEECH_RMS_THRESHOLD
}

fn transcript_text(segments: &[WhisperSegment]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speech(samples: usize) -> Vec<f32> {
        vec![0.1; samples]
    }

    fn silence(samples: usize) -> Vec<f32> {
        vec![0.0; samples]
    }

    #[test]
    fn partial_cadence_is_not_finality_and_keeps_stable_segment_id() {
        let mut state = WhisperUtteranceState::default();
        let first = speech(WHISPER_PARTIAL_INTERVAL_SAMPLES);
        assert_eq!(state.observe(&first, first.len()), InferenceAction::Partial);
        assert!(state.active);
        assert_eq!(state.segment_id, 0);

        let second = speech(WHISPER_PARTIAL_INTERVAL_SAMPLES);
        assert_eq!(
            state.observe(&second, first.len() + second.len()),
            InferenceAction::Partial
        );
        assert!(state.active);
        assert_eq!(state.segment_id, 0);
    }

    #[test]
    fn endpoint_silence_finalizes_once_then_advances_segment_id() {
        let mut state = WhisperUtteranceState::default();
        let voiced = speech(1_600);
        assert_eq!(state.observe(&voiced, voiced.len()), InferenceAction::None);

        let quiet = silence(WHISPER_ENDPOINT_SILENCE_SAMPLES);
        assert_eq!(
            state.observe(&quiet, voiced.len() + quiet.len()),
            InferenceAction::Final
        );
        state.finish();
        assert!(!state.active);
        assert_eq!(state.segment_id, 1);
    }

    #[test]
    fn maximum_utterance_duration_forces_finalization() {
        let mut state = WhisperUtteranceState::default();
        let voiced = speech(WHISPER_MAX_UTTERANCE_SAMPLES);
        assert_eq!(state.observe(&voiced, voiced.len()), InferenceAction::Final);
    }

    #[test]
    fn leading_silence_does_not_start_an_utterance() {
        let mut state = WhisperUtteranceState::default();
        let quiet = silence(1_600);
        assert_eq!(state.observe(&quiet, quiet.len()), InferenceAction::None);
        assert!(!state.active);
    }

    #[test]
    fn transcript_text_collapses_native_segments_into_one_utterance() {
        let segments = vec![
            WhisperSegment {
                text: " hello ".to_string(),
                start_ms: 0,
                end_ms: 100,
                no_speech_prob: 0.0,
            },
            WhisperSegment {
                text: "world".to_string(),
                start_ms: 100,
                end_ms: 200,
                no_speech_prob: 0.0,
            },
        ];
        assert_eq!(transcript_text(&segments), "hello world");
    }
}
