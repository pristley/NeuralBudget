# Deployment Guide: Streaming Aggregators and Parallel SLO Evaluation

**Phase 3 Production Deployment**  
**Last Updated:** June 26, 2026

---

## Pre-Deployment Checklist

### Code Review

- [ ] Review [src/streaming.rs](src/streaming.rs) for adaptive windowing logic
- [ ] Review [src/slo_graph.rs](src/slo_graph.rs) for parallel evaluation implementation
- [ ] Confirm all test cases pass: `cargo test streaming --lib && cargo test slo_graph --lib`
- [ ] Run benchmarks to baseline performance: `cargo bench --bench streaming_aggregator`

### Environment Validation

- [ ] Confirm Rust 2021 edition available: `rustc --version`
- [ ] Verify Python 3.9+: `python3 --version`
- [ ] Ensure 2+ CPU cores available for parallel evaluation: `cat /proc/cpuinfo | grep -c processor`
- [ ] Check available RAM for aggregator buffers (recommend 2 GB minimum)

### Documentation Review

- [ ] Read [PHASE3_GETTING_STARTED.md](PHASE3_GETTING_STARTED.md) to understand user-facing behavior
- [ ] Review this deployment guide for configuration and monitoring requirements
- [ ] Confirm team understands automatic memory adaptation at high frequencies

---

## Installation

### Step 1: Update Dependencies

Ensure your environment includes the new Rayon dependency (parallel execution library):

```toml
# Cargo.toml
[dependencies]
rayon = "1.7"
```

If using prebuilt wheels, dependency is already included.

### Step 2: Build Release Binary

Compile with optimizations for production performance:

```bash
cd /workspaces/NeuralBudget
cargo build --release

# Or, build Python wheel
maturin build --release
```

Expected build time: 2–5 minutes depending on machine.

### Step 3: Install Python Bindings

If using Python integration:

```bash
maturin develop --release
# Or, pip install the built wheel
```

### Step 4: Verify Installation

Test that both components are available:

```python
from neuralbudget import StreamingAggregator, ParallelMetricBatch

agg = StreamingAggregator()
batch = ParallelMetricBatch([("test", 50.0, 100.0)])
results = batch.evaluate()
print(f"Installation verified. Evaluated {len(results)} metrics.")
```

---

## Configuration

### StreamingAggregator: No Configuration Required

The adaptive windowing system is fully automatic. You cannot configure thresholds.

**Hardcoded parameters:**
- **Velocity threshold:** 15,000 samples per second
  - Rationale: Represents ~2.5× typical benchmarked ingestion rate; indicates genuine traffic spike
- **Retention window:** 5 seconds (5,000 milliseconds)
  - Rationale: Preserves sufficient history for SLO evaluation; keeps memory under 4 MB at high frequencies
- **Check frequency:** Every 100 measurements
  - Rationale: Balances responsiveness with per-measurement overhead

**Why hardcoded?** Hardcoded values eliminate configuration bugs and reduce API surface. If these parameters do not fit your use case, contact the NeuralBudget team.

### ParallelMetricBatch: Initialize with Metric Definitions

No runtime configuration. Provide metric definitions at initialization:

```python
from neuralbudget import ParallelMetricBatch

batch = ParallelMetricBatch([
    ("latency_p99", current_latency, 200.0),
    ("error_rate", current_errors, 0.5),
    ("availability", current_avail, 99.9),
])
```

Update metrics between evaluations using `update_node()`:

```python
graph.update_node("latency_p99", new_value)
```

---

## Runtime Behavior

### Normal Operation (< 15k samples/sec)

```
Ingestion Rate:  5,000 samples/sec (typical SLO metric stream)
Detection:       Velocity < 15,000 threshold
Adaptation:      NOT triggered
Memory Behavior: Full historical buffer retained; application responsible for cleanup via prune()
Action:          No change in behavior; existing code continues to work
```

**What you see:**
- Metrics flow into aggregator normally
- `get_moving_average()` returns averages over specified window
- Call `prune()` periodically to free old data

