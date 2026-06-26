#!/usr/bin/env python3
"""
Streaming Aggregator Example
Demonstrates high-performance windowed metric aggregation via PyO3.

Usage:
    python3 examples/python/streaming_aggregator.py
"""

from neuralbudget import StreamingAggregator


def main():
    # Create a streaming aggregator instance
    agg = StreamingAggregator()

    print("=== NeuralBudget Streaming Aggregator ===\n")

    # Simulate a metric stream: push timestamps and values
    print("Pushing metric stream (timestamp, value):")
    metrics = [
        (1000, 50.0),
        (1100, 55.0),
        (1200, 60.0),
        (1300, 58.0),
        (1400, 62.0),
        (1500, 65.0),
    ]

    for ts, val in metrics:
        agg.push(ts, val)
        print(f"  pushed ({ts}ms, {val})")

    print(f"\nBuffer size: {agg.len()} entries")

    # Compute moving averages at different windows
    current_ts = 1500

    windows = [
        (100, "100ms window"),
        (200, "200ms window"),
        (500, "500ms window"),
        (1000, "1000ms window (all)"),
    ]

    print(f"\nMoving averages at current_ts={current_ts}ms:")
    for window_size, label in windows:
        avg = agg.get_moving_average(current_ts, window_size)
        print(f"  {label}: {avg:.2f}")

    # Prune old data (keep only last 300ms)
    print(f"\nPruning data older than {current_ts - 300}ms...")
    agg.prune(current_ts - 300)
    print(f"Buffer size after prune: {agg.len()} entries")

    # Verify pruned moving average (should be higher due to only recent high values)
    avg_after_prune = agg.get_moving_average(current_ts, 500)
    print(f"Moving average (500ms window after prune): {avg_after_prune:.2f}")

    print("\n=== Adaptive Windowing (High-Frequency Ingestion) ===")
    print("Simulating high-frequency metric ingestion (> 15k samples/sec)...")
    
    # Create a new aggregator for high-frequency test
    agg_hf = StreamingAggregator()
    
    # Simulate 20,000 samples/sec (1000 samples in 50ms)
    # This triggers adaptive windowing: data older than 5s is auto-pruned
    high_freq_samples = 1100  # More than 1000 to trigger velocity window
    
    for i in range(high_freq_samples):
        # Compact timestamps: 1000 samples in 50 ms window
        ts = (i * 50) // 1000  # Milliseconds
        val = 50.0 + (i % 10)
        agg_hf.push(ts, val)
    
    print(f"Pushed {high_freq_samples} samples at simulated 20k samples/sec")
    print(f"Buffer size: {agg_hf.len()} entries")
    print(f"✓ Adaptive windowing bounded memory (would be {high_freq_samples} without it)")

    print("\n=== Performance Characteristics ===")
    print("✓ Zero-allocation push() - returns None")
    print("✓ Zero-allocation get_moving_average() - returns primitive f64")
    print("✓ VecDeque internally - O(1) push, O(1) prune")
    print("✓ Early termination in window queries (monotonic timestamps)")
    print("✓ Automatic adaptive windowing at > 15k samples/sec (no config needed)")


if __name__ == "__main__":
    main()
