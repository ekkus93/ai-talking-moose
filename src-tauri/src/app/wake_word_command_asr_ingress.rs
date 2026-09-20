use super::wake_word_command_handoff::WakeCommandHandoffAudio;

/// Provider-neutral acceptance boundary between Wake Word and the already-selected command ASR.
///
/// Implementations must enqueue the supplied canonical PCM before attaching subsequent live
/// microphone PCM to the same command-ASR interaction. The payload is owned by the call so a
/// successful handoff cannot be replayed accidentally by the Wake Word side.
pub(crate) trait WakeCommandAsrIngress {
    fn accept_wake_handoff(&mut self, audio: WakeCommandHandoffAudio) -> Result<(), String>;
}

/// Single-use command-ASR handoff coordinator.
///
/// The router already makes its audio transfer single-use. This additional boundary protects the
/// command activation side as well: one accepted Wake trigger may activate/prime command ASR at
/// most once, even if orchestration observes repeated positive KWS frames while transitioning.
pub(crate) struct WakeCommandAsrHandoff {
    audio: Option<WakeCommandHandoffAudio>,
}

impl WakeCommandAsrHandoff {
    pub(crate) fn new(audio: WakeCommandHandoffAudio) -> Self {
        Self { audio: Some(audio) }
    }

    pub(crate) fn deliver_once(
        &mut self,
        ingress: &mut impl WakeCommandAsrIngress,
    ) -> Result<bool, String> {
        let Some(audio) = self.audio.take() else {
            return Ok(false);
        };
        ingress.accept_wake_handoff(audio)?;
        Ok(true)
    }

    /// Discard any not-yet-delivered Wake audio at a cancellation boundary.
    pub(crate) fn cancel(&mut self) {
        self.audio.take();
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.audio.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

    #[derive(Default)]
    struct RecordingIngress {
        accepted: Vec<Vec<i16>>,
        fail: bool,
    }

    impl WakeCommandAsrIngress for RecordingIngress {
        fn accept_wake_handoff(&mut self, audio: WakeCommandHandoffAudio) -> Result<(), String> {
            if self.fail {
                return Err("command ASR startup failed".to_string());
            }
            self.accepted.push(audio.into_samples_i16());
            Ok(())
        }
    }

    fn handoff(samples: &[i16]) -> WakeCommandHandoffAudio {
        WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, samples.to_vec()).unwrap()
    }

    #[test]
    fn accepted_trigger_payload_is_delivered_exactly_once() {
        let mut transfer = WakeCommandAsrHandoff::new(handoff(&[1, 2, 3, 4]));
        let mut ingress = RecordingIngress::default();

        assert!(transfer.deliver_once(&mut ingress).unwrap());
        assert!(!transfer.deliver_once(&mut ingress).unwrap());
        assert_eq!(ingress.accepted, vec![vec![1, 2, 3, 4]]);
        assert!(!transfer.is_pending());
    }

    #[test]
    fn payload_preserves_wake_phrase_and_immediate_command_order() {
        let expected = vec![10, 11, 12, 20, 21, 22];
        let mut transfer = WakeCommandAsrHandoff::new(handoff(&expected));
        let mut ingress = RecordingIngress::default();

        transfer.deliver_once(&mut ingress).unwrap();

        assert_eq!(ingress.accepted[0], expected);
    }

    #[test]
    fn failed_startup_consumes_payload_instead_of_replaying_stale_audio() {
        let mut transfer = WakeCommandAsrHandoff::new(handoff(&[7, 8, 9]));
        let mut ingress = RecordingIngress {
            fail: true,
            ..Default::default()
        };

        assert_eq!(
            transfer.deliver_once(&mut ingress).unwrap_err(),
            "command ASR startup failed"
        );
        assert!(!transfer.is_pending());

        ingress.fail = false;
        assert!(!transfer.deliver_once(&mut ingress).unwrap());
        assert!(ingress.accepted.is_empty());
    }

    #[test]
    fn cancellation_discards_pending_audio_without_delivery() {
        let mut transfer = WakeCommandAsrHandoff::new(handoff(&[30, 31, 32]));
        let mut ingress = RecordingIngress::default();

        transfer.cancel();

        assert!(!transfer.is_pending());
        assert!(!transfer.deliver_once(&mut ingress).unwrap());
        assert!(ingress.accepted.is_empty());
    }
}
