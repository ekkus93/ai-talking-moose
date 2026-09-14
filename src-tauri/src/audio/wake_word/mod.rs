mod engine;
mod runtime;

pub use engine::{
    verify_model_artifacts, SherpaKwsEngine, WakeWordEngine, WakeWordError, WakeWordErrorKind,
    DEFAULT_KEYWORDS_SCORE, DEFAULT_KEYWORDS_THRESHOLD, WAKE_CHANNELS, WAKE_INFERENCE_THREADS,
    WAKE_SAMPLE_RATE_HZ,
};
pub use runtime::{
    WakeTriggerCallback, WakeWordDiagnostics, WakeWordRuntimeManager, WakeWordRuntimeState,
};

pub const DEFAULT_WAKE_PHRASE: &str = "Hey, Moose";
pub const CANONICAL_WAKE_KEYWORD: &str = "HEY MOOSE";
pub const WAKE_RING_BUFFER_SECONDS: u32 = 2;
