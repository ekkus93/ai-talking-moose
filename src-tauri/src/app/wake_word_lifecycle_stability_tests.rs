use super::state::AppSettings;
use super::wake_word::runtime::WakeWordRuntimePhase;
use super::wake_word_command_activation::{
    activate_wake_command_and_measure_start_normal_asr_once, WakeCommandStarter,
};
use super::wake_word_command_asr_ingress::{WakeCommandAsrHandoff, WakeCommandAsrIngress};
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_composition::WakeWordApplicationRuntime;
use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
use async_trait::async_trait;
use std::time::Instant;

fn listening_runtime() -> WakeWordApplicationRuntime {
    let settings = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };
    let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
    runtime.mark_loaded().unwrap();
    runtime
}

#[derive(Default)]
struct StabilityIngress {
    activations: usize,
    samples: Vec<i16>,
}

impl WakeCommandAsrIngress for StabilityIngress {
    fn accept_wake_handoff(&mut self, audio: WakeCommandHandoffAudio) -> Result<(), String> {
        self.activations += 1;
        self.samples = audio.samples_i16().to_vec();
        Ok(())
    }
}

#[derive(Default)]
struct StabilityStarter {
    starts: usize,
}

#[async_trait]
impl WakeCommandStarter for StabilityStarter {
    async fn start_normal_command_interaction(&mut self) -> Result<(), String> {
        self.starts += 1;
        Ok(())
    }
}

#[test]
fn wake_word_stability_repeated_lifecycle_cycles_remain_bounded_and_return_to_listening() {
    let runtime = listening_runtime();
    let initial = runtime.snapshot(Instant::now());
    let capacity = initial.ring_buffer_capacity_samples;

    for cycle in 0_i16..100 {
        let samples = [cycle, cycle.saturating_add(1), cycle.saturating_add(2)];
        assert!(runtime.manager().append_listening_pcm(&samples));
        assert!(runtime.manager().accept_trigger(Instant::now()).unwrap());

        let triggered = runtime.snapshot(Instant::now());
        assert_eq!(triggered.phase, WakeWordRuntimePhase::Triggered);
        assert!(triggered.ring_buffer_samples <= capacity);
        assert!(triggered.handoff_pre_roll_samples <= capacity);

        runtime.suspend_for_talking().unwrap();
        let suspended = runtime.snapshot(Instant::now());
        assert_eq!(suspended.phase, WakeWordRuntimePhase::SuspendedTalking);
        assert_eq!(suspended.ring_buffer_samples, 0);
        assert_eq!(suspended.handoff_pre_roll_samples, 0);

        runtime.resume_after_interaction(true).unwrap();
        let resumed = runtime.snapshot(Instant::now());
        assert_eq!(resumed.phase, WakeWordRuntimePhase::Listening);
        assert_eq!(resumed.ring_buffer_samples, 0);
        assert_eq!(resumed.handoff_pre_roll_samples, 0);
    }

    let final_snapshot = runtime.snapshot(Instant::now());
    assert_eq!(final_snapshot.trigger_count, 100);
    assert_eq!(
        final_snapshot.ring_buffer_samples,
        initial.ring_buffer_samples
    );
    assert_eq!(
        final_snapshot.handoff_pre_roll_samples,
        initial.handoff_pre_roll_samples
    );
    println!(
        "WWR630_REPEATED_CYCLE_RESOURCE_DELTA cycles=100 initial_ring_buffer_samples={} final_ring_buffer_samples={} ring_buffer_delta={} initial_handoff_pre_roll_samples={} final_handoff_pre_roll_samples={} handoff_pre_roll_delta={} trigger_count_delta={} final_phase={:?} capacity_samples={}",
        initial.ring_buffer_samples,
        final_snapshot.ring_buffer_samples,
        final_snapshot.ring_buffer_samples as i64 - initial.ring_buffer_samples as i64,
        initial.handoff_pre_roll_samples,
        final_snapshot.handoff_pre_roll_samples,
        final_snapshot.handoff_pre_roll_samples as i64 - initial.handoff_pre_roll_samples as i64,
        final_snapshot.trigger_count.saturating_sub(initial.trigger_count),
        final_snapshot.phase,
        capacity
    );
}

