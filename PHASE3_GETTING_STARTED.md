# Phase 3: Streaming Aggregators and Parallel Metric Batch Evaluation

Use the `StreamingAggregator` and `ParallelMetricBatch` classes to handle high-frequency metric collection and multi-core SLO evaluation in NeuralBudget.

**What you'll learn:**
- Collect metrics at 20,000+ samples per second
- Evaluate 100,000 SLO nodes in 150–250 milliseconds
- Automatically bound memory during traffic spikes

---

## Overview: What These Components Do

**StreamingAggregator** receives individual metric measurements and computes windowed averages (such as latency or error rates over the past 5 seconds).

**ParallelMetricBatch** takes a list of current metrics and checks whether each one meets its threshold (for example, latency < 200 ms). Unlike `CompositeSloGraph`, it does not model dependencies — each metric is evaluated independently. It uses all available CPU cores to evaluate metrics in parallel.

Together, these components enable you to evaluate service-level objectives at scale in real time.

### Why Use Parallel Evaluation?

Sequential evaluation of 1,000 metrics takes ~50 milliseconds on one CPU core. Parallel evaluation on 8 cores takes ~15 milliseconds. When you run evaluations every 5 seconds, this difference frees up CPU time for other work.

---

## Task 1: Collect Metrics with Automatic Memory Management

Use `StreamingAggregator` to accept individual metric points and retrieve windowed averages.

### Create an Aggregator

```python
from neuralbudget import StreamingAggregator

agg = StreamingAggregator()
```

### Add Metrics

Call `push()` once per metric measurement. Provide the timestamp (in milliseconds) and the value:

```python
agg.push(1000, 50.0)  # timestamp=1000ms, value=50.0
agg.push(1050, 52.0)
agg.push(1100, 51.0)
```

The aggregator stores each point in a queue. Processing speed: 20,000 metrics per second through Python's C interface.

### Retrieve Windowed Averages

Ask for the average over the last N milliseconds:

```python
window_ms = 100  # Average over the past 100 milliseconds
current_ts = 1100  # Current time
avg = agg.get_moving_average(current_ts, window_ms)  # Returns: 51.33...
```

This method returns a single number (average), not a Python object, so retrieval is fast (~10 microseconds per call).

### Clean Up Old Data

Remove data older than a cutoff timestamp to free memory:

```python
agg.prune(900)  # Remove all points before timestamp 900
```

#### Automatic Cleanup During Traffic Spikes

When metrics arrive faster than 15,000 per second for a sustained period, the aggregator automatically removes data older than 5 seconds. This prevents unbounded memory growth.

**You do not configure this behavior.** It activates automatically when the system detects high ingestion velocity.

**Example memory bounds:**
- Normal rate (< 15,000 samples/sec): Your application controls cleanup via `prune()` calls
- Traffic spike (> 15,000 samples/sec): Automatic pruning keeps buffer under 4 MB (roughly 100,000 entries)

---

## Task 2: Evaluate SLO Metrics in Parallel

Use `ParallelMetricBatch` to check whether a set of metrics pass their respective thresholds, using all available CPU cores.

### Create a Batch

Provide a list of (metric ID, current value, threshold) tuples:

```python
from neuralbudget import ParallelMetricBatch

batch = ParallelMetricBatch([
    ("latency_p99", 150.0, 200.0),      # 150 ms < 200 ms threshold: PASS
    ("availability", 99.95, 99.9),      # 99.95% > 99.9% threshold: PASS
    ("error_rate", 0.1, 0.5),           # 0.1% < 0.5% threshold: PASS
])
```

### Evaluate Metrics in Parallel

Call `evaluate()` to check all metrics against their thresholds. This releases Python's Global Interpreter Lock, allowing evaluation to use multiple CPU cores:

