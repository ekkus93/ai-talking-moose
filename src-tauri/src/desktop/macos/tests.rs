#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::observation::{ObserverKind, ObserverStatus};

    #[test]
    fn idle_conversion_rejects_invalid_values_and_floors_seconds() {
        assert_eq!(
            normalize_idle_duration(f64::NAN),
            Err(ObserverErrorCode::InvalidValue)
        );
        assert_eq!(
            normalize_idle_duration(-1.0),
            Err(ObserverErrorCode::InvalidValue)
        );
        assert_eq!(normalize_idle_duration(12.9).unwrap().seconds, 12);
    }

    #[test]
    fn battery_conversion_is_bounded_and_rejects_invalid_capacity() {
        assert_eq!(
            normalize_battery_state(25, 100, false).unwrap(),
            BatteryObservation {
                level_percent: 25,
                is_charging: false
            }
        );
        assert_eq!(
            normalize_battery_state(110, 100, true)
                .unwrap()
                .level_percent,
            100
        );
        assert_eq!(
            normalize_battery_state(1, 0, false),
            Err(ObserverErrorCode::InvalidValue)
        );
    }

    #[test]
    fn opt_out_denies_active_app_before_platform_observation() {
        let result = SystemDesktopMonitor::get_active_application(false);
        assert_eq!(
            result.diagnostic(ObserverKind::ActiveApplication).status,
            ObserverStatus::Denied
        );
    }

    #[test]
    fn window_titles_are_always_unsupported_in_v1_even_if_legacy_setting_is_true() {
        let result = SystemDesktopMonitor::get_window_title(true);
        assert_eq!(
            result.diagnostic(ObserverKind::WindowTitle).status,
            ObserverStatus::Unsupported
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unsupported_platform_never_fabricates_observer_values() {
        assert!(SystemDesktopMonitor::get_idle_time()
            .into_available()
            .is_none());
        assert!(SystemDesktopMonitor::get_battery_state()
            .into_available()
            .is_none());
        assert!(SystemDesktopMonitor::get_active_application(true)
            .into_available()
            .is_none());
    }
}