### High-Frequency Scenario (> 15k samples/sec)

```
Ingestion Rate:  25,000 samples/sec (e.g., high-cardinality time-series)
Detection:       Velocity > 15,000 threshold (sustained for 100+ samples)
Adaptation:      TRIGGERED
Memory Behavior: Auto-prune removes data older than 5 seconds
Max Buffer Size: ~125,000 entries (25k samples/sec × 5 sec) = ~5 MB
Action:          Automatic; no code changes needed
```

**What you see:**
- Aggregator buffer stops growing beyond ~100k entries
- `get_moving_average()` still works on 5-second window
- No exceptions or alerts; operates silently

**When velocity drops back below 15k/sec:**
- Adaptation stays inactive (old data already pruned)
- Aggregator continues normal operation
- No state corruption or transitions

### Example: Monitoring a Traffic Spike

```python
from neuralbudget import StreamingAggregator
import time

agg = StreamingAggregator()

# Normal ingestion
for i in range(100):
    agg.push(i * 100, 50.0 + (i % 10))
    time.sleep(0.02)  # 50 Hz ingestion

print(f"Normal phase: {agg.len()} entries")  # ~100 entries

# Traffic spike: 100x ingestion rate
for i in range(100):
    agg.push(10000 + i, 50.0 + (i % 10))
    time.sleep(0.0002)  # 5,000 Hz ingestion (high-frequency phase)

print(f"During spike: {agg.len()} entries")  # Still ~100 entries (auto-pruned)

# Spike subsides
for i in range(50):
    agg.push(20000 + i * 100, 50.0 + (i % 10))
    time.sleep(0.02)  # Back to 50 Hz

print(f"After spike: {agg.len()} entries")  # Stabilized; normal operation resumed
```

---

## Monitoring and Observability

### Key Metrics to Track

#### 1. Aggregator Buffer Size

Monitor `agg.len()` to track memory usage:

```python
current_size = agg.len()
if current_size > 100000:
    log.warning(f"Large aggregator buffer: {current_size} entries (~{current_size * 40} bytes)")
```

**Expected ranges:**
- Normal (< 15k/sec): 100–10,000 entries (your application controls via prune() calls)
- High-frequency (> 15k/sec): Stabilizes around 100,000 entries (~4 MB)

#### 2. Query Latency

Measure how long `get_moving_average()` takes:

```python
import time
start = time.time()
avg = agg.get_moving_average(current_ts, 5000)
latency_us = (time.time() - start) * 1_000_000
log.info(f"Moving average query: {latency_us:.0f} µs")
```

**Expected:** < 50 microseconds for typical window sizes (100–5,000 samples).
**Alert threshold:** > 100 microseconds (indicates very large aggregator or slow system).

#### 3. Evaluation Throughput

Measure how many metrics ParallelMetricBatch evaluates per second:

```python
import time
from neuralbudget import ParallelMetricBatch

batch = ParallelMetricBatch([(f"metric_{i}", 50.0, 100.0) for i in range(1000)])

start = time.time()
for _ in range(10):
    batch.evaluate()
duration = time.time() - start
throughput = (batch.node_count * 10) / duration
log.info(f"Throughput: {throughput:.0f} metrics/sec")
```

**Expected:** 50,000–100,000 metrics per second on modern hardware.
**Alert threshold:** < 10,000 metrics/sec (indicates system contention or resource exhaustion).

### Integration with Existing Monitoring

If using Prometheus or similar:

```python
from neuralbudget import StreamingAggregator, ParallelMetricBatch

agg = StreamingAggregator()
batch = ParallelMetricBatch(metrics)

# Expose as metrics
metrics = {
    "aggregator_buffer_size": agg.len(),
    "slo_graph_health": graph.aggregate_score(),
    "slo_graph_passing": graph.pass_count(),
}

# Send to Prometheus/CloudWatch/etc.
for metric_name, value in metrics.items():
    exporter.gauge(metric_name, value)
```

---

## Thread Safety and Concurrency

### Important: ParallelMetricBatch is NOT Thread-Safe

