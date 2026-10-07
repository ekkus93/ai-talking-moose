use super::*;
use std::ffi::{CStr, CString};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

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

struct CountingEngine {
    config: SherpaKwsConfig,
    accepted_batches: usize,
    accepted_samples: usize,
}

impl SherpaKwsEngine for CountingEngine {
    fn config(&self) -> &SherpaKwsConfig {
        &self.config
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        validate_pcm_frame(sample_rate_hz, samples)?;
        self.accepted_batches = self.accepted_batches.saturating_add(1);
        self.accepted_samples = self.accepted_samples.saturating_add(samples.len());
        Ok(None)
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        Ok(())
    }
}

#[test]
fn invalid_pcm_is_rejected_before_kws_feed_mutation() {
    let mut engine = CountingEngine {
        config: SherpaKwsConfig::default(),
        accepted_batches: 0,
        accepted_samples: 0,
    };

    assert!(engine.accept_pcm16_mono(48_000, &[1, 2]).is_err());
    assert!(engine.accept_pcm16_mono(16_000, &[]).is_err());
    assert_eq!(engine.accepted_batches, 0);
    assert_eq!(engine.accepted_samples, 0);

    assert!(engine.accept_pcm16_mono(16_000, &[1, 2]).unwrap().is_none());
    assert_eq!(engine.accepted_batches, 1);
    assert_eq!(engine.accepted_samples, 2);
}

#[test]
fn native_session_rejects_missing_verified_artifacts_before_creation() {
    let temp = TempDir::new().unwrap();
    let error = NativeKwsSession::new(NativeKwsSessionPaths {
        model_dir: temp.path().join("model"),
        runtime_dir: temp.path().join("runtime"),
    })
    .unwrap_err();
    assert_eq!(error.kind, WakeWordErrorKind::MissingArtifact);
    assert!(!error
        .message
        .contains(temp.path().to_string_lossy().as_ref()));
}

#[test]
fn native_session_rejects_corrupt_verified_artifact_before_creation() {
    let temp = TempDir::new().unwrap();
    let model_dir = temp.path().join("model");
    fs::create_dir_all(&model_dir).unwrap();
    for file in V1_SHERPA_KWS_MODEL_FILES {
        fs::write(model_dir.join(file.name), b"corrupt").unwrap();
    }
    fs::write(
        model_dir.join(SHERPA_KWS_KEYWORD_FILE),
        SHERPA_KWS_KEYWORD_REPRESENTATION.as_bytes(),
    )
    .unwrap();

    let error = NativeKwsSession::new(NativeKwsSessionPaths {
        model_dir,
        runtime_dir: temp.path().join("runtime"),
    })
    .unwrap_err();
    assert_eq!(error.kind, WakeWordErrorKind::InvalidArtifact);
    assert_eq!(error.message, "wake artifact identity mismatch");
}

#[test]
fn native_runtime_architecture_check_is_sanitized() {
    let error = verify_native_architecture(&[0_u8; 64], NativeArchitecture::ElfX86_64).unwrap_err();
    assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
    assert_eq!(error.message, "native runtime architecture mismatch");
}

#[test]
fn native_session_exposes_frozen_policy_without_network() {
    let paths = NativeKwsSessionPaths {
        model_dir: PathBuf::from("model"),
        runtime_dir: PathBuf::from("runtime"),
    };
    let session = NativeKwsSession {
        config: SherpaKwsConfig::default(),
        paths: paths.clone(),
        native_runtime: None,
        shutdown: false,
    };
    assert_eq!(session.paths(), &paths);
    assert_eq!(session.config().threads, 1);
    assert_eq!(session.config().threshold, 0.25);
    assert_eq!(session.config().score, 1.0);
    assert_eq!(session.keyword_representation(), "▁HE Y ▁MO O SE");
    assert!(!session.runtime_platform().is_empty());
}

#[test]
fn native_c_api_contract_is_kws_only_and_privacy_bounded() {
    let paths = NativeKwsSessionPaths {
        model_dir: PathBuf::from("model"),
        runtime_dir: PathBuf::from("runtime"),
    };
    let session = NativeKwsSession {
        config: SherpaKwsConfig::default(),
        paths,
        native_runtime: None,
        shutdown: false,
    };
    let symbols = session.required_native_c_api_symbols();
    assert_eq!(symbols.len(), 10);
    assert!(symbols.contains(&"SherpaOnnxCreateKeywordSpotter"));
    assert!(symbols.contains(&"SherpaOnnxDecodeKeywordStream"));
    assert!(symbols.contains(&"SherpaOnnxResetKeywordStream"));
    assert!(!symbols.iter().any(|symbol| symbol.contains("Transducer")));
    assert!(!symbols
        .iter()
        .any(|symbol| symbol.contains("OfflineRecognizer")));
    assert!(!symbols.iter().any(|symbol| symbol.contains("Whisper")));
}

