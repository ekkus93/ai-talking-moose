/// Fixed-capacity chronological PCM sample ring.
///
/// Ownership is deliberately single-threaded. Callers that need shared access must
/// synchronize outside this type; this keeps the microphone callback path allocation-free.
/// Raw samples are intentionally not `Debug`, `Serialize`, or otherwise exposed to diagnostics.
pub struct PcmRingBuffer {
    samples: Vec<i16>,
    write_index: usize,
    len: usize,
}

impl PcmRingBuffer {
    pub fn new(capacity_samples: usize) -> Self {
        assert!(capacity_samples > 0, "PCM ring capacity must be non-zero");
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
        // Do not depend on stale PCM after a lifecycle transition. Zeroing is cheap at
        // the V1 2-second/16-kHz capacity and makes accidental post-clear reads benign.
        self.samples.fill(0);
        self.write_index = 0;
        self.len = 0;
    }

    pub fn write(&mut self, input: &[i16]) {
        if input.is_empty() {
            return;
        }

        let capacity = self.capacity();
        let input = if input.len() > capacity {
            &input[input.len() - capacity..]
        } else {
            input
        };

        let first = input.len().min(capacity - self.write_index);
        self.samples[self.write_index..self.write_index + first].copy_from_slice(&input[..first]);
        let remaining = input.len() - first;
        if remaining > 0 {
            self.samples[..remaining].copy_from_slice(&input[first..]);
        }

        self.write_index = (self.write_index + input.len()) % capacity;
        self.len = (self.len + input.len()).min(capacity);
    }

    pub fn snapshot(&self) -> Vec<i16> {
        if self.len == 0 {
            return Vec::new();
        }

        let capacity = self.capacity();
        let start = (self.write_index + capacity - self.len) % capacity;
        let mut out = Vec::with_capacity(self.len);
        let first = self.len.min(capacity - start);
        out.extend_from_slice(&self.samples[start..start + first]);
        let remaining = self.len - first;
        if remaining > 0 {
            out.extend_from_slice(&self.samples[..remaining]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::PcmRingBuffer;

    #[test]
    fn empty_snapshot() {
        let ring = PcmRingBuffer::new(4);
        assert!(ring.snapshot().is_empty());
    }

    #[test]
    fn partial_fill_snapshot_is_chronological() {
        let mut ring = PcmRingBuffer::new(5);
        ring.write(&[1, 2, 3]);
        assert_eq!(ring.snapshot(), vec![1, 2, 3]);
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn exact_capacity_snapshot() {
        let mut ring = PcmRingBuffer::new(4);
        ring.write(&[1, 2, 3, 4]);
        assert_eq!(ring.snapshot(), vec![1, 2, 3, 4]);
        assert_eq!(ring.len(), 4);
    }

    #[test]
    fn single_wraparound_preserves_order() {
        let mut ring = PcmRingBuffer::new(4);
        ring.write(&[1, 2, 3]);
        ring.write(&[4, 5]);
        assert_eq!(ring.snapshot(), vec![2, 3, 4, 5]);
    }

    #[test]
    fn repeated_wraparound_preserves_order() {
        let mut ring = PcmRingBuffer::new(4);
        for chunk in [[1, 2], [3, 4], [5, 6], [7, 8]] {
            ring.write(&chunk);
        }
        assert_eq!(ring.snapshot(), vec![5, 6, 7, 8]);
    }

    #[test]
    fn oversized_write_keeps_newest_samples() {
        let mut ring = PcmRingBuffer::new(4);
        ring.write(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(ring.snapshot(), vec![3, 4, 5, 6]);
    }

    #[test]
    fn clear_drops_retained_samples() {
        let mut ring = PcmRingBuffer::new(4);
        ring.write(&[1, 2, 3]);
        ring.clear();
        assert!(ring.snapshot().is_empty());
        assert_eq!(ring.len(), 0);
        ring.write(&[9]);
        assert_eq!(ring.snapshot(), vec![9]);
    }

    #[test]
    fn capacity_stays_bounded_under_long_repeated_writes() {
        let mut ring = PcmRingBuffer::new(16);
        for n in 0..10_000_i16 {
            ring.write(&[n]);
            assert!(ring.len() <= ring.capacity());
        }
        assert_eq!(ring.len(), 16);
        assert_eq!(ring.snapshot(), (9_984_i16..10_000_i16).collect::<Vec<_>>());
    }
}
