pub(crate) mod request_snapshot;
#[rustfmt::skip]
pub(crate) mod runtime_preferences;
pub(crate) mod settings_policy;
#[rustfmt::skip]
pub mod state;
pub(crate) mod tray;
#[allow(dead_code)]
pub(crate) mod wake_word;
pub mod wake_word_acceptance;
#[allow(dead_code)]
pub(crate) mod wake_word_authoritative_capture;
#[allow(dead_code)]
pub(crate) mod wake_word_capture_consumer;
#[allow(dead_code)]
pub(crate) mod wake_word_capture_orchestrator;
#[allow(dead_code)]
pub(crate) mod wake_word_command_activation;
#[allow(dead_code)]
pub(crate) mod wake_word_command_asr_ingress;
#[allow(dead_code)]
pub(crate) mod wake_word_command_handoff;
#[allow(dead_code)]
pub(crate) mod wake_word_command_lifecycle;
#[allow(dead_code)]
pub(crate) mod wake_word_composition;
#[allow(dead_code)]
pub(crate) mod wake_word_local_listener_thread;
#[allow(dead_code)]
pub(crate) mod wake_word_pcm_router;
pub(crate) mod wake_word_state;
#[allow(dead_code)]
#[rustfmt::skip]
pub(crate) mod wake_word_engine;
pub(crate) mod wake_word_settings;
pub(crate) mod window_position;
pub use state::*;

#[cfg(test)]
mod provider_switch_tests;
#[cfg(test)]
mod wake_word_appstate_composition_tests;
#[cfg(test)]
mod wake_word_lifecycle_stability_tests;
