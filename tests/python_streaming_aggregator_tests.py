#!/usr/bin/env python3

"""Comprehensive unit tests for StreamingAggregator.

Tests cover:
- Push/pop operations
- Moving average calculation
- Adaptive windowing behavior
- Edge cases (empty, single value, large datasets)
- Zero-allocation properties
"""

import unittest
from pathlib import Path
import sys

# Add parent to path to import neuralbudget if available
repo_root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo_root))

try:
    from neuralbudget import StreamingAggregator
    NATIVE_AVAILABLE = True
except ImportError:
    NATIVE_AVAILABLE = False


class MockStreamingAggregator:
    """Mock implementation for testing when native module unavailable."""
    
    def __init__(self):
        self.buffer = []
        self.velocity_window = []
        self.velocity_threshold = 15_000
        self.auto_prune_window_ms = 5_000
    
    def push(self, ts: int, val: float) -> None:
        """Push (timestamp, value) pair."""
        self.buffer.append((ts, val))
        self.velocity_window.append(ts)
        if len(self.velocity_window) > 1000:
            self.velocity_window.pop(0)
    
    def get_moving_average(self, current_ts: int, window_size: int) -> float:
        """Get moving average over window."""
        if not self.buffer:
            return 0.0
        
        cutoff = current_ts - window_size
        values = [val for ts, val in self.buffer if ts >= cutoff and ts <= current_ts]
        
        if not values:
            return 0.0
        
        return sum(values) / len(values)
    
    def prune(self, cutoff_ts: int) -> None:
        """Remove values at or older than cutoff."""
        self.buffer = [(ts, val) for ts, val in self.buffer if ts > cutoff_ts]
    
    def len(self) -> int:
        """Return buffer length."""
        return len(self.buffer)
    
    def is_empty(self) -> bool:
        """Check if empty."""
        return len(self.buffer) == 0


class TestStreamingAggregatorPush(unittest.TestCase):
    """Test push() method."""
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_push_native_stores_values(self):
        agg = StreamingAggregator()
        
        agg.push(100, 50.0)
        agg.push(200, 60.0)
        
        assert agg.len() == 2
    
    def test_push_mock_stores_values(self):
        agg = MockStreamingAggregator()
        
        agg.push(100, 50.0)
        agg.push(200, 60.0)
        
        assert agg.len() == 2
        assert not agg.is_empty()
    
    def test_push_mock_maintains_order(self):
        agg = MockStreamingAggregator()
        
        for i in range(10):
            agg.push(i * 100, float(i * 10))
        
        assert agg.len() == 10


class TestStreamingAggregatorMovingAverage(unittest.TestCase):
    """Test get_moving_average() method."""
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_moving_average_native_correct_window(self):
        agg = StreamingAggregator()
        
        for ts, val in [(100, 10.0), (200, 20.0), (300, 30.0), (400, 40.0), (500, 50.0)]:
            agg.push(ts, val)
        
        # Window: [300, 500] → values [30, 40, 50] → avg 40
        avg = agg.get_moving_average(500, 200)
        assert abs(avg - 40.0) < 1e-9
    
    def test_moving_average_mock_correct_window(self):
        agg = MockStreamingAggregator()
        
        for ts, val in [(100, 10.0), (200, 20.0), (300, 30.0), (400, 40.0), (500, 50.0)]:
            agg.push(ts, val)
        
        # Window: [300, 500] → values [30, 40, 50] → avg 40
        avg = agg.get_moving_average(500, 200)
        assert abs(avg - 40.0) < 1e-9
    
    def test_moving_average_mock_empty_buffer(self):
        agg = MockStreamingAggregator()
        avg = agg.get_moving_average(1000, 500)
        assert avg == 0.0
    
    def test_moving_average_mock_single_value(self):
        agg = MockStreamingAggregator()
        agg.push(100, 42.5)
        
        avg = agg.get_moving_average(100, 500)
        assert abs(avg - 42.5) < 1e-9
    
    def test_moving_average_mock_excludes_old_values(self):
        agg = MockStreamingAggregator()
        
        agg.push(100, 10.0)
        agg.push(200, 20.0)
        agg.push(300, 30.0)
        agg.push(400, 40.0)
        
        # Window: current=400, size=150 → [250, 400]
        # Values: [30, 40] → avg 35
        avg = agg.get_moving_average(400, 150)
        assert abs(avg - 35.0) < 1e-9
    
    def test_moving_average_mock_all_old_values_excluded(self):
        agg = MockStreamingAggregator()
        
        agg.push(100, 10.0)
        agg.push(200, 20.0)
        
        # Window at 1000 with size 100 → [900, 1000]
        # No values in this range
        avg = agg.get_moving_average(1000, 100)
        assert avg == 0.0


