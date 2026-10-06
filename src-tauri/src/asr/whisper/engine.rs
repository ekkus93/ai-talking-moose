use std::sync::Arc;

use crate::asr::pipeline::PipelineEngine;
use crate::asr::transcript_state::StreamingTranscriptUpdate;
use crate::asr::types::{AsrError, AsrErrorKind};
use crate::asr::whisper::installer::WhisperModelInstaller;
use crate::asr::whisper::{
    path_to_cstring, NativeWhisperApi, WhisperApi, WhisperModel, WhisperSegment,
};
use tracing::debug;

/// Minimum accumulated samples before running a whisper.cpp batch transcription.
///
/// 4,800 samples at 16 kHz = 300 ms. Shorter windows produce poor segment
/// boundaries; longer windows increase perceived latency. This is a V1
/// trade-off and is not user-tunable yet.
pub const WHISPER_BATCH_THRESHOLD_SAMPLES: usize = 4_800;

/// Whisper batch transcription engine.
///
/// Owns the model and an opaque FFI handle. Audio accumulates into a buffer
/// until `WHISPER_BATCH_THRESHOLD_SAMPLES` is reached; on each threshold
/// crossing `whisper_full` is invoked and all resulting segments are emitted
/// as `Final`. On `stop` the remaining buffer is transcribed and the model
/// is released.
pub struct WhisperEngine {
    api: Box<dyn WhisperApi>,
    model: WhisperModel,
    audio_buffer: Vec<f32>,
    segment_counter: u64,
    cancelled: bool,
}

/// Open the Whisper engine, loading the model from the verified installer lease.
///
/// Returns `ModelNotInstalled` if the model is missing or `ModelLoadFailed`
/// if the native load fails. The lease is retained internally until the
/// model is fully loaded.
pub fn open(installer: Arc<WhisperModelInstaller>) -> Result<WhisperEngine, AsrError> {
    let lease = match installer.acquire_verified_model_lease() {
        Ok(Some(lease)) => lease,
        Ok(None) => {
            return Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: "The Whisper Small model is not installed. Install it in Settings before starting local speech recognition.".to_string(),
                retryable: false,
            });
        }
        Err(error) => {
            return Err(AsrError {
                kind: AsrErrorKind::ModelLoadFailed,
                message: error.message,
                retryable: error.retryable,
            });
        }
    };

    let api: Box<dyn WhisperApi> = Box::new(NativeWhisperApi);
    let model_path = lease.model_path().to_path_buf();

    let c_model_path = path_to_cstring(&model_path).map_err(|e| AsrError {
        kind: AsrErrorKind::ModelLoadFailed,
        message: e.message,
        retryable: false,
    })?;

    let model = api.load_model(&c_model_path).map_err(|e| AsrError {
        kind: AsrErrorKind::ModelLoadFailed,
        message: e.message,
        retryable: false,
    })?;

    debug!(
        "whisper_model_loaded {model_path}",
        model_path = model_path.display(),
    );

    Ok(WhisperEngine {
        api,
        model,
        audio_buffer: Vec::new(),
        segment_counter: 0,
        cancelled: false,
    })
}

impl PipelineEngine for WhisperEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        16_000
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        if self.cancelled {
            return Err(AsrError {
                kind: AsrErrorKind::Cancelled,
                message:
                    "Whisper Small local ASR was cancelled. No microphone audio was sent to Google."
                        .to_string(),
                retryable: true,
            });
        }
        if pcm.is_empty() {
            return Ok(Vec::new());
        }

        self.audio_buffer.extend_from_slice(pcm);

        if self.audio_buffer.len() < WHISPER_BATCH_THRESHOLD_SAMPLES {
            return Ok(Vec::new());
        }

        let samples = std::mem::take(&mut self.audio_buffer);
        debug!(
            "whisper_transcription_started {sample_count}",
            sample_count = samples.len(),
        );

        // Transcribe-call failure is a Whisper inference failure (the model
        // loaded fine; only the run itself failed), matching Moonshine's
        // transcribe-stream mapping (Inference, retryable).
        let transcript = self
            .api
            .transcribe(&self.model, &samples)
            .map_err(|e| AsrError {
                kind: AsrErrorKind::Inference,
                message: e.message,
                retryable: true,
            })?;

        Ok(self.collect_updates(transcript.segments))
    }

    fn stop(&mut self) -> Result<(), AsrError> {
        if self.cancelled {
            return Ok(());
        }

        if !self.audio_buffer.is_empty() {
            let samples = std::mem::take(&mut self.audio_buffer);
            debug!(
                "whisper_final_transcription {sample_count}",
                sample_count = samples.len(),
            );
            // Final flush on shutdown is still a Whisper inference failure,
            // not a missing runtime.
            let transcript = self
                .api
                .transcribe(&self.model, &samples)
                .map_err(|e| AsrError {
                    kind: AsrErrorKind::Inference,
                    message: e.message,
                    retryable: true,
                })?;
            let _ = self.collect_updates(transcript.segments);
        }

        self.cancelled = true;
        Ok(())
    }
}

impl Drop for WhisperEngine {
    fn drop(&mut self) {
        if self.cancelled {
            return;
        }
        self.cancelled = true;
        if !self.audio_buffer.is_empty() {
            let samples = std::mem::take(&mut self.audio_buffer);
            let _ = self.api.transcribe(&self.model, &samples);
        }
    }
}

impl WhisperEngine {
    fn collect_updates(&mut self, segments: Vec<WhisperSegment>) -> Vec<StreamingTranscriptUpdate> {
        let mut updates = Vec::new();
        for segment in segments {
            if segment.text.trim().is_empty() {
                continue;
            }

            let segment_id = self.segment_counter;
            self.segment_counter += 1;

            let update = StreamingTranscriptUpdate::Final {
                segment_id,
                text: segment.text,
                latency_ms: 0,
            };

            updates.push(update);
        }
        updates
    }
}
