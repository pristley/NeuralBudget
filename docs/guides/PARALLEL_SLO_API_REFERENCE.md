# Parallel SLO Evaluation: Complete API Reference

**Class:** `ParallelMetricBatch` and `StreamingAggregator`  
**Module:** `neuralbudget`  
**Python 3.9+, Rust 2021 Edition**

---

## StreamingAggregator

The `StreamingAggregator` class collects individual metric measurements and provides windowed statistics without requiring you to store the full history.

### Constructor

```python
StreamingAggregator()
```

Create a new aggregator. No arguments required.

**Returns:** New aggregator instance with empty buffer.

**Example:**
```python
try:
    from neuralbudget import StreamingAggregator
    agg = StreamingAggregator()
    print("✓ StreamingAggregator created successfully")
except ImportError as e:
    print(f"✗ Import error: {e}")
    print("  → Run: pip install neuralbudget")
```

### Methods

#### push(timestamp: int, value: float) → None

Add a single metric measurement to the aggregator.

**Parameters:**
- `timestamp` (integer, milliseconds): When this measurement occurred. Assume timestamps increase over time; out-of-order measurements may produce incorrect results.
- `value` (float): The metric value. Examples: latency in milliseconds, availability percentage, error rate.

**Returns:** None

**Behavior:**
- Stores the (timestamp, value) pair in an internal queue.
- If ingestion exceeds 15,000 measurements per second continuously, automatically removes data older than 5 seconds to prevent unbounded memory growth.
- Time complexity: O(1) amortized; throughput 20,000+ measurements per second via Python.

**Example:**
```python
try:
    agg.push(1000, 50.0)   # At time 1000ms, value was 50
    agg.push(1050, 52.0)   # At time 1050ms, value was 52
    agg.push(1100, 51.0)   # At time 1100ms, value was 51
    print("✓ Metrics pushed successfully")
except TypeError as e:
    print(f"✗ Type error: {e}")
    print("  → Ensure timestamp is integer and value is float")
except OverflowError as e:
    print(f"✗ Value out of range: {e}")
    print("  → Check that values are within valid ranges")
```

#### get_moving_average(current_timestamp: int, window_milliseconds: int) → float

Retrieve the arithmetic mean of all values within a time window.

**Parameters:**
- `current_timestamp` (integer, milliseconds): The reference time; defines the window end.
- `window_milliseconds` (integer): Duration of the window in milliseconds. Retrieves all measurements between `current_timestamp - window_milliseconds` and `current_timestamp`.

**Returns:** Float representing the mean value. If no measurements exist in the window, returns 0.0.

**Behavior:**
- Scans measurements within the specified window; computes their arithmetic mean.
- Time complexity: O(n) where n = measurements in window; typical window sizes (100–5000 ms) make this fast (~10 microseconds).
- Returns a single float value, not a Python object, so overhead is minimal.

**Example:**
```python
try:
    agg.push(1000, 50.0)
    agg.push(1050, 52.0)
    agg.push(1100, 51.0)

    # Average of measurements within 100ms before time 1100
    avg = agg.get_moving_average(1100, 100)  # Returns 51.5 (mean of 52 and 51)
    print(f"✓ 100ms average: {avg:.1f}")

    # Average of all measurements (window size 150ms)
    avg = agg.get_moving_average(1100, 150)  # Returns 51.0 (mean of 52, 51, and 50)
    print(f"✓ 150ms average: {avg:.1f}")
except TypeError as e:
    print(f"✗ Error: {e}")
    print("  → Ensure both timestamp and window_ms are integers")
```

#### prune(cutoff_timestamp: int) → None

Remove all measurements before the specified timestamp to free memory.

**Parameters:**
- `cutoff_timestamp` (integer, milliseconds): Remove all measurements with timestamp < cutoff_timestamp. Measurements at or after this timestamp remain.

**Returns:** None

**Behavior:**
- Scans the internal queue and removes old entries.
- Time complexity: O(k) where k = entries removed; typical cleanup is fast.
- If ingestion exceeds 15,000 measurements per second, this method is called automatically; you do not need to call it manually in that case.

**Example:**
```python
try:
    agg.prune(900)  # Remove all measurements before timestamp 900
    print(f"✓ Pruned old data. Remaining entries: {agg.len()}")
except TypeError as e:
    print(f"✗ Error: {e}")
    print("  → Ensure cutoff_timestamp is an integer")
```

#### len() → int

Return the number of measurements currently stored.

