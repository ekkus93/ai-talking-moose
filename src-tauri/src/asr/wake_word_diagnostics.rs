use crate::asr::wake_word_runtime::{WakeWordRuntimePhase, WakeWordRuntimeSnapshot};
use crate::asr::wake_word_sherpa_manifest::{
    SHERPA_KWS_KEYWORD_SHA256, V1_SHERPA_KWS_MODEL_MANIFEST,
};
use crate::wake_word_policy::{
    V1_KWS_CHANNELS, V1_KWS_SAMPLE_RATE_HZ, V1_KWS_THREADS, V1_WAKE_SCORE, V1_WAKE_THRESHOLD,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const WAKE_WORD_CANONICAL_SAMPLE_RATE_HZ: u32 = V1_KWS_SAMPLE_RATE_HZ;
pub const WAKE_WORD_CANONICAL_CHANNELS: u8 = V1_KWS_CHANNELS as u8;
pub const WAKE_WORD_ENGINE_ID: &str = "sherpa-onnx-kws";
pub const WAKE_WORD_RUNTIME_ID: &str = "sherpa-onnx-v1.13.8";
pub const WAKE_WORD_RUNTIME_LICENSE: &str = "Apache-2.0";
pub const WAKE_WORD_RUNTIME_LINUX_X86_64_C_API_SHA256: &str =
    "b8351ca1632571ac108adbb317bcc4bf7cfe84b72690e3017316b0da3e1e344f";
pub const WAKE_WORD_RUNTIME_MACOS_ARM64_C_API_SHA256: &str =
    "ee098d8b419d49b92101cde3c970a333b361066eb2d79a11ab480a116552b908";

/// Privacy-safe Wake Word V1 runtime diagnostics.
///
/// This intentionally exposes only bounded counters, fixed configuration,
/// immutable artifact identities, and lifecycle state. Raw PCM, transcripts,
/// credentials, and filesystem paths are not representable in this type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WakeWordDiagnostics {
    pub enabled: bool,
    pub runtime_phase: WakeWordRuntimePhase,
    pub engine_id: &'static str,
    pub model_id: &'static str,
    pub model_archive_sha256: &'static str,
    pub model_license: &'static str,
    pub keyword_sha256: &'static str,
    pub runtime_id: &'static str,
    pub runtime_license: &'static str,
    pub runtime_c_api_sha256: Option<&'static str>,
    pub platform: &'static str,
    pub architecture: &'static str,
    pub canonical_sample_rate_hz: u32,
    pub canonical_channels: u8,
    pub inference_threads: u8,
    pub threshold: f32,
    pub score: f32,
    pub ring_buffer_capacity_samples: usize,
    pub ring_buffer_capacity_ms: u64,
    pub ring_buffer_samples: usize,
    pub handoff_pre_roll_samples: usize,
    pub trigger_count: u64,
    pub last_trigger_age_ms: Option<u64>,
    pub runtime_initialization_ms: Option<u64>,
    pub talking_suspended: bool,
    pub last_error: Option<String>,
}

impl WakeWordDiagnostics {
    pub fn from_runtime(snapshot: &WakeWordRuntimeSnapshot) -> Self {
        Self {
            enabled: !matches!(
                snapshot.phase,
                WakeWordRuntimePhase::Disabled | WakeWordRuntimePhase::ShuttingDown
            ),
            runtime_phase: snapshot.phase,
            engine_id: WAKE_WORD_ENGINE_ID,
            model_id: V1_SHERPA_KWS_MODEL_MANIFEST.id,
            model_archive_sha256: V1_SHERPA_KWS_MODEL_MANIFEST.archive_sha256,
            model_license: V1_SHERPA_KWS_MODEL_MANIFEST.license,
            keyword_sha256: SHERPA_KWS_KEYWORD_SHA256,
            runtime_id: WAKE_WORD_RUNTIME_ID,
            runtime_license: WAKE_WORD_RUNTIME_LICENSE,
            runtime_c_api_sha256: platform_runtime_c_api_sha256(),
            platform: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            canonical_sample_rate_hz: V1_KWS_SAMPLE_RATE_HZ,
            canonical_channels: V1_KWS_CHANNELS as u8,
            inference_threads: V1_KWS_THREADS as u8,
            threshold: V1_WAKE_THRESHOLD,
            score: V1_WAKE_SCORE,
            ring_buffer_capacity_samples: snapshot.ring_buffer_capacity_samples,
            ring_buffer_capacity_ms: samples_to_ms(snapshot.ring_buffer_capacity_samples),
            ring_buffer_samples: snapshot.ring_buffer_samples,
            handoff_pre_roll_samples: snapshot.handoff_pre_roll_samples,
            trigger_count: snapshot.trigger_count,
            last_trigger_age_ms: snapshot.last_trigger_age.map(duration_ms),
            runtime_initialization_ms: snapshot.initialization_duration.map(duration_ms),
            talking_suspended: snapshot.phase == WakeWordRuntimePhase::SuspendedTalking,
            last_error: snapshot.last_error.map(str::to_string),
        }
    }
}

fn platform_runtime_c_api_sha256() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Some(WAKE_WORD_RUNTIME_LINUX_X86_64_C_API_SHA256)
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Some(WAKE_WORD_RUNTIME_MACOS_ARM64_C_API_SHA256)
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        None
    }
}

