use crate::app::state::AppSettings;
use crate::character::ambient::AmbientScheduler;
use crate::desktop::events::{DesktopEvent, DesktopEventSummarizer};
use crate::desktop::macos::{SystemDesktopMonitor, SystemPowerObserver};
use crate::desktop::observation::{ObserverKind, ObserverResult, PowerEvent};
use parking_lot::RwLock;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::debug;

const OBSERVER_POLL_INTERVAL: Duration = Duration::from_secs(15);
const BATTERY_POLL_EVERY_TICKS: u32 = 4;
const POWER_EVENT_QUEUE_CAPACITY: usize = 8;

struct DesktopRuntimeState {
    cancellation: CancellationToken,
    shutdown_complete: watch::Sender<bool>,
    power_observer: Option<SystemPowerObserver>,
    summarizer: Arc<Mutex<DesktopEventSummarizer>>,
}

struct ShutdownSignal(watch::Sender<bool>);

impl Drop for ShutdownSignal {
    fn drop(&mut self) {
        let _ = self.0.send(true);
    }
}

static DESKTOP_RUNTIME: OnceLock<Mutex<Option<DesktopRuntimeState>>> = OnceLock::new();

fn runtime_state() -> &'static Mutex<Option<DesktopRuntimeState>> {
    DESKTOP_RUNTIME.get_or_init(|| Mutex::new(None))
}

fn reset_summarizer(summarizer: &Mutex<DesktopEventSummarizer>) {
    let mut summarizer = summarizer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *summarizer = DesktopEventSummarizer::new();
}

pub fn reset_observation_state() {
    let state = runtime_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(runtime) = state.as_ref() {
        reset_summarizer(runtime.summarizer.as_ref());
    }
}

fn log_observer_result<T>(kind: ObserverKind, result: &ObserverResult<T>) {
    let diagnostic = result.diagnostic(kind);
    debug!(
        observer_kind = ?diagnostic.kind,
        observer_status = ?diagnostic.status,
        observer_error_code = ?diagnostic.error_code,
        "Desktop observer result"
    );
}

fn submit_event(scheduler: &AmbientScheduler, event: DesktopEvent) {
    let ambient = DesktopEventSummarizer::to_ambient_event(event);
    match scheduler.try_submit_background(ambient) {
        Ok(true) => {}
        Ok(false) => {
            debug!("Ambient scheduler queue full; dropping newest desktop observation");
        }
        Err(_) => {
            debug!("Desktop observation could not be submitted to ambient scheduler");
        }
    }
}

fn handle_power_event(
    summarizer: &mut DesktopEventSummarizer,
    scheduler: &AmbientScheduler,
    event: PowerEvent,
) {
    submit_event(scheduler, summarizer.record_power(event));
}

fn handle_available<T, F>(kind: ObserverKind, result: ObserverResult<T>, mut on_available: F)
where
    F: FnMut(T),
{
    log_observer_result(kind, &result);
    if let Some(value) = result.into_available() {
        on_available(value);
    }
}

fn poll_observers(
    settings: &RwLock<AppSettings>,
    scheduler: &AmbientScheduler,
    summarizer: &mut DesktopEventSummarizer,
    poll_tick: u32,
) {
    handle_available(
        ObserverKind::IdleTime,
        SystemDesktopMonitor::get_idle_time(),
        |observation| {
            if let Some(event) = summarizer.record_idle(observation) {
                submit_event(scheduler, event);
            }
        },
    );

    let active_app_allowed = settings.read().active_app_observation;
    if !active_app_allowed {
        // Treat privacy opt-out as an observation boundary. Derived fingerprints and
        // switch timestamps from the previously enabled period must not survive the
        // transition or influence the first observation after re-enable.
        summarizer.clear_active_application_history();
    }
    handle_available(
        ObserverKind::ActiveApplication,
        SystemDesktopMonitor::get_active_application(active_app_allowed),
        |observation| {
            if let Some(event) = summarizer.record_app_switch(observation) {
                submit_event(scheduler, event);
            }
        },
    );

    if poll_tick.is_multiple_of(BATTERY_POLL_EVERY_TICKS) {
        handle_available(
            ObserverKind::Battery,
            SystemDesktopMonitor::get_battery_state(),
            |observation| {
                if let Some(event) = summarizer.record_battery(observation) {
                    submit_event(scheduler, event);
                }
            },
        );
    }
}