**Returns:** Integer count of (timestamp, value) pairs.

**Example:**
```python
try:
    count = agg.len()  # Returns current buffer size
    if count > 100000:
        print(f"✗ Warning: buffer is large ({count} entries)")
    else:
        print(f"✓ Buffer size: {count} entries")
except Exception as e:
    print(f"✗ Error: {e}")
```

#### is_empty() → bool

Check whether the aggregator contains any measurements.

**Returns:** True if the buffer is empty; False otherwise.

**Example:**
```python
try:
    if agg.is_empty():
        print("✗ No measurements yet")
    else:
        print(f"✓ Aggregator has {agg.len()} measurements")
except Exception as e:
    print(f"✗ Error: {e}")
```

---

## ParallelMetricBatch

The `ParallelMetricBatch` class evaluates whether a set of metrics meet their thresholds in parallel using all available CPU cores. Unlike `CompositeSloGraph`, this does **not** model dependencies. Each metric is checked against its own threshold independently and concurrently.

### ⚠️ Thread Safety

**IMPORTANT:** `ParallelMetricBatch` is **NOT safe for concurrent access** across threads.

**Safe usage patterns:**
```python
# Pattern 1: Single-threaded use (SAFE)
batch = ParallelMetricBatch([...])
batch.evaluate()  # OK
results = batch.all_pass()  # OK

# Pattern 2: Protected with lock (SAFE)
from threading import Lock
batch_lock = Lock()
with batch_lock:
    batch.update_node("metric", new_value)
    results = batch.evaluate()
```

**Unsafe usage patterns (will cause data races or crashes):**
```python
# Pattern 1: Concurrent mutations (UNSAFE)
# Thread 1:
batch.update_node("metric", 100.0)

# Thread 2 (simultaneously):
batch.evaluate()  # Data race! May see partial updates or crash

# Pattern 2: Concurrent updates (UNSAFE)
# Thread 1:
batch.update_node("metric1", 100.0)

# Thread 2 (simultaneously):
batch.update_node("metric2", 200.0)  # Data race!
```

**If you need concurrent access:**
- Synchronize externally with `threading.Lock()` or similar
- Create separate `ParallelMetricBatch` instances per thread if possible
- Consider wrapping in a thread-safe queue or API

### Result Consistency Guarantee

All query methods (`all_pass()`, `aggregate_score()`, `pass_count()`, `nodes_as_tuples()`) **compute results from the current node state on every call**. They do NOT cache results from `evaluate()`.

This means:
- Query methods always return results consistent with current node values
- Calling a query method before `evaluate()` is valid and returns correct results
- If you call `update_node()`, query methods immediately reflect the updated values
- `evaluate()` is primarily used for **GIL release** to allow parallel computation; results are not cached

**Example:**
```python
batch = ParallelMetricBatch([("metric", 100.0, 200.0)])

# Before calling evaluate() - still works correctly
print(batch.all_pass())  # False (100 < 200)

# evaluate() computes and returns results
results = batch.evaluate()  # Returns computed results

# Query methods recompute from current state
batch.update_node("metric", 250.0)
print(batch.all_pass())  # True (updated value 250 >= 200)
# Note: This is computed on-the-fly, not from cached evaluate() result
```

### Constructor

```python
ParallelMetricBatch(nodes: List[Tuple[str, float, float]])
```

Create a new graph from a list of metric definitions.

**Parameters:**
- `nodes` (list of tuples): Each tuple contains:
  - `node_id` (string): Unique identifier for the metric
  - `value` (float): Current metric value
  - `threshold` (float): Pass/fail threshold

**Returns:** New graph instance; `node_count` attribute contains the metric count.

**Raises:** ValueError if any tuple is malformed or contains non-numeric values.

**Behavior:**
- Stores metrics in memory (no persistence).
- Does not validate that `value <= threshold`; evaluation is independent of current value.

**Example:**
```python
try:
    from neuralbudget import ParallelMetricBatch

    batch = ParallelMetricBatch([
        ("latency_p99", 150.0, 200.0),
        ("availability", 99.95, 99.9),
        ("error_rate", 0.1, 0.5),
    ])

    print(f"✓ Batch created with {batch.node_count} metrics")
except ValueError as e:
    print(f"✗ Invalid batch configuration: {e}")
    print("  → Check that all tuples have 3 numeric elements")
except TypeError as e:
    print(f"✗ Type error: {e}")
    print("  → Ensure values and thresholds are floats")
```

### Methods

