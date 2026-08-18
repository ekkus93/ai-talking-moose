use crate::character::state::MouthShape;

#[derive(Debug, Clone)]
pub struct LevelMeter {
    pub smoothed_rms: f32,
    pub smoothing_factor: f32, // 0.0 to 1.0 (e.g. 0.3 for responsiveness)
    pub current_mouth: MouthShape,
}

impl Default for LevelMeter {
    fn default() -> Self {
        Self {
            smoothed_rms: 0.0,
            smoothing_factor: 0.35,
            current_mouth: MouthShape::Closed,
        }
    }
}

impl LevelMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate RMS (Root Mean Square) energy of PCM audio samples (f32 format)
    pub fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Calculate RMS of i16 PCM samples
    pub fn calculate_rms_i16(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f64 = samples
            .iter()
            .map(|&s| {
                let norm = s as f64 / 32768.0;
                norm * norm
            })
            .sum();
        (sum_squares / samples.len() as f64).sqrt() as f32
    }

    /// Feed audio samples and derive smoothed RMS and current mouth shape with hysteresis
    pub fn feed_samples(&mut self, samples: &[f32]) -> (f32, MouthShape) {
        let raw_rms = Self::calculate_rms(samples);
        self.smoothed_rms =
            (self.smoothing_factor * raw_rms) + ((1.0 - self.smoothing_factor) * self.smoothed_rms);

        // Hysteresis thresholds to avoid fluttering
        let new_mouth = match self.current_mouth {
            MouthShape::Closed => {
                if self.smoothed_rms > 0.35 {
                    MouthShape::Wide
                } else if self.smoothed_rms > 0.20 {
                    MouthShape::Medium
                } else if self.smoothed_rms > 0.06 {
                    MouthShape::Small
                } else {
                    MouthShape::Closed
                }
            }
            MouthShape::Small => {
                if self.smoothed_rms > 0.35 {
                    MouthShape::Wide
                } else if self.smoothed_rms > 0.20 {
                    MouthShape::Medium
                } else if self.smoothed_rms < 0.03 {
                    MouthShape::Closed
                } else {
                    MouthShape::Small
                }
            }
            MouthShape::Medium => {
                if self.smoothed_rms > 0.38 {
                    MouthShape::Wide
                } else if self.smoothed_rms < 0.12 {
                    MouthShape::Small
                } else {
                    MouthShape::Medium
                }
            }
            MouthShape::Wide => {
                if self.smoothed_rms < 0.28 {
                    MouthShape::Medium
                } else {
                    MouthShape::Wide
                }
            }
        };

        self.current_mouth = new_mouth;
        (self.smoothed_rms, self.current_mouth)
    }

    pub fn reset(&mut self) {
        self.smoothed_rms = 0.0;
        self.current_mouth = MouthShape::Closed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_silence() {
        let silence = vec![0.0; 100];
        assert_eq!(LevelMeter::calculate_rms(&silence), 0.0);
    }

    #[test]
    fn test_mouth_progression() {
        let mut meter = LevelMeter::new();

        // Feed silence -> should stay closed
        let (_, mouth) = meter.feed_samples(&vec![0.0; 500]);
        assert_eq!(mouth, MouthShape::Closed);

        // Feed small signal
        let small_signal = vec![0.12; 500];
        meter.feed_samples(&small_signal);
        meter.feed_samples(&small_signal);
        let (_, mouth) = meter.feed_samples(&small_signal);
        assert_eq!(mouth, MouthShape::Small);

        // Feed strong signal -> wide
        let loud_signal = vec![0.8; 500];
        meter.feed_samples(&loud_signal);
        meter.feed_samples(&loud_signal);
        let (_, mouth) = meter.feed_samples(&loud_signal);
        assert_eq!(mouth, MouthShape::Wide);
    }
}
