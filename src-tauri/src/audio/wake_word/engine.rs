use ring::digest::{digest, SHA256};
use serde::Deserialize;
use sherpa_onnx::{KeywordSpotter, KeywordSpotterConfig, OnlineStream};
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const WAKE_SAMPLE_RATE_HZ: u32 = 16_000;
pub const WAKE_CHANNELS: u16 = 1;
pub const WAKE_INFERENCE_THREADS: i32 = 1;
pub const DEFAULT_KEYWORDS_SCORE: f32 = 1.0;
pub const DEFAULT_KEYWORDS_THRESHOLD: f32 = 0.25;
const KEYWORD_TOKENS: &str = "HH EY1 M UW1 S @HEY_MOOSE";
const ARTIFACT_MANIFEST: &str = include_str!("../../../resources/wake_word/artifacts-v1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[serde(rename_all = "snake_case")]
pub enum WakeWordErrorKind {
    #[error("artifact")]
    Artifact,
    #[error("runtime_unavailable")]
    RuntimeUnavailable,
    #[error("audio_input")]
    AudioInput,
    #[error("inference")]
    Inference,
    #[error("invalid_state")]
    InvalidState,
    #[error("unsupported_platform")]
    UnsupportedPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct WakeWordError {
    pub kind: WakeWordErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl WakeWordError {
    pub fn artifact(message: &str) -> Self {
        Self {
            kind: WakeWordErrorKind::Artifact,
            message: message.to_string(),
            retryable: false,
        }
    }

    pub fn runtime(message: &str) -> Self {
        Self {
            kind: WakeWordErrorKind::RuntimeUnavailable,
            message: message.to_string(),
            retryable: true,
        }
    }

    pub fn inference(message: &str) -> Self {
        Self {
            kind: WakeWordErrorKind::Inference,
            message: message.to_string(),
            retryable: true,
        }
    }
}

#[derive(Deserialize)]
struct Manifest {
    sample_rate_hz: u32,
    channels: u16,
    keyword_tokens: String,
    model_artifacts: Vec<ModelArtifact>,
}

#[derive(Deserialize)]
struct ModelArtifact {
    path: String,
    bytes: u64,
    sha256: String,
}

pub trait WakeWordEngine: Send {
    fn process_pcm_i16(&mut self, samples: &[i16]) -> Result<bool, WakeWordError>;
    fn reset(&mut self) -> Result<(), WakeWordError>;
    fn shutdown(&mut self) -> Result<(), WakeWordError>;
    fn inference_threads(&self) -> i32;
}

pub fn verify_model_artifacts(model_root: &Path) -> Result<(), WakeWordError> {
    let manifest: Manifest = serde_json::from_str(ARTIFACT_MANIFEST)
        .map_err(|_| WakeWordError::artifact("Wake-word artifact manifest is invalid."))?;
    if manifest.sample_rate_hz != WAKE_SAMPLE_RATE_HZ
        || manifest.channels != WAKE_CHANNELS
        || manifest.keyword_tokens != KEYWORD_TOKENS
    {
        return Err(WakeWordError::artifact(
            "Wake-word artifact manifest does not match the V1 audio/keyword contract.",
        ));
    }

    for artifact in manifest.model_artifacts {
        let path = model_root.join(&artifact.path);
        let metadata = fs::metadata(&path).map_err(|_| {
            WakeWordError::artifact("A required wake-word model artifact is unavailable.")
        })?;
        if metadata.len() != artifact.bytes {
            return Err(WakeWordError::artifact(
                "A wake-word model artifact failed size verification.",
            ));
        }
        let bytes = fs::read(path).map_err(|_| {
            WakeWordError::artifact("A required wake-word model artifact could not be read.")
        })?;
        let actual = digest(&SHA256, &bytes);
        let actual_hex = actual
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual_hex != artifact.sha256 {
            return Err(WakeWordError::artifact(
                "A wake-word model artifact failed digest verification.",
            ));
        }
    }
    Ok(())
}

pub struct SherpaKwsEngine {
    spotter: Option<KeywordSpotter>,
    stream: Option<OnlineStream>,
    stopped: bool,
}

impl SherpaKwsEngine {
    pub fn open(model_root: &Path) -> Result<Self, WakeWordError> {
        verify_model_artifacts(model_root)?;

        let mut config = KeywordSpotterConfig::default();
        config.model_config.transducer.encoder = Some(
            model_root
                .join("assets/onnx/encoder.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.transducer.decoder = Some(
            model_root
                .join("assets/onnx/decoder.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.transducer.joiner = Some(
            model_root
                .join("assets/onnx/joiner.onnx")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.tokens = Some(
            model_root
                .join("assets/tokens.txt")
                .to_string_lossy()
                .into_owned(),
        );
        config.model_config.num_threads = WAKE_INFERENCE_THREADS;
        config.model_config.provider = Some("cpu".to_string());
        config.keywords_file = Some(
            model_root
                .join("keywords.txt")
                .to_string_lossy()
                .into_owned(),
        );
        config.keywords_score = DEFAULT_KEYWORDS_SCORE;
        config.keywords_threshold = DEFAULT_KEYWORDS_THRESHOLD;

        let spotter = KeywordSpotter::create(&config).ok_or_else(|| {
            WakeWordError::runtime("The local wake-word engine could not be initialized.")
        })?;
        let stream = spotter.create_stream();
        Ok(Self {
            spotter: Some(spotter),
            stream: Some(stream),
            stopped: false,
        })
    }

    fn active_parts(&self) -> Result<(&KeywordSpotter, &OnlineStream), WakeWordError> {
        if self.stopped {
            return Err(WakeWordError {
                kind: WakeWordErrorKind::InvalidState,
                message: "The wake-word engine is stopped.".to_string(),
                retryable: true,
            });
        }
        match (self.spotter.as_ref(), self.stream.as_ref()) {
            (Some(spotter), Some(stream)) => Ok((spotter, stream)),
            _ => Err(WakeWordError::runtime(
                "The local wake-word engine is unavailable.",
            )),
        }
    }
}

impl WakeWordEngine for SherpaKwsEngine {
    fn process_pcm_i16(&mut self, samples: &[i16]) -> Result<bool, WakeWordError> {
        if samples.is_empty() {
            return Ok(false);
        }
        let pcm = samples
            .iter()
            .map(|sample| f32::from(*sample) / 32768.0)
            .collect::<Vec<_>>();
        let (spotter, stream) = self.active_parts()?;
        stream.accept_waveform(WAKE_SAMPLE_RATE_HZ as i32, &pcm);

        let mut detected = false;
        while spotter.is_ready(stream) {
            spotter.decode(stream);
            if spotter
                .get_result(stream)
                .is_some_and(|result| !result.keyword.trim().is_empty())
            {
                detected = true;
                break;
            }
        }
        if detected {
            spotter.reset(stream);
        }
        Ok(detected)
    }

    fn reset(&mut self) -> Result<(), WakeWordError> {
        let (spotter, stream) = self.active_parts()?;
        spotter.reset(stream);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        if self.stopped {
            return Ok(());
        }
        self.stopped = true;
        self.stream.take();
        self.spotter.take();
        Ok(())
    }

    fn inference_threads(&self) -> i32 {
        WAKE_INFERENCE_THREADS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_artifacts_fail_with_sanitized_error() {
        let root = tempdir().unwrap();
        let error = verify_model_artifacts(root.path()).unwrap_err();
        assert_eq!(error.kind, WakeWordErrorKind::Artifact);
        assert!(!error
            .message
            .contains(root.path().to_string_lossy().as_ref()));
        assert!(!error.message.to_ascii_lowercase().contains("pcm"));
    }

    #[test]
    fn v1_kws_configuration_is_explicit_and_deterministic() {
        assert_eq!(WAKE_SAMPLE_RATE_HZ, 16_000);
        assert_eq!(WAKE_CHANNELS, 1);
        assert_eq!(WAKE_INFERENCE_THREADS, 1);
        assert_eq!(DEFAULT_KEYWORDS_SCORE, 1.0);
        assert_eq!(DEFAULT_KEYWORDS_THRESHOLD, 0.25);
        assert_eq!(KEYWORD_TOKENS, "HH EY1 M UW1 S @HEY_MOOSE");
    }
}
