pub mod fake;
pub mod google;
pub mod local;
pub mod local_tts;
pub mod loopback;
pub mod openai;
pub mod system_tts_pcm;
pub mod system_tts_request;
pub mod system_tts_selection;
pub mod system_tts_voice_catalog;
pub mod traits;
pub mod tts_catalog;
pub mod types;
pub mod unavailable;

pub use fake::*;
pub use traits::*;
pub use types::*;

#[cfg(test)]
mod loopback_integration_tests;

#[cfg(test)]
mod loopback_request_contract_tests;
