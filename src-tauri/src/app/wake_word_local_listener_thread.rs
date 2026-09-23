use super::wake_word::engine::SherpaKwsEngine;
use super::wake_word_authoritative_capture::AuthoritativeWakeCaptureOwner;
use super::wake_word_capture_consumer::WakeCapturePcmConsumer;
use super::wake_word_command_handoff::WakeCommandHandoffAudio;
use super::wake_word_composition::WakeWordApplicationRuntime;
use crate::audio::capture::AudioCapture;
use parking_lot::Mutex as CaptureMutex;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tokio::sync::mpsc;

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
    pub(crate) fn shutdown(mut self) -> thread::Result<()> {
        let _ = self.command_tx.send(WakeLocalListenerCommand::Shutdown);
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join()
        } else {
            Ok(())
        }
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
        .spawn(move || run_listener_thread(capture, runtime, device_name, build_consumer, command_rx, event_tx))?;

    Ok(WakeLocalListenerHandle {
        command_tx,
        join_handle: Some(join_handle),
    })
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
        let consumer = match build_consumer(runtime.clone()) {
            Ok(consumer) => consumer,
            Err(error) => {
                runtime.record_runtime_error();
                let _ = event_tx.send(WakeLocalListenerEvent::StartupFailed(error));
                return;
            }
        };

        let owner = AuthoritativeWakeCaptureOwner::from_shared_capture(capture);
        if let Err(error) = owner.start_wake(device_name, consumer).await {
            runtime.record_capture_error();
            let _ = event_tx.send(WakeLocalListenerEvent::StartupFailed(format!("{error:?}")));
            return;
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
    async fn listener_thread_keeps_non_send_engine_local_and_stops_authoritative_capture() {
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
        assert!(capture.lock().is_active());
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Listening);

        handle.shutdown().unwrap();

        let stopped = tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stopped, WakeLocalListenerEvent::Stopped);
        assert!(!capture.lock().is_active());
        assert_eq!(runtime.phase(), WakeWordRuntimePhase::Disabled);
    }
}
