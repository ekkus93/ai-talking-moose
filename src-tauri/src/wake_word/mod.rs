pub mod ring_buffer;

pub const WAKE_WORD_SAMPLE_RATE_HZ: u32 = 16_000;
pub const WAKE_WORD_CHANNELS: u16 = 1;
pub const WAKE_WORD_PRE_ROLL_SECONDS: usize = 2;
pub const WAKE_WORD_RING_CAPACITY_SAMPLES: usize =
    WAKE_WORD_SAMPLE_RATE_HZ as usize * WAKE_WORD_PRE_ROLL_SECONDS;
pub const DEFAULT_WAKE_PHRASE: &str = "Hey, Moose";
pub const CANONICAL_WAKE_KEYWORD: &str = "HEY MOOSE";
pub const WAKE_WORD_KWS_THREADS: usize = 1;
pub const WAKE_WORD_KEYWORDS_THRESHOLD: f32 = 0.25;
pub const WAKE_WORD_KEYWORDS_SCORE: f32 = 1.0;