#### evaluate(py: Python) → List[Tuple[str, float, float, bool, float]]

Evaluate all metrics against their thresholds in parallel using multiple CPU cores.

**Parameters:**
- Implicit; the Python interpreter passes itself to release the Global Interpreter Lock during evaluation.

**Returns:** List of tuples. Each tuple contains:
- `node_id` (string): The metric identifier
- `value` (float): The metric's current value
- `threshold` (float): The metric's threshold
- `pass` (bool): True if value >= threshold; False otherwise
- `score` (float): Normalized score in range [0.0, 1.0], calculated as `min(value / threshold, 1.0)`

**Behavior:**
- Releases Python's Global Interpreter Lock, allowing evaluation to run on multiple CPU cores simultaneously.
- Each metric is evaluated independently; no inter-metric dependencies.
- Processing speed: 50,000–100,000+ metrics per second on modern hardware.

**Example:**
```python
try:
    results = graph.evaluate()
    # Returns:
    # [
    #   ("latency_p99", 150.0, 200.0, True, 0.75),
    #   ("availability", 99.95, 99.9, True, 1.0),
    #   ("error_rate", 0.1, 0.5, True, 0.2),
    # ]

    print(f"✓ Evaluated {len(results)} metrics:")
    for node_id, value, threshold, passed, score in results:
        status = "PASS" if passed else "FAIL"
        print(f"  {node_id}: {value:.2f} (threshold {threshold}) {status}")
except RuntimeError as e:
    print(f"✗ Evaluation failed: {e}")
```

#### all_pass() → bool

Check whether every metric passed (all values >= thresholds).

**Parameters:** None

**Returns:** True if all metrics pass; False if any metric fails. Returns True for an empty batch (vacuous truth).

**Behavior:**
- **Computes results from current node state on every call** (does not use cached results from `evaluate()`).
- Safe to call before `evaluate()` has been invoked.
- Time complexity: O(n) where n = number of metrics.

**Example:**
```python
try:
    batch = ParallelMetricBatch([
        ("latency", 150.0, 200.0),  # Fails: 150 < 200
        ("availability", 99.95, 99.9),  # Passes: 99.95 >= 99.9
    ])

    # Works correctly before evaluate() is called
    if batch.all_pass():
        print("✓ All metrics pass")
    else:
        print("✗ Some metrics fail")

    # After updating a metric
    batch.update_node("latency", 250.0)  # Now passes
    if batch.all_pass():
        print("✓ All metrics now pass (after update)")
except ValueError as e:
    print(f"✗ Error: {e}")
```

#### aggregate_score() → float

Calculate the mean score across all metrics.

**Parameters:** None

**Returns:** Float in range [0.0, 1.0], computed as the arithmetic mean of all per-metric scores. Returns 1.0 for an empty batch.

**Behavior:**
- **Computes results from current node state on every call** (does not use cached results from `evaluate()`).
- Safe to call before `evaluate()` has been invoked.
- Handles zero thresholds by treating score as 1.0 for that metric.
- Time complexity: O(n) where n = number of metrics.
- Equal weight for all metrics (no weighted aggregation).

**Interpretation:**
- 1.0 = all metrics at or exceed their thresholds
- 0.5 = on average, metrics achieve 50% of their thresholds
- 0.0 = all metrics fail (or threshold values are zero)

**Example:**
```python
try:
    batch = ParallelMetricBatch([
        ("metric1", 100.0, 100.0),  # Score: 1.0
        ("metric2", 50.0, 100.0),   # Score: 0.5
    ])

    # Works correctly before evaluate() is called
    health = batch.aggregate_score()  # Returns 0.75 (mean of 1.0 and 0.5)
    print(f"✓ Composite health: {health:.1%}")  # "Composite health: 75.0%"

    # After updating metrics
    batch.update_node("metric2", 75.0)  # New score: 0.75
    health = batch.aggregate_score()  # Returns 0.875 (mean of 1.0 and 0.75)
    print(f"✓ Updated health: {health:.1%}")
except ValueError as e:
    print(f"✗ Error: {e}")
```

#### pass_count() → int

Count how many metrics passed.

**Parameters:** None

**Returns:** Integer count of metrics where value >= threshold.

**Behavior:**
- **Computes results from current node state on every call** (does not use cached results from `evaluate()`).
- Safe to call before `evaluate()` has been invoked.
- Time complexity: O(n) where n = number of metrics.