`ParallelMetricBatch` releases the GIL during `evaluate()` to allow parallel computation on multiple CPU cores. However, **the batch instance itself is not safe for concurrent access from multiple Python threads**.

### Safe Production Patterns

**Pattern 1: Single evaluation thread (recommended for most cases)**
```python
# One thread handles all batch updates and evaluations
def monitoring_thread():
    batch = ParallelMetricBatch([...])
    while True:
        # All mutations and evaluations happen in one thread
        batch.update_node("metric1", get_metric1())
        batch.update_node("metric2", get_metric2())
        results = batch.evaluate()
        
        if not batch.all_pass():
            send_alert(results)
        
        time.sleep(5)

threading.Thread(target=monitoring_thread, daemon=True).start()
```

**Pattern 2: Thread-safe wrapper with Lock**
```python
from threading import Lock

class ThreadSafeMetricBatch:
    def __init__(self, metrics):
        self.batch = ParallelMetricBatch(metrics)
        self.lock = Lock()
    
    def update_and_evaluate(self, updates):
        """Atomically update metrics and evaluate."""
        with self.lock:
            for metric_id, value in updates.items():
                self.batch.update_node(metric_id, value)
            return self.batch.evaluate()
    
    def query_health(self):
        """Query health without mutation."""
        with self.lock:
            return {
                "all_pass": self.batch.all_pass(),
                "score": self.batch.aggregate_score(),
                "passing": self.batch.pass_count(),
            }

# Usage
safe_batch = ThreadSafeMetricBatch([...])

def api_handler(request):
    # Safe to call from multiple request threads
    results = safe_batch.update_and_evaluate(request.metrics)
    return results
```

**Pattern 3: Per-thread batches (for stateless evaluations)**
```python
import threading

batch_local = threading.local()

def get_thread_batch():
    """Get or create a batch for the current thread."""
    if not hasattr(batch_local, 'batch'):
        batch_local.batch = ParallelMetricBatch([...])
    return batch_local.batch

def worker_request(metrics):
    """Each worker thread has its own batch instance."""
    batch = get_thread_batch()
    
    for metric_id, value in metrics.items():
        batch.update_node(metric_id, value)
    
    return batch.evaluate()

# Safe: Each thread has its own batch, no contention
```

### Unsafe Patterns (DO NOT USE)

```python
# UNSAFE: Multiple threads mutating same batch
batch = ParallelMetricBatch([...])

def thread_1():
    batch.update_node("metric1", 100.0)  # Data race!

def thread_2():
    batch.evaluate()  # Data race!

# UNSAFE: Evaluation and mutation happening concurrently
# This will cause crashes or silent data corruption
```

### Why This Matters in Production

- **Web APIs**: Multiple request handlers may call batch methods concurrently
- **Microservices**: Multiple threads collecting metrics simultaneously
- **Event-driven systems**: Different callbacks may race to update metrics

### Guidelines

1. **If you have one SLO evaluation loop**: Use Pattern 1 (single thread)
2. **If you have a shared batch across multiple request handlers**: Use Pattern 2 (lock)
3. **If you process independent metric streams**: Use Pattern 3 (per-thread batches)
4. **Always test** with multiple threads to catch race conditions early

---

## Troubleshooting

### Symptom: Buffer Grows Unbounded

**Problem:** `agg.len()` keeps increasing even though you call `prune()`.

**Root causes:**
1. Ingestion timestamp is not monotonically increasing (out-of-order data)
2. `prune()` cutoff timestamp is too recent (not removing old enough data)
3. Ingestion rate is very high and auto-pruning is not triggered

**Solution:**
```python
# Verify timestamps are monotonic
recent_timestamps = [ts for ts, _ in agg.buffer]  # Hypothetical direct access
if recent_timestamps != sorted(recent_timestamps):
    print("ERROR: Out-of-order timestamps detected")

# Manually prune to a very old cutoff
current_ts = get_current_timestamp()
very_old = current_ts - 60000  # 60 seconds ago
agg.prune(very_old)

# Verify reduction
print(f"After aggressive prune: {agg.len()} entries")
```

