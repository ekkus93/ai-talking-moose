use crate::character::cooldown::CooldownTracker;
use crate::character::personality::CharacterConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientDecision {
    pub should_speak: bool,
    pub event_summary: String,
    pub topic: String,
}

pub struct BehaviorEngine {
    pub config: CharacterConfig,
    pub cooldowns: CooldownTracker,
}

impl BehaviorEngine {
    pub fn new(config: CharacterConfig) -> Self {
        Self {
            config,
            cooldowns: CooldownTracker::new(),
        }
    }

    pub fn evaluate_event(
        &mut self,
        event_name: &str,
        event_summary: &str,
        importance: f32, // 0.0 to 1.0
    ) -> Option<AmbientDecision> {
        self.evaluate_event_at(Utc::now(), event_name, event_summary, importance)
    }

    pub(crate) fn evaluate_event_at(
        &mut self,
        now: DateTime<Utc>,
        event_name: &str,
        event_summary: &str,
        importance: f32,
    ) -> Option<AmbientDecision> {
        if !self.config.behavior.unsolicited_comments {
            return None;
        }

        let allowed = self.cooldowns.can_speak_ambient(
            now,
            self.config.behavior.min_cooldown_seconds,
            self.config.behavior.max_comments_per_hour,
            self.config.behavior.quiet_hours_enabled,
            self.config.behavior.quiet_hours_start,
            self.config.behavior.quiet_hours_end,
        );

        if !allowed {
            return None;
        }

        // Check if importance exceeds threshold scaled by talkativeness
        // higher talkativeness (e.g. 1.0) makes threshold lower (0.2), lower talkativeness (0.0) makes threshold higher (0.8)
        let threshold = 0.8 - (self.config.personality.talkativeness * 0.6);
        if importance < threshold {
            return None;
        }

        // Record speech cooldown
        self.cooldowns.record_speech(now);

        Some(AmbientDecision {
            should_speak: true,
            event_summary: event_summary.to_string(),
            topic: event_name.to_string(),
        })
    }

    pub fn get_canned_greeting() -> &'static str {
        "Hey! Don't look now, but you're working on a computer again."
    }

    pub fn get_canned_click_reaction() -> &'static str {
        "Yeah? What's on your mind, pal?"
    }

    pub fn get_canned_dismiss_reaction() -> &'static str {
        "Fine, fine. I was just leaving anyway."
    }

    pub fn get_canned_error_phrase() -> &'static str {
        "Oops. My thinking cap seems to have blown a tube."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::cooldown::DISMISSAL_COOLDOWN_SECONDS;
    use chrono::Duration;

    #[test]
    fn dismissal_blocks_immediate_ambient_reappearance_then_expires() {
        let mut config = CharacterConfig::default();
        config.behavior.unsolicited_comments = true;
        config.behavior.quiet_hours_enabled = false;
        config.behavior.min_cooldown_seconds = 0;
        config.behavior.max_comments_per_hour = 10;
        config.personality.talkativeness = 1.0;
        let mut engine = BehaviorEngine::new(config);
        let dismissed_at = Utc::now();
        engine.cooldowns.record_dismissal(dismissed_at);

        assert!(engine
            .evaluate_event_at(
                dismissed_at + Duration::seconds(DISMISSAL_COOLDOWN_SECONDS - 1),
                "window_change",
                "safe synthetic event",
                1.0,
            )
            .is_none());

        assert!(engine
            .evaluate_event_at(
                dismissed_at + Duration::seconds(DISMISSAL_COOLDOWN_SECONDS),
                "window_change",
                "safe synthetic event",
                1.0,
            )
            .is_some());
    }
}
