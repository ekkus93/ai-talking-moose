use crate::app::wake_word::engine::SherpaKwsEngine;
use crate::app::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;

#[cfg(test)]
fn capture_owner_from_app_state<E: SherpaKwsEngine>(
    state: &AppState,
) -> AuthoritativeWakeCaptureOwner<E> {
    AuthoritativeWakeCaptureOwner::from_shared_capture(state.audio_capture.clone())
}

#[cfg(test)]
#[derive(Debug)]
enum WakeWordStartupError {
    Runtime,
    Native,
    Capture,
}

#[cfg(test)]
async fn start_native_wake_from_app_state(
    state: &AppState,
    paths: NativeKwsSessionPaths,
) -> Result<Option<AuthoritativeWakeCaptureOwner<NativeKwsSession>>, WakeWordStartupError> {
    let settings = state.settings.read().clone();
    if !settings.wake_word_enabled {
        state
            .wake_word_runtime
            .apply_enabled_setting(false)
            .map_err(|_| WakeWordStartupError::Runtime)?;
        return Ok(None);
    }

    state
        .wake_word_runtime
        .apply_enabled_setting(true)
        .map_err(|_| WakeWordStartupError::Runtime)?;

    let consumer = match state.wake_word_runtime.native_capture_consumer(paths) {
        Ok(consumer) => consumer,
        Err(_) => {
            state.wake_word_runtime.record_runtime_error();
            return Err(WakeWordStartupError::Native);
        }
    };

    state
        .wake_word_runtime
        .mark_loaded()
        .map_err(|_| WakeWordStartupError::Runtime)?;

    let owner = capture_owner_from_app_state::<NativeKwsSession>(state);
    if owner
        .start_wake(settings.input_device.clone(), consumer)
        .await
        .is_err()
    {
        state.wake_word_runtime.record_capture_error();
        return Err(WakeWordStartupError::Capture);
    }

    Ok(Some(owner))
}

use super::*;
use crate::app::wake_word::engine::{
    validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
};
use crate::app::wake_word_capture_consumer::WakeCapturePcmConsumer;
use crate::app::wake_word_pcm_router::CanonicalWakePcmRouter;
use crate::asr::wake_word_runtime::WakeWordRuntimePhase;
use crate::audio::capture::AudioCapture;
use crate::wake_word_policy::V1_KWS_SAMPLE_RATE_HZ;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Default)]
struct TestWakeEngine {
    config: SherpaKwsConfig,
}

impl SherpaKwsEngine for TestWakeEngine {
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

#[test]
fn wake_v1_supports_local_command_asr_modes() {
    assert!(wake_word_asr_mode_supported(
        AsrMode::MoonshineTinyStreaming
    ));
    assert!(wake_word_asr_mode_supported(
        AsrMode::MoonshineSmallStreaming
    ));
    assert!(wake_word_asr_mode_supported(AsrMode::WhisperSmall));
    assert!(!wake_word_asr_mode_supported(AsrMode::GeminiLiveAudio));
}

#[test]
fn controller_concurrent_start_reservation_has_one_owner() {
    let controller = NativeWakeListenerController::default();
    let barrier = Arc::new(Barrier::new(3));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut workers = Vec::new();

    for _ in 0..2 {
        let controller = controller.clone();
        let barrier = barrier.clone();
        let tx = tx.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            tx.send(controller.reserve_start()).unwrap();
        }));
    }

    barrier.wait();
    let results = [rx.recv().unwrap(), rx.recv().unwrap()];
    for worker in workers {
        worker.join().unwrap();
    }

    assert_eq!(results.iter().filter(|value| value.is_some()).count(), 1);
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Starting
    );
    assert!(matches!(
        controller.begin_stop(),
        NativeWakeListenerStopPlan::AwaitingStart
    ));
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Stopping
    );
    let start_generation = results.into_iter().flatten().next().unwrap();
    controller.cancel_start(start_generation);
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Stopped
    );
}