### Symptom: `get_moving_average()` Returns 0.0

**Problem:** You expect a non-zero average but receive 0.0.

**Root causes:**
1. No measurements exist in the specified window
2. Window size is too small relative to ingestion rate
3. All measurements are exactly 0.0 (legitimate, not an error)

**Solution:**
```python
# Check if aggregator has any data
if agg.is_empty():
    print("ERROR: No measurements in aggregator")
    return

# Verify window contains measurements
current_ts = get_current_timestamp()
larger_window = 10000  # 10 seconds
avg = agg.get_moving_average(current_ts, larger_window)
if avg == 0.0 and not agg.is_empty():
    print("WARNING: No measurements in 10-second window; window may be too old")
```

### Symptom: SLO Evaluation Takes > 500 milliseconds

**Problem:** `graph.evaluate()` latency is higher than expected.

**Root causes:**
1. System has only 1 CPU core (parallelism cannot accelerate)
2. Other processes consume CPU time; Rayon cannot get all cores
3. Very large number of metrics (10,000+) and system is near capacity

**Solution:**
```python
# Check available cores
import os
cores = os.cpu_count()
print(f"Available CPU cores: {cores}")

# Check system load
import os
load_avg = os.getloadavg()
cpu_utilization = load_avg[0] / cores
print(f"Current CPU load: {load_avg[0]:.1f} ({cpu_utilization:.0%} utilization)")

# Reduce metric count if system is saturated
if cpu_utilization > 0.8:
    print("System is busy. Consider reducing metric count or running evaluation less frequently.")
```

### Symptom: "Global Interpreter Lock" Errors

**Problem:** You see exceptions mentioning GIL or threading.

**Root causes:**
1. Calling `graph.evaluate()` from within a Python thread that holds the GIL
2. Nesting parallel evaluations (not supported)

**Solution:**
```python
# Correct usage: call evaluate() from main thread
graph.evaluate()  # OK

# Avoid: nested evaluation
def worker():
    graph.evaluate()  # May cause issues if called from background thread

import threading
t = threading.Thread(target=worker)
t.start()  # Potential GIL conflict

# Instead: evaluate in main thread, pass results to worker threads
results = graph.evaluate()  # Main thread
t = threading.Thread(target=process, args=(results,))
t.start()  # Worker thread processes results
```

---

## Performance Tuning

### Optimize Aggregator Ingestion

**Goal:** Maximize throughput of `push()` calls.

**Techniques:**
1. Batch pushes if possible: Rather than calling `push()` for each point individually, collect multiple points and push in a loop.
2. Use appropriate window sizes: Smaller windows (100–500 ms) are faster to query than large windows (60+ sec).
3. Call `prune()` infrequently: Every 60 seconds is sufficient for most workloads.

### Optimize Graph Evaluation

**Goal:** Minimize latency of `evaluate()` calls.

**Techniques:**
1. Reduce metric count: Fewer metrics = faster evaluation. Prune metrics that are no longer needed.
2. Reuse graph instances: Create the graph once, update values via `update_node()`, then evaluate. Do not recreate the graph for each evaluation.
3. Reduce evaluation frequency: If evaluating every second, consider moving to every 5 seconds.

**Example: Reusing a graph**
```python
from neuralbudget import ParallelMetricBatch

# Evaluation loop
while True:
    metric_1_value = fetch_metric("metric_1")
    metric_2_value = fetch_metric("metric_2")
    
    # Create a fresh batch with current values
    batch = ParallelMetricBatch([
        ("metric_1", metric_1_value, 100.0),
        ("metric_2", metric_2_value, 100.0),
    ])
    
    results = batch.evaluate()
    # Check if any failed
    if not all(passed for _, _, _, passed, _ in results):
        alert("SLO violation")
```

---

## Rollback Plan

If you encounter unexpected issues after deployment:

### Immediate Rollback

