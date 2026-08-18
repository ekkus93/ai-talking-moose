use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum DesktopEvent {
    IdleTime {
        seconds: u64,
    },
    AppSwitched {
        from: String,
        to: String,
    },
    AppSwitchPattern {
        apps: Vec<String>,
        switch_count: u32,
        window_seconds: u32,
    },
    BatteryState {
        level: u8,
        is_charging: bool,
    },
    PowerState {
        is_sleeping: bool,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DesktopEventSummarizer {
    recent_app_switches: Vec<(DateTime<Utc>, String)>,
    last_battery_level: Option<u8>,
}

impl DesktopEventSummarizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_app_switch(&mut self, app_name: String) -> Option<DesktopEvent> {
        let now = Utc::now();
        self.recent_app_switches.push((now, app_name.clone()));

        // Keep last 2 minutes
        let cutoff = now - chrono::Duration::seconds(120);
        self.recent_app_switches.retain(|(t, _)| *t > cutoff);

        if self.recent_app_switches.len() >= 6 {
            let mut apps: Vec<String> = self
                .recent_app_switches
                .iter()
                .map(|(_, a)| a.clone())
                .collect();
            apps.dedup();

            if apps.len() >= 2 {
                let count = self.recent_app_switches.len() as u32;
                self.recent_app_switches.clear(); // Reset after generating summary
                return Some(DesktopEvent::AppSwitchPattern {
                    apps,
                    switch_count: count,
                    window_seconds: 120,
                });
            }
        }

        None
    }

    pub fn record_battery(&mut self, level: u8, is_charging: bool) -> Option<DesktopEvent> {
        if let Some(last) = self.last_battery_level {
            // Only trigger on major thresholds (e.g. dropping to 20% or 10%)
            if (last > 20 && level <= 20) || (last > 10 && level <= 10) {
                self.last_battery_level = Some(level);
                return Some(DesktopEvent::BatteryState { level, is_charging });
            }
        }
        self.last_battery_level = Some(level);
        None
    }

    pub fn to_semantic_summary(event: &DesktopEvent) -> (String, f32) {
        match event {
            DesktopEvent::IdleTime { seconds } => {
                let mins = seconds / 60;
                (format!("User has been idle for {} minutes", mins), 0.4)
            }
            DesktopEvent::AppSwitched { from, to } => {
                (format!("Switched from {} to {}", from, to), 0.2)
            }
            DesktopEvent::AppSwitchPattern {
                apps,
                switch_count,
                window_seconds,
            } => (
                format!(
                    "User switched rapidly between {} ({} times in {} seconds)",
                    apps.join(" and "),
                    switch_count,
                    window_seconds
                ),
                0.7,
            ),
            DesktopEvent::BatteryState { level, is_charging } => {
                if *is_charging {
                    (format!("Battery is charging at {}%", level), 0.3)
                } else {
                    (format!("Battery is low at {}%", level), 0.8)
                }
            }
            DesktopEvent::PowerState { is_sleeping } => {
                if *is_sleeping {
                    ("Computer is going to sleep".to_string(), 0.5)
                } else {
                    ("Computer just woke up from sleep".to_string(), 0.6)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_switch_pattern_summarizer() {
        let mut summarizer = DesktopEventSummarizer::new();
        let mut event = None;
        for _ in 0..3 {
            summarizer.record_app_switch("Terminal".to_string());
            event = summarizer.record_app_switch("VSCode".to_string());
        }

        assert!(event.is_some());
        if let Some(DesktopEvent::AppSwitchPattern {
            apps, switch_count, ..
        }) = event
        {
            assert!(apps.contains(&"Terminal".to_string()));
            assert!(apps.contains(&"VSCode".to_string()));
            assert!(switch_count >= 6);
        }
    }
}
