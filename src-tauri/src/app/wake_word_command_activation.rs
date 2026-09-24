use super::wake_word_command_asr_ingress::{WakeCommandAsrHandoff, WakeCommandAsrIngress};
use super::wake_word_command_lifecycle::{
    resume_after_command_interaction, suspend_for_command_interaction,
};
use super::wake_word_composition::WakeWordApplicationRuntime;
use async_trait::async_trait;
use std::time::Instant;

/// Production boundary that starts the existing normal command interaction after Wake audio is
/// accepted by the command-ASR ingress.
///
/// Implementations are expected to call the same command-start path used by manual listen. The
/// Wake side owns only single-use activation discipline: one accepted trigger may prime command
/// ASR and request one normal command start exactly once. Startup failure must be reported so the
/// Wake runtime can recover and stale handoff audio cannot replay.
#[async_trait]
pub(crate) trait WakeCommandStarter {
    async fn start_normal_command_interaction(&mut self) -> Result<(), String>;
}

/// Timing record for the Wake handoff and normal command-start boundary.
///
/// These values are deliberately bounded metadata. They contain no PCM, transcript, path, or
/// provider payload and are suitable for WWR-630 performance reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WakeCommandActivationTiming {
    pub wake_to_command_asr_ms: u64,
    pub command_start_ms: u64,
    pub total_activation_ms: u64,
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

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

/// Prime command ASR with the single-use Wake handoff, request exactly one normal command
/// interaction, and return privacy-safe timing metadata for WWR-630 reporting.
pub(crate) async fn activate_wake_command_and_measure_start_normal_asr_once(
    runtime: &WakeWordApplicationRuntime,
    handoff: &mut WakeCommandAsrHandoff,
    ingress: &mut impl WakeCommandAsrIngress,
    starter: &mut impl WakeCommandStarter,
    wake_word_enabled: bool,
) -> Result<(bool, WakeCommandActivationTiming), String> {
    let total_started = Instant::now();
    let handoff_started = Instant::now();
    let delivered = activate_wake_command_once(runtime, handoff, ingress, wake_word_enabled)?;
    let wake_to_command_asr_ms = elapsed_ms(handoff_started);

    if !delivered {
        return Ok((
            false,
            WakeCommandActivationTiming {
                wake_to_command_asr_ms,
                command_start_ms: 0,
                total_activation_ms: elapsed_ms(total_started),
            },
        ));
    }

    let command_start_started = Instant::now();
    match starter.start_normal_command_interaction().await {
        Ok(()) => Ok((
            true,
            WakeCommandActivationTiming {
                wake_to_command_asr_ms,
                command_start_ms: elapsed_ms(command_start_started),
                total_activation_ms: elapsed_ms(total_started),
            },
        )),
        Err(start_error) => {
            let recovery = resume_after_command_interaction(runtime, wake_word_enabled);
            match recovery {
                Ok(()) => Err(start_error),
                Err(recovery_error) => Err(format!(
                    "{start_error}; Wake Word recovery also failed: {recovery_error}"
                )),
            }
        }
    }
}