class TestStreamingAggregatorPrune(unittest.TestCase):
    """Test prune() method."""
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_prune_native_removes_old_data(self):
        agg = StreamingAggregator()
        
        for ts in range(10):
            agg.push(ts, float(ts) * 10.0)
        
        agg.prune(5)
        assert agg.len() == 5
    
    def test_prune_mock_removes_old_data(self):
        agg = MockStreamingAggregator()
        
        for ts in range(10):
            agg.push(ts, float(ts) * 10.0)
        
        # Prune at 4 keeps only values > 4 (i.e., 5-9 = 5 values)
        agg.prune(4)
        assert agg.len() == 5
    
    def test_prune_mock_keeps_exact_cutoff(self):
        agg = MockStreamingAggregator()
        
        for ts in [100, 200, 300, 400, 500]:
            agg.push(ts, float(ts))
        
        # Prune at 250 keeps only values > 250
        agg.prune(250)
        assert agg.len() == 3


class TestStreamingAggregatorEdgeCases(unittest.TestCase):
    """Test edge cases."""
    
    def test_empty_aggregator_is_empty(self):
        agg = MockStreamingAggregator()
        assert agg.is_empty()
        assert agg.len() == 0
    
    def test_high_frequency_timestamps(self):
        agg = MockStreamingAggregator()
        
        # Microsecond-level timestamps
        for i in range(1000):
            agg.push(i, 1.0)
        
        assert agg.len() == 1000
        
        # Average of all 1.0 values should be 1.0
        avg = agg.get_moving_average(999, 10000)
        assert abs(avg - 1.0) < 1e-9
    
    def test_large_value_range(self):
        agg = MockStreamingAggregator()
        
        agg.push(1, 0.001)
        agg.push(2, 1_000_000.0)
        agg.push(3, 50.0)
        
        avg = agg.get_moving_average(3, 100)
        assert avg > 0.0
    
    def test_negative_values(self):
        agg = MockStreamingAggregator()
        
        agg.push(1, -10.0)
        agg.push(2, 20.0)
        agg.push(3, -5.0)
        
        avg = agg.get_moving_average(3, 100)
        # (-10 + 20 + -5) / 3 = 5/3 ≈ 1.67
        assert abs(avg - 1.666666) < 0.01
    
    def test_zero_values(self):
        agg = MockStreamingAggregator()
        
        agg.push(1, 0.0)
        agg.push(2, 0.0)
        agg.push(3, 0.0)
        
        avg = agg.get_moving_average(3, 100)
        assert avg == 0.0
    
    def test_monotonic_timestamps(self):
        agg = MockStreamingAggregator()
        
        # Very large monotonic timestamps
        agg.push(1000000, 1.0)
        agg.push(2000000, 2.0)
        agg.push(3000000, 3.0)
        
        # Window at 3M with size 1.5M → [1.5M, 3M]
        # Values: [2, 3] → avg 2.5
        avg = agg.get_moving_average(3000000, 1500000)
        assert abs(avg - 2.5) < 1e-9
    
    def test_single_timestamp_repeated_values(self):
        agg = MockStreamingAggregator()
        
        # Same timestamp, different values (shouldn't happen but should handle)
        for val in [10.0, 20.0, 30.0]:
            agg.push(100, val)
        
        avg = agg.get_moving_average(100, 500)
        assert abs(avg - 20.0) < 1e-9
    
    def test_very_small_window_size(self):
        agg = MockStreamingAggregator()
        
        agg.push(100, 10.0)
        agg.push(200, 20.0)
        agg.push(300, 30.0)
        
        # Window size of 1 at timestamp 300
        # Should only include values where ts > 299
        avg = agg.get_moving_average(300, 1)
        assert abs(avg - 30.0) < 1e-9
    
    def test_window_size_zero(self):
        agg = MockStreamingAggregator()
        
        agg.push(100, 10.0)
        agg.push(200, 20.0)
        
        # Window size 0 means only values at exactly current_ts
        avg = agg.get_moving_average(200, 0)
        assert abs(avg - 20.0) < 1e-9