#[test]
fn controller_stop_invalidates_pending_start_generation() {
    let controller = NativeWakeListenerController::default();
    let first_generation = controller.reserve_start().unwrap();
    assert!(matches!(
        controller.begin_stop(),
        NativeWakeListenerStopPlan::AwaitingStart
    ));
    controller.cancel_start(first_generation);

    let second_generation = controller.reserve_start().unwrap();
    assert_ne!(first_generation, second_generation);
    assert!(second_generation > first_generation);
    controller.cancel_start(second_generation);
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Stopped
    );
}

#[test]
fn newest_deferred_restart_generation_is_the_only_one_that_can_claim_stopped() {
    let controller = NativeWakeListenerController::default();
    let stale_start = controller.reserve_start().unwrap();
    assert!(matches!(
        controller.begin_stop(),
        NativeWakeListenerStopPlan::AwaitingStart
    ));

    let older_restart = controller.schedule_restart();
    let newest_restart = controller.schedule_restart();
    assert_ne!(older_restart, newest_restart);
    assert!(newest_restart > older_restart);

    controller.cancel_start(stale_start);
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Stopped
    );
    assert_eq!(controller.claim_scheduled_restart(older_restart), None);
    assert_eq!(
        controller.claim_scheduled_restart(newest_restart),
        Some(newest_restart)
    );
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Starting
    );
    controller.cancel_start(newest_restart);
    assert_eq!(
        controller.phase(),
        NativeWakeListenerLifecyclePhase::Stopped
    );
}

#[test]
fn explicit_start_or_stop_cancels_deferred_restart_intent() {
    let controller = NativeWakeListenerController::default();
    let pending = controller.schedule_restart();
    let direct = controller.reserve_start().unwrap();
    assert!(!controller.restart_request_is_current(pending));
    controller.cancel_start(direct);

    let pending = controller.schedule_restart();
    assert!(controller.restart_request_is_current(pending));
    controller.cancel_all_restart_requests();
    assert!(!controller.restart_request_is_current(pending));
}

#[tokio::test]
async fn stale_start_completion_is_rejected_and_disposed() {
    let state = AppState::new_for_tests().unwrap();
    *state.audio_capture.lock() = AudioCapture::new_mock();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    let generation = state.wake_listener_controller.reserve_start().unwrap();
    let runtime_for_consumer = state.wake_word_runtime.clone();
    let (event_tx, _event_rx) = mpsc::unbounded_channel();

    let handle = spawn_wake_local_listener_thread::<TestWakeEngine, _>(
        state.audio_capture.clone(),
        state.wake_word_runtime.clone(),
        None,
        move |_| {
            Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                runtime_for_consumer.manager().clone(),
                TestWakeEngine::default(),
            )))
        },
        event_tx,
    )
    .unwrap();

    assert!(matches!(
        state.wake_listener_controller.begin_stop(),
        NativeWakeListenerStopPlan::AwaitingStart
    ));
    let (stale, finish_generation) = state
        .wake_listener_controller
        .publish_start(generation, handle)
        .expect_err("stale start generation must never publish Running");
    let done = supervise_native_wake_listener_shutdown(
        state.wake_listener_controller.clone(),
        finish_generation,
        stale,
    )
    .unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();

    assert_eq!(
        state.wake_listener_controller.phase(),
        NativeWakeListenerLifecyclePhase::Stopped
    );
    assert!(!state.audio_capture.lock().is_active());
}