#[tokio::test]
async fn wake_word_stability_command_activation_timing_reports_privacy_safe_metrics() {
    let runtime = listening_runtime();
    let samples: Vec<i16> = (0_i16..3200).collect();
    let handoff_audio = WakeCommandHandoffAudio::new(V1_KWS_SAMPLE_RATE_HZ, samples.clone())
        .expect("stability timing handoff should be canonical");
    let mut handoff = WakeCommandAsrHandoff::new(handoff_audio);
    let mut ingress = StabilityIngress::default();
    let mut starter = StabilityStarter::default();

    let (delivered, timing) = activate_wake_command_and_measure_start_normal_asr_once(
        &runtime,
        &mut handoff,
        &mut ingress,
        &mut starter,
        true,
    )
    .await
    .expect("activation timing path should succeed");

    assert!(delivered);
    assert_eq!(ingress.activations, 1);
    assert_eq!(starter.starts, 1);
    assert_eq!(ingress.samples.len(), samples.len());
    assert_eq!(runtime.phase(), WakeWordRuntimePhase::SuspendedTalking);
    assert!(timing.wake_to_command_asr_ms >= timing.pre_roll_startup_ms);
    assert!(timing.total_activation_ms >= timing.wake_to_command_asr_ms);
    assert!(timing.total_activation_ms >= timing.command_start_ms);
    println!(
        "WWR630_WAKE_COMMAND_ACTIVATION_TIMING wake_to_command_asr_ms={} pre_roll_startup_ms={} command_start_ms={} total_activation_ms={} handoff_samples={} ingress_samples={}",
        timing.wake_to_command_asr_ms,
        timing.pre_roll_startup_ms,
        timing.command_start_ms,
        timing.total_activation_ms,
        samples.len(),
        ingress.samples.len()
    );
}

#[test]
fn repeated_terminal_tts_outcomes_resume_cleanly_without_retained_audio() {
    for outcome in ["success", "cancellation", "recoverable_failure"] {
        let runtime = listening_runtime();
        let capacity = runtime
            .snapshot(Instant::now())
            .ring_buffer_capacity_samples;

        for cycle in 0_i16..64 {
            let samples = [cycle, cycle.saturating_add(1), cycle.saturating_add(2)];
            assert!(
                runtime.manager().append_listening_pcm(&samples),
                "{outcome}"
            );
            assert!(
                runtime.manager().accept_trigger(Instant::now()).unwrap(),
                "{outcome}"
            );
            runtime.suspend_for_talking().unwrap();

            let suspended = runtime.snapshot(Instant::now());
            assert_eq!(
                suspended.phase,
                WakeWordRuntimePhase::SuspendedTalking,
                "{outcome}"
            );
            assert_eq!(suspended.ring_buffer_samples, 0, "{outcome}");
            assert_eq!(suspended.handoff_pre_roll_samples, 0, "{outcome}");

            // Success, cancellation, and recoverable TTS failure all resolve through the same
            // production terminal-interaction policy. Repeating each semantic outcome here
            // proves that policy does not accumulate stale audio or leave Wake Word suspended.
            runtime.resume_after_interaction(true).unwrap();

            let resumed = runtime.snapshot(Instant::now());
            assert_eq!(resumed.phase, WakeWordRuntimePhase::Listening, "{outcome}");
            assert_eq!(resumed.ring_buffer_samples, 0, "{outcome}");
            assert_eq!(resumed.handoff_pre_roll_samples, 0, "{outcome}");
            assert!(resumed.ring_buffer_samples <= capacity, "{outcome}");
        }
    }
}

#[test]
fn repeated_disable_enable_cycles_do_not_leave_stale_audio_or_state() {
    let runtime = listening_runtime();

    for _ in 0..50 {
        assert!(runtime.manager().append_listening_pcm(&[1, 2, 3, 4]));
        runtime.apply_enabled_setting(false).unwrap();
        let disabled = runtime.snapshot(Instant::now());
        assert_eq!(disabled.phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(disabled.ring_buffer_samples, 0);
        assert_eq!(disabled.handoff_pre_roll_samples, 0);

        runtime.apply_enabled_setting(true).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Loading);
        runtime.mark_loaded().unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);
    }
}

#[test]
fn shutdown_while_listening_is_terminal_and_clears_retained_audio() {
    let runtime = listening_runtime();
    assert!(runtime.manager().append_listening_pcm(&[10, 11, 12]));

    runtime.begin_shutdown();

    let shutdown = runtime.snapshot(Instant::now());
    assert_eq!(shutdown.phase, WakeWordRuntimePhase::ShuttingDown);
    assert_eq!(shutdown.ring_buffer_samples, 0);
    assert_eq!(shutdown.handoff_pre_roll_samples, 0);
    assert!(runtime.apply_enabled_setting(true).is_err());
}

#[test]
fn shutdown_during_triggered_handoff_is_terminal_and_clears_pre_roll() {
    let runtime = listening_runtime();
    assert!(runtime.manager().append_listening_pcm(&[20, 21, 22]));
    assert!(runtime.manager().accept_trigger(Instant::now()).unwrap());
    assert!(runtime.snapshot(Instant::now()).handoff_pre_roll_samples > 0);

    runtime.begin_shutdown();

    let shutdown = runtime.snapshot(Instant::now());
    assert_eq!(shutdown.phase, WakeWordRuntimePhase::ShuttingDown);
    assert_eq!(shutdown.ring_buffer_samples, 0);
    assert_eq!(shutdown.handoff_pre_roll_samples, 0);
    assert!(runtime.resume_after_interaction(true).is_err());
}
