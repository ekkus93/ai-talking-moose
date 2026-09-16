use std::collections::BTreeSet;

use super::state::{AppSettings, AudioDeviceInfo};

pub fn validate_selected_device(
    selected: Option<&str>,
    available: &[AudioDeviceInfo],
    label: &str,
) -> Result<(), String> {
    let Some(selected) = selected else {
        return Ok(());
    };
    if available.iter().any(|device| device.id == selected) {
        Ok(())
    } else {
        Err(format!("selected {label} device is unavailable"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_device_must_come_from_current_enumeration() {
        let available = vec![AudioDeviceInfo {
            id: "Built-in Mic".to_string(),
            name: "Built-in Mic".to_string(),
            is_default: true,
        }];
        assert!(validate_selected_device(Some("Built-in Mic"), &available, "input").is_ok());
        assert!(validate_selected_device(Some("Missing Mic"), &available, "input").is_err());
        assert!(validate_selected_device(Some("Missing Output"), &available, "output").is_err());
        assert!(validate_selected_device(None, &available, "input").is_ok());
    }

    #[test]
    fn persisted_settings_are_classified_and_user_preferences_have_runtime_consumers() {
        let user_consumers = [
            ("asr_mode", "conversation start selects local or Gemini ASR"),
            ("launch_at_login", "startup runtime preferences"),
            ("show_in_menu_bar", "startup runtime preferences"),
            ("always_on_top", "window runtime preferences"),
            ("restore_position", "startup window restoration"),
            ("unsolicited_comments", "behavior engine ambient policy"),
            ("talkativeness", "behavior engine personality"),
            ("quiet_hours_enabled", "behavior engine ambient policy"),
            ("quiet_hours_start", "behavior engine ambient policy"),
            ("quiet_hours_end", "behavior engine ambient policy"),
            ("max_comments_per_hour", "behavior engine ambient budget"),
            ("hide_delay_seconds", "ambient post-speech hide delay"),
            ("idle_banter_enabled", "Idle Banter dedicated feature gate"),
            (
                "idle_banter_initial_delay_minutes",
                "Idle Banter initial inactivity scheduler",
            ),
            (
                "idle_banter_repeat_interval_minutes",
                "Idle Banter repeat scheduler",
            ),
            (
                "idle_banter_seed_topics",
                "Idle Banter creative-direction prompt selection",
            ),
            ("input_device", "conversation capture and microphone test"),
            ("output_device", "conversation and standalone playback"),
            ("volume", "shared CPAL playback gain"),
            ("tts_provider", "standalone TTS provider selection"),
            ("google_tts_voice", "Google standalone speech selection"),
            ("local_tts_voice", "Local standalone speech selection"),
            ("live_voice", "Gemini Live session voice selection"),
            ("speaking_rate", "standalone TTS request"),
            ("pitch", "standalone TTS request"),
            ("text_provider", "text model provider selection"),
            ("live_model", "Gemini Live session configuration"),
            ("google_text_model", "Google text model construction"),
            ("local_text_model", "Local model catalog selection"),
            ("google_tts_model", "Google speech synthesizer construction"),
            ("local_tts_model", "Local speech model selection"),
            ("active_app_observation", "desktop observation privacy gate"),
            ("memory_enabled", "prompt memory privacy gate"),
            ("save_transcripts", "transcript retention policy"),
            ("dry", "character personality prompt"),
            ("sarcastic", "character personality prompt"),
            ("friendly", "character personality prompt"),
            ("absurd", "character personality prompt"),
            ("helpful", "character personality prompt"),
            ("verbosity", "character personality prompt"),
        ];
        let metadata = [
            ("settings_version", "schema and migration metadata"),
            (
                "window_title_observation",
                "V1 compatibility field normalized fail-closed to false",
            ),
            (
                "wake_word_enabled",
                "Wake Word V1 persisted feature gate; runtime consumer lands with wake lifecycle integration",
            ),
            (
                "wake_word_phrase",
                "Wake Word V1 canonical phrase configuration validated fail-closed before runtime integration",
            ),
        ];

        let serialized = serde_json::to_value(AppSettings::default()).unwrap();
        let actual = serialized
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let classified = user_consumers
            .iter()
            .chain(metadata.iter())
            .map(|(field, _)| (*field).to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(actual, classified);
        assert!(user_consumers
            .iter()
            .all(|(_, consumer)| !consumer.is_empty()));
        assert!(metadata
            .iter()
            .all(|(_, classification)| !classification.is_empty()));
        assert!(serialized.get("provider").is_none());
        assert!(serialized.get("text_model").is_none());
        assert!(serialized.get("tts_model").is_none());
        assert!(serialized.get("tts_voice").is_none());
        assert!(serialized.get("microphone_permission_granted").is_none());
    }
}
