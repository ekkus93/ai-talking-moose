pub use super::wake_word::policy::{
    DEFAULT_WAKE_PHRASE, V1_KWS_CHANNELS, V1_KWS_FEATURE_DIM, V1_KWS_KEYWORD,
    V1_KWS_SAMPLE_RATE_HZ, V1_KWS_THREADS, V1_WAKE_SCORE, V1_WAKE_THRESHOLD,
};
use crate::asr::wake_word_sherpa_manifest::SHERPA_KWS_REQUIRED_FILES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeWordErrorKind {
    MissingArtifact,
    InvalidArtifact,
    InvalidConfiguration,
    RuntimeUnavailable,
    Inference,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeWordError {
    pub kind: WakeWordErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl WakeWordError {
    pub fn sanitized(kind: WakeWordErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        let message = sanitize_error_message(&message.into());
        Self {
            kind,
            message,
            retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SherpaKwsConfig {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub feature_dim: u16,
    pub threads: u16,
    pub keyword: String,
    pub score: f32,
    pub threshold: f32,
}

impl Default for SherpaKwsConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: V1_KWS_SAMPLE_RATE_HZ,
            channels: V1_KWS_CHANNELS,
            feature_dim: V1_KWS_FEATURE_DIM,
            threads: V1_KWS_THREADS,
            keyword: V1_KWS_KEYWORD.to_string(),
            score: V1_WAKE_SCORE,
            threshold: V1_WAKE_THRESHOLD,
        }
    }
}

impl SherpaKwsConfig {
    pub fn required_artifact_files(&self) -> &'static [&'static str; 5] {
        &SHERPA_KWS_REQUIRED_FILES
    }

    pub fn validate(&self) -> Result<(), WakeWordError> {
        if self.sample_rate_hz != V1_KWS_SAMPLE_RATE_HZ {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS sample rate must be 16000 Hz",
                false,
            ));
        }
        if self.channels != V1_KWS_CHANNELS {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS input must be mono",
                false,
            ));
        }
        if self.feature_dim != V1_KWS_FEATURE_DIM {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS feature dimension must be 80",
                false,
            ));
        }
        if self.threads != V1_KWS_THREADS {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS V1 must use one inference thread",
                false,
            ));
        }
        if !self.keyword.trim().eq_ignore_ascii_case(V1_KWS_KEYWORD) {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                format!("wake KWS keyword must match {DEFAULT_WAKE_PHRASE}"),
                false,
            ));
        }
        if !self.score.is_finite() || self.score != V1_WAKE_SCORE {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS V1 score must be 1.0",
                false,
            ));
        }
        if !self.threshold.is_finite() || self.threshold != V1_WAKE_THRESHOLD {
            return Err(WakeWordError::sanitized(
                WakeWordErrorKind::InvalidConfiguration,
                "wake KWS V1 threshold must be 0.25",
                false,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WakeWordDetection {
    pub keyword: String,
    pub score: f32,
}

impl WakeWordDetection {
    pub fn v1_detected(score: f32) -> Self {
        Self {
            keyword: DEFAULT_WAKE_PHRASE.to_string(),
            score,
        }
    }
}

pub trait SherpaKwsEngine {
    fn config(&self) -> &SherpaKwsConfig;
    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError>;
    fn reset_stream(&mut self) -> Result<(), WakeWordError>;
    fn shutdown(&mut self) -> Result<(), WakeWordError>;
}

pub fn validate_pcm_frame(sample_rate_hz: u32, samples: &[i16]) -> Result<(), WakeWordError> {
    if sample_rate_hz != V1_KWS_SAMPLE_RATE_HZ {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidConfiguration,
            "wake KWS frame sample rate must be 16000 Hz",
            false,
        ));
    }
    if samples.is_empty() {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidConfiguration,
            "wake KWS frame must contain at least one sample",
            false,
        ));
    }
    Ok(())
}

