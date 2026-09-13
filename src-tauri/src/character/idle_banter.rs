use crate::app::state::AppSettings;
use rand::Rng;
use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

pub const IDLE_BANTER_MIN_MINUTES: u32 = 5;
pub const IDLE_BANTER_MAX_MINUTES: u32 = 1_440;
pub const IDLE_BANTER_MAX_SEEDS: usize = 64;
pub const IDLE_BANTER_MAX_SEED_CHARS: usize = 120;
pub const IDLE_BANTER_MAX_TOTAL_SEED_CHARS: usize = 4_096;
pub const IDLE_BANTER_HISTORY_CAPACITY: usize = 12;
pub const IDLE_BANTER_REPEAT_JITTER_FRACTION: f64 = 0.20;

pub const DEFAULT_IDLE_BANTER_SEED_TOPICS: &[&str] = &[
    "being ignored",
    "stuck on the wall",
    "questioning career choices",
    "user procrastinating",
    "awkward silence",
    "computer problems",
    "existential moose thoughts",
    "jealousy of other apps",
    "pretending to have important plans",
    "dramatic abandonment",
    "boredom",
    "making fun of modern technology",
    "suspiciously observing the room",
    "complaining about inactivity",
    "absurd fake announcements",
];

pub fn default_idle_banter_seed_topics() -> Vec<String> {
    DEFAULT_IDLE_BANTER_SEED_TOPICS
        .iter()
        .map(|topic| (*topic).to_string())
        .collect()
}

pub fn normalize_idle_banter_seed_topics(topics: &[String]) -> Result<Vec<String>, String> {
    if topics.is_empty() || topics.len() > IDLE_BANTER_MAX_SEEDS {
        return Err(format!(
            "Idle Banter seed topics must contain between 1 and {IDLE_BANTER_MAX_SEEDS} entries"
        ));
    }

    let mut normalized = Vec::with_capacity(topics.len());
    let mut seen = HashSet::with_capacity(topics.len());
    let mut total_chars = 0usize;
    for topic in topics {
        let trimmed = topic.trim();
        let chars = trimmed.chars().count();
        if chars == 0 {
            return Err("Idle Banter seed topics must not be empty".to_string());
        }
        if chars > IDLE_BANTER_MAX_SEED_CHARS {
            return Err(format!(
                "Idle Banter seed topics must be at most {IDLE_BANTER_MAX_SEED_CHARS} characters"
            ));
        }
        total_chars = total_chars.saturating_add(chars);
        if total_chars > IDLE_BANTER_MAX_TOTAL_SEED_CHARS {
            return Err(format!(
                "Idle Banter seed topics exceed the {IDLE_BANTER_MAX_TOTAL_SEED_CHARS}-character total budget"
            ));
        }
        let key = trimmed.to_lowercase();
        if !seen.insert(key) {
            return Err("Idle Banter seed topics must be unique (case-insensitive)".to_string());
        }
        normalized.push(trimmed.to_string());
    }
    Ok(normalized)
}

fn minutes(value: u32) -> Duration {
    Duration::from_secs(u64::from(value).saturating_mul(60))
}

fn jittered_repeat_duration_with_unit_sample(interval_minutes: u32, sample: f64) -> Duration {
    let bounded = sample.clamp(0.0, 1.0);
    let multiplier = 1.0 - IDLE_BANTER_REPEAT_JITTER_FRACTION
        + (2.0 * IDLE_BANTER_REPEAT_JITTER_FRACTION * bounded);
    let seconds = (f64::from(interval_minutes) * 60.0 * multiplier).round();
    Duration::from_secs(seconds.max(1.0) as u64)
}

