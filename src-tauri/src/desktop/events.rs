use crate::character::ambient::{AmbientEvent, AmbientEventCategory};
use crate::desktop::observation::{
    ActiveApplicationObservation, BatteryObservation, IdleObservation, PowerEvent,
};
use chrono::{DateTime, Duration, Utc};
use ring::digest::{digest, SHA256};
use std::collections::VecDeque;

const APP_SWITCH_WINDOW_SECONDS: i64 = 120;
const MAX_RETAINED_APP_SWITCHES: usize = 16;
const MAX_APPLICATION_NAME_CHARS: usize = 80;
const IDLE_BUCKET_SECONDS: u64 = 300;
const MAX_IDLE_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopEvent {
    IdleTime {
        seconds: u64,
    },
    AppSwitched {
        application: String,
    },
    AppSwitchPattern {
        switch_count: u32,
        window_seconds: u32,
    },
    BatteryState {
        level: u8,
        is_charging: bool,
    },
    PowerState(PowerEvent),
}

#[derive(Debug, Clone, Default)]
pub struct DesktopEventSummarizer {
    recent_app_switches: VecDeque<DateTime<Utc>>,
    last_application_fingerprint: Option<[u8; 32]>,
    last_battery_level: Option<u8>,
    last_idle_bucket: Option<u64>,
}

fn normalize_application_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_APPLICATION_NAME_CHARS)
        .collect()
}

fn application_fingerprint(name: &str) -> [u8; 32] {
    let hash = digest(&SHA256, name.as_bytes());
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(hash.as_ref());
    bytes
}