fn sanitize_error_message(message: &str) -> String {
    let mut sanitized = String::with_capacity(message.len());
    for token in message.split_whitespace() {
        let looks_like_path = token.contains('/') || token.contains('\\');
        let looks_like_secret = token.len() >= 24
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
        if looks_like_path {
            sanitized.push_str("<path>");
        } else if looks_like_secret {
            sanitized.push_str("<redacted>");
        } else {
            sanitized.push_str(token);
        }
        sanitized.push(' ');
    }
    sanitized.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_freezes_v1_sherpa_policy() {
        let config = SherpaKwsConfig::default();
        assert_eq!(config.sample_rate_hz, 16_000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.feature_dim, 80);
        assert_eq!(config.threads, 1);
        assert_eq!(config.keyword, "HEY MOOSE");
        assert_eq!(config.score, 1.0);
        assert_eq!(config.threshold, 0.25);
        config.validate().unwrap();
    }

    #[test]
    fn config_rejects_drift_from_frozen_v1_policy() {
        let config = SherpaKwsConfig {
            sample_rate_hz: 48_000,
            ..Default::default()
        };
        assert_eq!(
            config.validate().unwrap_err().kind,
            WakeWordErrorKind::InvalidConfiguration
        );

        let config = SherpaKwsConfig {
            threads: 2,
            ..Default::default()
        };
        assert_eq!(
            config.validate().unwrap_err().message,
            "wake KWS V1 must use one inference thread"
        );

        let config = SherpaKwsConfig {
            channels: 2,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = SherpaKwsConfig {
            score: 0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = SherpaKwsConfig {
            keyword: "HEY BRUCE".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = SherpaKwsConfig {
            threshold: 0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn engine_artifact_contract_matches_manifest_exactly() {
        let config = SherpaKwsConfig::default();
        assert_eq!(
            config.required_artifact_files(),
            &[
                "encoder-epoch-12-avg-2-chunk-16-left-64.onnx",
                "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
                "joiner-epoch-12-avg-2-chunk-16-left-64.onnx",
                "tokens.txt",
                "bpe.model",
            ]
        );
    }

    #[test]
    fn pcm_frames_must_be_canonical_nonempty_mono_stream_samples() {
        validate_pcm_frame(16_000, &[0, 1, -1]).unwrap();
        assert!(validate_pcm_frame(48_000, &[0]).is_err());
        assert!(validate_pcm_frame(16_000, &[]).is_err());
    }

    #[test]
    fn detection_event_is_bounded_and_never_contains_audio() {
        let detection = WakeWordDetection::v1_detected(0.77);
        assert_eq!(detection.keyword, DEFAULT_WAKE_PHRASE);
        assert_eq!(detection.score, 0.77);
    }

    #[test]
    fn errors_sanitize_paths_and_token_like_secrets() {
        let error = WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "failed /tmp/private/model.onnx token abcdefghijklmnopqrstuvwxyz123456",
            true,
        );
        assert_eq!(error.message, "failed <path> token <redacted>");
        assert!(error.retryable);
    }

    struct FakeEngine {
        config: SherpaKwsConfig,
        triggered: bool,
        shutdowns: u8,
    }

    impl SherpaKwsEngine for FakeEngine {
        fn config(&self) -> &SherpaKwsConfig {
            &self.config
        }

        fn accept_pcm16_mono(
            &mut self,
            sample_rate_hz: u32,
            samples: &[i16],
        ) -> Result<Option<WakeWordDetection>, WakeWordError> {
            validate_pcm_frame(sample_rate_hz, samples)?;
            if self.triggered {
                Ok(None)
            } else {
                self.triggered = true;
                Ok(Some(WakeWordDetection::v1_detected(V1_WAKE_SCORE)))
            }
        }

        fn reset_stream(&mut self) -> Result<(), WakeWordError> {
            self.triggered = false;
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), WakeWordError> {
            self.shutdowns = self.shutdowns.saturating_add(1);
            Ok(())
        }
    }

    #[test]
    fn engine_boundary_supports_feed_reset_and_idempotent_shutdown_contract() {
        let mut engine = FakeEngine {
            config: SherpaKwsConfig::default(),
            triggered: false,
            shutdowns: 0,
        };
        engine.config().validate().unwrap();
        assert!(engine
            .accept_pcm16_mono(16_000, &[1, 2, 3])
            .unwrap()
            .is_some());
        assert!(engine
            .accept_pcm16_mono(16_000, &[1, 2, 3])
            .unwrap()
            .is_none());
        engine.reset_stream().unwrap();
        assert!(engine
            .accept_pcm16_mono(16_000, &[1, 2, 3])
            .unwrap()
            .is_some());
        engine.shutdown().unwrap();
        engine.shutdown().unwrap();
        assert_eq!(engine.shutdowns, 2);
    }
}
