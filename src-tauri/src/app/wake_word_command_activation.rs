use super::wake_word_command_asr_ingress::{WakeCommandAsrHandoff, WakeCommandAsrIngress};
use super::wake_word_command_lifecycle::{
    resume_after_command_interaction, suspend_for_command_interaction,
};
use super::wake_word_composition::WakeWordApplicationRuntime;

/// Activate the normal command-ASR boundary for one accepted Wake trigger.
///
/// Wake is suspended before command ASR can receive audio. A successful delivery deliberately
/// leaves Wake suspended for the rest of ASR/Thinking/Talking. Startup failure consumes the stale
/// handoff and immediately restores the runtime according to the latest enabled setting.
pub(crate) fn activate_wake_command_once(
    runtime: &WakeWordApplicationRuntime,
    handoff: &mut WakeCommandAsrHandoff,
    ingress: &mut impl WakeCommandAsrIngress,
    wake_word_enabled: bool,
) -> Result<bool, String> {
    if !suspend_for_command_interaction(runtime)? {
        return Ok(false);
    }

    match handoff.deliver_once(ingress) {
        Ok(delivered) => Ok(delivered),
        Err(startup_error) => {
            let recovery = resume_after_command_interaction(runtime, wake_word_enabled);
            match recovery {
                Ok(()) => Err(startup_error),
                Err(recovery_error) => Err(format!(
                    "{startup_error}; Wake Word recovery also failed: {recovery_error}"
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;
    use crate::app::wake_word::runtime::WakeWordRuntimePhase;
    use crate::app::wake_word_command_handoff::WakeCommandHandoffAudio;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;

    #[derive(Default)]
    struct RecordingIngress {
        activations: usize,
        samples: Vec<i16>,
        fail: bool,
    }

    impl WakeCommandAsrIngress for RecordingIngress {
        fn accept_wake_handoff(&mut self, audio: WakeCommandHandoffAudio) -> Result<(), String> {
            self.activations += 1;
            if self.fail {
                return Err("command ASR unavailable".to_string());
            }
            self.samples = audio.into_samples_i16();
            Ok(())
        }
    }

    fn listening_runtime() -> WakeWordApplicationRuntime {
        let settings = AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        runtime.mark_loaded().unwrap();
        runtime
    }

    fn handoff() -> WakeCommandAsrHandoff {
        WakeCommandAsrHandoff::new(
            WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, vec![1, 2, 3, 4]).unwrap(),
        )
    }

    #[test]
    fn accepted_trigger_activates_command_asr_once_and_keeps_wake_suspended() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress::default();

        assert!(activate_wake_command_once(&runtime, &mut handoff, &mut ingress, true).unwrap());
        assert!(!activate_wake_command_once(&runtime, &mut handoff, &mut ingress, true).unwrap());

        assert_eq!(ingress.activations, 1);
        assert_eq!(ingress.samples, vec![1, 2, 3, 4]);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    }

    #[test]
    fn later_phrase_after_resume_activates_once_without_cooldown() {
        let runtime = listening_runtime();
        let mut ingress = RecordingIngress::default();

        let mut first_handoff = handoff();
        assert!(activate_wake_command_once(
            &runtime,
            &mut first_handoff,
            &mut ingress,
            true,
        )
        .unwrap());
        assert!(!activate_wake_command_once(
            &runtime,
            &mut first_handoff,
            &mut ingress,
            true,
        )
        .unwrap());
        assert_eq!(ingress.activations, 1);

        resume_after_command_interaction(&runtime, true).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);

        let mut second_handoff = handoff();
        assert!(activate_wake_command_once(
            &runtime,
            &mut second_handoff,
            &mut ingress,
            true,
        )
        .unwrap());
        assert!(!activate_wake_command_once(
            &runtime,
            &mut second_handoff,
            &mut ingress,
            true,
        )
        .unwrap());

        assert_eq!(ingress.activations, 2);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    }

    #[test]
    fn command_asr_startup_failure_returns_to_listening_without_stale_replay() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress {
            fail: true,
            ..Default::default()
        };

        assert_eq!(
            activate_wake_command_once(&runtime, &mut handoff, &mut ingress, true).unwrap_err(),
            "command ASR unavailable"
        );
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);
        assert!(!handoff.is_pending());

        ingress.fail = false;
        assert!(!activate_wake_command_once(&runtime, &mut handoff, &mut ingress, true).unwrap());
        assert_eq!(ingress.activations, 1);
    }

    #[test]
    fn startup_failure_honors_disable_instead_of_resuming_listening() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress {
            fail: true,
            ..Default::default()
        };

        activate_wake_command_once(&runtime, &mut handoff, &mut ingress, false).unwrap_err();

        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
    }
}
