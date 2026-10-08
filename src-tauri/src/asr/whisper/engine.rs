use std::sync::Arc;
use std::time::Instant;

use crate::asr::pipeline::PipelineEngine;
use crate::asr::transcript_state::StreamingTranscriptUpdate;
use crate::asr::types::AsrError;
use crate::asr::whisper::installer::{
    map_whisper_failure, WhisperFailure, WhisperModelInstaller, WhisperVerifiedModelLease,
};
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

/// Runtime-tunable utterance behavior. Production uses the defaults; keeping
/// these values in an explicit config allows acceptance and future UX settings
/// to select cadence without making cadence a finality boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhisperEngineConfig {
    pub partial_interval_samples: usize,
    pub endpoint_silence_samples: usize,
    pub maximum_utterance_samples: usize,
    pub speech_rms_threshold: f32,
}

impl Default for WhisperEngineConfig {
    fn default() -> Self {
        Self {
            partial_interval_samples: WHISPER_PARTIAL_INTERVAL_SAMPLES,
            endpoint_silence_samples: WHISPER_ENDPOINT_SILENCE_SAMPLES,
            maximum_utterance_samples: WHISPER_MAX_UTTERANCE_SAMPLES,
            speech_rms_threshold: WHISPER_SPEECH_RMS_THRESHOLD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferenceAction {
    None,
    Partial,
    Final,
}

#[derive(Debug, Default)]
struct WhisperUtteranceState {
    config: WhisperEngineConfig,
    segment_id: u64,
    active: bool,
    final_update_pending_delivery: bool,
    last_partial_at_samples: usize,
    trailing_silence_samples: usize,
}

impl WhisperUtteranceState {
    fn with_config(config: WhisperEngineConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    fn observe(&mut self, pcm: &[f32], total_samples: usize) -> InferenceAction {
        let has_speech = chunk_has_speech(pcm, self.config.speech_rms_threshold);
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
            self.trailing_silence_samples = self.trailing_silence_samples.saturating_add(pcm.len());
        }

        if total_samples >= self.config.maximum_utterance_samples
            || self.trailing_silence_samples >= self.config.endpoint_silence_samples
        {
            return InferenceAction::Final;
        }

        if total_samples.saturating_sub(self.last_partial_at_samples)
            >= self.config.partial_interval_samples
        {
            self.last_partial_at_samples = total_samples;
            return InferenceAction::Partial;
        }

        InferenceAction::None
    }

    fn finish(&mut self) {
        self.active = false;
        self.final_update_pending_delivery = false;
        self.last_partial_at_samples = 0;
        self.trailing_silence_samples = 0;
        self.segment_id = self.segment_id.saturating_add(1);
    }

    fn acknowledge_final_delivery(&mut self, segment_id: u64) -> bool {
        if !self.final_update_pending_delivery || !self.active || self.segment_id != segment_id {
            return false;
        }
        self.finish();
        true
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
    /// Holds the installer operation lock for the whole native model lifetime so
    /// user-initiated delete/reinstall cannot race the verified model path while
    /// the worker is loading or transcribing with it.
    _lease: WhisperVerifiedModelLease,
    audio_buffer: Vec<f32>,
    utterance: WhisperUtteranceState,
    config: WhisperEngineConfig,
    stopped: bool,
}

/// Open the Whisper engine from a verified model.
///
/// Missing/corrupt/runtime/load failures retain distinct public error kinds.
pub fn open(installer: Arc<WhisperModelInstaller>) -> Result<WhisperEngine, AsrError> {
    open_with_config(installer, WhisperEngineConfig::default())
}

pub fn open_with_config(
    installer: Arc<WhisperModelInstaller>,
    config: WhisperEngineConfig,
) -> Result<WhisperEngine, AsrError> {
    if !cfg!(whisper_native_linked) {
        return Err(map_whisper_failure(WhisperFailure::RuntimeUnavailable));
    }

    let lease = match installer.acquire_verified_model_lease() {
        Ok(Some(lease)) => lease,
        Ok(None) => return Err(map_whisper_failure(WhisperFailure::ModelNotInstalled)),
        Err(error) => return Err(map_whisper_failure(WhisperFailure::Verification(error))),
    };

    let api: Box<dyn WhisperApi> = Box::new(NativeWhisperApi);
    let model_path = lease.model_path().to_path_buf();
    let c_model_path = path_to_cstring(&model_path)
        .map_err(|error| map_whisper_failure(WhisperFailure::ModelLoad(error.message)))?;
    let model = api
        .load_model(&c_model_path)
        .map_err(|error| map_whisper_failure(WhisperFailure::ModelLoad(error.message)))?;

    debug!(
        "whisper_model_loaded {model_path}",
        model_path = model_path.display(),
    );

    Ok(WhisperEngine {
        api,
        model,
        _lease: lease,
        audio_buffer: Vec::with_capacity(WHISPER_MAX_UTTERANCE_SAMPLES.min(32_000)),
        utterance: WhisperUtteranceState::with_config(config),
        config,
        stopped: false,
    })
}

impl PipelineEngine for WhisperEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        16_000
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        if self.stopped {
            return Err(map_whisper_failure(WhisperFailure::InvalidState(
                "Whisper Small local ASR received audio after it was stopped.".to_string(),
            )));
        }
        if self.utterance.final_update_pending_delivery {
            return Err(map_whisper_failure(WhisperFailure::Internal(
                "Whisper Small received audio before its final transcript was delivered."
                    .to_string(),
            )));
        }
        if pcm.is_empty() {
            return Ok(Vec::new());
        }
        if pcm.iter().any(|sample| !sample.is_finite()) {
            return Err(map_whisper_failure(WhisperFailure::AudioInput));
        }

        let was_active = self.utterance.active;
        let has_speech = chunk_has_speech(pcm, self.config.speech_rms_threshold);
        if !was_active && !has_speech {
            return Ok(Vec::new());
        }

        append_bounded_pcm(
            &mut self.audio_buffer,
            pcm,
            self.config.maximum_utterance_samples,
        );
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

    fn updates_delivered(&mut self, updates: &[StreamingTranscriptUpdate]) {
        if updates.iter().any(|update| {
            matches!(
                update,
                StreamingTranscriptUpdate::Final { segment_id, .. }
                    if self.utterance.acknowledge_final_delivery(*segment_id)
            )
        }) {
            self.audio_buffer.clear();
        }
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
        let latency_ms = u32::try_from(inference_started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let segment_id = self.utterance.segment_id;

        let transcript = result
            .map_err(|error| map_whisper_failure(WhisperFailure::Inference(error.message)))?;
        let text = transcript_text(&transcript.segments);

        if finalize {
            self.utterance.final_update_pending_delivery = true;
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

fn chunk_has_speech(pcm: &[f32], threshold: f32) -> bool {
    if pcm.is_empty() {
        return false;
    }
    let sum_squares = pcm
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let rms = (sum_squares / pcm.len() as f64).sqrt() as f32;
    rms >= threshold
}

fn append_bounded_pcm(buffer: &mut Vec<f32>, pcm: &[f32], maximum_samples: usize) {
    let remaining = maximum_samples.saturating_sub(buffer.len());
    buffer.extend_from_slice(&pcm[..pcm.len().min(remaining)]);
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
    fn partial_cadence_can_be_configured_independently_of_finality() {
        let config = WhisperEngineConfig {
            partial_interval_samples: 1_600,
            ..WhisperEngineConfig::default()
        };
        let mut state = WhisperUtteranceState::with_config(config);
        let first = speech(1_600);
        assert_eq!(state.observe(&first, first.len()), InferenceAction::Partial);
        let second = speech(1_600);
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
    fn utterance_resets_only_after_final_update_delivery_acknowledgement() {
        let mut state = WhisperUtteranceState::default();
        let voiced = speech(WHISPER_PARTIAL_INTERVAL_SAMPLES);
        assert_eq!(
            state.observe(&voiced, voiced.len()),
            InferenceAction::Partial
        );
        let quiet = silence(WHISPER_ENDPOINT_SILENCE_SAMPLES);
        assert_eq!(
            state.observe(&quiet, voiced.len() + quiet.len()),
            InferenceAction::Final
        );

        state.final_update_pending_delivery = true;
        assert!(state.active);
        assert_eq!(state.segment_id, 0);
        assert!(!state.acknowledge_final_delivery(1));
        assert!(
            state.active,
            "a mismatched update must not reset the utterance"
        );
        assert!(state.acknowledge_final_delivery(0));
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
    fn pcm_window_never_exceeds_configured_maximum() {
        let mut buffer = Vec::new();
        append_bounded_pcm(&mut buffer, &speech(6), 4);
        assert_eq!(buffer, speech(4));
        append_bounded_pcm(&mut buffer, &speech(3), 4);
        assert_eq!(buffer, speech(4));
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
