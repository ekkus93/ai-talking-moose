use chrono::{FixedOffset, TimeZone, Timelike, Utc};
use talking_moose_lib::character::CooldownTracker;

#[test]
fn pacific_spring_forward_uses_local_wall_clock_boundaries() {
    let standard = FixedOffset::west_opt(8 * 60 * 60).unwrap();
    let daylight = FixedOffset::west_opt(7 * 60 * 60).unwrap();

    let before_jump = Utc
        .with_ymd_and_hms(2026, 3, 8, 9, 30, 0)
        .unwrap()
        .with_timezone(&standard);
    let after_jump = Utc
        .with_ymd_and_hms(2026, 3, 8, 10, 30, 0)
        .unwrap()
        .with_timezone(&daylight);

    assert_eq!(before_jump.hour(), 1);
    assert_eq!(after_jump.hour(), 3);
    assert!(CooldownTracker::is_in_quiet_hours(&before_jump, 22, 3));
    assert!(!CooldownTracker::is_in_quiet_hours(&after_jump, 22, 3));
}

#[test]
fn pacific_fall_back_repeated_hour_has_identical_quiet_hour_policy() {
    let daylight = FixedOffset::west_opt(7 * 60 * 60).unwrap();
    let standard = FixedOffset::west_opt(8 * 60 * 60).unwrap();

    let first_one_thirty = Utc
        .with_ymd_and_hms(2026, 11, 1, 8, 30, 0)
        .unwrap()
        .with_timezone(&daylight);
    let second_one_thirty = Utc
        .with_ymd_and_hms(2026, 11, 1, 9, 30, 0)
        .unwrap()
        .with_timezone(&standard);

    assert_eq!(first_one_thirty.hour(), 1);
    assert_eq!(second_one_thirty.hour(), 1);
    assert!(CooldownTracker::is_in_quiet_hours(&first_one_thirty, 22, 8));
    assert!(CooldownTracker::is_in_quiet_hours(&second_one_thirty, 22, 8));
}