#[tokio::test]
async fn delayed_listener_join_is_bounded_and_reconciles_once_after_late_exit() {
    let state = AppState::new_for_tests().unwrap();
    *state.audio_capture.lock() = AudioCapture::new_mock();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    let runtime_for_consumer = state.wake_word_runtime.clone();
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let (entered_tx, entered_rx) = std_mpsc::channel();
    let (release_tx, release_rx) = std_mpsc::channel();

    let handle = spawn_wake_local_listener_thread::<TestWakeEngine, _>(
        state.audio_capture.clone(),
        state.wake_word_runtime.clone(),
        None,
        move |_| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                runtime_for_consumer.manager().clone(),
                TestWakeEngine::default(),
            )))
        },
        event_tx,
    )
    .unwrap();
    state
        .wake_listener_controller
        .install_running_for_test(1, handle);

    entered_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("listener initialization did not reach deterministic barrier");

    let stop_started = Instant::now();
    stop_native_wake_listener_thread(&state).unwrap();
    assert!(
        stop_started.elapsed() < Duration::from_millis(500),
        "stop must not wait for the full native initialization body"
    );
    assert_eq!(
        native_wake_listener_lifecycle_phase(&state),
        NativeWakeListenerLifecyclePhase::Stopping
    );

    // A repeated stop while the join supervisor owns shutdown must be idempotent and must not
    // report Stopped before the native thread actually exits.
    stop_native_wake_listener_thread(&state).unwrap();
    assert_eq!(
        native_wake_listener_lifecycle_phase(&state),
        NativeWakeListenerLifecyclePhase::Stopping
    );

    release_tx.send(()).unwrap();
    assert!(
        wait_for_native_wake_listener_stopped(&state, Duration::from_secs(2)).await,
        "late native termination must reconcile Stopping to Stopped"
    );
    assert_eq!(
        native_wake_listener_lifecycle_phase(&state),
        NativeWakeListenerLifecyclePhase::Stopped
    );
    assert!(!state.audio_capture.lock().is_active());
}

#[test]
fn settings_reject_unsupported_asr_before_listener_start() {
    let state = AppState::new_for_tests().unwrap();
    let previous = AppSettings::default();
    let next = AppSettings {
        wake_word_enabled: true,
        asr_mode: AsrMode::GeminiLiveAudio,
        ..Default::default()
    };
    let error = apply_configured_native_wake_listener_settings_change(&state, &previous, &next)
        .unwrap_err();
    assert_eq!(error, WAKE_WORD_UNSUPPORTED_ASR_MESSAGE);
    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
}

#[test]
fn app_state_directly_owns_wake_runtime_without_owning_capture() {
    let state = AppState::new_for_tests().unwrap();
    assert!(!state.settings.read().wake_word_enabled);
    assert_eq!(
        runtime_from_app_state(&state).phase(),
        WakeWordRuntimePhase::Disabled
    );
    assert!(!state.audio_capture.lock().diagnostics().active);
}

#[test]
fn native_paths_live_under_application_data_directory() {
    let temp = tempfile::tempdir().unwrap();
    let paths = native_kws_paths_from_app_data_dir(temp.path());

    assert!(paths.model_dir.starts_with(temp.path()));
    assert!(paths
        .model_dir
        .ends_with("models/wake-word/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"));
    assert!(paths.runtime_dir.ends_with(native_runtime_platform_dir()));
}

#[tokio::test]
async fn capture_owner_from_app_state_uses_exact_app_capture() {
    let state = AppState::new_for_tests().unwrap();
    *state.audio_capture.lock() = AudioCapture::new_mock();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();

    let owner = capture_owner_from_app_state::<TestWakeEngine>(&state);
    let consumer = state
        .wake_word_runtime
        .capture_consumer(TestWakeEngine::default());

    owner.start_wake(None, consumer).await.unwrap();
    assert!(state.audio_capture.lock().is_active());
    assert_eq!(
        state.audio_capture.lock().diagnostics().sample_rate_hz,
        Some(V1_KWS_SAMPLE_RATE_HZ)
    );

    owner.disable().await;
    assert!(!state.audio_capture.lock().is_active());
    assert_eq!(
        state
            .wake_word_runtime
            .snapshot(std::time::Instant::now())
            .phase,
        WakeWordRuntimePhase::Disabled
    );
}