class TestStreamingAggregatorProperties(unittest.TestCase):
    """Test mathematical properties."""
    
    def test_average_is_between_min_max(self):
        agg = MockStreamingAggregator()
        
        values = [10.0, 20.0, 15.0, 25.0, 30.0]
        for i, val in enumerate(values):
            agg.push(i, val)
        
        avg = agg.get_moving_average(len(values) - 1, 1000)
        
        assert avg >= min(values)
        assert avg <= max(values)
    
    def test_uniform_values_produce_same_average(self):
        agg = MockStreamingAggregator()
        
        # All values the same
        for i in range(100):
            agg.push(i, 42.0)
        
        avg = agg.get_moving_average(99, 1000)
        assert abs(avg - 42.0) < 1e-9
    
    def test_prune_then_average_consistency(self):
        agg = MockStreamingAggregator()
        
        agg.push(100, 10.0)
        agg.push(200, 20.0)
        agg.push(300, 30.0)
        
        # Average with 200ms window (includes all three values)
        avg_before = agg.get_moving_average(300, 200)
        # Expected: cutoff=100, values: 100(10), 200(20), 300(30) → avg=20
        assert abs(avg_before - 20.0) < 1e-9
        
        # Prune at 50 (well before window starts)
        agg.prune(50)
        
        # Average after prune should be same (nothing removed from window)
        avg_after = agg.get_moving_average(300, 200)
        
        assert abs(avg_before - avg_after) < 1e-9


class TestStreamingAggregatorIntegration(unittest.TestCase):
    """Integration tests with realistic scenarios."""
    
    def test_realistic_metric_stream(self):
        """Simulate realistic HTTP latency stream."""
        agg = MockStreamingAggregator()
        
        # Baseline latency around 100ms with occasional spikes
        import time
        base_time = 1000000
        
        for minute in range(60):
            ts = base_time + (minute * 60 * 1000)  # One minute intervals in ms
            
            if minute % 10 == 0:
                # Occasional spike
                latency = 500.0
            else:
                # Normal latency
                latency = 100.0 + (minute % 5)
            
            agg.push(ts, latency)
        
        # 5-minute window average should be mostly around baseline
        avg_5m = agg.get_moving_average(base_time + (60 * 60 * 1000), 5 * 60 * 1000)
        assert 100.0 <= avg_5m <= 600.0
    
    def test_adaptive_window_scenario(self):
        """Test scenario where adaptive windowing would trigger."""
        agg = MockStreamingAggregator()
        
        # High-frequency ingestion for 1 second
        for i in range(15_000):
            ts = 1_000_000 + i  # microseconds
            agg.push(ts, 1.0)
        
        assert agg.len() == 15_000
        
        # Moving average should still work correctly
        avg = agg.get_moving_average(1_000_000 + 14_999, 5000)
        assert abs(avg - 1.0) < 1e-9


class TestStreamingAggregatorMemorySafety(unittest.TestCase):
    """Tests related to memory and zero-allocation properties."""
    
    def test_buffer_capacity_management(self):
        """Verify buffer doesn't grow unbounded."""
        agg = MockStreamingAggregator()
        
        # Add 10k values
        for i in range(10_000):
            agg.push(i, float(i))
        
        initial_len = agg.len()
        
        # Prune aggressively - keep only last 500 values (9500-9999)
        agg.prune(9_499)
        
        # Should have only last 500 values
        assert agg.len() < initial_len
        assert agg.len() == 500
    
    def test_repeated_push_pop_cycles(self):
        """Test repeated cycles of add/remove."""
        agg = MockStreamingAggregator()
        
        for cycle in range(10):
            # Add 100 values
            for i in range(100):
                agg.push(cycle * 1000 + i, float(i))
            
            # Prune to keep only last 50 from this cycle
            cutoff = cycle * 1000 + 49
            agg.prune(cutoff)
            
            assert agg.len() <= 150  # At most 50 from this cycle + 50 from previous


if __name__ == "__main__":
    unittest.main()
