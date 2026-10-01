use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_capture_consumer::WakeCapturePcmConsumer;
use super::wake_word_capture_orchestrator::WakeCaptureOrchestratorError;
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_composition::WakeWordApplicationRuntime;
use crate::audio::capture::AudioCapture;
use parking_lot::Mutex as CaptureMutex;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const SHUTDOWN_CAPTURE_CLOSE_GRACE: Duration = Duration::from_millis(25);

/// Events emitted by the dedicated Wake listener thread.
///
/// The event payloads are privacy-safe lifecycle signals plus the already-reviewed one-shot
/// command-ASR handoff payload. Raw capture chunks never cross this boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WakeLocalListenerEvent {
    Started,
    Triggered(WakeCommandHandoffAudio),
    StartupFailed(String),
    CaptureFailed(String),
    Stopped,
}

enum WakeLocalListenerCommand {
    Shutdown,
}

/// Send-capable handle for a Wake listener whose non-`Send` KWS engine remains on one OS thread.
///
/// Native sherpa sessions are loaded and driven inside the thread body, never moved into Tauri's
/// multithreaded async executor and never stored in a process-global `Sync` static. This is the
/// production runner seam WWR-400 needs before a real trigger can safely activate command ASR.
pub(crate) struct WakeLocalListenerHandle {
    command_tx: mpsc::UnboundedSender<WakeLocalListenerCommand>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl WakeLocalListenerHandle {
    pub(crate) fn request_shutdown(&self) {
        let _ = self.command_tx.send(WakeLocalListenerCommand::Shutdown);
    }

    pub(crate) fn join(mut self) -> thread::Result<()> {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join()
        } else {
            Ok(())
        }
    }

    pub(crate) fn shutdown(self) -> thread::Result<()> {
        self.request_shutdown();
        self.join()
    }
}

/// Spawn a dedicated single-thread Wake listener.
///
/// `build_consumer` executes inside the spawned OS thread, so callers may build non-`Send` engines
/// such as the native sherpa KWS session without ever transferring the engine across threads. The
/// returned handle is intentionally small: callers can only request shutdown, while normal trigger
/// delivery is reported through `event_tx`.
pub(crate) fn spawn_wake_local_listener_thread<E, Build>(
    capture: Arc<CaptureMutex<AudioCapture>>,
    runtime: WakeWordApplicationRuntime,
    device_name: Option<String>,
    build_consumer: Build,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
) -> Result<WakeLocalListenerHandle, std::io::Error>
where
    E: SherpaKwsEngine + 'static,
    Build: FnOnce(WakeWordApplicationRuntime) -> Result<WakeCapturePcmConsumer<E>, String>
        + Send
        + 'static,
{
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let join_handle = thread::Builder::new()
        .name("wake-word-listener".to_string())
        .spawn(move || {
            run_listener_thread(
                capture,
                runtime,
                device_name,
                build_consumer,
                command_rx,
                event_tx,
            )
        })?;

    Ok(WakeLocalListenerHandle {
        command_tx,
        join_handle: Some(join_handle),
    })
}

fn shutdown_command_ready(
    command_rx: &mut mpsc::UnboundedReceiver<WakeLocalListenerCommand>,
) -> bool {
    matches!(
        command_rx.try_recv(),
        Ok(WakeLocalListenerCommand::Shutdown)
            | Err(tokio::sync::mpsc::error::TryRecvError::Disconnected)
    )
}

async fn shutdown_command_ready_after_capture_close(
    command_rx: &mut mpsc::UnboundedReceiver<WakeLocalListenerCommand>,
) -> bool {
    if shutdown_command_ready(command_rx) {
        return true;
    }

    matches!(
        tokio::time::timeout(SHUTDOWN_CAPTURE_CLOSE_GRACE, command_rx.recv()).await,
        Ok(Some(WakeLocalListenerCommand::Shutdown)) | Ok(None)
    )
}

