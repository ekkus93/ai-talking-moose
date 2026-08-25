use chrono::{DateTime, Duration, Local, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};

pub const DISMISSAL_COOLDOWN_SECONDS: i64 = 180;
pub const EVENT_DEDUP_WINDOW_SECONDS: i64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientCooldownBlockReason {
    QuietHours,
    AnnoyanceBudget,
    DismissalCooldown,
    DuplicateEvent,
    Cooldown,
    HourlyLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnoyanceBudget {
    pub score: f32,
    pub threshold: f32,
    pub decay_per_minute: f32,
    #[serde(skip, default = "Utc::now")]
    pub last_decay_check: DateTime<Utc>,
}

impl Default for AnnoyanceBudget {
    fn default() -> Self {
        Self {
            score: 0.0,
            threshold: 60.0,
            decay_per_minute: 5.0,
            last_decay_check: Utc::now(),
        }
    }
}

impl AnnoyanceBudget {
    pub fn update_decay(&mut self, now: DateTime<Utc>) {
        let elapsed_seconds = (now - self.last_decay_check).num_seconds();
        if elapsed_seconds <= 0 {
            return;
        }
        let decay_amount = (elapsed_seconds as f32 / 60.0) * self.decay_per_minute;
        self.score = (self.score - decay_amount).max(0.0);
        self.last_decay_check = now;
    }

    pub fn record_unsolicited_speech_at(&mut self, now: DateTime<Utc>) {
        self.update_decay(now);
        self.score = (self.score + 15.0).min(100.0);
    }

    pub fn record_unsolicited_speech(&mut self) {
        self.record_unsolicited_speech_at(Utc::now());
    }

    pub fn record_interruption_at(&mut self, now: DateTime<Utc>) {
        self.update_decay(now);
        self.score = (self.score + 20.0).min(100.0);
    }

    pub fn record_interruption(&mut self) {
        self.record_interruption_at(Utc::now());
    }

    pub fn record_dismissal_at(&mut self, now: DateTime<Utc>) {
        self.update_decay(now);
        self.score = (self.score + 35.0).min(100.0);
    }

    pub fn record_dismissal(&mut self) {
        self.record_dismissal_at(Utc::now());
    }

    pub fn is_suppressed_at(&mut self, now: DateTime<Utc>) -> bool {
        self.update_decay(now);
        self.score >= self.threshold
    }

    pub fn is_suppressed(&mut self) -> bool {
        self.is_suppressed_at(Utc::now())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CooldownTracker {
    pub last_speech_time: Option<DateTime<Utc>>,
    pub last_dismissal_time: Option<DateTime<Utc>>,
    pub speech_timestamps: Vec<DateTime<Utc>>,
    pub annoyance_budget: AnnoyanceBudget,
    recent_event_fingerprints: Vec<(String, DateTime<Utc>)>,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_speech(&mut self, now: DateTime<Utc>) {
        self.record_ambient_speech(now, "");
    }

    pub fn record_ambient_speech(&mut self, now: DateTime<Utc>, fingerprint: &str) {
        self.last_speech_time = Some(now);
        self.speech_timestamps.push(now);
        if !fingerprint.is_empty() {
            self.recent_event_fingerprints
                .push((fingerprint.to_string(), now));
        }
        self.annoyance_budget.record_unsolicited_speech_at(now);
        self.prune_old_timestamps(now);
        self.prune_old_fingerprints(now);
    }

    pub fn record_dismissal(&mut self, now: DateTime<Utc>) {
        self.last_dismissal_time = Some(now);
        self.annoyance_budget.record_dismissal_at(now);
    }

    pub fn record_interruption(&mut self) {
        self.annoyance_budget.record_interruption();
    }

    pub fn record_interruption_at(&mut self, now: DateTime<Utc>) {
        self.annoyance_budget.record_interruption_at(now);
    }

    fn prune_old_timestamps(&mut self, now: DateTime<Utc>) {
        let one_hour_ago = now - Duration::hours(1);
        self.speech_timestamps.retain(|&ts| ts > one_hour_ago);
    }

    fn prune_old_fingerprints(&mut self, now: DateTime<Utc>) {
        let cutoff = now - Duration::seconds(EVENT_DEDUP_WINDOW_SECONDS);
        self.recent_event_fingerprints
            .retain(|(_, timestamp)| *timestamp > cutoff);
    }

    pub fn clear_event_fingerprints(&mut self) {
        self.recent_event_fingerprints.clear();
    }

    fn is_duplicate_event(&mut self, now: DateTime<Utc>, fingerprint: &str) -> bool {
        if fingerprint.is_empty() {
            return false;
        }
        self.prune_old_fingerprints(now);
        self.recent_event_fingerprints
            .iter()
            .any(|(recent, _)| recent == fingerprint)
    }

    pub fn is_in_quiet_hours<Tz: TimeZone>(
        now: &DateTime<Tz>,
        start_hour: u8,
        end_hour: u8,
    ) -> bool {
        let current_hour = now.hour() as u8;
        if start_hour == end_hour {
            return false;
        }
        if start_hour < end_hour {
            current_hour >= start_hour && current_hour < end_hour
        } else {
            current_hour >= start_hour || current_hour < end_hour
        }
    }

    pub fn check_ambient_gate(
        &mut self,
        now: DateTime<Utc>,
        min_cooldown_seconds: u64,
        max_comments_per_hour: u32,
        quiet_hours: (bool, u8, u8),
        fingerprint: Option<&str>,
    ) -> Result<(), AmbientCooldownBlockReason> {
        let (quiet_hours_enabled, quiet_hours_start, quiet_hours_end) = quiet_hours;
        let local_now = now.with_timezone(&Local);
        if quiet_hours_enabled
            && Self::is_in_quiet_hours(&local_now, quiet_hours_start, quiet_hours_end)
        {
            return Err(AmbientCooldownBlockReason::QuietHours);
        }

        if self.annoyance_budget.is_suppressed_at(now) {
            return Err(AmbientCooldownBlockReason::AnnoyanceBudget);
        }

        if let Some(dismissed) = self.last_dismissal_time {
            if now - dismissed < Duration::seconds(DISMISSAL_COOLDOWN_SECONDS) {
                return Err(AmbientCooldownBlockReason::DismissalCooldown);
            }
        }

        if fingerprint.is_some_and(|value| self.is_duplicate_event(now, value)) {
            return Err(AmbientCooldownBlockReason::DuplicateEvent);
        }

        if let Some(last_speech) = self.last_speech_time {
            if now - last_speech < Duration::seconds(min_cooldown_seconds as i64) {
                return Err(AmbientCooldownBlockReason::Cooldown);
            }
        }

        self.prune_old_timestamps(now);
        if self.speech_timestamps.len() >= max_comments_per_hour as usize {
            return Err(AmbientCooldownBlockReason::HourlyLimit);
        }

        Ok(())
    }

    pub fn can_speak_ambient(
        &mut self,
        now: DateTime<Utc>,
        min_cooldown_seconds: u64,
        max_comments_per_hour: u32,
        quiet_hours_enabled: bool,
        quiet_hours_start: u8,
        quiet_hours_end: u8,
    ) -> bool {
        self.check_ambient_gate(
            now,
            min_cooldown_seconds,
            max_comments_per_hour,
            (quiet_hours_enabled, quiet_hours_start, quiet_hours_end),
            None,
        )
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annoyance_budget_and_decay() {
        let mut budget = AnnoyanceBudget::default();
        let now = Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap();
        budget.last_decay_check = now;
        assert!(!budget.is_suppressed_at(now));

        budget.record_dismissal_at(now);
        budget.record_dismissal_at(now);
        assert!(budget.is_suppressed_at(now));

        let future = now + Duration::minutes(15);
        budget.update_decay(future);
        assert!(!budget.is_suppressed_at(future));
        assert_eq!(budget.score, 0.0);
    }

    #[test]
    fn backward_clock_step_preserves_decay_checkpoint_and_future_credit() {
        let checkpoint = Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap();
        let mut budget = AnnoyanceBudget {
            score: 60.0,
            threshold: 100.0,
            decay_per_minute: 5.0,
            last_decay_check: checkpoint,
        };

        budget.update_decay(checkpoint - Duration::minutes(5));
        assert_eq!(budget.score, 60.0);
        assert_eq!(budget.last_decay_check, checkpoint);

        budget.update_decay(checkpoint + Duration::minutes(1));
        assert_eq!(budget.score, 55.0);
        assert_eq!(budget.last_decay_check, checkpoint + Duration::minutes(1));
    }

    #[test]
    fn speech_interruption_and_dismissal_share_one_recovering_annoyance_budget() {
        let mut tracker = CooldownTracker::new();
        let now = Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap();
        tracker.annoyance_budget.last_decay_check = now;

        tracker.record_speech(now);
        assert_eq!(tracker.annoyance_budget.score, 15.0);
        tracker.record_interruption_at(now);
        assert_eq!(tracker.annoyance_budget.score, 35.0);
        tracker.record_dismissal(now);
        assert_eq!(tracker.annoyance_budget.score, 70.0);
        assert!(tracker.annoyance_budget.is_suppressed_at(now));

        let recovered = now + Duration::minutes(15);
        assert!(!tracker.annoyance_budget.is_suppressed_at(recovered));
        assert_eq!(tracker.annoyance_budget.score, 0.0);
    }

    #[test]
    fn test_quiet_hours_normal_and_overnight_ranges() {
        use chrono::FixedOffset;

        let zone = FixedOffset::east_opt(0).unwrap();
        let late = zone.with_ymd_and_hms(2026, 8, 17, 23, 30, 0).unwrap();
        let day = zone.with_ymd_and_hms(2026, 8, 17, 14, 0, 0).unwrap();
        let early = zone.with_ymd_and_hms(2026, 8, 17, 2, 0, 0).unwrap();

        assert!(CooldownTracker::is_in_quiet_hours(&late, 22, 8));
        assert!(!CooldownTracker::is_in_quiet_hours(&day, 22, 8));
        assert!(CooldownTracker::is_in_quiet_hours(&early, 1, 6));
        assert!(!CooldownTracker::is_in_quiet_hours(&day, 1, 6));
    }

    #[test]
    fn test_quiet_hours_boundaries() {
        use chrono::FixedOffset;

        let zone = FixedOffset::east_opt(0).unwrap();
        let start = zone.with_ymd_and_hms(2026, 8, 17, 22, 0, 0).unwrap();
        let end = zone.with_ymd_and_hms(2026, 8, 18, 8, 0, 0).unwrap();
        let same = zone.with_ymd_and_hms(2026, 8, 17, 4, 0, 0).unwrap();

        assert!(CooldownTracker::is_in_quiet_hours(&start, 22, 8));
        assert!(!CooldownTracker::is_in_quiet_hours(&end, 22, 8));
        assert!(!CooldownTracker::is_in_quiet_hours(&same, 4, 4));
    }

    #[test]
    fn test_quiet_hours_with_pacific_offset_independent_of_host_timezone() {
        use chrono::FixedOffset;

        let utc = Utc.with_ymd_and_hms(2026, 8, 18, 5, 30, 0).unwrap();
        let pacific_summer = FixedOffset::west_opt(7 * 60 * 60).unwrap();
        let pacific_local = utc.with_timezone(&pacific_summer);

        assert_eq!(pacific_local.hour(), 22);
        assert!(CooldownTracker::is_in_quiet_hours(&pacific_local, 22, 8));
    }

    #[test]
    fn event_fingerprint_history_can_be_cleared_for_privacy_reset() {
        let mut tracker = CooldownTracker::new();
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 12, 0, 0).unwrap();
        tracker.record_ambient_speech(now, "private-derived-fingerprint");

        assert_eq!(
            tracker.check_ambient_gate(
                now,
                0,
                12,
                (false, 22, 8),
                Some("private-derived-fingerprint"),
            ),
            Err(AmbientCooldownBlockReason::DuplicateEvent)
        );

        tracker.clear_event_fingerprints();

        assert!(tracker
            .check_ambient_gate(
                now,
                0,
                12,
                (false, 22, 8),
                Some("private-derived-fingerprint"),
            )
            .is_ok());
    }

    #[test]
    fn test_cooldown_tracker() {
        let mut tracker = CooldownTracker::new();
        let t0 = Utc.with_ymd_and_hms(2026, 8, 17, 12, 0, 0).unwrap();

        assert!(tracker.can_speak_ambient(t0, 300, 3, false, 22, 8));
        tracker.record_speech(t0);

        let t1 = t0 + Duration::seconds(60);
        assert!(!tracker.can_speak_ambient(t1, 300, 3, false, 22, 8));

        let t2 = t0 + Duration::seconds(350);
        assert!(tracker.can_speak_ambient(t2, 300, 3, false, 22, 8));
    }

    #[test]
    fn duplicate_fingerprint_expires_at_the_dedup_boundary() {
        let mut tracker = CooldownTracker::new();
        tracker.annoyance_budget.threshold = 101.0;
        let now = Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap();
        tracker.record_ambient_speech(now, "fingerprint");

        assert_eq!(
            tracker.check_ambient_gate(
                now + Duration::seconds(EVENT_DEDUP_WINDOW_SECONDS - 1),
                0,
                12,
                (false, 22, 8),
                Some("fingerprint"),
            ),
            Err(AmbientCooldownBlockReason::DuplicateEvent)
        );
        assert!(tracker
            .check_ambient_gate(
                now + Duration::seconds(EVENT_DEDUP_WINDOW_SECONDS),
                0,
                12,
                (false, 22, 8),
                Some("fingerprint"),
            )
            .is_ok());
    }
}
