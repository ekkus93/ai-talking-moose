use super::state::AppSettings;
use super::wake_word::engine::{
    validate_pcm_frame, SherpaKwsConfig, SherpaKwsEngine, WakeWordDetection, WakeWordError,
};
use super::wake_word::runtime::{WakeWordRuntimeManager, WakeWordRuntimePhase};
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_capture_consumer::WakeCapturePcmConsumer;
use super::wake_word_composition::WakeWordApplicationRuntime;
use super::wake_word_pcm_router::CanonicalWakePcmRouter;
use crate::audio::capture::AudioCapture;
use parking_lot::Mutex as CaptureMutex;
use std::sync::Arc;
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
struct CaptureCycleEngine {
    config: SherpaKwsConfig,
}

impl SherpaKwsEngine for CaptureCycleEngine {
    fn config(&self) -> &SherpaKwsConfig {
        &self.config
    }

    fn accept_pcm16_mono(
        &mut self,
        sample_rate_hz: u32,
        samples: &[i16],
    ) -> Result<Option<WakeWordDetection>, WakeWordError> {
        validate_pcm_frame(sample_rate_hz, samples)?;
        Ok(None)
    }

    fn reset_stream(&mut self) -> Result<(), WakeWordError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), WakeWordError> {
        Ok(())
    }
}

fn capture_cycle_consumer() -> WakeCapturePcmConsumer<CaptureCycleEngine> {
    let runtime = WakeWordRuntimeManager::new();
    runtime.begin_enable().unwrap();
    runtime.mark_loaded().unwrap();
    WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
        runtime,
        CaptureCycleEngine::default(),
    ))
}

#[test]
fn repeated_lifecycle_cycles_remain_bounded_and_return_to_listening() {
    let runtime = listening_runtime();
    let capacity = runtime
        .snapshot(Instant::now())
        .ring_buffer_capacity_samples;

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

    assert_eq!(runtime.snapshot(Instant::now()).trigger_count, 100);
}

#[tokio::test]
async fn repeated_wake_command_wake_cycles_reuse_one_authoritative_capture_owner() {
    let app_capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
    let capture_identity = Arc::as_ptr(&app_capture);
    let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(app_capture.clone());

    owner
        .start_wake(None, capture_cycle_consumer())
        .await
        .unwrap();

    for _ in 0..100 {
        assert_eq!(Arc::as_ptr(&app_capture), capture_identity);
        assert!(app_capture.lock().is_active());

        owner.transfer_to_command_asr().await.unwrap();
        assert!(!app_capture.lock().is_active());
        assert_eq!(Arc::as_ptr(&app_capture), capture_identity);

        owner.return_to_wake_listening(None).await.unwrap();
        assert!(app_capture.lock().is_active());
        assert_eq!(Arc::as_ptr(&app_capture), capture_identity);
    }

    owner.disable().await;
    assert!(!app_capture.lock().is_active());
    assert_eq!(Arc::as_ptr(&app_capture), capture_identity);
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
