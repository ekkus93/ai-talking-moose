use crate::app::wake_word::policy::{
    V1_KWS_SAMPLE_RATE_HZ, V1_PRE_ROLL_SAMPLES, V1_PRE_ROLL_SECONDS,
};

/// Canonical wake-word microphone sample rate used for pre-roll sizing.
pub const WAKE_PCM_SAMPLE_RATE_HZ: usize = V1_KWS_SAMPLE_RATE_HZ as usize;
/// Wake Word V1 retains two seconds of canonical mono PCM.
pub const WAKE_PCM_PRE_ROLL_SECONDS: usize = V1_PRE_ROLL_SECONDS;
/// Number of i16 mono samples retained by the nominal Wake Word V1 pre-roll buffer.
pub const WAKE_PCM_PRE_ROLL_SAMPLES: usize = V1_PRE_ROLL_SAMPLES;

/// Fixed-capacity chronological buffer for mono signed 16-bit PCM samples.
///
/// `PcmRingBuffer` deliberately has no internal synchronization. Mutating methods require
/// `&mut self`, so the capture/router owner must serialize access (or place the buffer behind
/// its existing synchronization primitive). The backing allocation is created once and never
/// grows on append. Buffered PCM is exposed only as an in-memory snapshot and is never logged
/// or serialized by this component.
#[derive(Debug)]
pub struct PcmRingBuffer {
    samples: Box<[i16]>,
    write_index: usize,
    len: usize,
}

impl PcmRingBuffer {
    /// Creates a fixed-capacity ring. Capacity must be greater than zero.
    pub fn new(capacity_samples: usize) -> Self {
        assert!(
            capacity_samples > 0,
            "PCM ring-buffer capacity must be non-zero"
        );
        Self {
            samples: vec![0; capacity_samples].into_boxed_slice(),
            write_index: 0,
            len: 0,
        }
    }

    /// Creates the nominal Wake Word V1 two-second, 16 kHz mono pre-roll buffer.
    pub fn wake_word_v1() -> Self {
        Self::new(WAKE_PCM_PRE_ROLL_SAMPLES)
    }

    pub fn capacity_samples(&self) -> usize {
        self.samples.len()
    }

    pub fn len_samples(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends samples while retaining only the newest `capacity_samples()` values.
    ///
    /// Writes larger than the entire capacity are reduced to their newest capacity-sized suffix,
    /// avoiding per-sample work for audio that cannot possibly remain in the buffer.
    pub fn append(&mut self, input: &[i16]) {
        if input.is_empty() {
            return;
        }

        let capacity = self.capacity_samples();
        if input.len() >= capacity {
            let newest = &input[input.len() - capacity..];
            self.samples.copy_from_slice(newest);
            self.write_index = 0;
            self.len = capacity;
            return;
        }

        let first_len = input.len().min(capacity - self.write_index);
        self.samples[self.write_index..self.write_index + first_len]
            .copy_from_slice(&input[..first_len]);

        let remaining = input.len() - first_len;
        if remaining > 0 {
            self.samples[..remaining].copy_from_slice(&input[first_len..]);
        }

        self.write_index = (self.write_index + input.len()) % capacity;
        self.len = (self.len + input.len()).min(capacity);
    }

    /// Returns the retained PCM in exact chronological order, oldest sample first.
    pub fn snapshot(&self) -> Vec<i16> {
        if self.len == 0 {
            return Vec::new();
        }

        let capacity = self.capacity_samples();
        let oldest = if self.len == capacity {
            self.write_index
        } else {
            0
        };

        if oldest + self.len <= capacity {
            return self.samples[oldest..oldest + self.len].to_vec();
        }

        let first_len = capacity - oldest;
        let mut snapshot = Vec::with_capacity(self.len);
        snapshot.extend_from_slice(&self.samples[oldest..]);
        snapshot.extend_from_slice(&self.samples[..self.len - first_len]);
        snapshot
    }

    /// Removes all retained audio and overwrites the backing storage with silence.
    pub fn clear(&mut self) {
        self.samples.fill(0);
        self.write_index = 0;
        self.len = 0;
    }
}

impl Default for PcmRingBuffer {
    fn default() -> Self {
        Self::wake_word_v1()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_snapshot_is_empty() {
        let ring = PcmRingBuffer::new(4);
        assert!(ring.snapshot().is_empty());
        assert_eq!(ring.len_samples(), 0);
    }

    #[test]
    fn partial_fill_snapshot_is_chronological() {
        let mut ring = PcmRingBuffer::new(5);
        ring.append(&[10, 11, 12]);
        assert_eq!(ring.snapshot(), vec![10, 11, 12]);
    }

    #[test]
    fn exact_capacity_snapshot_is_chronological() {
        let mut ring = PcmRingBuffer::new(4);
        ring.append(&[1, 2, 3, 4]);
        assert_eq!(ring.snapshot(), vec![1, 2, 3, 4]);
        assert_eq!(ring.len_samples(), 4);
    }

    #[test]
    fn single_wraparound_retains_newest_samples_in_order() {
        let mut ring = PcmRingBuffer::new(5);
        ring.append(&[1, 2, 3, 4]);
        ring.append(&[5, 6, 7]);
        assert_eq!(ring.snapshot(), vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn repeated_wraparound_retains_exact_order() {
        let mut ring = PcmRingBuffer::new(4);
        for chunk in [[1, 2], [3, 4], [5, 6], [7, 8], [9, 10]] {
            ring.append(&chunk);
        }
        assert_eq!(ring.snapshot(), vec![7, 8, 9, 10]);
    }

    #[test]
    fn oversized_write_keeps_only_newest_capacity_samples() {
        let mut ring = PcmRingBuffer::new(4);
        ring.append(&[99, 98]);
        ring.append(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(ring.snapshot(), vec![3, 4, 5, 6]);
        assert_eq!(ring.len_samples(), 4);
    }

    #[test]
    fn clear_removes_all_retained_samples_and_resets_ordering() {
        let mut ring = PcmRingBuffer::new(4);
        ring.append(&[1, 2, 3, 4, 5]);
        ring.clear();
        assert!(ring.is_empty());
        assert!(ring.snapshot().is_empty());

        ring.append(&[8, 9]);
        assert_eq!(ring.snapshot(), vec![8, 9]);
    }

    #[test]
    fn chronological_replay_is_exact_across_partial_wrap() {
        let mut ring = PcmRingBuffer::new(6);
        ring.append(&[10, 20, 30, 40, 50]);
        ring.append(&[60, 70, 80]);
        assert_eq!(ring.snapshot(), vec![30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn capacity_remains_bounded_under_long_repeated_writes() {
        let mut ring = PcmRingBuffer::new(32);
        for value in 0..10_000_i16 {
            ring.append(&[value]);
            assert!(ring.len_samples() <= ring.capacity_samples());
            assert_eq!(ring.capacity_samples(), 32);
        }
        assert_eq!(ring.len_samples(), 32);
        assert_eq!(ring.snapshot().len(), 32);
    }

    #[test]
    fn wake_word_default_is_two_seconds_of_canonical_pcm() {
        let ring = PcmRingBuffer::wake_word_v1();
        assert_eq!(WAKE_PCM_SAMPLE_RATE_HZ, 16_000);
        assert_eq!(WAKE_PCM_PRE_ROLL_SECONDS, 2);
        assert_eq!(ring.capacity_samples(), 32_000);
    }
}
