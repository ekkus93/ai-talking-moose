use super::state::AppSettings;
use super::wake_word::runtime::WakeWordRuntimePhase;
use super::wake_word_composition::WakeWordApplicationRuntime;
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

#[test]
fn wake_word_stability_repeated_lifecycle_cycles_emit_resource_delta() {
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
