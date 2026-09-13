use super::idle_banter::IdleBanterRuntime;
use super::personality::CharacterConfig;
use super::prompt::PromptBuilder;
use crate::app::state::AppSettings;
use crate::test_support::{assert_log_capture_live, capture_logs};
use std::time::{Duration, Instant};

#[test]
fn idle_banter_prompt_private_inputs_do_not_enter_tracing() {
    const PRIVATE_SEED: &str = "PRIVATE_IDLE_BANTER_SEED_7f3a91";
    const PRIVATE_RECENT: &str = "PRIVATE_IDLE_BANTER_RECENT_2c8d44";

    let config = CharacterConfig::default();
    let recent = vec![PRIVATE_RECENT.to_string()];
    let (prompt, logs) = capture_logs(|| {
        PromptBuilder::build_idle_banter_prompt(&config, 75, PRIVATE_SEED, &recent, &[])
    });

    assert!(prompt.contains(PRIVATE_SEED));
    assert!(prompt.contains(PRIVATE_RECENT));
    assert_log_capture_live(&logs);
    assert!(!logs.contains(PRIVATE_SEED));
    assert!(!logs.contains(PRIVATE_RECENT));
}

#[test]
fn single_topic_idle_banter_selection_never_invents_another_topic() {
    let mut settings = AppSettings::default();
    // Runtime-level test hook: zero makes the first poll immediately due without
    // introducing a real-time sleep. Persisted settings validation still rejects zero.
    settings.idle_banter_initial_delay_minutes = 0;
    settings.idle_banter_seed_topics = vec!["only configured topic".to_string()];

    let mut runtime = IdleBanterRuntime::new(&settings);
    let due = runtime
        .poll_due(&settings)
        .expect("zero-delay runtime fixture should be immediately due");

    assert_eq!(due.seed_topic, "only configured topic");
}

#[test]
fn wake_reset_uses_current_initial_delay_and_starts_a_fresh_episode() {
    let mut settings = AppSettings::default();
    settings.idle_banter_initial_delay_minutes = 5;
    let mut runtime = IdleBanterRuntime::new(&settings);

    settings.idle_banter_initial_delay_minutes = 9;
    let reset_started = Instant::now();
    runtime.reset_after_wake(&settings);
    let reset_finished = Instant::now();
    let next_due = runtime.next_due_at();
    let expected_delay = Duration::from_secs(9 * 60);

    assert!(next_due >= reset_started + expected_delay);
    assert!(next_due <= reset_finished + expected_delay);
}