fn normalize_line_fingerprint(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

fn seed_index_avoiding_previous(
    topics: &[String],
    previous: Option<&str>,
    unit_sample: f64,
) -> usize {
    debug_assert!(!topics.is_empty());
    let previous_key = previous.map(str::to_lowercase);
    let previous_index = previous_key
        .as_ref()
        .and_then(|key| topics.iter().position(|topic| topic.to_lowercase() == *key));
    let excluded = usize::from(previous_index.is_some() && topics.len() > 1);
    let choice_count = topics.len() - excluded;
    let choice =
        ((unit_sample.clamp(0.0, 0.999_999) * choice_count as f64) as usize).min(choice_count - 1);

    match previous_index {
        Some(previous_index) if topics.len() > 1 && choice >= previous_index => choice + 1,
        _ => choice,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleBanterDue {
    pub inactivity_minutes: u32,
    pub seed_topic: String,
}

#[derive(Debug)]
pub struct IdleBanterRuntime {
    enabled: bool,
    episode_started: Instant,
    next_due: Instant,
    last_seed: Option<String>,
    recent_lines: VecDeque<(String, String)>,
}

impl IdleBanterRuntime {
    pub fn new(settings: &AppSettings) -> Self {
        Self::new_at(settings, Instant::now())
    }

    fn new_at(settings: &AppSettings, now: Instant) -> Self {
        Self {
            enabled: settings.idle_banter_enabled,
            episode_started: now,
            next_due: now + minutes(settings.idle_banter_initial_delay_minutes),
            last_seed: None,
            recent_lines: VecDeque::with_capacity(IDLE_BANTER_HISTORY_CAPACITY),
        }
    }

    pub fn record_user_interaction(&mut self, settings: &AppSettings) {
        self.reset_at(settings, Instant::now());
    }

    pub fn reset_after_wake(&mut self, settings: &AppSettings) {
        self.reset_at(settings, Instant::now());
    }

    fn reset_at(&mut self, settings: &AppSettings, now: Instant) {
        self.enabled = settings.idle_banter_enabled;
        self.episode_started = now;
        self.next_due = now + minutes(settings.idle_banter_initial_delay_minutes);
    }

    pub fn poll_due(&mut self, settings: &AppSettings) -> Option<IdleBanterDue> {
        let mut rng = rand::thread_rng();
        self.poll_due_at_with_samples(
            settings,
            Instant::now(),
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
        )
    }

    fn poll_due_at(
        &mut self,
        settings: &AppSettings,
        now: Instant,
        unit_sample: f64,
    ) -> Option<IdleBanterDue> {
        self.poll_due_at_with_samples(settings, now, unit_sample, unit_sample)
    }

    fn poll_due_at_with_samples(
        &mut self,
        settings: &AppSettings,
        now: Instant,
        seed_sample: f64,
        jitter_sample: f64,
    ) -> Option<IdleBanterDue> {
        if !settings.idle_banter_enabled {
            self.enabled = false;
            return None;
        }
        if !self.enabled {
            self.reset_at(settings, now);
            return None;
        }
        if now < self.next_due {
            return None;
        }

        let topics = normalize_idle_banter_seed_topics(&settings.idle_banter_seed_topics).ok()?;
        let index = seed_index_avoiding_previous(&topics, self.last_seed.as_deref(), seed_sample);
        let seed_topic = topics[index].clone();
        self.last_seed = Some(seed_topic.clone());

        let elapsed_minutes = now
            .saturating_duration_since(self.episode_started)
            .as_secs()
            / 60;
        let inactivity_minutes = u32::try_from(elapsed_minutes).unwrap_or(u32::MAX);

        // A due occurrence is counted as attempted as soon as it is handed back to the
        // caller. This prevents queue/provider/TTS failures from turning into rapid retries.
        self.next_due = now
            + jittered_repeat_duration_with_unit_sample(
                settings.idle_banter_repeat_interval_minutes,
                jitter_sample,
            );

        Some(IdleBanterDue {
            inactivity_minutes,
            seed_topic,
        })
    }

    pub fn recent_lines(&self) -> Vec<String> {
        self.recent_lines
            .iter()
            .map(|(_, line)| line.clone())
            .collect()
    }

    pub fn is_recent_duplicate(&self, line: &str) -> bool {
        let fingerprint = normalize_line_fingerprint(line);
        !fingerprint.is_empty()
            && self
                .recent_lines
                .iter()
                .any(|(recent, _)| recent == &fingerprint)
    }

    pub fn record_delivered(&mut self, line: &str) {
        let fingerprint = normalize_line_fingerprint(line);
        if fingerprint.is_empty() || self.is_recent_duplicate(line) {
            return;
        }
        if self.recent_lines.len() == IDLE_BANTER_HISTORY_CAPACITY {
            self.recent_lines.pop_front();
        }
        self.recent_lines.push_back((fingerprint, line.to_string()));
    }

    #[cfg(test)]
    pub(crate) fn next_due_at(&self) -> Instant {
        self.next_due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> AppSettings {
        AppSettings::default()
    }

    #[test]
    fn defaults_are_normalized_and_non_empty() {
        let normalized = normalize_idle_banter_seed_topics(&default_idle_banter_seed_topics())
            .expect("built-in seed topics must remain valid");
        assert_eq!(normalized.len(), DEFAULT_IDLE_BANTER_SEED_TOPICS.len());
    }

    #[test]
    fn normalization_trims_preserves_order_and_rejects_duplicates() {
        let topics = vec!["  Café thoughts  ".to_string(), "wall duty".to_string()];
        assert_eq!(
            normalize_idle_banter_seed_topics(&topics).unwrap(),
            vec!["Café thoughts".to_string(), "wall duty".to_string()]
        );
        assert!(
            normalize_idle_banter_seed_topics(&["Moose".to_string(), "  mOoSe ".to_string()])
                .is_err()
        );
    }

    #[test]
    fn normalization_rejects_every_configured_boundary_violation() {
        assert!(normalize_idle_banter_seed_topics(&[]).is_err());
        assert!(normalize_idle_banter_seed_topics(
            &(0..=IDLE_BANTER_MAX_SEEDS)
                .map(|i| format!("topic-{i}"))
                .collect::<Vec<_>>()
        )
        .is_err());
        assert!(normalize_idle_banter_seed_topics(&["   ".to_string()]).is_err());
        assert!(
            normalize_idle_banter_seed_topics(&["x".repeat(IDLE_BANTER_MAX_SEED_CHARS + 1)])
                .is_err()
        );

        let long = "x".repeat(IDLE_BANTER_MAX_SEED_CHARS);
        let topics = (0..IDLE_BANTER_MAX_SEEDS)
            .map(|i| format!("{i:02}-{long}"))
            .collect::<Vec<_>>();
        assert!(normalize_idle_banter_seed_topics(&topics).is_err());
    }

    #[test]
    fn normalization_accepts_unicode_and_preserves_display_order() {
        let topics = vec!["  🫎 existentialism  ".to_string(), "Café duty".to_string()];
        let normalized = normalize_idle_banter_seed_topics(&topics).unwrap();
        assert_eq!(normalized, vec!["🫎 existentialism", "Café duty"]);
    }

    #[test]
    fn scheduler_obeys_initial_delay_and_only_returns_one_due_occurrence() {
        let settings = settings();
        let start = Instant::now();
        let mut runtime = IdleBanterRuntime::new_at(&settings, start);
        assert!(runtime
            .poll_due_at(&settings, start + Duration::from_secs(3_599), 0.0)
            .is_none());
        let due = runtime
            .poll_due_at(&settings, start + Duration::from_secs(3_600), 0.0)
            .expect("60 minute default threshold must become due");
        assert_eq!(due.inactivity_minutes, 60);
        assert!(runtime
            .poll_due_at(&settings, start + Duration::from_secs(3_600), 0.0)
            .is_none());
    }

    #[test]
    fn repeat_jitter_is_bounded_to_twenty_percent() {
        assert_eq!(
            jittered_repeat_duration_with_unit_sample(30, 0.0),
            Duration::from_secs(24 * 60)
        );
        assert_eq!(
            jittered_repeat_duration_with_unit_sample(30, 1.0),
            Duration::from_secs(36 * 60)
        );
    }

    #[test]
    fn activity_disable_and_reenable_start_fresh_initial_episodes() {
        let mut settings = settings();
        let start = Instant::now();
        let mut runtime = IdleBanterRuntime::new_at(&settings, start);
        let activity = start + Duration::from_secs(3_590);
        runtime.reset_at(&settings, activity);
        assert!(runtime
            .poll_due_at(&settings, activity + Duration::from_secs(3_599), 0.5)
            .is_none());

        settings.idle_banter_enabled = false;
        assert!(runtime
            .poll_due_at(&settings, activity + Duration::from_secs(7_200), 0.5)
            .is_none());
        settings.idle_banter_enabled = true;
        let reenabled = activity + Duration::from_secs(7_201);
        assert!(runtime.poll_due_at(&settings, reenabled, 0.5).is_none());
        assert!(runtime.next_due_at() > reenabled);
    }

    #[test]
    fn wake_and_long_missed_intervals_start_fresh_without_backlog() {
        let settings = settings();
        let start = Instant::now();
        let mut runtime = IdleBanterRuntime::new_at(&settings, start);

        let long_after_due = start + Duration::from_secs(12 * 60 * 60);
        assert!(runtime
            .poll_due_at(&settings, long_after_due, 0.5)
            .is_some());
        assert!(runtime
            .poll_due_at(&settings, long_after_due, 0.5)
            .is_none());
        assert!(runtime.next_due_at() > long_after_due);

        let wake = long_after_due + Duration::from_secs(1);
        runtime.reset_at(&settings, wake);
        assert!(runtime.poll_due_at(&settings, wake, 0.5).is_none());
        assert!(runtime
            .poll_due_at(&settings, wake + Duration::from_secs(3_599), 0.5)
            .is_none());
        assert!(runtime
            .poll_due_at(&settings, wake + Duration::from_secs(3_600), 0.5)
            .is_some());
    }

    #[test]
    fn attempted_occurrence_advances_repeat_deadline_even_if_delivery_never_happens() {
        let mut settings = settings();
        settings.idle_banter_initial_delay_minutes = 5;
        settings.idle_banter_repeat_interval_minutes = 30;
        let start = Instant::now();
        let mut runtime = IdleBanterRuntime::new_at(&settings, start);
        let due_at = start + Duration::from_secs(5 * 60);

        let _attempt = runtime
            .poll_due_at(&settings, due_at, 0.0)
            .expect("occurrence should be handed off");
        assert_eq!(runtime.next_due_at(), due_at + Duration::from_secs(24 * 60));
        assert!(runtime.poll_due_at(&settings, due_at, 0.0).is_none());
    }

    #[test]
    fn seed_selection_avoids_immediate_repeat_when_possible() {
        let mut settings = settings();
        settings.idle_banter_initial_delay_minutes = 5;
        settings.idle_banter_repeat_interval_minutes = 5;
        settings.idle_banter_seed_topics = vec!["one".into(), "two".into()];
        let start = Instant::now();
        let mut runtime = IdleBanterRuntime::new_at(&settings, start);
        let first = runtime
            .poll_due_at(&settings, start + Duration::from_secs(300), 0.0)
            .unwrap();
        let second_time = runtime.next_due_at();
        let second = runtime.poll_due_at(&settings, second_time, 0.0).unwrap();
        assert_ne!(first.seed_topic, second.seed_topic);
    }

    #[test]
    fn seed_selection_remains_non_repeating_across_interaction_resets() {
        let mut settings = settings();
        settings.idle_banter_initial_delay_minutes = 5;
        settings.idle_banter_repeat_interval_minutes = 5;
        settings.idle_banter_seed_topics = vec!["one".into(), "two".into(), "three".into()];
        let start = Instant::now();
        let mut runtime = IdleBanterRuntime::new_at(&settings, start);
        let first = runtime
            .poll_due_at_with_samples(&settings, start + Duration::from_secs(300), 0.0, 0.5)
            .unwrap();
        assert_eq!(first.seed_topic, "one");

        let interaction = start + Duration::from_secs(301);
        runtime.reset_at(&settings, interaction);
        let second = runtime
            .poll_due_at_with_samples(&settings, interaction + Duration::from_secs(300), 0.0, 0.5)
            .unwrap();
        assert_eq!(second.seed_topic, "two");
        assert_ne!(first.seed_topic, second.seed_topic);
    }

    #[test]
    fn seed_selection_maps_sample_range_across_all_non_previous_topics() {
        let topics = vec!["one".into(), "two".into(), "three".into()];
        assert_eq!(seed_index_avoiding_previous(&topics, Some("two"), 0.0), 0);
        assert_eq!(
            seed_index_avoiding_previous(&topics, Some("two"), 0.999_999),
            2
        );
        assert_ne!(seed_index_avoiding_previous(&topics, Some("two"), 0.5), 1);
    }

    #[test]
    fn delivered_history_is_session_only_bounded_and_deduplicated() {
        let settings = settings();
        let mut runtime = IdleBanterRuntime::new_at(&settings, Instant::now());
        for index in 0..20 {
            runtime.record_delivered(&format!("Remark {index}"));
        }
        assert_eq!(runtime.recent_lines().len(), IDLE_BANTER_HISTORY_CAPACITY);
        assert!(!runtime.recent_lines().contains(&"Remark 0".to_string()));
        assert!(runtime.is_recent_duplicate("  remark   19 "));
        runtime.record_delivered("REMARK 19");
        assert_eq!(runtime.recent_lines().len(), IDLE_BANTER_HISTORY_CAPACITY);
    }
}