```python
results = batch.evaluate()
# Returns a list: [
#   ("latency_p99", 150.0, 200.0, True, 0.75),
#   ("availability", 99.95, 99.9, True, 1.0),
#   ("error_rate", 0.1, 0.5, True, 0.2),
# ]
```

**Result format:** (metric ID, value, threshold, pass, score)

- **pass** (True/False): Value meets the threshold
- **score** (0.0–1.0): Normalized ratio; capped at 1.0
  - Formula: `min(value / threshold, 1.0)`
  - Example: latency 150 / threshold 200 = 0.75 score

### Query Graph Status

Get aggregate statistics without re-evaluating:

```python
### Aggregate Results

Compute overall health from the results:

```python
all_pass = all(passed for _, _, _, passed, _ in results)       # True if every metric passed
avg_score = sum(score for _, _, _, _, score in results) / len(results)  # Mean score
pass_count = sum(1 for _, _, _, passed, _ in results if passed)  # Number of passing metrics
total = batch.node_count                                         # Total metrics in batch
```

### Re-evaluate with New Values

Create a new batch with updated metric values:

```python
batch = ParallelMetricBatch([
    ("latency_p99", 180.0, 200.0),  # Updated value
    ("availability", 99.98, 99.9),  # Updated value
    ("error_rate", 0.05, 0.5),      # Updated value
])
results = batch.evaluate()
```

---

## Task 3: Combine Aggregation and Parallel Evaluation

Build a real-time health check loop.

### Example: Monitor Service Latency

```python
from neuralbudget import StreamingAggregator, ParallelMetricBatch

# Step 1: Collect latency measurements into an aggregator
latency_agg = StreamingAggregator()

for timestamp_ms, latency_value in incoming_metrics:
    latency_agg.push(timestamp_ms, latency_value)

# Step 2: Extract 5-second moving average
current_time_ms = timestamp_ms
window_ms = 5000
avg_latency = latency_agg.get_moving_average(current_time_ms, window_ms)

# Step 3: Build a batch with current metrics
batch = ParallelMetricBatch([
    ("latency_p99", avg_latency, 200.0),
    ("availability", 99.95, 99.9),
])

# Step 4: Evaluate all metrics in parallel
results = batch.evaluate()

# Step 5: Alert if any metric failed
if not graph.all_pass():
    print(f"Health: {graph.aggregate_score():.1%}")
    for metric_id, value, threshold, passed, score in results:
        if not passed:
            print(f"  FAIL: {metric_id} = {value} (threshold {threshold})")
else:
    print("All metrics pass")
