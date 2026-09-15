/// Fixed-capacity in-memory PCM pre-roll buffer.
///
/// The buffer stores mono `i16` samples and never grows after construction. Writes
/// larger than the capacity retain only the newest samples. Snapshot allocation is
/// intentionally outside the real-time capture callback path.
#[derive(Debug, Clone)]
pub struct PcmRingBuffer {
    storage: Vec<i16>,
    write_index: usize,
    len: usize,
}

impl PcmRingBuffer {
    pub fn new(capacity_samples: usize) -> Self {
        Self {
            storage: vec![0; capacity_samples],
            write_index: 0,
            len: 0,
        }
    }

    pub fn for_duration(sample_rate_hz: u32, channels: u16, seconds: u32) -> Self {
        let capacity = usize::try_from(sample_rate_hz)
            .ok()
            .and_then(|rate| rate.checked_mul(usize::from(channels)))
            .and_then(|per_second| per_second.checked_mul(seconds as usize))
            .unwrap_or(0);
        Self::new(capacity)
    }

    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.write_index = 0;
        self.len = 0;
    }

    pub fn write(&mut self, samples: &[i16]) {
        let capacity = self.capacity();
        if capacity == 0 || samples.is_empty() {
            return;
        }

        let samples = if samples.len() > capacity {
            &samples[samples.len() - capacity..]
        } else {
            samples
        };

        for &sample in samples {
            self.storage[self.write_index] = sample;
            self.write_index = (self.write_index + 1) % capacity;
            self.len = (self.len + 1).min(capacity);
        }
    }

    /// Returns retained samples from oldest to newest.
    pub fn snapshot(&self) -> Vec<i16> {
        if self.len == 0 {
            return Vec::new();
        }

        let capacity = self.capacity();
        let start = if self.len == capacity {
            self.write_index
        } else {
            0
        };
        let mut result = Vec::with_capacity(self.len);
        for offset in 0..self.len {
            result.push(self.storage[(start + offset) % capacity]);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::PcmRingBuffer;

    #[test]
    fn empty_buffer_snapshot_is_empty() {
        let buffer = PcmRingBuffer::new(4);
        assert!(buffer.snapshot().is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 4);
    }

    #[test]
    fn partial_fill_preserves_chronological_order() {
        let mut buffer = PcmRingBuffer::new(5);
        buffer.write(&[1, 2, 3]);
        assert_eq!(buffer.snapshot(), vec![1, 2, 3]);
    }

    #[test]
    fn exact_capacity_snapshot_is_ordered() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.write(&[1, 2, 3, 4]);
        assert_eq!(buffer.snapshot(), vec![1, 2, 3, 4]);
        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn single_wraparound_retains_newest_samples_in_order() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.write(&[1, 2, 3, 4]);
        buffer.write(&[5, 6]);
        assert_eq!(buffer.snapshot(), vec![3, 4, 5, 6]);
    }

    #[test]
    fn repeated_wraparound_remains_bounded_and_ordered() {
        let mut buffer = PcmRingBuffer::new(3);
        for value in 1..=10 {
            buffer.write(&[value]);
            assert!(buffer.len() <= buffer.capacity());
        }
        assert_eq!(buffer.snapshot(), vec![8, 9, 10]);
    }

    #[test]
    fn oversized_write_keeps_only_newest_capacity() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.write(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(buffer.snapshot(), vec![3, 4, 5, 6]);
    }

    #[test]
    fn clear_removes_retained_samples_and_restarts_ordering() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.write(&[1, 2, 3]);
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.snapshot(), Vec::<i16>::new());
        buffer.write(&[7, 8]);
        assert_eq!(buffer.snapshot(), vec![7, 8]);
    }

    #[test]
    fn v1_duration_is_two_seconds_at_sixteen_khz_mono() {
        let buffer = PcmRingBuffer::for_duration(16_000, 1, 2);
        assert_eq!(buffer.capacity(), 32_000);
    }

    #[test]
    fn zero_capacity_is_a_safe_no_op() {
        let mut buffer = PcmRingBuffer::new(0);
        buffer.write(&[1, 2, 3]);
        assert!(buffer.snapshot().is_empty());
    }
}