/// Prime command ASR with the single-use Wake handoff, then request exactly one normal command
/// interaction on the same successful activation.
///
/// This keeps WWR-310's ordering explicit: the wake phrase plus immediate command PCM is accepted
/// before command microphone capture begins, then the ordinary command-start path owns the rest of
/// ASR/Thinking/Talking. If the ordinary command start fails after priming, the runtime returns to
/// the latest enabled/disabled state and the consumed handoff is not replayable.
pub(crate) async fn activate_wake_command_and_start_normal_asr_once(
    runtime: &WakeWordApplicationRuntime,
    handoff: &mut WakeCommandAsrHandoff,
    ingress: &mut impl WakeCommandAsrIngress,
    starter: &mut impl WakeCommandStarter,
    wake_word_enabled: bool,
) -> Result<bool, String> {
    let (delivered, _) = activate_wake_command_and_measure_start_normal_asr_once(
        runtime,
        handoff,
        ingress,
        starter,
        wake_word_enabled,
    )
    .await?;
    Ok(delivered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppSettings;
    use crate::app::wake_word::runtime::WakeWordRuntimePhase;
    use crate::app::wake_word_command_handoff::WakeCommandHandoffAudio;
    use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

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

    #[derive(Default)]
    struct RecordingStarter {
        start_count: usize,
        fail: bool,
    }

    #[async_trait]
    impl WakeCommandStarter for RecordingStarter {
        async fn start_normal_command_interaction(&mut self) -> Result<(), String> {
            self.start_count += 1;
            if self.fail {
                Err("normal command start failed".to_string())
            } else {
                Ok(())
            }
        }
    }

    struct AwaitingStarter {
        entered: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    }

    #[async_trait]
    impl WakeCommandStarter for AwaitingStarter {
        async fn start_normal_command_interaction(&mut self) -> Result<(), String> {
            self.entered.store(true, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
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

    #[tokio::test]
    async fn successful_wake_activation_starts_the_normal_command_path_once() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress::default();
        let mut starter = RecordingStarter::default();

        assert!(activate_wake_command_and_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
        .unwrap());
        assert!(!activate_wake_command_and_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
        .unwrap());

        assert_eq!(ingress.activations, 1);
        assert_eq!(starter.start_count, 1);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    }

    #[tokio::test]
    async fn measured_wake_activation_records_privacy_safe_timing() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress::default();
        let mut starter = RecordingStarter::default();

        let (delivered, timing) = activate_wake_command_and_measure_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
        .unwrap();

        assert!(delivered);
        assert_eq!(ingress.activations, 1);
        assert_eq!(starter.start_count, 1);
        assert!(timing.total_activation_ms >= timing.wake_to_command_asr_ms);
        assert!(timing.total_activation_ms >= timing.command_start_ms);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    }

    #[tokio::test]
    async fn measured_consumed_handoff_records_no_command_start_timing() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress::default();
        let mut starter = RecordingStarter::default();

        assert!(activate_wake_command_and_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
        .unwrap());
        let (delivered, timing) = activate_wake_command_and_measure_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
        .unwrap();

        assert!(!delivered);
        assert_eq!(timing.command_start_ms, 0);
        assert!(timing.total_activation_ms >= timing.wake_to_command_asr_ms);
        assert_eq!(ingress.activations, 1);
        assert_eq!(starter.start_count, 1);
    }

    #[tokio::test]
    async fn activation_awaits_the_real_normal_command_start_boundary() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress::default();
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let mut starter = AwaitingStarter {
            entered: entered.clone(),
            release: release.clone(),
        };

        {
            let activation = activate_wake_command_and_start_normal_asr_once(
                &runtime,
                &mut handoff,
                &mut ingress,
                &mut starter,
                true,
            );
            tokio::pin!(activation);

            tokio::select! {
                result = &mut activation => {
                    panic!("activation completed before starter release: {result:?}");
                }
                _ = async {
                    while !entered.load(Ordering::SeqCst) {
                        tokio::task::yield_now().await;
                    }
                } => {}
            }

            release.store(true, Ordering::SeqCst);
            assert!(activation.await.unwrap());
        }

        assert_eq!(ingress.activations, 1);
        assert!(!handoff.is_pending());
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    }

    #[tokio::test]
    async fn normal_command_start_failure_recovers_without_replaying_handoff() {
        let runtime = listening_runtime();
        let mut handoff = handoff();
        let mut ingress = RecordingIngress::default();
        let mut starter = RecordingStarter {
            fail: true,
            ..Default::default()
        };

        assert_eq!(
            activate_wake_command_and_start_normal_asr_once(
                &runtime,
                &mut handoff,
                &mut ingress,
                &mut starter,
                true,
            )
            .await
            .unwrap_err(),
            "normal command start failed"
        );

        assert_eq!(ingress.activations, 1);
        assert_eq!(starter.start_count, 1);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);
        assert!(!handoff.is_pending());

        starter.fail = false;
        assert!(!activate_wake_command_and_start_normal_asr_once(
            &runtime,
            &mut handoff,
            &mut ingress,
            &mut starter,
            true,
        )
        .await
        .unwrap());
        assert_eq!(ingress.activations, 1);
        assert_eq!(starter.start_count, 1);
    }
}
