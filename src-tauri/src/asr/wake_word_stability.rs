use super::wake_word_runtime::{WakeWordRuntimeManager, WakeWordRuntimePhase};
use crate::audio::pcm_ring_buffer::WAKE_PCM_PRE_ROLL_SAMPLES;
use std::time::{Duration, Instant};

const STABILITY_CYCLES: u64 = 256;

#[test]
fn repeated_wake_talking_resume_cycles_remain_bounded_and_reusable() {
    let manager = WakeWordRuntimeManager::new();
    manager.begin_enable().unwrap();
    manager.mark_loaded().unwrap();
    let started = Instant::now();

    for cycle in 0..STABILITY_CYCLES {
        let sample = i16::try_from(cycle).unwrap_or(i16::MAX);
        assert!(manager.append_listening_pcm(&vec![sample; 512]));
        assert!(manager
            .accept_trigger(started + Duration::from_millis(cycle))
            .unwrap());
        assert!(!manager
            .accept_trigger(started + Duration::from_millis(cycle) + Duration::from_micros(1))
            .unwrap());

        let triggered = manager.snapshot(started + Duration::from_millis(cycle));
        assert_eq!(triggered.phase, WakeWordRuntimePhase::Triggered);
        assert!(triggered.handoff_pre_roll_samples <= WAKE_PCM_PRE_ROLL_SAMPLES);
        assert!(triggered.ring_buffer_samples <= WAKE_PCM_PRE_ROLL_SAMPLES);

        let pre_roll = manager.take_triggered_pre_roll().unwrap().unwrap();
        assert!(pre_roll.len() <= WAKE_PCM_PRE_ROLL_SAMPLES);
        manager.suspend_for_talking().unwrap();
        let suspended = manager.snapshot(Instant::now());
        assert_eq!(suspended.phase, WakeWordRuntimePhase::SuspendedTalking);
        assert_eq!(suspended.ring_buffer_samples, 0);
        assert_eq!(suspended.handoff_pre_roll_samples, 0);
        assert!(!manager.append_listening_pcm(&[1, 2, 3]));

        manager.resume_after_interaction().unwrap();
        let resumed = manager.snapshot(Instant::now());
        assert_eq!(resumed.phase, WakeWordRuntimePhase::Listening);
        assert_eq!(resumed.ring_buffer_samples, 0);
        assert_eq!(resumed.handoff_pre_roll_samples, 0);
    }

    assert_eq!(manager.snapshot(Instant::now()).trigger_count, STABILITY_CYCLES);
}

#[test]
fn repeated_disable_enable_cycles_return_to_one_clean_listening_state() {
    let manager = WakeWordRuntimeManager::new();

    for _ in 0..128 {
        manager.begin_enable().unwrap();
        manager.mark_loaded().unwrap();
        assert!(manager.append_listening_pcm(&[10, 11, 12, 13]));
        manager.disable();
        let disabled = manager.snapshot(Instant::now());
        assert_eq!(disabled.phase, WakeWordRuntimePhase::Disabled);
        assert_eq!(disabled.ring_buffer_samples, 0);
        assert_eq!(disabled.handoff_pre_roll_samples, 0);
    }

    manager.begin_enable().unwrap();
    manager.mark_loaded().unwrap();
    assert_eq!(
        manager.snapshot(Instant::now()).phase,
        WakeWordRuntimePhase::Listening
    );
}

#[test]
fn shutdown_is_clean_from_listening_and_triggered_handoff() {
    let listening = WakeWordRuntimeManager::new();
    listening.begin_enable().unwrap();
    listening.mark_loaded().unwrap();
    assert!(listening.append_listening_pcm(&[1, 2, 3]));
    listening.begin_shutdown();
    let snapshot = listening.snapshot(Instant::now());
    assert_eq!(snapshot.phase, WakeWordRuntimePhase::ShuttingDown);
    assert_eq!(snapshot.ring_buffer_samples, 0);
    assert_eq!(snapshot.handoff_pre_roll_samples, 0);

    let handoff = WakeWordRuntimeManager::new();
    handoff.begin_enable().unwrap();
    handoff.mark_loaded().unwrap();
    assert!(handoff.append_listening_pcm(&[4, 5, 6]));
    assert!(handoff.accept_trigger(Instant::now()).unwrap());
    handoff.begin_shutdown();
    let snapshot = handoff.snapshot(Instant::now());
    assert_eq!(snapshot.phase, WakeWordRuntimePhase::ShuttingDown);
    assert_eq!(snapshot.ring_buffer_samples, 0);
    assert_eq!(snapshot.handoff_pre_roll_samples, 0);
}
