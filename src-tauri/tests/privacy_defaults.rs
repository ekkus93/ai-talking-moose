use talking_moose_lib::app::state::AppSettings;
use talking_moose_lib::asr::AsrMode;

#[test]
fn fresh_profile_uses_local_asr_and_conservative_privacy_defaults() {
    let settings = AppSettings::default();

    assert_eq!(settings.asr_mode, AsrMode::MoonshineTinyStreaming);
    assert!(!settings.active_app_observation);
    assert!(!settings.window_title_observation);
    assert!(!settings.memory_enabled);
    assert!(!settings.save_transcripts);
}