**Example:**
```python
try:
    batch = ParallelMetricBatch([
        ("metric1", 250.0, 200.0),  # Passes
        ("metric2", 99.9, 99.0),    # Passes
        ("metric3", 0.1, 0.5),      # Fails
    ])

    # Works correctly before evaluate() is called
    passed = batch.pass_count()  # Returns 2
    total = batch.node_count     # Returns 3
    print(f"✓ Passed: {passed}/{total}")  # "Passed: 2/3"

    # After updating a metric that failed
    batch.update_node("metric3", 1.0)  # Now passes
    passed = batch.pass_count()  # Returns 3
    print(f"✓ Updated: {passed}/{total}")
except ValueError as e:
    print(f"✗ Error: {e}")
```

#### get_node(node_id: str) → Tuple[str, float, float] | None

Retrieve a specific metric's current definition.

**Parameters:**
- `node_id` (string): The metric identifier to look up.

**Returns:** Tuple of (id, value, threshold) if the metric exists; None if not found.

**Behavior:**
- Time complexity: O(n) where n = number of metrics (linear search).
- Does not perform evaluation; returns the stored definition.

**Example:**
```python
try:
    result = graph.get_node("latency_p99")
    if result:
        node_id, value, threshold = result
        print(f"✓ Current: {value}, Threshold: {threshold}")
    else:
        print("✗ Metric not found: latency_p99")
        print("  → See batch.nodes_as_tuples() to list all metrics")
except TypeError as e:
    print(f"✗ Error: {e}")
```

#### update_node(node_id: str, new_value: float) → bool

Change a metric's current value and keep the threshold unchanged.

**Parameters:**
- `node_id` (string): The metric identifier to update.
- `new_value` (float): The new value for this metric.

**Returns:** True if the update succeeded; False if the metric was not found.

**Behavior:**
- Modifies the batch in place; does not re-evaluate automatically.
- Threshold remains unchanged.
- Time complexity: O(n) where n = number of metrics (linear search for the node).

**⚠️ Thread Safety:**
- **DO NOT** call `update_node()` while `evaluate()` is running on another thread on the same batch instance.
- **DO NOT** call `update_node()` from multiple threads concurrently on the same batch instance.
- Use external synchronization (e.g., `threading.Lock()`) if you need concurrent access.
- Creating separate batch instances per thread is the safest approach.

**Example (Safe):**
```python
try:
    from threading import Lock

    batch = ParallelMetricBatch([("latency", 100.0, 200.0)])
    batch_lock = Lock()

    # Safe pattern: protect with lock
    with batch_lock:
        success = batch.update_node("latency", 150.0)
        if success:
            print("✓ Metric updated")
            results = batch.evaluate()
        else:
            print("✗ Metric not found")

    # Or: update before concurrent evaluations
    batch.update_node("latency", 150.0)
    results = batch.evaluate()  # Single evaluation (no lock needed)
    print("✓ Single evaluation completed")
except (ValueError, RuntimeError) as e:
    print(f"✗ Error: {e}")
```

**Example (Unsafe - avoid):**
```python
# UNSAFE: Concurrent mutation and evaluation
import threading

batch = ParallelMetricBatch([("latency", 100.0, 200.0)])

def update_thread():
    batch.update_node("latency", 150.0)  # Data race!

def eval_thread():
    batch.evaluate()  # Data race!

t1 = threading.Thread(target=update_thread)
t2 = threading.Thread(target=eval_thread)
t1.start()
t2.start()  # UNSAFE: Both run concurrently
t1.join()
t2.join()
```

#### nodes_as_tuples() → List[Tuple[str, float, float, bool, float]]

Export all metrics with their current evaluation results.

**Parameters:** None

**Returns:** List of tuples identical in format to `evaluate()`; includes pass/fail and score for each metric. Each tuple contains (node_id, value, threshold, pass, score).

**Behavior:**
- **Computes results from current node state on every call** (does not use cached results from `evaluate()`).
- Safe to call before `evaluate()` has been invoked. Returns an empty list if the batch is empty.
- Time complexity: O(n) where n = number of metrics.

**Example:**
```python
batch = ParallelMetricBatch([
    ("latency", 150.0, 200.0),
    ("availability", 99.95, 99.9),
])

# Works correctly before evaluate() is called
all_nodes = batch.nodes_as_tuples()
# Returns:
# [
#   ("latency", 150.0, 200.0, False, 0.75),
#   ("availability", 99.95, 99.9, True, 1.0),
# ]

for node_id, value, threshold, passed, score in all_nodes:
    status = "PASS" if passed else "FAIL"
    print(f"{node_id}: {value:.2f} (threshold {threshold}) {status}")

# After mutation, results reflect current state
batch.update_node("latency", 250.0)
updated_nodes = batch.nodes_as_tuples()
# Returns:
# [
#   ("latency", 250.0, 200.0, True, 1.0),  # Now passes
#   ("availability", 99.95, 99.9, True, 1.0),
# ]
```