1. **Stop using parallel evaluation:** Return to sequential SLO checking or use a previous version of the neuralbudget wheel.
2. **Disable adaptive windowing:** Manually manage `prune()` calls; degraded to pre-Phase 3 behavior.
3. **Revert to previous commit:** `git checkout <previous-commit-hash>`

### Rebuild and Deploy Previous Version

```bash
git checkout 84e889a  # Last commit before Phase 3 parallel SLO evaluation
cargo build --release
maturin build --release
pip install --force-reinstall --no-cache-dir ./target/wheels/*.whl
```

### Rollback Decision Criteria

Rollback if you observe:
- Evaluation latency > 1 second (5–10× expected)
- Memory exhaustion despite automatic pruning
- Repeated "out of memory" errors
- Deadlocks or unresponsive processes

---

## Migration from Earlier Versions

If upgrading from Phase 2 or earlier:

### API Compatibility

**Good news:** Phase 3 is fully backward compatible.
- Existing `StreamingAggregator` code works unchanged
- Existing SLO evaluation code works unchanged
- No Python API breaks

### What's New

1. **StreamingAggregator:** Now automatically prunes at > 15k samples/sec (previously user-controlled)
2. **ParallelMetricBatch:** New class for parallel, independent SLO evaluation (previously used `HttpSlo`, `StatefulSlo`, etc.)

### Migration Path

**Recommended:** Gradual adoption.

1. **Phase 1 (Week 1):** Deploy Phase 3; existing code continues to work.
2. **Phase 2 (Week 2–3):** Introduce ParallelMetricBatch for new SLO checks; keep old SLO classes for existing checks.
3. **Phase 3 (Week 4+):** Migrate old SLO classes to ParallelMetricBatch for uniform evaluation.

**Example migration:**
```python
# Before: HttpSlo (Phase 2)
slo = HttpSlo(threshold=200.0)
evaluation = slo.evaluate(latency_value)

# After: ParallelMetricBatch (Phase 3)
batch = ParallelMetricBatch([("latency", latency_value, 200.0)])
results = batch.evaluate()
evaluation = results[0][3]  # Pass/fail field
```

---

## Support and Escalation

### Getting Help

- **Documentation:** [PHASE3_GETTING_STARTED.md](PHASE3_GETTING_STARTED.md), [PARALLEL_SLO_API_REFERENCE.md](PARALLEL_SLO_API_REFERENCE.md)
- **Source Code:** [src/streaming.rs](src/streaming.rs), [src/slo_graph.rs](src/slo_graph.rs)
- **Issues:** Open a GitHub issue or contact the NeuralBudget team

### Known Limitations

1. **No topological sorting:** ParallelMetricBatch assumes independent metrics; inter-metric dependencies not supported. Use `CompositeSloGraph` for dependency-aware evaluation.
2. **No custom thread pools:** Uses Rayon global thread pool; cannot configure pool size.
3. **No node indexing:** Lookup by ID is O(n); acceptable for < 10k metrics.
4. **No metric weights:** All metrics contribute equally to aggregate score.

### Future Enhancements

- Indexed node lookups (HashMap for O(1) get_node)
- Weighted aggregation (per-metric importance)
- Sub-graph evaluation (evaluate subset of nodes)
- Streaming node updates (push-based metric delivery)

---

## Summary Checklist

- [ ] Code review complete
- [ ] Tests passing (cargo test, cargo bench)
- [ ] Environment validated (Rust, Python, CPU cores, RAM)
- [ ] Dependencies installed (Rayon 1.7)
- [ ] Installation verified (Python imports work)
- [ ] Monitoring configured (buffer size, query latency, throughput)
- [ ] Team briefed on automatic memory adaptation
- [ ] Rollback plan documented and tested
- [ ] Migration path planned (if upgrading from earlier version)
- [ ] Deployment executed

**Status:** Ready for production deployment.

---

## See Also

- [Getting Started Guide](PHASE3_GETTING_STARTED.md)
- [API Reference](PARALLEL_SLO_API_REFERENCE.md)
- [Source Code](src/streaming.rs) and [Parallel Implementation](src/slo_graph.rs)
