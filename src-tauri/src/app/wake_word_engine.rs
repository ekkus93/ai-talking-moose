pub use super::wake_word::policy::{
    DEFAULT_WAKE_PHRASE, V1_KWS_CHANNELS, V1_KWS_FEATURE_DIM, V1_KWS_KEYWORD,
    V1_KWS_SAMPLE_RATE_HZ, V1_KWS_THREADS, V1_WAKE_SCORE, V1_WAKE_THRESHOLD,
};

mod artifacts;
mod native;
mod types;

pub use artifacts::validate_pcm_frame;
pub use native::{NativeKwsSession, NativeKwsSessionPaths};
pub use types::*;

#[cfg(test)]
use crate::asr::wake_word_sherpa_manifest::{
    SHERPA_KWS_KEYWORD_FILE, SHERPA_KWS_KEYWORD_REPRESENTATION, V1_SHERPA_KWS_MODEL_FILES,
};
#[cfg(test)]
use artifacts::{runtime_files, verify_native_architecture, NativeArchitecture};
#[cfg(test)]
use native::{
    build_keyword_spotter_config, keyword_result_has_detection, native_capi_contract,
    verify_native_c_api_symbols, NativeKwsCStringStore, SherpaOnnxKeywordResult,
};

#[cfg(test)]
#[path = "wake_word_engine/tests.rs"]
mod tests;
