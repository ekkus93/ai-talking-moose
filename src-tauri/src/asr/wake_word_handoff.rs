use crate::app::wake_word_engine::{validate_pcm_frame, WakeWordError};
use std::collections::VecDeque;

/// Maximum post-trigger live PCM retained while command ASR starts.
///
/// Canonical wake/ASR PCM is 16 kHz mono i16. Two seconds bounds startup
/// buffering without clipping an immediate command under normal startup latency.
pub const WAKE_ASR_LIVE_HANDOFF_CAPACITY_SAMPLES: usize = 32_000;

/// Bounded, engine-independent wake-to-command-ASR audio handoff.
///
/// The pre-roll snapshot is emitted exactly once before any post-trigger live
/// samples. Live samples are retained chronologically in a fixed-capacity queue;
/// if startup exceeds the bound, the oldest live samples are dropped and the
/// overflow is observable without exposing PCM contents.
#[derive(Debug)]
pub struct WakeAsrHandoff {
    pre_roll: Option<Vec<i16>>,
    live: VecDeque<i16>,
    live_capacity_samples: usize,
    dropped_live_samples: u64,
}

impl WakeAsrHandoff {
    pub fn new(pre_roll: Vec<i16>) -> Self {
        Self::with_capacity(pre_roll, WAKE_ASR_LIVE_HANDOFF_CAPACITY_SAMPLES)
    }

    fn with_capacity(pre_roll: Vec<i16>, live_capacity_samples: usize) -> Self {
        Self {
            pre_roll: Some(pre_roll),
            live: VecDeque::with_capacity(live_capacity_samples),
            live_capacity_samples,
            dropped_live_samples: 0,
        }
    }

    /// Retain post-trigger live PCM while ASR initializes.
    ///
    /// Validation occurs before any queue or overflow-counter mutation so a
    /// non-canonical frame cannot contaminate the command handoff.
    pub fn append_live(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<(), WakeWordError> {
        validate_pcm_frame(sample_rate_hz, samples)?;

        if self.live_capacity_samples == 0 {
            self.dropped_live_samples = self
                .dropped_live_samples
                .saturating_add(samples.len() as u64);
            return Ok(());
        }

        if samples.len() >= self.live_capacity_samples {
            self.dropped_live_samples = self
                .dropped_live_samples
                .saturating_add(self.live.len() as u64)
                .saturating_add((samples.len() - self.live_capacity_samples) as u64);
            self.live.clear();
            self.live.extend(
                samples[samples.len() - self.live_capacity_samples..]
                    .iter()
                    .copied(),
            );
            return Ok(());
        }

        let overflow = self
            .live
            .len()
            .saturating_add(samples.len())
            .saturating_sub(self.live_capacity_samples);
        if overflow > 0 {
            self.live.drain(..overflow);
            self.dropped_live_samples = self.dropped_live_samples.saturating_add(overflow as u64);
        }
        self.live.extend(samples.iter().copied());
        Ok(())
    }

    /// Consume the handoff exactly once in command-ASR order: pre-roll, then live PCM.
    pub fn take_for_asr(&mut self) -> Option<Vec<i16>> {
        let pre_roll = self.pre_roll.take()?;
        let mut ordered = Vec::with_capacity(pre_roll.len().saturating_add(self.live.len()));
        ordered.extend(pre_roll);
        ordered.extend(self.live.drain(..));
        Some(ordered)
    }

    pub fn clear(&mut self) {
        self.pre_roll = None;
        self.live.clear();
    }

    pub fn live_samples(&self) -> usize {
        self.live.len()
    }

    pub fn live_capacity_samples(&self) -> usize {
        self.live_capacity_samples
    }

    pub fn dropped_live_samples(&self) -> u64 {
        self.dropped_live_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_roll_precedes_live_pcm_without_gap_duplicate_or_inversion() {
        let mut handoff = WakeAsrHandoff::with_capacity(vec![1, 2, 3, 4], 8);
        handoff.append_live(16_000, &[5, 6]).unwrap();
        handoff.append_live(16_000, &[7, 8]).unwrap();

        assert_eq!(
            handoff.take_for_asr().unwrap(),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(handoff.take_for_asr(), None);
    }

    #[test]
    fn immediate_command_samples_after_wake_are_retained() {
        let mut handoff = WakeAsrHandoff::with_capacity(vec![1, 2, 3, 4], 16);
        handoff.append_live(16_000, &[5, 6, 7]).unwrap();
        assert_eq!(handoff.take_for_asr().unwrap(), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn live_startup_buffer_is_bounded_and_reports_overflow_without_pcm() {
        let mut handoff = WakeAsrHandoff::with_capacity(vec![1, 2], 4);
        handoff.append_live(16_000, &[3, 4, 5]).unwrap();
        handoff.append_live(16_000, &[6, 7, 8]).unwrap();

        assert_eq!(handoff.live_samples(), 4);
        assert_eq!(handoff.live_capacity_samples(), 4);
        assert_eq!(handoff.dropped_live_samples(), 2);
        assert_eq!(handoff.take_for_asr().unwrap(), vec![1, 2, 5, 6, 7, 8]);
    }

    #[test]
    fn oversized_live_write_keeps_newest_bounded_samples() {
        let mut handoff = WakeAsrHandoff::with_capacity(vec![10], 3);
        handoff.append_live(16_000, &[11, 12, 13, 14, 15]).unwrap();

        assert_eq!(handoff.dropped_live_samples(), 2);
        assert_eq!(handoff.take_for_asr().unwrap(), vec![10, 13, 14, 15]);
    }

    #[test]
    fn invalid_live_frame_is_rejected_before_handoff_mutation() {
        let mut handoff = WakeAsrHandoff::with_capacity(vec![1, 2], 4);
        handoff.append_live(16_000, &[3, 4]).unwrap();
        let before_live = handoff.live_samples();
        let before_dropped = handoff.dropped_live_samples();

        assert!(handoff.append_live(48_000, &[5, 6]).is_err());
        assert!(handoff.append_live(16_000, &[]).is_err());
        assert_eq!(handoff.live_samples(), before_live);
        assert_eq!(handoff.dropped_live_samples(), before_dropped);
        assert_eq!(handoff.take_for_asr().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn failed_handoff_clear_discards_stale_pre_roll_and_live_audio() {
        let mut handoff = WakeAsrHandoff::with_capacity(vec![1, 2, 3], 8);
        handoff.append_live(16_000, &[4, 5]).unwrap();
        handoff.clear();

        assert_eq!(handoff.live_samples(), 0);
        assert_eq!(handoff.take_for_asr(), None);
    }
}