impl DesktopEventSummarizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_idle(&mut self, observation: IdleObservation) -> Option<DesktopEvent> {
        let seconds = observation.seconds.min(MAX_IDLE_SECONDS);
        let bucket = seconds / IDLE_BUCKET_SECONDS * IDLE_BUCKET_SECONDS;
        if bucket < IDLE_BUCKET_SECONDS || self.last_idle_bucket == Some(bucket) {
            return None;
        }
        self.last_idle_bucket = Some(bucket);
        Some(DesktopEvent::IdleTime { seconds: bucket })
    }

    pub fn record_app_switch(
        &mut self,
        observation: ActiveApplicationObservation,
    ) -> Option<DesktopEvent> {
        self.record_app_switch_at(Utc::now(), observation)
    }

    pub(crate) fn record_app_switch_at(
        &mut self,
        now: DateTime<Utc>,
        observation: ActiveApplicationObservation,
    ) -> Option<DesktopEvent> {
        let application = normalize_application_name(&observation.name);
        if application.is_empty() {
            return None;
        }
        let fingerprint = application_fingerprint(&application);
        if self.last_application_fingerprint == Some(fingerprint) {
            return None;
        }
        let was_initialized = self.last_application_fingerprint.is_some();
        self.last_application_fingerprint = Some(fingerprint);
        if !was_initialized {
            return None;
        }

        let cutoff = now - Duration::seconds(APP_SWITCH_WINDOW_SECONDS);
        self.recent_app_switches
            .retain(|timestamp| *timestamp > cutoff);
        self.recent_app_switches.push_back(now);
        while self.recent_app_switches.len() > MAX_RETAINED_APP_SWITCHES {
            self.recent_app_switches.pop_front();
        }

        if self.recent_app_switches.len() >= 6 {
            let switch_count = u32::try_from(self.recent_app_switches.len()).unwrap_or(u32::MAX);
            self.recent_app_switches.clear();
            return Some(DesktopEvent::AppSwitchPattern {
                switch_count,
                window_seconds: APP_SWITCH_WINDOW_SECONDS as u32,
            });
        }

        Some(DesktopEvent::AppSwitched { application })
    }

    pub fn record_battery(&mut self, observation: BatteryObservation) -> Option<DesktopEvent> {
        let level = observation.level_percent.min(100);
        let event = self.last_battery_level.and_then(|last| {
            ((last > 20 && level <= 20) || (last > 10 && level <= 10)).then_some(
                DesktopEvent::BatteryState {
                    level,
                    is_charging: observation.is_charging,
                },
            )
        });
        self.last_battery_level = Some(level);
        event
    }

    pub fn record_power(&mut self, event: PowerEvent) -> DesktopEvent {
        DesktopEvent::PowerState(event)
    }

    pub fn to_ambient_event(event: DesktopEvent) -> AmbientEvent {
        let (category, summary, importance) = match event {
            DesktopEvent::IdleTime { seconds } => (
                AmbientEventCategory::Idle,
                format!("User has been idle for about {} minutes", seconds / 60),
                0.4,
            ),
            DesktopEvent::AppSwitched { application } => (
                AmbientEventCategory::Application,
                format!("Active application changed to {application}"),
                0.3,
            ),
            DesktopEvent::AppSwitchPattern {
                switch_count,
                window_seconds,
            } => (
                AmbientEventCategory::Application,
                format!(
                    "User changed active applications repeatedly ({switch_count} times in {window_seconds} seconds)"
                ),
                0.65,
            ),
            DesktopEvent::BatteryState { level, is_charging } => {
                let summary = if is_charging {
                    format!("Battery reached {level}% while charging")
                } else {
                    format!("Battery is low at {level}%")
                };
                (AmbientEventCategory::Power, summary, 0.8)
            }
            DesktopEvent::PowerState(PowerEvent::Sleep) => (
                AmbientEventCategory::Power,
                "Computer is going to sleep".to_string(),
                0.5,
            ),
            DesktopEvent::PowerState(PowerEvent::Wake) => (
                AmbientEventCategory::Wake,
                "Computer just woke from sleep".to_string(),
                0.6,
            ),
        };
        AmbientEvent {
            category,
            summary,
            importance,
        }
    }

    #[cfg(test)]
    fn retained_app_switch_count(&self) -> usize {
        self.recent_app_switches.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn app(name: &str) -> ActiveApplicationObservation {
        ActiveApplicationObservation {
            name: name.to_string(),
        }
    }

    #[test]
    fn first_active_app_seeds_state_without_generating_a_remark() {
        let mut summarizer = DesktopEventSummarizer::new();
        assert!(summarizer.record_app_switch(app("Terminal")).is_none());
        assert!(summarizer.record_app_switch(app("Terminal")).is_none());
    }

    #[test]
    fn rapid_switch_pattern_does_not_retain_or_repeat_historical_app_names() {
        let mut summarizer = DesktopEventSummarizer::new();
        let start = Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap();
        assert!(summarizer
            .record_app_switch_at(start, app("Secret Project Alpha"))
            .is_none());

        let mut final_event = None;
        for index in 1..=6 {
            let name = if index % 2 == 0 {
                "Terminal"
            } else {
                "Secret Project Beta"
            };
            final_event = summarizer
                .record_app_switch_at(start + Duration::seconds(i64::from(index)), app(name));
        }

        let ambient = DesktopEventSummarizer::to_ambient_event(final_event.unwrap());
        assert!(ambient
            .summary
            .contains("changed active applications repeatedly"));
        assert!(!ambient.summary.contains("Secret Project"));
        assert_eq!(summarizer.retained_app_switch_count(), 0);
    }

    #[test]
    fn idle_events_are_bounded_and_emit_once_per_five_minute_bucket() {
        let mut summarizer = DesktopEventSummarizer::new();
        assert!(summarizer
            .record_idle(IdleObservation { seconds: 299 })
            .is_none());
        let first = summarizer
            .record_idle(IdleObservation { seconds: 301 })
            .unwrap();
        assert_eq!(first, DesktopEvent::IdleTime { seconds: 300 });
        assert!(summarizer
            .record_idle(IdleObservation { seconds: 599 })
            .is_none());
        assert_eq!(
            summarizer
                .record_idle(IdleObservation { seconds: u64::MAX })
                .unwrap(),
            DesktopEvent::IdleTime {
                seconds: MAX_IDLE_SECONDS
            }
        );
    }

    #[test]
    fn battery_only_emits_on_major_downward_threshold_crossings() {
        let mut summarizer = DesktopEventSummarizer::new();
        assert!(summarizer
            .record_battery(BatteryObservation {
                level_percent: 80,
                is_charging: false,
            })
            .is_none());
        assert!(summarizer
            .record_battery(BatteryObservation {
                level_percent: 21,
                is_charging: false,
            })
            .is_none());
        assert_eq!(
            summarizer
                .record_battery(BatteryObservation {
                    level_percent: 20,
                    is_charging: false,
                })
                .unwrap(),
            DesktopEvent::BatteryState {
                level: 20,
                is_charging: false
            }
        );
    }
}