fn run_listener_thread<E, Build>(
    capture: Arc<CaptureMutex<AudioCapture>>,
    runtime: WakeWordApplicationRuntime,
    device_name: Option<String>,
    build_consumer: Build,
    mut command_rx: mpsc::UnboundedReceiver<WakeLocalListenerCommand>,
    event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
) where
    E: SherpaKwsEngine + 'static,
    Build: FnOnce(WakeWordApplicationRuntime) -> Result<WakeCapturePcmConsumer<E>, String>
        + Send
        + 'static,
{
    let Ok(tokio_runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    else {
        let _ = event_tx.send(WakeLocalListenerEvent::StartupFailed(
            "failed to initialize Wake listener runtime".to_string(),
        ));
        return;
    };

    tokio_runtime.block_on(async move {
        if shutdown_command_ready(&mut command_rx) {
            let _ = event_tx.send(WakeLocalListenerEvent::Stopped);
            return;
        }

        let consumer_result = build_consumer(runtime.clone());
        if shutdown_command_ready(&mut command_rx) {
            drop(consumer_result);
            let _ = event_tx.send(WakeLocalListenerEvent::Stopped);
            return;
        }
        let consumer = match consumer_result {
            Ok(consumer) => consumer,
            Err(error) => {
                runtime.record_runtime_error();
                let _ = event_tx.send(WakeLocalListenerEvent::StartupFailed(error));
                return;
            }
        };

        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(capture);
        let start_result = owner.start_wake(device_name, consumer).await;
        if shutdown_command_ready(&mut command_rx) {
            owner.disable().await;
            let _ = event_tx.send(WakeLocalListenerEvent::Stopped);
            return;
        }
        if let Err(error) = start_result {
            runtime.record_capture_error();
            let _ = event_tx.send(WakeLocalListenerEvent::StartupFailed(format!("{error:?}")));
            return;
        }

        if runtime.phase() == super::wake_word::runtime::WakeWordRuntimePhase::Loading {
            if let Err(error) = runtime.mark_loaded() {
                owner.disable().await;
                runtime.record_runtime_error();
                let _ = event_tx.send(WakeLocalListenerEvent::StartupFailed(error.to_string()));
                return;
            }
        }

        let _ = event_tx.send(WakeLocalListenerEvent::Started);

        loop {
            tokio::select! {
                command = command_rx.recv() => {
                    match command {
                        Some(WakeLocalListenerCommand::Shutdown) | None => {
                            owner.disable().await;
                            let _ = event_tx.send(WakeLocalListenerEvent::Stopped);
                            return;
                        }
                    }
                }
                routed = owner.route_next(Instant::now()) => {
                    // An intentional shutdown closes capture to unblock route_next. If that
                    // closure wins the select race, consume the queued shutdown before treating
                    // CaptureClosed as a real capture failure.
                    if shutdown_command_ready(&mut command_rx) {
                        owner.disable().await;
                        let _ = event_tx.send(WakeLocalListenerEvent::Stopped);
                        return;
                    }
                    match routed {
                        Ok(outcome) if outcome.trigger_accepted => {
                            match owner.transfer_to_command_asr().await {
                                Ok(Some(handoff)) => {
                                    let _ = event_tx.send(WakeLocalListenerEvent::Triggered(handoff));
                                }
                                Ok(None) => {
                                    runtime.record_runtime_error();
                                    let _ = event_tx.send(WakeLocalListenerEvent::CaptureFailed(
                                        "accepted Wake trigger did not produce handoff audio".to_string(),
                                    ));
                                }
                                Err(error) => {
                                    runtime.record_capture_error();
                                    let _ = event_tx.send(WakeLocalListenerEvent::CaptureFailed(format!("{error:?}")));
                                }
                            }
                            return;
                        }
                        Ok(_) => {}
                        Err(WakeCaptureOrchestratorError::CaptureClosed) => {
                            if shutdown_command_ready_after_capture_close(&mut command_rx).await {
                                owner.disable().await;
                                let _ = event_tx.send(WakeLocalListenerEvent::Stopped);
                                return;
                            }
                            runtime.record_capture_error();
                            let _ = event_tx.send(WakeLocalListenerEvent::CaptureFailed(
                                "CaptureClosed".to_string(),
                            ));
                            return;
                        }
                        Err(error) => {
                            runtime.record_capture_error();
                            let _ = event_tx.send(WakeLocalListenerEvent::CaptureFailed(format!("{error:?}")));
                            return;
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::wake_word::engine::{
        validate_pcm_frame, SherpaKwsConfig, WakeWordDetection, WakeWordError,
    };
    use crate::app::wake_word::runtime::WakeWordRuntimePhase;
    use crate::app::wake_word_pcm_router::CanonicalWakePcmRouter;
    use std::rc::Rc;
    use std::time::Duration;

    struct NonSendTestEngine {
        config: SherpaKwsConfig,
        _marker: Rc<()>,
    }

    impl Default for NonSendTestEngine {
        fn default() -> Self {
            Self {
                config: SherpaKwsConfig::default(),
                _marker: Rc::new(()),
            }
        }
    }

    impl SherpaKwsEngine for NonSendTestEngine {
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

    fn enabled_runtime() -> WakeWordApplicationRuntime {
        let settings = super::super::state::AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        runtime.mark_loaded().unwrap();
        runtime
    }

    #[tokio::test]
    async fn successful_listener_start_transitions_loading_runtime_to_listening() {
        let capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let settings = super::super::state::AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Loading);
        let runtime_for_consumer = runtime.clone();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        let handle = spawn_wake_local_listener_thread::<NonSendTestEngine, _>(
            capture.clone(),
            runtime.clone(),
            None,
            move |_| {
                Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                    runtime_for_consumer.manager().clone(),
                    NonSendTestEngine::default(),
                )))
            },
            event_tx,
        )
        .unwrap();

        let started = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(started, WakeLocalListenerEvent::Started);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);
        assert!(capture.lock().is_active());

        handle.shutdown().unwrap();
    }

    #[test]
    fn shutdown_request_during_blocked_initialization_is_prompt_and_prevents_started() {
        let capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let settings = super::super::state::AppSettings {
            wake_word_enabled: true,
            ..Default::default()
        };
        let runtime = WakeWordApplicationRuntime::from_settings(&settings).unwrap();
        let runtime_for_consumer = runtime.clone();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();

        let handle = spawn_wake_local_listener_thread::<NonSendTestEngine, _>(
            capture.clone(),
            runtime,
            None,
            move |_| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                    runtime_for_consumer.manager().clone(),
                    NonSendTestEngine::default(),
                )))
            },
            event_tx,
        )
        .unwrap();

        entered_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("native Wake initialization did not reach the deterministic barrier");
        let shutdown_started = Instant::now();
        handle.request_shutdown();
        assert!(
            shutdown_started.elapsed() < Duration::from_millis(100),
            "shutdown signalling must not wait for native initialization"
        );

        release_tx.send(()).unwrap();
        handle.join().unwrap();

        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }
        assert_eq!(events, vec![WakeLocalListenerEvent::Stopped]);
        assert!(!capture.lock().is_active());
    }

    #[tokio::test]
    async fn repeated_listener_start_stop_cycles_leave_no_capture_owner() {
        let capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));

        for cycle in 0..8 {
            let runtime = enabled_runtime();
            let runtime_for_consumer = runtime.clone();
            let (event_tx, mut event_rx) = mpsc::unbounded_channel();
            let handle = spawn_wake_local_listener_thread::<NonSendTestEngine, _>(
                capture.clone(),
                runtime,
                None,
                move |_| {
                    Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                        runtime_for_consumer.manager().clone(),
                        NonSendTestEngine::default(),
                    )))
                },
                event_tx,
            )
            .unwrap();

            let started = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(started, WakeLocalListenerEvent::Started, "cycle {cycle}");
            assert!(capture.lock().is_active(), "cycle {cycle}");

            handle.shutdown().unwrap();
            let stopped = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stopped, WakeLocalListenerEvent::Stopped, "cycle {cycle}");
            assert!(!capture.lock().is_active(), "cycle {cycle}");
        }
    }

    #[tokio::test]
    async fn listener_thread_keeps_non_send_engine_local_and_terminates_capture() {
        let capture = Arc::new(CaptureMutex::new(AudioCapture::new_mock()));
        let runtime = enabled_runtime();
        let runtime_for_consumer = runtime.clone();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        let handle = spawn_wake_local_listener_thread::<NonSendTestEngine, _>(
            capture.clone(),
            runtime.clone(),
            None,
            move |_| {
                Ok(WakeCapturePcmConsumer::new(CanonicalWakePcmRouter::new(
                    runtime_for_consumer.manager().clone(),
                    NonSendTestEngine::default(),
                )))
            },
            event_tx,
        )
        .unwrap();

        let started = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(started, WakeLocalListenerEvent::Started);

        handle.shutdown().unwrap();

        let terminal = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(terminal, WakeLocalListenerEvent::Stopped);
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
    }
}
