use crate::app::wake_word_settings::{V1_WAKE_SCORE, V1_WAKE_THRESHOLD};
use crate::asr::wake_word_runtime::{
    WakeWordRuntimeManager, WakeWordRuntimePhase, WakeWordRuntimeSnapshot,
};
use crate::audio::pcm_ring_buffer::{WAKE_PCM_PRE_ROLL_SECONDS, WAKE_PCM_SAMPLE_RATE_HZ};
use serde::Serialize;
use std::time::Instant;

pub const WAKE_PCM_CHANNELS: u8 = 1;
pub const WAKE_KWS_INFERENCE_THREADS: u8 = 1;

/// Privacy-safe operational view of Wake Word V1.
///
/// This type intentionally contains no PCM, transcript, utterance, credential, or filesystem
/// path fields. It is safe to expose through application diagnostics without serializing the
/// in-memory pre-roll buffer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WakeWordDiagnostics {
    pub enabled: bool,
    pub runtime_phase: WakeWordRuntimePhase,
    pub talking_suspended: bool,
    pub platform: &'static str,
    pub architecture: &'static str,
    pub inference_threads: u8,
    pub sample_rate_hz: usize,
    pub channels: u8,
    pub ring_buffer_capacity_samples: usize,
    pub ring_buffer_duration_seconds: usize,
    pub ring_buffer_retained_samples: usize,
    pub handoff_pre_roll_samples: usize,
    pub threshold: f32,
    pub score: f32,
    pub trigger_count: u64,
    pub last_trigger_age_ms: Option<u64>,
    pub last_error: Option<&'static str>,
}

impl WakeWordDiagnostics {
    pub fn from_runtime(manager: &WakeWordRuntimeManager, now: Instant) -> Self {
        Self::from_snapshot(manager.snapshot(now))
    }

    fn from_snapshot(snapshot: WakeWordRuntimeSnapshot) -> Self {
        Self {
            enabled: !matches!(
                snapshot.phase,
                WakeWordRuntimePhase::Disabled | WakeWordRuntimePhase::ShuttingDown
            ),
            runtime_phase: snapshot.phase,
            talking_suspended: snapshot.phase == WakeWordRuntimePhase::SuspendedTalking,
            platform: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            inference_threads: WAKE_KWS_INFERENCE_THREADS,
            sample_rate_hz: WAKE_PCM_SAMPLE_RATE_HZ,
            channels: WAKE_PCM_CHANNELS,
            ring_buffer_capacity_samples: snapshot.ring_buffer_capacity_samples,
            ring_buffer_duration_seconds: WAKE_PCM_PRE_ROLL_SECONDS,
            ring_buffer_retained_samples: snapshot.ring_buffer_samples,
            handoff_pre_roll_samples: snapshot.handoff_pre_roll_samples,
            threshold: V1_WAKE_THRESHOLD,
            score: V1_WAKE_SCORE,
            trigger_count: snapshot.trigger_count,
            last_trigger_age_ms: snapshot
                .last_trigger_age
                .map(|age| u64::try_from(age.as_millis()).unwrap_or(u64::MAX)),
            last_error: snapshot.last_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn disabled_diagnostics_expose_fixed_v1_policy_without_audio() {
        let manager = WakeWordRuntimeManager::new();
        let diagnostics = WakeWordDiagnostics::from_runtime(&manager, Instant::now());

        assert!(!diagnostics.enabled);
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Disabled);
        assert!(!diagnostics.talking_suspended);
        assert_eq!(diagnostics.inference_threads, 1);
        assert_eq!(diagnostics.sample_rate_hz, 16_000);
        assert_eq!(diagnostics.channels, 1);
        assert_eq!(diagnostics.ring_buffer_duration_seconds, 2);
        assert_eq!(diagnostics.threshold, 0.25);
        assert_eq!(diagnostics.score, 1.0);
        assert_eq!(diagnostics.trigger_count, 0);
        assert_eq!(diagnostics.last_trigger_age_ms, None);
        assert_eq!(diagnostics.last_error, None);

        let serialized = serde_json::to_string(&diagnostics).unwrap();
        assert!(!serialized.contains("pcm"));
        assert!(!serialized.contains("utterance"));
        assert!(!serialized.contains("transcript"));
        assert!(!serialized.contains("path"));
    }

    #[test]
    fn trigger_and_talking_state_are_reported_without_sample_contents() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        assert!(manager.append_listening_pcm(&[11, 22, 33, 44]));
        let triggered_at = Instant::now();
        assert!(manager.accept_trigger(triggered_at).unwrap());

        let triggered = WakeWordDiagnostics::from_runtime(
            &manager,
            triggered_at + Duration::from_millis(125),
        );
        assert!(triggered.enabled);
        assert_eq!(triggered.trigger_count, 1);
        assert_eq!(triggered.last_trigger_age_ms, Some(125));
        assert_eq!(triggered.handoff_pre_roll_samples, 4);

        let serialized = serde_json::to_string(&triggered).unwrap();
        assert!(!serialized.contains("11"));
        assert!(!serialized.contains("22"));
        assert!(!serialized.contains("33"));
        assert!(!serialized.contains("44"));

        manager.suspend_for_talking().unwrap();
        let talking = WakeWordDiagnostics::from_runtime(&manager, Instant::now());
        assert!(talking.talking_suspended);
        assert_eq!(talking.ring_buffer_retained_samples, 0);
        assert_eq!(talking.handoff_pre_roll_samples, 0);
    }

    #[test]
    fn sanitized_runtime_error_is_the_only_error_detail_exposed() {
        let manager = WakeWordRuntimeManager::new();
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        manager.record_runtime_error();

        let diagnostics = WakeWordDiagnostics::from_runtime(&manager, Instant::now());
        assert_eq!(diagnostics.runtime_phase, WakeWordRuntimePhase::Error);
        assert_eq!(
            diagnostics.last_error,
            Some("The Wake Word runtime encountered an internal error.")
        );
    }
}
