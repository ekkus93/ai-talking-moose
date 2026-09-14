/// Fixed-capacity circular buffer for canonical mono PCM samples.
///
/// The buffer owns a preallocated `Vec<i16>` and never grows after construction.
/// Callers are responsible for external synchronization when it is shared across
/// threads. The type deliberately has no serialization or debug representation of
/// sample contents so microphone pre-roll cannot accidentally enter diagnostics.
pub struct PcmRingBuffer {
    samples: Vec<i16>,
    write_index: usize,
    len: usize,
}

impl PcmRingBuffer {
    pub fn new(capacity_samples: usize) -> Self {
        Self {
            samples: vec![0; capacity_samples],
            write_index: 0,
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.samples.len()
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

    /// Append samples, retaining only the newest `capacity()` samples.
    pub fn push(&mut self, input: &[i16]) {
        let capacity = self.capacity();
        if capacity == 0 || input.is_empty() {
            return;
        }

        // For an oversized write, only the final capacity samples can survive.
        let input = if input.len() > capacity {
            &input[input.len() - capacity..]
        } else {
            input
        };

        let first = input.len().min(capacity - self.write_index);
        self.samples[self.write_index..self.write_index + first]
            .copy_from_slice(&input[..first]);
        let remaining = input.len() - first;
        if remaining > 0 {
            self.samples[..remaining].copy_from_slice(&input[first..]);
        }

        self.write_index = (self.write_index + input.len()) % capacity;
        self.len = (self.len + input.len()).min(capacity);
    }

    /// Return retained samples in chronological order, oldest first.
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
        if start + self.len <= capacity {
            return self.samples[start..start + self.len].to_vec();
        }

        let first_len = capacity - start;
        let mut out = Vec::with_capacity(self.len);
        out.extend_from_slice(&self.samples[start..]);
        out.extend_from_slice(&self.samples[..self.len - first_len]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::PcmRingBuffer;

    #[test]
    fn empty_snapshot_is_empty() {
        let buffer = PcmRingBuffer::new(4);
        assert_eq!(buffer.snapshot(), Vec::<i16>::new());
        assert!(buffer.is_empty());
    }

    #[test]
    fn partial_fill_preserves_order() {
        let mut buffer = PcmRingBuffer::new(5);
        buffer.push(&[1, 2, 3]);
        assert_eq!(buffer.snapshot(), vec![1, 2, 3]);
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn exact_capacity_preserves_order() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.push(&[1, 2, 3, 4]);
        assert_eq!(buffer.snapshot(), vec![1, 2, 3, 4]);
        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn single_wraparound_returns_oldest_to_newest() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.push(&[1, 2, 3]);
        buffer.push(&[4, 5]);
        assert_eq!(buffer.snapshot(), vec![2, 3, 4, 5]);
    }

    #[test]
    fn repeated_wraparound_stays_ordered() {
        let mut buffer = PcmRingBuffer::new(3);
        for chunk in [[1, 2], [3, 4], [5, 6], [7, 8]] {
            buffer.push(&chunk);
        }
        assert_eq!(buffer.snapshot(), vec![6, 7, 8]);
    }

    #[test]
    fn oversized_write_keeps_newest_samples() {
        let mut buffer = PcmRingBuffer::new(4);
        buffer.push(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(buffer.snapshot(), vec![3, 4, 5, 6]);
        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn clear_removes_retained_samples() {
        let mut buffer = PcmRingBuffer::new(3);
        buffer.push(&[1, 2, 3]);
        buffer.clear();
        assert!(buffer.snapshot().is_empty());
        assert!(buffer.is_empty());
        buffer.push(&[4]);
        assert_eq!(buffer.snapshot(), vec![4]);
    }

    #[test]
    fn capacity_never_changes_under_long_repeated_writes() {
        let mut buffer = PcmRingBuffer::new(32);
        let original_capacity = buffer.capacity();
        for value in 0..10_000_i16 {
            buffer.push(&[value]);
        }
        assert_eq!(buffer.capacity(), original_capacity);
        assert_eq!(buffer.len(), original_capacity);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.len(), original_capacity);
        assert_eq!(snapshot[original_capacity - 1], 9_999);
    }

    #[test]
    fn zero_capacity_is_safe_and_bounded() {
        let mut buffer = PcmRingBuffer::new(0);
        buffer.push(&[1, 2, 3]);
        assert!(buffer.snapshot().is_empty());
        assert_eq!(buffer.capacity(), 0);
        assert_eq!(buffer.len(), 0);
    }
}
