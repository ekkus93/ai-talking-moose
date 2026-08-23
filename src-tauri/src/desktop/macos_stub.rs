use crate::desktop::observation::{
    ActiveApplicationObservation, BatteryObservation, IdleObservation, ObserverResult, PowerEvent,
};
use tokio::sync::mpsc::UnboundedSender;

/// Non-macOS production implementation. Real desktop observation is a macOS-only V1
/// capability; unsupported platforms fail closed instead of fabricating values.
pub struct SystemDesktopMonitor;

impl SystemDesktopMonitor {
    pub fn get_active_application(allowed: bool) -> ObserverResult<ActiveApplicationObservation> {
        if allowed {
            ObserverResult::Unsupported
        } else {
            ObserverResult::Denied
        }
    }

    pub fn get_battery_state() -> ObserverResult<BatteryObservation> {
        ObserverResult::Unsupported
    }

    pub fn get_idle_time() -> ObserverResult<IdleObservation> {
        ObserverResult::Unsupported
    }

    pub fn get_window_title(allowed: bool) -> ObserverResult<String> {
        if allowed {
            ObserverResult::Unsupported
        } else {
            ObserverResult::Denied
        }
    }

    pub fn start_power_events(
        _sender: UnboundedSender<PowerEvent>,
    ) -> ObserverResult<SystemPowerObserver> {
        ObserverResult::Unsupported
    }
}

pub struct SystemPowerObserver;

impl SystemPowerObserver {
    pub fn stop(self) {}
}