async fn run_observer_loop(
    settings: Arc<RwLock<AppSettings>>,
    scheduler: AmbientScheduler,
    cancellation: CancellationToken,
    shutdown_complete: watch::Sender<bool>,
    summarizer: Arc<Mutex<DesktopEventSummarizer>>,
    mut power_rx: mpsc::Receiver<PowerEvent>,
    _power_sender_keepalive: mpsc::Sender<PowerEvent>,
) {
    let _shutdown_signal = ShutdownSignal(shutdown_complete);
    let mut interval = tokio::time::interval(OBSERVER_POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut poll_tick = 0_u32;

    loop {
        tokio::select! {
            _ = cancellation.cancelled() => break,
            power = power_rx.recv() => {
                if let Some(power) = power {
                    let mut summarizer = summarizer
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    handle_power_event(&mut summarizer, &scheduler, power);
                }
            }
            _ = interval.tick() => {
                let mut summarizer = summarizer
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                poll_observers(&settings, &scheduler, &mut summarizer, poll_tick);
                poll_tick = poll_tick.wrapping_add(1);
            }
        }
    }
}

pub fn start(
    settings: Arc<RwLock<AppSettings>>,
    scheduler: AmbientScheduler,
) -> Result<(), String> {
    let mut state = runtime_state()
        .lock()
        .map_err(|_| "desktop observer runtime lock is poisoned".to_string())?;
    if state.is_some() {
        return Err("desktop observer runtime is already started".to_string());
    }

    let cancellation = CancellationToken::new();
    let (shutdown_complete, _shutdown_rx) = watch::channel(false);
    let (power_tx, power_rx) = mpsc::channel(POWER_EVENT_QUEUE_CAPACITY);
    let power_result = SystemDesktopMonitor::start_power_events(power_tx.clone());
    log_observer_result(ObserverKind::SleepWake, &power_result);
    let power_observer = power_result.into_available();

    let title_result = SystemDesktopMonitor::get_window_title(true);
    log_observer_result(ObserverKind::WindowTitle, &title_result);

    let summarizer = Arc::new(Mutex::new(DesktopEventSummarizer::new()));
    let task_cancellation = cancellation.clone();
    let task_shutdown = shutdown_complete.clone();
    let task_summarizer = summarizer.clone();
    tauri::async_runtime::spawn(run_observer_loop(
        settings,
        scheduler,
        task_cancellation,
        task_shutdown,
        task_summarizer,
        power_rx,
        power_tx,
    ));

    *state = Some(DesktopRuntimeState {
        cancellation,
        shutdown_complete,
        power_observer,
        summarizer,
    });
    Ok(())
}

pub async fn stop() {
    let runtime = runtime_state()
        .lock()
        .ok()
        .and_then(|mut state| state.take());
    let Some(runtime) = runtime else {
        return;
    };

    let mut shutdown_complete = runtime.shutdown_complete.subscribe();
    runtime.cancellation.cancel();
    if let Some(observer) = runtime.power_observer {
        let _ = tokio::task::spawn_blocking(move || observer.stop()).await;
    }
    if !*shutdown_complete.borrow() {
        let _ = shutdown_complete.changed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::observation::{
        ActiveApplicationObservation, IdleObservation, ObserverErrorCode, ObserverStatus,
    };
    use crate::test_support::{assert_log_capture_live, capture_logs};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn power_event_queue_has_a_hard_capacity() {
        let (sender, mut receiver) = mpsc::channel(POWER_EVENT_QUEUE_CAPACITY);
        for _ in 0..POWER_EVENT_QUEUE_CAPACITY {
            sender.try_send(PowerEvent::Sleep).unwrap();
        }
        assert!(matches!(
            sender.try_send(PowerEvent::Wake),
            Err(mpsc::error::TrySendError::Full(PowerEvent::Wake))
        ));
        for _ in 0..POWER_EVENT_QUEUE_CAPACITY {
            assert_eq!(receiver.try_recv().unwrap(), PowerEvent::Sleep);
        }
    }

    #[test]
    fn non_available_observer_results_never_reach_consumer() {
        let calls = AtomicUsize::new(0);
        for result in [
            ObserverResult::Denied,
            ObserverResult::Unavailable(ObserverErrorCode::NoFrontmostApplication),
            ObserverResult::Unsupported,
            ObserverResult::Error(ObserverErrorCode::PlatformApiFailure),
        ] {
            handle_available(
                ObserverKind::ActiveApplication,
                result,
                |_observation: ActiveApplicationObservation| {
                    calls.fetch_add(1, Ordering::SeqCst);
                },
            );
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn private_desktop_observation_is_consumed_without_entering_tracing() {
        const PRIVATE_APP: &str = "PRIVATE_DESKTOP_APP_84a17e";
        let mut observed = None;

        let (_, logs) = capture_logs(|| {
            handle_available(
                ObserverKind::ActiveApplication,
                ObserverResult::Available(ActiveApplicationObservation {
                    name: PRIVATE_APP.to_string(),
                }),
                |observation| observed = Some(observation.name),
            );
        });

        assert_eq!(observed.as_deref(), Some(PRIVATE_APP));
        assert_log_capture_live(&logs);
        assert!(!logs.contains(PRIVATE_APP));
    }

    #[test]
    fn observation_state_reset_clears_derived_idle_and_application_history() {
        let summarizer = Mutex::new(DesktopEventSummarizer::new());
        {
            let mut state = summarizer.lock().unwrap();
            assert!(state
                .record_idle(IdleObservation { seconds: 301 })
                .is_some());
            assert!(state
                .record_app_switch(ActiveApplicationObservation {
                    name: "Terminal".to_string(),
                })
                .is_none());
            assert!(state
                .record_app_switch(ActiveApplicationObservation {
                    name: "Browser".to_string(),
                })
                .is_some());
        }

        reset_summarizer(&summarizer);

        let mut state = summarizer.lock().unwrap();
        assert!(state
            .record_idle(IdleObservation { seconds: 301 })
            .is_some());
        assert!(state
            .record_app_switch(ActiveApplicationObservation {
                name: "Browser".to_string(),
            })
            .is_none());
    }

    #[test]
    fn window_title_runtime_contract_is_unsupported_not_available() {
        let result = SystemDesktopMonitor::get_window_title(true);
        assert_eq!(
            result.diagnostic(ObserverKind::WindowTitle).status,
            ObserverStatus::Unsupported
        );
    }
}