#[tokio::test]
async fn disabled_startup_does_not_load_native_or_open_capture() {
    let state = AppState::new_for_tests().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let paths = NativeKwsSessionPaths {
        model_dir: temp.path().join("missing-model"),
        runtime_dir: temp.path().join("missing-runtime"),
    };

    let owner = start_native_wake_from_app_state(&state, paths)
        .await
        .unwrap();

    assert!(owner.is_none());
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
    assert!(!state.audio_capture.lock().is_active());
}

#[tokio::test]
async fn enabled_startup_fails_closed_before_capture_when_native_artifacts_missing() {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().wake_word_enabled = true;
    let temp = tempfile::tempdir().unwrap();
    let paths = NativeKwsSessionPaths {
        model_dir: temp.path().join("missing-model"),
        runtime_dir: temp.path().join("missing-runtime"),
    };

    let result = start_native_wake_from_app_state(&state, paths).await;
    assert!(matches!(result, Err(WakeWordStartupError::Native)));
    assert_eq!(state.wake_word_runtime.phase(), WakeWordRuntimePhase::Error);
    assert!(!state.audio_capture.lock().is_active());
}

#[test]
fn disabled_local_thread_start_retains_restart_configuration_without_listener() {
    let state = AppState::new_for_tests().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (event_tx, _event_rx) = mpsc::unbounded_channel();

    let started = control_native_wake_listener(
        &state,
        NativeWakeListenerControl::ConfigureAndStart {
            app_data_dir: temp.path().to_path_buf(),
            event_tx: event_tx.clone(),
        },
    )
    .unwrap();

    assert!(!started);
    assert_eq!(
        native_wake_listener_lifecycle_phase(&state),
        NativeWakeListenerLifecyclePhase::Stopped
    );
    assert!(state.wake_listener_controller.config().is_some());
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
}

#[test]
fn settings_disable_stops_listener_and_runtime_state() {
    let state = AppState::new_for_tests().unwrap();
    let previous = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    let next = AppSettings::default();

    apply_configured_native_wake_listener_settings_change(&state, &previous, &next).unwrap();

    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
}

#[test]
fn listener_start_validates_pending_settings_instead_of_persisted_settings() {
    let state = AppState::new_for_tests().unwrap();
    assert_eq!(
        state.settings.read().asr_mode,
        AsrMode::MoonshineTinyStreaming
    );
    let pending = AppSettings {
        wake_word_enabled: true,
        asr_mode: AsrMode::GeminiLiveAudio,
        ..state.settings.read().clone()
    };
    let temp = tempfile::tempdir().unwrap();
    let (event_tx, _event_rx) = mpsc::unbounded_channel();

    let error =
        start_native_wake_listener_thread_with_config(&state, &pending, temp.path(), event_tx)
            .unwrap_err();

    assert_eq!(error, WAKE_WORD_UNSUPPORTED_ASR_MESSAGE);
    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
}

#[test]
fn pending_restart_preserves_suspended_command_ownership() {
    let state = AppState::new_for_tests().unwrap();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();
    state.wake_word_runtime.suspend_for_talking().unwrap();

    prepare_runtime_for_pending_listener_restart(&state.wake_word_runtime).unwrap();

    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::SuspendedTalking
    );
}

#[test]
fn pending_restart_moves_disabled_runtime_to_loading() {
    let state = AppState::new_for_tests().unwrap();

    prepare_runtime_for_pending_listener_restart(&state.wake_word_runtime).unwrap();

    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
}

#[test]
fn command_transfer_suspends_listening_runtime_without_recording_error() {
    let state = AppState::new_for_tests().unwrap();
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();

    let was_active =
        control_native_wake_listener(&state, NativeWakeListenerControl::TransferToCommand).unwrap();

    assert!(!was_active);
    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::SuspendedTalking
    );
    assert_eq!(
        state
            .wake_word_runtime
            .snapshot(std::time::Instant::now())
            .last_error,
        None
    );
}