fn samples_to_ms(samples: usize) -> u64 {
    (samples as u64).saturating_mul(1_000) / u64::from(V1_KWS_SAMPLE_RATE_HZ)
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::wake_word_runtime::WakeWordRuntimeManager;
    use std::time::Instant;

    #[test]
    fn disabled_diagnostics_are_fail_closed_and_audio_free() {
        let manager = WakeWordRuntimeManager::new();
        let diagnostics = WakeWordDiagnostics::from_runtime(&manager.snapshot(Instant::now()));

        assert!(!diagnostics.enabled);
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(diagnostics.engine_id, "sherpa-onnx-kws");
        assert_eq!(diagnostics.model_id, V1_SHERPA_KWS_MODEL_MANIFEST.id);
        assert_eq!(
            diagnostics.model_archive_sha256,
            V1_SHERPA_KWS_MODEL_MANIFEST.archive_sha256
        );
        assert_eq!(diagnostics.model_license, "Apache-2.0");
        assert_eq!(diagnostics.keyword_sha256, SHERPA_KWS_KEYWORD_SHA256);
        assert_eq!(diagnostics.runtime_id, WAKE_WORD_RUNTIME_ID);
        assert_eq!(diagnostics.runtime_license, "Apache-2.0");
        assert_eq!(diagnostics.platform, std::env::consts::OS);
        assert_eq!(diagnostics.architecture, std::env::consts::ARCH);
        assert_eq!(diagnostics.canonical_sample_rate_hz, V1_KWS_SAMPLE_RATE_HZ);
        assert_eq!(diagnostics.canonical_channels, V1_KWS_CHANNELS as u8);
        assert_eq!(diagnostics.inference_threads, V1_KWS_THREADS as u8);
        assert_eq!(diagnostics.threshold, V1_WAKE_THRESHOLD);
        assert_eq!(diagnostics.score, V1_WAKE_SCORE);
        assert_eq!(diagnostics.ring_buffer_capacity_ms, 2_000);
        assert_eq!(diagnostics.ring_buffer_samples, 0);
        assert_eq!(diagnostics.handoff_pre_roll_samples, 0);
        assert_eq!(diagnostics.trigger_count, 0);
        assert_eq!(diagnostics.runtime_initialization_ms, None);
        assert!(!diagnostics.talking_suspended);
        assert!(diagnostics.last_error.is_none());

        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(!json.contains("pcm"));
        assert!(!json.contains("transcript"));
        assert!(!json.contains("credential"));
        assert!(!json.contains("path"));
    }

    #[test]
    fn diagnostics_runtime_identity_matches_platform_or_is_explicitly_unsupported() {
        let sha = platform_runtime_c_api_sha256();
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(sha, Some(WAKE_WORD_RUNTIME_LINUX_X86_64_C_API_SHA256));
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(sha, Some(WAKE_WORD_RUNTIME_MACOS_ARM64_C_API_SHA256));
        } else {
            assert_eq!(sha, None);
        }
    }

    #[test]
    fn diagnostic_identity_constants_track_manifest_json() {
        let manifest = include_str!("../../../wake-word-artifacts.json");
        assert!(manifest.contains(V1_SHERPA_KWS_MODEL_MANIFEST.id));
        assert!(manifest.contains(V1_SHERPA_KWS_MODEL_MANIFEST.archive_sha256));
        assert!(manifest.contains(SHERPA_KWS_KEYWORD_SHA256));
        assert!(manifest.contains(WAKE_WORD_RUNTIME_LINUX_X86_64_C_API_SHA256));
        assert!(manifest.contains(WAKE_WORD_RUNTIME_MACOS_ARM64_C_API_SHA256));
    }

    #[test]
    fn initialization_duration_is_observable_without_unbounded_details() {
        let manager = WakeWordRuntimeManager::new();
        let started = Instant::now();
        manager.begin_enable_at(started).unwrap();
        manager
            .mark_loaded_at(started + Duration::from_millis(42))
            .unwrap();

        let diagnostics = WakeWordDiagnostics::from_runtime(
            &manager.snapshot(started + Duration::from_millis(42)),
        );
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Listening);
        assert_eq!(diagnostics.runtime_initialization_ms, Some(42));
        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(!json.contains("model_path"));
        assert!(!json.contains("runtime_path"));
    }

    #[test]
    fn trigger_and_talking_state_are_observable_without_audio_content() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        assert!(manager.append_listening_pcm(&[101, 202, 303]));
        let triggered_at = Instant::now();
        assert!(manager.accept_trigger(triggered_at).unwrap());

        let triggered = WakeWordDiagnostics::from_runtime(
            &manager.snapshot(triggered_at + Duration::from_millis(25)),
        );
        assert!(triggered.enabled);
        assert_eq!(triggered.trigger_count, 1);
        assert_eq!(triggered.last_trigger_age_ms, Some(25));
        assert_eq!(triggered.handoff_pre_roll_samples, 3);
        assert!(!triggered.talking_suspended);
        let json = serde_json::to_string(&triggered).unwrap();
        assert!(!json.contains("101"));
        assert!(!json.contains("202"));
        assert!(!json.contains("303"));

        manager.suspend_for_talking().unwrap();
        let suspended = WakeWordDiagnostics::from_runtime(&manager.snapshot(Instant::now()));
        assert!(suspended.talking_suspended);
        assert_eq!(suspended.ring_buffer_samples, 0);
        assert_eq!(suspended.handoff_pre_roll_samples, 0);
    }

    #[test]
    fn sanitized_runtime_error_is_the_only_error_detail_exposed() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        manager.record_runtime_error();

        let diagnostics = WakeWordDiagnostics::from_runtime(&manager.snapshot(Instant::now()));
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Error);
        assert_eq!(
            diagnostics.last_error.as_deref(),
            Some("The Wake Word runtime encountered an internal error.")
        );
    }
}