---

## Performance Characteristics

### Throughput and Latency

| Operation | Input Size | Throughput | Latency |
|-----------|-----------|-----------|---------|
| `StreamingAggregator.push()` | 1 measurement | 20,000+ samples/sec | ~50 microseconds (Python) |
| `StreamingAggregator.get_moving_average()` | 100-sample window | 100,000+ queries/sec | ~10 microseconds |
| `ParallelMetricBatch.evaluate()` | 100 metrics | 50,000+ metrics/sec | ~2 milliseconds |
| `ParallelMetricBatch.evaluate()` | 1,000 metrics | 50,000+ metrics/sec | ~20 milliseconds |
| `ParallelMetricBatch.evaluate()` | 10,000 metrics | 50,000+ metrics/sec | ~200 milliseconds |

### Parallelism

- `ParallelMetricBatch.evaluate()` uses all available CPU cores (typical: 4–32 cores on modern servers)
- On single-core systems, evaluation still completes sequentially; throughput remains 50,000+ metrics/sec
- No additional synchronization or locking overhead; each metric is independent

### Memory

| Component | Typical Usage | Memory Bound |
|-----------|---------------|--------------|
| `StreamingAggregator` buffer | 5-second window at 5k samples/sec | ~100 KB |
| `StreamingAggregator` buffer | Traffic spike at 25k samples/sec | ~4 MB (auto-pruned) |
| `ParallelMetricBatch` | 1,000 metrics | ~100 KB |
| `ParallelMetricBatch` | 10,000 metrics | ~1 MB |

---

## Error Handling

### StreamingAggregator

Most errors are prevented by type checking at the Python boundary:

**Invalid input:**
```python
agg.push("not_a_timestamp", 50.0)  # Raises TypeError
agg.push(1000, "not_a_value")      # Raises TypeError
```

**Expected behavior:** If timestamps are provided out of order, results may be incorrect. Always provide monotonically increasing timestamps.

### ParallelMetricBatch

**Invalid initialization:**
```python
ParallelMetricBatch([('metric', 'not_a_float', 100.0)])  # Raises ValueError
ParallelMetricBatch([('metric', 50.0)])                  # Raises ValueError (wrong tuple length)
```

**Invalid operations:**
```python
graph.get_node("nonexistent")  # Returns None (not an error)
graph.update_node("nonexistent", 50.0)  # Returns False (not an error)
```

---

## Integration Example: End-to-End SLO Evaluation

```python
from neuralbudget import StreamingAggregator, ParallelMetricBatch
import time

# Step 1: Aggregate metrics over 5-second window
latency_agg = StreamingAggregator()
error_agg = StreamingAggregator()

current_time_ms = int(time.time() * 1000)
for i in range(100):
    latency_agg.push(current_time_ms + i * 50, 100 + i % 50)  # Varies 100-150
    error_agg.push(current_time_ms + i * 50, 0.1 + (i % 10) * 0.02)  # Varies 0.1-0.3%

# Step 2: Query windows
avg_latency = latency_agg.get_moving_average(current_time_ms + 5000, 5000)
avg_errors = error_agg.get_moving_average(current_time_ms + 5000, 5000)

# Step 3: Create evaluation batch
batch = ParallelMetricBatch([
    ("latency_p99", avg_latency, 200.0),
    ("error_rate", avg_errors, 0.5),
    ("availability", 99.9, 99.0),  # Hypothetical current value
])

# Step 4: Evaluate in parallel
results = batch.evaluate()

# Step 5: Report results
print(f"Overall health: {batch.aggregate_score():.1%}")
print(f"Passing metrics: {batch.pass_count()}/{batch.node_count}")

for node_id, value, threshold, passed, score in results:
    status = "PASS" if passed else "FAIL"
    print(f"  {node_id}: {value:.2f} / {threshold} ({status}, score={score:.2f})")
```

---

## See Also

- [Getting Started Guide](#) — Quick introduction and task-based examples
- [Deployment Guide](#) — Production configuration, monitoring, and troubleshooting
- [Source Code](../../src/streaming.rs) — Rust implementation details
