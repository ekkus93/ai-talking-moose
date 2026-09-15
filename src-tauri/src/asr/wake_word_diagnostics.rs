use crate::app::wake_word_settings::{V1_WAKE_SCORE, V1_WAKE_THRESHOLD};
use crate::asr::wake_word_runtime::{WakeWordRuntimePhase, WakeWordRuntimeSnapshot};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const WAKE_WORD_CANONICAL_SAMPLE_RATE_HZ: u32 = 16_000;
pub const WAKE_WORD_CANONICAL_CHANNELS: u8 = 1;

/// Privacy-safe Wake Word V1 runtime diagnostics.
///
/// This intentionally exposes only bounded counters, fixed configuration, and
/// lifecycle state. Raw PCM, transcripts, credentials, and filesystem paths are
/// not representable in this type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WakeWordDiagnostics {
    pub enabled: bool,
    pub runtime_phase: WakeWordRuntimePhase,
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
            canonical_sample_rate_hz: WAKE_WORD_CANONICAL_SAMPLE_RATE_HZ,
            canonical_channels: WAKE_WORD_CANONICAL_CHANNELS,
            inference_threads: 1,
            threshold: V1_WAKE_THRESHOLD,
            score: V1_WAKE_SCORE,
            ring_buffer_capacity_samples: snapshot.ring_buffer_capacity_samples,
            ring_buffer_capacity_ms: samples_to_ms(snapshot.ring_buffer_capacity_samples),
            ring_buffer_samples: snapshot.ring_buffer_samples,
            handoff_pre_roll_samples: snapshot.handoff_pre_roll_samples,
            trigger_count: snapshot.trigger_count,
            last_trigger_age_ms: snapshot.last_trigger_age.map(duration_ms),
            talking_suspended: snapshot.phase == WakeWordRuntimePhase::SuspendedTalking,
            last_error: snapshot.last_error.map(str::to_string),
        }
    }
}

fn samples_to_ms(samples: usize) -> u64 {
    (samples as u64).saturating_mul(1_000) / u64::from(WAKE_WORD_CANONICAL_SAMPLE_RATE_HZ)
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
        assert_eq!(diagnostics.canonical_sample_rate_hz, 16_000);
        assert_eq!(diagnostics.canonical_channels, 1);
        assert_eq!(diagnostics.inference_threads, 1);
        assert_eq!(diagnostics.threshold, V1_WAKE_THRESHOLD);
        assert_eq!(diagnostics.score, V1_WAKE_SCORE);
        assert_eq!(diagnostics.ring_buffer_capacity_ms, 2_000);
        assert_eq!(diagnostics.ring_buffer_samples, 0);
        assert_eq!(diagnostics.handoff_pre_roll_samples, 0);
        assert_eq!(diagnostics.trigger_count, 0);
        assert!(!diagnostics.talking_suspended);
        assert!(diagnostics.last_error.is_none());

        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(!json.contains("pcm"));
        assert!(!json.contains("transcript"));
        assert!(!json.contains("credential"));
        assert!(!json.contains("path"));
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
