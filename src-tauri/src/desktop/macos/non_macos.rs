#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub(super) fn active_application() -> ObserverResult<ActiveApplicationObservation> {
        ObserverResult::Unsupported
    }

    pub(super) fn battery_state() -> ObserverResult<BatteryObservation> {
        ObserverResult::Unsupported
    }

    pub(super) fn idle_time() -> ObserverResult<IdleObservation> {
        ObserverResult::Unsupported
    }

    pub(super) fn start_power_events(
        _sender: Sender<PowerEvent>,
    ) -> ObserverResult<SystemPowerObserver> {
        ObserverResult::Unsupported
    }
}

