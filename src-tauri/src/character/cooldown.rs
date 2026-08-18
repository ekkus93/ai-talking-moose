use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};

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
        let elapsed_seconds = (now - self.last_decay_check).num_seconds().max(0);
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
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_speech(&mut self, now: DateTime<Utc>) {
        self.last_speech_time = Some(now);
        self.speech_timestamps.push(now);
        self.annoyance_budget.record_unsolicited_speech_at(now);
        self.prune_old_timestamps(now);
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

    pub fn is_in_quiet_hours(now: DateTime<Utc>, start_hour: u8, end_hour: u8) -> bool {
        let current_hour = now.hour() as u8;
        if start_hour == end_hour {
            return false;
        }
        if start_hour < end_hour {
            // e.g. 1 to 6 (1 AM to 6 AM)
            current_hour >= start_hour && current_hour < end_hour
        } else {
            // Overnight: e.g. 22 to 8 (10 PM to 8 AM)
            current_hour >= start_hour || current_hour < end_hour
        }
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
        // Check quiet hours
        if quiet_hours_enabled && Self::is_in_quiet_hours(now, quiet_hours_start, quiet_hours_end) {
            return false;
        }

        // Check annoyance budget
        if self.annoyance_budget.is_suppressed_at(now) {
            return false;
        }

        // Check recent dismissal cooldown (at least 3 minutes)
        if let Some(dismissed) = self.last_dismissal_time {
            if now - dismissed < Duration::seconds(180) {
                return false;
            }
        }

        // Check minimum inter-comment cooldown
        if let Some(last_speech) = self.last_speech_time {
            if now - last_speech < Duration::seconds(min_cooldown_seconds as i64) {
                return false;
            }
        }

        // Check hourly limit
        self.prune_old_timestamps(now);
        if self.speech_timestamps.len() >= max_comments_per_hour as usize {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annoyance_budget_and_decay() {
        let mut budget = AnnoyanceBudget::default();
        assert!(!budget.is_suppressed());

        budget.record_dismissal(); // +35
        budget.record_dismissal(); // +35 -> 70 (>= 60 threshold)
        assert!(budget.is_suppressed());

        // Fast forward 15 minutes (should decay 15 * 5 = 75 points)
        let future = budget.last_decay_check + Duration::minutes(15);
        budget.update_decay(future);
        assert!(!budget.is_suppressed());
        assert_eq!(budget.score, 0.0);
    }

    #[test]
    fn test_quiet_hours() {
        use chrono::TimeZone;
        let dt = Utc.with_ymd_and_hms(2026, 8, 17, 23, 30, 0).unwrap(); // 11:30 PM
        assert!(CooldownTracker::is_in_quiet_hours(dt, 22, 8));

        let dt_day = Utc.with_ymd_and_hms(2026, 8, 17, 14, 0, 0).unwrap(); // 2:00 PM
        assert!(!CooldownTracker::is_in_quiet_hours(dt_day, 22, 8));
    }

    #[test]
    fn test_cooldown_tracker() {
        use chrono::TimeZone;
        let mut tracker = CooldownTracker::new();
        let t0 = Utc.with_ymd_and_hms(2026, 8, 17, 12, 0, 0).unwrap();

        assert!(tracker.can_speak_ambient(t0, 300, 3, false, 22, 8));
        tracker.record_speech(t0);

        // 60 seconds later (under 300s cooldown) -> false
        let t1 = t0 + Duration::seconds(60);
        assert!(!tracker.can_speak_ambient(t1, 300, 3, false, 22, 8));

        // 350 seconds later -> true
        let t2 = t0 + Duration::seconds(350);
        assert!(tracker.can_speak_ambient(t2, 300, 3, false, 22, 8));
    }
}
