use crate::desktop::observation::{
    ActiveApplicationObservation, BatteryObservation, IdleObservation, ObserverErrorCode,
    ObserverResult, PowerEvent,
};
use tokio::sync::mpsc::Sender;

fn normalize_idle_duration(seconds: f64) -> Result<IdleObservation, ObserverErrorCode> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(ObserverErrorCode::InvalidValue);
    }
    Ok(IdleObservation {
        seconds: seconds.floor().min(u64::MAX as f64) as u64,
    })
}

fn normalize_battery_state(
    current: i32,
    maximum: i32,
    is_charging: bool,
) -> Result<BatteryObservation, ObserverErrorCode> {
    if current < 0 || maximum <= 0 {
        return Err(ObserverErrorCode::InvalidValue);
    }
    let percentage = (f64::from(current) / f64::from(maximum) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    Ok(BatteryObservation {
        level_percent: percentage,
        is_charging,
    })
}

pub struct SystemDesktopMonitor;

impl SystemDesktopMonitor {
    pub fn get_active_application(allowed: bool) -> ObserverResult<ActiveApplicationObservation> {
        if !allowed {
            return ObserverResult::Denied;
        }
        platform::active_application()
    }

    pub fn get_battery_state() -> ObserverResult<BatteryObservation> {
        platform::battery_state()
    }

    pub fn get_idle_time() -> ObserverResult<IdleObservation> {
        platform::idle_time()
    }

    pub fn get_window_title(allowed: bool) -> ObserverResult<String> {
        if !allowed {
            ObserverResult::Denied
        } else {
            // V1 intentionally does not request Accessibility/Screen Recording-style
            // access to inspect another application's window title.
            ObserverResult::Unsupported
        }
    }

    pub fn start_power_events(sender: Sender<PowerEvent>) -> ObserverResult<SystemPowerObserver> {
        platform::start_power_events(sender)
    }
}

pub struct SystemPowerObserver {
    #[cfg(target_os = "macos")]
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[cfg(target_os = "macos")]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SystemPowerObserver {
    pub fn stop(self) {
        #[cfg(target_os = "macos")]
        {
            let mut observer = self;
            observer.stop_inner();
        }
    }

    fn stop_inner(&mut self) {
        #[cfg(target_os = "macos")]
        {
            use std::sync::atomic::Ordering;
            self.running.store(false, Ordering::SeqCst);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for SystemPowerObserver {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

