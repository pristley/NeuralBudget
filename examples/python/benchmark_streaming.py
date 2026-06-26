#!/usr/bin/env python3
"""
PyO3 Boundary Overhead Benchmark
Measures GIL contention and cross-language call overhead for StreamingAggregator.

Usage:
    python3 examples/python/benchmark_streaming.py
"""

import timeit
import sys
from neuralbudget import StreamingAggregator


def benchmark_push_throughput():
    """Measure push() throughput: 100k samples in tight loop."""
    print("\n=== Benchmark: push() Throughput ===")
    
    setup = """
from neuralbudget import StreamingAggregator
agg = StreamingAggregator()
"""
    
    stmt = """
for i in range(100_000):
    agg.push(i, float(i % 100))
"""
    
    # Run benchmark 1 time (large operation)
    number = 1
    repeat = 3
    
    times = timeit.repeat(stmt, setup=setup, number=number, repeat=repeat)
    avg_time = sum(times) / len(times)
    samples_per_sec = 100_000 / avg_time
    
    print(f"Time for 100k pushes: {avg_time:.4f}s")
    print(f"Throughput: {samples_per_sec:,.0f} samples/sec")
    print(f"Per-sample latency: {(avg_time / 100_000) * 1_000_000:.2f} µs")
    print(f"Runs: {repeat}, each with {number} iteration(s)")
    
    return samples_per_sec >= 20_000  # Pass if >= 20k samples/sec


def benchmark_moving_average_overhead():
    """Measure get_moving_average() call overhead (PyO3 boundary)."""
    print("\n=== Benchmark: get_moving_average() PyO3 Overhead ===")
    
    setup = """
from neuralbudget import StreamingAggregator
agg = StreamingAggregator()
for i in range(10_000):
    agg.push(i, float(i % 100))
"""
    
    # Measure single call overhead
    stmt = """
avg = agg.get_moving_average(9_999, 1_000)
"""
    
    number = 100_000  # High number of iterations for precision
    repeat = 5
    
    times = timeit.repeat(stmt, setup=setup, number=number, repeat=repeat)
    avg_time_per_call = (min(times) / number) * 1_000_000  # Convert to microseconds
    
    print(f"Best time for {number:,} calls: {min(times):.4f}s")
    print(f"Per-call latency: {avg_time_per_call:.2f} µs")
    print(f"Calls per second: {1_000_000 / avg_time_per_call:,.0f}")
    print(f"Runs: {repeat}, each with {number:,} iterations")
    
    return avg_time_per_call < 10.0  # Pass if < 10 µs per call (sub-microsecond GIL overhead)


def benchmark_mixed_workload():
    """Realistic mixed workload: push + periodic get_moving_average."""
    print("\n=== Benchmark: Mixed Workload (realistic) ===")
    
    setup = """
from neuralbudget import StreamingAggregator
agg = StreamingAggregator()
"""
    
    # Simulate realistic workload: push 1000 samples, then query
    stmt = """
for i in range(1_000):
    agg.push(i, float(i % 100))
    if i % 100 == 0:  # Query every 100 pushes
        avg = agg.get_moving_average(i, 100)
"""
    
    number = 1_000  # 1 million total pushes across all runs
    repeat = 3
    
    times = timeit.repeat(stmt, setup=setup, number=number, repeat=repeat)
    avg_time = sum(times) / len(times)
    total_pushes = number * 1_000
    samples_per_sec = total_pushes / avg_time
    
    print(f"Time for {total_pushes:,} pushes (1M) + queries: {avg_time:.4f}s")
    print(f"Throughput: {samples_per_sec:,.0f} samples/sec")
    print(f"Runs: {repeat}, each with {number:,} iterations")
    
    return samples_per_sec >= 20_000  # Pass if >= 20k samples/sec


def benchmark_prune_overhead():
    """Measure prune() memory management overhead."""
    print("\n=== Benchmark: prune() Memory Management ===")
    
    setup = """
from neuralbudget import StreamingAggregator
agg = StreamingAggregator()
for i in range(100_000):
    agg.push(i, float(i % 100))
"""
    
    stmt = """
agg.prune(50_000)
"""
    
    number = 100  # 100 prune operations
    repeat = 3
    
    times = timeit.repeat(stmt, setup=setup, number=number, repeat=repeat)
    avg_time_per_prune = (min(times) / number) * 1_000_000  # Convert to microseconds
    
    print(f"Best time for {number} prune operations: {min(times):.6f}s")
    print(f"Per-prune latency (50k items): {avg_time_per_prune:.2f} µs")
    print(f"Runs: {repeat}, each with {number} iterations")
    
    return avg_time_per_prune < 100.0  # Pass if < 100 µs per prune


def main():
    print("=" * 60)
    print("NeuralBudget StreamingAggregator PyO3 Benchmark Suite")
    print("=" * 60)
    
    results = {
        "Push Throughput (100k samples)": benchmark_push_throughput(),
        "Moving Average Overhead (PyO3)": benchmark_moving_average_overhead(),
        "Mixed Workload (realistic)": benchmark_mixed_workload(),
        "Prune Overhead (memory mgmt)": benchmark_prune_overhead(),
    }
    
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    
    for name, passed in results.items():
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{status}: {name}")
    
    all_passed = all(results.values())
    print("=" * 60)
    
    if all_passed:
        print("✅ All benchmarks passed. StreamingAggregator ready for production.")
        return 0
    else:
        print("❌ Some benchmarks failed. Investigate performance.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