```

---

## Performance Expectations

### Throughput

| Operation | Speed |
|-----------|-------|
| Ingest metrics (StreamingAggregator.push) | 20,000+ per second |
| Query window average (get_moving_average) | 100,000+ per second |
| Evaluate 100 metrics in parallel | 50,000–100,000+ metrics per second |

### End-to-End Example

To evaluate 100,000 SLO metrics on an 8-core machine:

1. Aggregation (extract 5-second windows): < 1 millisecond
2. Graph construction: < 1 millisecond
3. Parallel evaluation: 100–200 milliseconds
4. Aggregation (compute mean score): < 1 millisecond

**Total: 150–250 milliseconds per evaluation cycle**

If you run evaluations every 5 seconds, total CPU usage stays under 10%.

---

## Automatic Memory Management Details

### How Velocity Tracking Works

The aggregator measures how fast metrics arrive by examining the most recent 1,000 timestamps. Every 100 measurements, it calculates:

```
Velocity = 1,000,000 milliseconds / (time to collect 1,000 samples in milliseconds)
```

**Example:** If 1,000 samples arrive in 50 milliseconds:
- Velocity = 1,000,000 / 50 = **20,000 samples per second**

### Adaptation Threshold

When velocity exceeds 15,000 samples per second continuously, the aggregator activates automatic pruning. It removes any data older than 5 seconds.

**Why these values?**
- 15,000 samples/sec: Roughly 2.5× the standard benchmarked ingestion rate; indicates a spike
- 5 seconds: Preserves recent history for 5-second window queries; keeps memory under 4 MB

### No Configuration Required

Do not try to configure these values. They are optimized for typical usage. If your traffic patterns differ substantially, reach out to the NeuralBudget team.

---

## Troubleshooting

### Memory Still Growing During Traffic Spikes

**Symptom:** Aggregator buffer exceeds 100,000 entries despite automatic pruning.

**Check:**
1. Verify metrics arrive consistently above 15,000 per second for several seconds (not just a brief spike)
2. Confirm you do not hold references to old aggregator instances

**Solution:** Restart the aggregator instance or explicitly call `prune()` with a recent cutoff timestamp.

### Evaluation Takes Longer Than Expected

**Symptom:** `graph.evaluate()` takes > 500 milliseconds for 100,000 metrics.

**Check:**
1. Confirm you run on a machine with multiple CPU cores (run `cat /proc/cpuinfo | grep -c processor`)
2. Verify no other processes consume all CPU time (run `top`)

**Solution:** Reduce the number of metrics in the graph, or upgrade to a machine with more cores.

### All Metrics Show score=1.0

**Symptom:** Every metric's score is clamped at 1.0 even though values differ.

**This is expected.** The score formula is `min(value / threshold, 1.0)`. Any metric that exceeds or meets its threshold gets a score of 1.0. Use the `pass` field (True/False) to distinguish passing from exceeded metrics.

---

## Thread Safety Best Practices

`ParallelMetricBatch` evaluates metrics in parallel on multiple CPU cores (releasing the GIL), but it is **NOT thread-safe for concurrent access across Python threads**.

### Safe Patterns

**Pattern 1: Single-threaded use (simplest)**
```python
batch = ParallelMetricBatch([...])
while True:
    batch.update_node("latency", get_latest_latency())
    results = batch.evaluate()
    if not batch.all_pass():
        alert()
```

**Pattern 2: Protect with a lock (for multi-threaded applications)**
```python
from threading import Lock

batch = ParallelMetricBatch([...])
batch_lock = Lock()

def update_metrics():
    with batch_lock:
        batch.update_node("latency", get_latest_latency())

def check_health():
    with batch_lock:
        results = batch.evaluate()
        if not batch.all_pass():
            alert()
```

**Pattern 3: Separate batches per thread (if possible)**
```python
import threading

def worker_thread():
    # Each thread gets its own batch instance
    local_batch = ParallelMetricBatch([...])
    while True:
        local_batch.update_node("metric", get_value())
        results = local_batch.evaluate()
```

### Unsafe Patterns (Avoid)

**Pattern 1: Concurrent mutations**
```python
# UNSAFE - DO NOT DO THIS
batch = ParallelMetricBatch([...])

def thread_1():
    batch.update_node("metric1", 100.0)  # Data race!

def thread_2():
    batch.update_node("metric2", 200.0)  # Data race!

# Running these concurrently will cause crashes or silent corruption
```

**Pattern 2: Mutation while evaluating**
```python
# UNSAFE - DO NOT DO THIS
batch = ParallelMetricBatch([...])

def thread_1():
    batch.evaluate()  # Data race!

def thread_2():
    batch.update_node("metric", 100.0)  # Data race!

# Running these concurrently will cause crashes or silent corruption
```

### Important Reminder

The GIL release in `evaluate()` allows Python code to run on other threads *while evaluation happens on Rust's thread pool*. This is safe. But updating the batch instance itself from multiple threads is NOT safe — synchronize with a lock.

---

## What's Next

- [API Reference](PARALLEL_SLO_API_REFERENCE.md) — Full method signatures and return types
- [Deployment Guide](DEPLOYMENT_GUIDE.md) — Production configuration and monitoring
- [Examples](examples/python/) — Code samples for Prometheus integration, Kubernetes, and dashboards
