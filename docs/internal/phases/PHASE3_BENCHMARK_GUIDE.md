# High-Frequency Benchmarks: StreamingAggregator Performance Validation

**Goal:** Validate that `StreamingAggregator` can easily process **20,000+ samples/second** with minimal PyO3 boundary overhead.

---

## Benchmark Suite

### 1. Rust Micro-Benchmarks (`benches/streaming_aggregator.rs`)

Uses **Criterion** for precise measurement with statistical rigor.

#### Tests

| Benchmark | Purpose | Expected Result |
|-----------|---------|-----------------|
| `push_throughput` | Measure insert throughput at 10k and 100k samples | > 20M samples/sec |
| `moving_average` | Measure window query latency (100, 1k, 10k windows) | < 1 µs per call |
| `prune` | Measure memory cleanup overhead | < 1 ms for 50k entries |

#### Running Rust Benchmarks

```bash
# Run all benchmarks
cargo bench --bench streaming_aggregator

# Run specific benchmark
cargo bench --bench streaming_aggregator -- push_throughput

# Run with detailed output
cargo bench --bench streaming_aggregator -- --verbose
```

**Expected Output:**
```
streaming_aggregator/push_throughput/10000
                        time:   [1.5 ms 1.6 ms 1.7 ms]
streaming_aggregator/push_throughput/100000
                        time:   [15.2 ms 15.8 ms 16.4 ms]
```

**Analysis:**
- 100k samples in ~16ms → **6.25M samples/sec** (pure Rust)
- With PyO3 FFI boundary, expect ~20k-50k samples/sec from Python

---

### 2. Python Benchmark Suite (`examples/python/benchmark_streaming.py`)

Uses **timeit** to measure PyO3 boundary overhead and GIL contention.

#### Tests

| Benchmark | Purpose | Expected Result |
|-----------|---------|-----------------|
| `push_throughput` | Python-side push() call throughput (100k samples) | > 20k samples/sec |
| `moving_average_overhead` | Single PyO3 call latency (GIL release cost) | < 10 µs per call |
| `mixed_workload` | Realistic push + periodic query pattern | > 20k samples/sec |
| `prune_overhead` | Memory management from Python side | < 100 µs per prune |

#### Running Python Benchmarks

```bash
# First, build and install the extension
maturin develop --release

# Run Python benchmarks
python3 examples/python/benchmark_streaming.py
```

**Expected Output:**
```
=== Benchmark: push() Throughput ===
Time for 100k pushes: 5.2340s
Throughput: 19,105 samples/sec
Per-sample latency: 52.34 µs

=== Benchmark: get_moving_average() PyO3 Overhead ===
Per-call latency: 3.45 µs

=== Benchmark: Mixed Workload (realistic) ===
Throughput: 24,580 samples/sec

✅ All benchmarks passed. StreamingAggregator ready for production.
```

---

## Performance Baseline

### Expectations

| Layer | Operation | Throughput | Latency |
|-------|-----------|-----------|---------|
| **Rust (native)** | push() | > 6M samples/sec | < 0.2 µs |
| **Rust (native)** | get_moving_average() | > 100k calls/sec | < 10 µs |
| **PyO3 boundary** | push() call | > 20k samples/sec | 50-100 µs |
| **PyO3 boundary** | get_moving_average() call | > 20k calls/sec | 3-10 µs |

### Why the Difference?

- **Python interpreter overhead:** Call stack, argument parsing, GIL acquire/release
- **Type marshalling:** i64, f64 conversion (minimal for primitives)
- **PyO3 setup/teardown:** Object creation for return values
- **GIL contention:** Depends on other Python threads (unlikely in production streaming)

---

## Running the Full Benchmark Suite

```bash
# 1. Ensure you have criterion and maturin installed
cargo install cargo-criterion
python3 -m pip install maturin

# 2. Build Rust and Python extensions
cargo build --release
maturin develop --release

# 3. Run Rust benchmarks
cargo bench --bench streaming_aggregator

# 4. Run Python benchmarks
python3 examples/python/benchmark_streaming.py

# 5. Compare results against baseline expectations
```

---

## Performance Tuning Tips

### If Throughput < 20k samples/sec

1. **Check system load:** Run benchmarks on idle system
2. **Verify release build:** Ensure `--release` flag is used
3. **Profile with perf:**
   ```bash
   cargo build --release --bench streaming_aggregator
   perf record -g ./target/release/deps/streaming_aggregator-*
   perf report
   ```

### If PyO3 Overhead > 10 µs

1. **Check Python version:** Ensure Python 3.9+ (better GIL implementation)
2. **Verify maturin build:** `maturin develop --release`
3. **Check for debuginfo:** Remove debug symbols in Rust compilation

---

## YAGNI Principles Applied

✅ **What We Measure:**
- Throughput (samples/sec)
- Latency (microseconds)
- Memory operation cost

❌ **What We Don't Measure (Yet):**
- Network I/O overhead
- Disk persistence
- Complex telemetry
- Cache behavior
- Multi-threaded contention (assume single-threaded producer)

These can be added when the feature is needed.

---

## Validation Checklist

Before considering `StreamingAggregator` production-ready:

- [ ] Rust benchmarks: All tests complete without error
- [ ] Python benchmarks: All tests pass (4/4 ✅)
- [ ] Throughput: >= 20,000 samples/sec
- [ ] PyO3 call latency: <= 10 µs per call
- [ ] No memory leaks: Monitor with `valgrind` or `heaptrack`
- [ ] CI/CD integration: Benchmarks run on every commit

---

**Status:** Benchmark suite ready. Execute for performance validation.
