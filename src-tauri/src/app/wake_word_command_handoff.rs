use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

/// Provider-neutral audio payload produced by the Wake Word router for normal command ASR.
///
/// The payload keeps the exact chronological sample vector returned by
/// `CanonicalWakePcmRouter::transfer_handoff_to_asr`. It deliberately performs no acoustic
/// trimming in V1: the wake phrase and immediate command audio remain present for the downstream
/// command-ASR path. Provider-specific callers can request little-endian PCM bytes without
/// reordering, dropping, or duplicating samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeCommandHandoffAudio {
    sample_rate_hz: u32,
    samples_i16: Vec<i16>,
}

impl WakeCommandHandoffAudio {
    pub(crate) fn new(sample_rate_hz: u32, samples_i16: Vec<i16>) -> Result<Self, String> {
        if sample_rate_hz != V1_KWS_SAMPLE_RATE_HZ {
            return Err("Wake handoff audio must be canonical 16 kHz PCM".to_string());
        }
        if samples_i16.is_empty() {
            return Err("Wake handoff audio cannot be empty".to_string());
        }
        Ok(Self {
            sample_rate_hz,
            samples_i16,
        })
    }

    pub(crate) fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub(crate) fn samples_i16(&self) -> &[i16] {
        &self.samples_i16
    }

    pub(crate) fn into_samples_i16(self) -> Vec<i16> {
        self.samples_i16
    }

    pub(crate) fn to_pcm16_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.samples_i16.len() * 2);
        for sample in &self.samples_i16 {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_canonical_non_empty_audio() {
        assert!(WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, vec![1]).is_ok());
        assert_eq!(
            WakeCommandHandoffAudio::new(48_000, vec![1]).unwrap_err(),
            "Wake handoff audio must be canonical 16 kHz PCM"
        );
        assert_eq!(
            WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, Vec::new()).unwrap_err(),
            "Wake handoff audio cannot be empty"
        );
    }

    #[test]
    fn preserves_exact_sample_order_without_v1_trimming() {
        let audio =
            WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, vec![100, -200, 300, -400, 500])
                .unwrap();

        assert_eq!(audio.sample_rate_hz(), V1_KWS_SAMPLE_RATE_HZ);
        assert_eq!(audio.samples_i16(), &[100, -200, 300, -400, 500]);
        assert_eq!(
            audio.to_pcm16_le_bytes(),
            vec![100, 0, 56, 255, 44, 1, 112, 254, 244, 1]
        );
    }

    #[test]
    fn ownership_transfer_returns_the_complete_chronological_vector() {
        let source = vec![1, 2, 3, 4, 5, 6];
        let audio = WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, source.clone()).unwrap();

        assert_eq!(audio.into_samples_i16(), source);
    }
}