#[tokio::test]
async fn active_listener_transfer_releases_capture_and_preserves_command_suspension() {
    let state = AppState::new_for_tests().unwrap();
    *state.audio_capture.lock() = AudioCapture::new_mock();
    state.settings.write().wake_word_enabled = true;
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    let runtime_for_consumer = state.wake_word_runtime.clone();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let handle = spawn_wake_local_listener_thread::<TestWakeEngine, _>(
        state.audio_capture.clone(),
        state.wake_word_runtime.clone(),
        None,
        move |_| {
            Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                runtime_for_consumer.manager().clone(),
                TestWakeEngine::default(),
            )))
        },
        event_tx,
    )
    .unwrap();
    state
        .wake_listener_controller
        .install_running_for_test(1, handle);

    let started = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(started, WakeLocalListenerEvent::Started);
    assert!(native_wake_listener_is_active(&state));
    assert!(state.audio_capture.lock().is_active());
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Listening
    );

    let was_active =
        control_native_wake_listener(&state, NativeWakeListenerControl::TransferToCommand).unwrap();

    assert!(was_active);
    assert!(!native_wake_listener_is_active(&state));
    let stopped = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stopped, WakeLocalListenerEvent::Stopped);
    assert!(!state.audio_capture.lock().is_active());
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::SuspendedTalking
    );
    assert_eq!(
        state
            .wake_word_runtime
            .snapshot(std::time::Instant::now())
            .last_error,
        None
    );
}

#[test]
fn command_transfer_preserves_manual_availability_from_non_listening_wake_states() {
    for phase in ["disabled", "loading", "error"] {
        let state = AppState::new_for_tests().unwrap();
        match phase {
            "disabled" => {}
            "loading" => {
                state.wake_word_runtime.apply_enabled_setting(true).unwrap();
            }
            "error" => {
                state.wake_word_runtime.apply_enabled_setting(true).unwrap();
                state.wake_word_runtime.record_runtime_error();
            }
            _ => unreachable!(),
        }

        control_native_wake_listener(&state, NativeWakeListenerControl::TransferToCommand).unwrap();

        let expected = match phase {
            "disabled" => WakeWordRuntimePhase::Disabled,
            "loading" => WakeWordRuntimePhase::Loading,
            "error" => WakeWordRuntimePhase::Error,
            _ => unreachable!(),
        };
        assert_eq!(state.wake_word_runtime.phase(), expected, "{phase}");
    }
}

#[test]
fn terminal_command_completion_uses_latest_enable_setting() {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().wake_word_enabled = true;

    let should_restart = complete_native_wake_command_interaction(
        &state,
        CommandInteractionTerminalOutcome::Success,
    )
    .unwrap();

    assert!(should_restart);
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
}

#[test]
fn terminal_command_completion_honors_latest_disable_setting() {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().wake_word_enabled = true;
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.mark_loaded().unwrap();
    state.wake_word_runtime.suspend_for_talking().unwrap();
    state.settings.write().wake_word_enabled = false;

    let should_restart = complete_native_wake_command_interaction(
        &state,
        CommandInteractionTerminalOutcome::Cancelled,
    )
    .unwrap();

    assert!(!should_restart);
    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Disabled
    );
}

#[test]
fn terminal_command_completion_recovers_wake_error_when_still_enabled() {
    let state = AppState::new_for_tests().unwrap();
    state.settings.write().wake_word_enabled = true;
    state.wake_word_runtime.apply_enabled_setting(true).unwrap();
    state.wake_word_runtime.record_runtime_error();

    complete_native_wake_command_interaction(
        &state,
        CommandInteractionTerminalOutcome::RecoverableFailure,
    )
    .unwrap();

    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
}

#[test]
fn settings_enable_without_startup_config_enters_loading_without_false_listener_claim() {
    let state = AppState::new_for_tests().unwrap();
    let previous = AppSettings::default();
    let next = AppSettings {
        wake_word_enabled: true,
        ..Default::default()
    };

    apply_configured_native_wake_listener_settings_change(&state, &previous, &next).unwrap();

    assert!(!native_wake_listener_is_active(&state));
    assert_eq!(
        state.wake_word_runtime.phase(),
        WakeWordRuntimePhase::Loading
    );
}