#[test]
fn runtime_contract_uses_shared_c_api_artifacts_not_jni() {
    let files = runtime_files().unwrap();
    assert!(files.iter().any(|f| f.relative_path.contains("c-api")));
    assert!(!files.iter().any(|f| f.relative_path.contains("jni")));
    let contract = native_capi_contract().unwrap();
    assert!(contract
        .library_relative_path
        .contains("shared/lib/libsherpa-onnx-c-api"));
}

#[test]
fn missing_native_c_api_library_is_sanitized_before_inference() {
    let temp = TempDir::new().unwrap();
    let error = verify_native_c_api_symbols(temp.path()).unwrap_err();
    assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
    assert_eq!(
        error.message,
        "missing required Wake Word native C API library"
    );
    assert!(error.retryable);
    assert!(!error
        .message
        .contains(temp.path().to_string_lossy().as_ref()));
}

#[test]
fn native_session_shutdown_is_idempotent_and_feed_errors_are_sanitized() {
    let mut session = NativeKwsSession {
        config: SherpaKwsConfig::default(),
        paths: NativeKwsSessionPaths {
            model_dir: PathBuf::from("model"),
            runtime_dir: PathBuf::from("runtime"),
        },
        native_runtime: None,
        shutdown: false,
    };
    let error = session.accept_pcm16_mono(16_000, &[1, 2]).unwrap_err();
    assert_eq!(error.kind, WakeWordErrorKind::RuntimeUnavailable);
    assert_eq!(
        error.message,
        "missing required Wake Word native C API library"
    );
    assert!(error.retryable);
    session.shutdown().unwrap();
    session.shutdown().unwrap();
    assert_eq!(
        session.accept_pcm16_mono(16_000, &[1]).unwrap_err().kind,
        WakeWordErrorKind::Cancelled
    );
}

#[test]
fn native_keyword_config_uses_frozen_policy_and_verified_paths() {
    let model_dir = PathBuf::from("model");
    let strings = NativeKwsCStringStore::new(&model_dir).unwrap();
    let config = build_keyword_spotter_config(&SherpaKwsConfig::default(), &strings);
    assert_eq!(config.feat_config.sample_rate, 16_000);
    assert_eq!(config.feat_config.feature_dim, 80);
    assert_eq!(config.model_config.num_threads, 1);
    assert_eq!(config.keywords_score, 1.0);
    assert_eq!(config.keywords_threshold, 0.25);
    unsafe {
        assert!(CStr::from_ptr(config.model_config.provider)
            .to_str()
            .unwrap()
            .eq("cpu"));
        assert!(CStr::from_ptr(config.model_config.transducer.encoder)
            .to_str()
            .unwrap()
            .ends_with("encoder-epoch-12-avg-2-chunk-16-left-64.onnx"));
        assert!(CStr::from_ptr(config.model_config.transducer.decoder)
            .to_str()
            .unwrap()
            .ends_with("decoder-epoch-12-avg-2-chunk-16-left-64.onnx"));
        assert!(CStr::from_ptr(config.model_config.transducer.joiner)
            .to_str()
            .unwrap()
            .ends_with("joiner-epoch-12-avg-2-chunk-16-left-64.onnx"));
        assert!(CStr::from_ptr(config.model_config.tokens)
            .to_str()
            .unwrap()
            .ends_with("tokens.txt"));
        assert!(CStr::from_ptr(config.model_config.bpe_vocab)
            .to_str()
            .unwrap()
            .ends_with("bpe.model"));
        assert!(CStr::from_ptr(config.keywords_file)
            .to_str()
            .unwrap()
            .ends_with(SHERPA_KWS_KEYWORD_FILE));
    }
}

#[test]
fn keyword_result_detection_is_bounded_to_keyword_presence() {
    let keyword = CString::new("HEY MOOSE").unwrap();
    let result = SherpaOnnxKeywordResult {
        keyword: keyword.as_ptr(),
        tokens: std::ptr::null(),
        tokens_arr: std::ptr::null(),
        count: 0,
        timestamps: std::ptr::null_mut(),
        start_time: 0.0,
        json: std::ptr::null(),
    };
    assert!(unsafe { keyword_result_has_detection(&result) });

    let empty = CString::new("").unwrap();
    let result = SherpaOnnxKeywordResult {
        keyword: empty.as_ptr(),
        tokens: std::ptr::null(),
        tokens_arr: std::ptr::null(),
        count: 0,
        timestamps: std::ptr::null_mut(),
        start_time: 0.0,
        json: std::ptr::null(),
    };
    assert!(!unsafe { keyword_result_has_detection(&result) });
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
