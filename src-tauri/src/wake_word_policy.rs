//! Canonical immutable Wake Word V1 policy.
//!
//! All Wake Word configuration consumers must use or explicitly verify these
//! values. Keeping them together prevents settings, diagnostics, ring sizing,
//! and KWS configuration from silently drifting apart.

pub const DEFAULT_WAKE_PHRASE: &str = "Hey, Moose";
pub const V1_KWS_KEYWORD: &str = "HEY MOOSE";
pub const V1_KWS_SAMPLE_RATE_HZ: u32 = 16_000;
pub const V1_KWS_CHANNELS: u16 = 1;
pub const V1_KWS_FEATURE_DIM: u16 = 80;
pub const V1_KWS_THREADS: u16 = 1;
pub const V1_WAKE_SCORE: f32 = 1.0;
pub const V1_WAKE_THRESHOLD: f32 = 0.25;
pub const V1_PRE_ROLL_SECONDS: usize = 2;
pub const V1_PRE_ROLL_SAMPLES: usize = V1_KWS_SAMPLE_RATE_HZ as usize * V1_PRE_ROLL_SECONDS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_v1_policy_is_frozen() {
        assert_eq!(DEFAULT_WAKE_PHRASE, "Hey, Moose");
        assert_eq!(V1_KWS_KEYWORD, "HEY MOOSE");
        assert_eq!(V1_KWS_SAMPLE_RATE_HZ, 16_000);
        assert_eq!(V1_KWS_CHANNELS, 1);
        assert_eq!(V1_KWS_FEATURE_DIM, 80);
        assert_eq!(V1_KWS_THREADS, 1);
        assert_eq!(V1_WAKE_SCORE, 1.0);
        assert_eq!(V1_WAKE_THRESHOLD, 0.25);
        assert_eq!(V1_PRE_ROLL_SECONDS, 2);
        assert_eq!(V1_PRE_ROLL_SAMPLES, 32_000);
    }
}
