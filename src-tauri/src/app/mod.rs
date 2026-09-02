pub(crate) mod runtime_preferences;
pub(crate) mod settings_policy;
pub mod state;
pub(crate) mod tray;
pub(crate) mod window_position;
pub use state::*;

#[cfg(test)]
mod provider_switch_tests;
