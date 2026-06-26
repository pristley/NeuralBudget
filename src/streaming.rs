use pyo3::prelude::*;
use std::collections::VecDeque;

/// Minimalist windowed aggregator for streaming metric evaluation.
/// Assumes monotonically increasing timestamps.
/// Zero-allocation in hot paths: push() and get_moving_average() return primitives.
#[pyclass]
pub struct StreamingAggregator {
    /// (timestamp, value) pairs in insertion order.
    /// Oldest data at front, newest at back.
    buffer: VecDeque<(i64, f64)>,
}

#[pymethods]
impl StreamingAggregator {
    /// Create a new streaming aggregator.
    #[new]
    pub fn new() -> Self {
        StreamingAggregator {
            buffer: VecDeque::with_capacity(1024), // Reasonable default; no allocation overhead
        }
    }

    /// Push a (timestamp, value) pair into the window.
    /// Assumes timestamps are monotonically increasing.
    pub fn push(&mut self, ts: i64, val: f64) {
        self.buffer.push_back((ts, val));
    }

    /// Compute moving average of values within [current_ts - window_size, current_ts].
    /// Returns the average as f64 (primitive, no PyObject allocation).
    pub fn get_moving_average(&self, current_ts: i64, window_size: i64) -> f64 {
        if self.buffer.is_empty() {
            return 0.0;
        }

        let cutoff_ts = current_ts - window_size;
        let mut sum = 0.0;
        let mut count = 0;

        // Iterate from back (most recent) to front, collecting values in window.
        // Stop early once we exit the window (monotonic assumption).
        for (ts, val) in self.buffer.iter().rev() {
            if *ts > cutoff_ts {
                sum += val;
                count += 1;
            } else {
                break; // Timestamps are ordered; no values before cutoff exist
            }
        }

        if count == 0 {
            0.0
        } else {
            sum / count as f64
        }
    }

    /// Remove all values older than cutoff timestamp to keep buffer bounded.
    /// Call this periodically to manage memory.
    pub fn prune(&mut self, cutoff_ts: i64) {
        while let Some((ts, _)) = self.buffer.front() {
            if *ts <= cutoff_ts {
                self.buffer.pop_front();
            } else {
                break;
            }
        }
    }

    /// Return the current number of buffered (timestamp, value) pairs.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for StreamingAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_moving_average() {
        let mut agg = StreamingAggregator::new();

        // Push 5 values: (1, 10.0), (2, 20.0), (3, 30.0), (4, 40.0), (5, 50.0)
        for (ts, val) in &[(1, 10.0), (2, 20.0), (3, 30.0), (4, 40.0), (5, 50.0)] {
            agg.push(*ts, *val);
        }

        // Window of 2 seconds from ts=5: includes (5, 50.0) and (4, 40.0)
        // Average = (50.0 + 40.0) / 2 = 45.0
        assert_eq!(agg.get_moving_average(5, 2), 45.0);

        // Window of 1 second from ts=5: includes (5, 50.0) only
        // Average = 50.0
        assert_eq!(agg.get_moving_average(5, 1), 50.0);

        // Window of 10 seconds from ts=5: includes all 5 values
        // Average = (10 + 20 + 30 + 40 + 50) / 5 = 30.0
        assert_eq!(agg.get_moving_average(5, 10), 30.0);
    }

    #[test]
    fn test_prune() {
        let mut agg = StreamingAggregator::new();

        for (ts, val) in &[(1, 10.0), (2, 20.0), (3, 30.0), (4, 40.0), (5, 50.0)] {
            agg.push(*ts, *val);
        }

        assert_eq!(agg.len(), 5);

        // Prune values with ts <= 2
        agg.prune(2);
        assert_eq!(agg.len(), 3); // (3, 30.0), (4, 40.0), (5, 50.0) remain

        // Window of 1 second from ts=5 should now return 50.0 (unchanged)
        assert_eq!(agg.get_moving_average(5, 1), 50.0);
    }

    #[test]
    fn test_empty_buffer() {
        let agg = StreamingAggregator::new();
        assert!(agg.is_empty());
        assert_eq!(agg.len(), 0);
        assert_eq!(agg.get_moving_average(100, 10), 0.0);
    }
}
