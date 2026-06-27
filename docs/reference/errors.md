# Error Reference Guide

**Last Updated:** June 27, 2026

This guide documents all common errors you may encounter when using NeuralBudget, their root causes, and step-by-step solutions.

---

## How to Use This Guide

1. **Find your error message** in the sections below
2. **Read the Root Cause** to understand why it happened
3. **Follow the Resolution Steps** to fix it
4. **If stuck**, see the **Troubleshooting Tips** section at the end

---

## Installation & Setup Errors

### ERROR: `ModuleNotFoundError: No module named 'neuralbudget'`

**When:** Trying to import NeuralBudget in Python  
**Root Cause:** Package not installed or installed in different Python environment

**Resolution Steps:**
1. Verify installation:
   ```bash
   pip list | grep neuralbudget
   ```
   If not listed, proceed to step 2.

2. Install from PyPI:
   ```bash
   pip install neuralbudget
   ```

3. Verify installation worked:
   ```bash
   python -c "import neuralbudget; print('OK')"
   ```

**Common Variations:**
- Using `python` instead of `python3` in wrong environment
- Multiple Python versions installed (use `python3 -m pip install...` explicitly)
- Virtual environment not activated

---

### ERROR: `PyYAML is required to load YAML config files`

**When:** Calling `load_config("slo.yaml")`  
**Root Cause:** PyYAML library not installed

**Resolution Steps:**
1. Install PyYAML:
   ```bash
   pip install pyyaml
   ```

2. Verify:
   ```bash
   python -c "import yaml; print('OK')"
   ```

3. Retry your code:
   ```python
   client = NeuralBudgetClient().load_config("slo.yaml")
   ```

**Why it happens:**
- NeuralBudget core only supports JSON; YAML requires optional dependency
- Minimal installation skips PyYAML to reduce dependencies

---

### ERROR: Build fails with `maturin` during source installation

**When:** Running `maturin develop --release`  
**Root Cause:** Rust toolchain not installed or outdated

**Resolution Steps:**
1. Install Rust (if not already):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

2. Update Rust:
   ```bash
   rustup update
   ```

3. Install maturin and pip tools:
   ```bash
   pip install --upgrade pip setuptools maturin
   ```

4. Retry build:
   ```bash
   maturin develop --release
   ```

---

## Configuration Errors

### ERROR: `RuntimeError: No config loaded. Call load_config(path) first.`

**When:** Calling `evaluate()` before `load_config()`  
**Root Cause:** Config not loaded in correct order

**Resolution Steps:**
1. Verify order of operations:
   ```python
   # ❌ WRONG
   client = NeuralBudgetClient()
   result = client.evaluate({...})  # Error!

   # ✅ RIGHT
   client = NeuralBudgetClient()
   client.load_config("slo.json")
   result = client.evaluate({...})
   ```

2. Check config file exists:
   ```bash
   ls -la slo.json
   ```

3. Verify config loading succeeded:
   ```python
   client = NeuralBudgetClient()
   try:
       client.load_config("slo.json")
       print("Config loaded successfully")
   except Exception as e:
       print(f"Load failed: {e}")
   ```

---

### ERROR: `ValueError: Unsupported config extension`

**When:** Calling `load_config()` with unsupported file type  
**Root Cause:** Config file has wrong extension

**Supported Extensions:**
- `.json` — JSON format
- `.yaml` or `.yml` — YAML format (requires PyYAML)

**Resolution:**
1. Verify file extension:
   ```bash
   ls -la config*
   ```

2. Rename if needed:
   ```bash
   # If you have config.txt, rename to config.json
   mv config.txt config.json
   ```

3. If YAML file, ensure PyYAML installed:
   ```bash
   pip install pyyaml
   ```

---

### ERROR: `KeyError: 'mode'` or `ValueError: unknown preset`

**When:** Loading config with invalid structure  
**Root Cause:** Config missing required fields or uses unknown mode/preset

**Resolution:**
1. Verify config has required fields:
   ```json
   {
     "schema_version": 1,
     "mode": "http",           // Required
     "profile": "strict_latency" // Required for most modes
   }
   ```

2. Check valid modes and presets:
   ```python
   # Valid modes
   modes = ["http", "stateful", "ml", "genai"]  # "composite" coming soon

   # Valid presets for mode="http"
   presets = ["strict_latency", "moderate_latency", "loose_latency", "custom"]
   ```

3. Use a working example config:
   ```json
   {
     "schema_version": 1,
     "mode": "http",
     "profile": "strict_latency",
     "params": {
       "latency_threshold_ms": 200.0,
       "percentile": 99,
       "success_rate_target": 0.999
     }
   }
   ```

---

## Runtime Evaluation Errors

### ERROR: `TypeError: evaluate() got unexpected keyword argument`

**When:** Passing wrong parameters to `evaluate()`  
**Root Cause:** Calling with incorrect parameter names or types

**Resolution:**
1. Check `evaluate()` signature:
   ```python
   result = client.evaluate({
       "timestamp": 1,                    # Required: int (milliseconds)
       "success": 9995,                   # Required: int (count)
       "total": 10000,                    # Required: int (count)
       "buckets": [...],                  # Required for latency modes: list
       "format": "prometheus_cumulative"  # Required: string
   })
   ```

2. Verify all required fields present:
   ```python
   required = ["timestamp", "success", "total", "format"]
   for field in required:
       if field not in metric_data:
           print(f"Missing required field: {field}")
   ```

3. Verify data types:
   ```python
   assert isinstance(metric_data["timestamp"], int), "timestamp must be int"
   assert isinstance(metric_data["success"], int), "success must be int"
   assert isinstance(metric_data["buckets"], list), "buckets must be list"
   ```

---

### ERROR: `ValueError: Buckets must be sorted by upper_bound`

**When:** Evaluating HTTP SLO with unsorted histogram buckets  
**Root Cause:** Latency buckets not in ascending order

**Resolution:**
1. Verify buckets are sorted:
   ```python
   # ❌ WRONG - unsorted
   buckets = [
       {"upper_bound_ms": 200.0, "count": 100},
       {"upper_bound_ms": 100.0, "count": 50},  # Out of order!
   ]

   # ✅ RIGHT - sorted
   buckets = [
       {"upper_bound_ms": 100.0, "count": 50},
       {"upper_bound_ms": 200.0, "count": 100},
   ]
   ```

2. Sort programmatically if needed:
   ```python
   sorted_buckets = sorted(buckets, key=lambda b: b["upper_bound_ms"])
   metric_data["buckets"] = sorted_buckets
   ```

---

### ERROR: `RuntimeError: Invalid percentile. Must be 0 < p < 100`

**When:** Config specifies invalid percentile  
**Root Cause:** Percentile not in range (0, 100)

**Resolution:**
1. Valid percentiles: 1-99 (not 0, not 100)
   ```python
   # ❌ WRONG
   "percentile": 0    # Invalid
   "percentile": 100  # Invalid

   # ✅ RIGHT
   "percentile": 50   # Median
   "percentile": 95   # 95th percentile
   "percentile": 99   # 99th percentile
   ```

2. Common percentile values:
   - 50 = Median (50th percentile)
   - 95 = Tail latency for most users
   - 99 = Tail latency for most sensitive users
   - 99.9 = Extreme tail latency

---

## Data Validation Errors

### ERROR: `ValueError: Timestamp must be monotonically increasing`

**When:** Calling `StreamingAggregator.push()` with out-of-order timestamps  
**Root Cause:** Metrics arriving out of chronological order

**Resolution:**
1. Sort data before pushing:
   ```python
   # ❌ WRONG - out of order
   agg = StreamingAggregator()
   agg.push(1100, 50.0)  # Second timestamp
   agg.push(1000, 49.0)  # First timestamp - Error!

   # ✅ RIGHT - sorted
   agg = StreamingAggregator()
   agg.push(1000, 49.0)
   agg.push(1100, 50.0)
   ```

2. Sort metrics before pushing:
   ```python
   metrics = [(ts, val) for ts, val in incoming_metrics]
   metrics.sort(key=lambda x: x[0])  # Sort by timestamp

   agg = StreamingAggregator()
   for ts, val in metrics:
       agg.push(ts, val)
   ```

3. If out-of-order data is expected:
   - Use a buffer to reorder metrics
   - Push with small tolerance for clock drift:
     ```python
     last_ts = 0
     for ts, val in metrics:
         if ts < last_ts:
             # Clock drift detected; skip or log
             continue
         agg.push(ts, val)
         last_ts = ts
     ```

---

### ERROR: `ValueError: Window must be positive`

**When:** Calling `get_moving_average()` with invalid window  
**Root Cause:** Window size <= 0

**Resolution:**
1. Use positive window sizes (milliseconds):
   ```python
   # ❌ WRONG
   avg = agg.get_moving_average(1000, -100)  # Negative window
   avg = agg.get_moving_average(1000, 0)     # Zero window

   # ✅ RIGHT
   avg = agg.get_moving_average(1000, 100)   # 100ms window
   avg = agg.get_moving_average(1000, 5000)  # 5 second window
   ```

2. Common window sizes:
   - 100ms = Captures recent spikes
   - 1000ms = Captures recent second
   - 5000ms = 5-second moving average
   - 60000ms = 1-minute moving average

---

### ERROR: `ValueError: No measurements in window`

**When:** Calling `get_moving_average()` with empty window  
**Root Cause:** No data collected yet or window too old

**Resolution:**
1. Verify aggregator has data:
   ```python
   agg = StreamingAggregator()
   # Push some data first
   agg.push(1000, 50.0)
   agg.push(1050, 52.0)

   # Now retrieve average
   avg = agg.get_moving_average(1100, 200)  # Returns 51.0
   ```

2. If empty, check if data older than window:
   ```python
   # ❌ WRONG - querying too far in past
   agg.push(1000, 50.0)
   agg.push(1050, 52.0)
   avg = agg.get_moving_average(10000, 100)  # Window is 100ms before 10000ms
   # This queries [9900-10000], but data only exists at [1000-1050]

   # ✅ RIGHT - query recent data
   current_ts = agg.get_latest_timestamp()  # Get most recent timestamp
   avg = agg.get_moving_average(current_ts, 5000)  # Last 5 seconds
   ```

3. Use `is_empty()` to check state:
   ```python
   if agg.is_empty():
       print("No data yet; cannot compute average")
   else:
       avg = agg.get_moving_average(current_ts, 5000)
   ```

---

## Memory & Performance Errors

### ERROR: `RuntimeError: Buffer exceeded max size limit`

**When:** StreamingAggregator accumulating too much data  
**Root Cause:** Not pruning old data; ingestion rate exceeds threshold

**Resolution:**
1. Call `prune()` manually:
   ```python
   # Remove data older than 5 seconds
   current_ts = 10000
   agg.prune(current_ts - 5000)
   ```

2. Or rely on automatic pruning (activates at 15k+ samples/sec):
   - Automatic pruning keeps buffer under 4 MB
   - Removes data older than 5 seconds
   - No action needed; happens automatically

3. Monitor buffer size:
   ```python
   size = agg.len()  # Returns number of measurements
   if size > 100000:
       print("Warning: large buffer; consider pruning")
       agg.prune(current_ts - 3600000)  # Keep last hour
   ```

---

## Thread Safety Errors

### ERROR: `RuntimeError: Data corruption detected` or sudden crashes

**When:** Calling methods concurrently from multiple threads  
**Root Cause:** ParallelMetricBatch is not thread-safe for mutations

**Resolution:**
1. **Single-threaded use (recommended):**
   ```python
   # ✅ SAFE - all operations in one thread
   batch = ParallelMetricBatch([...])
   batch.update_node("metric", 100.0)
   results = batch.evaluate()
   ```

2. **Multi-threaded use - protect with Lock:**
   ```python
   # ✅ SAFE - protected with lock
   from threading import Lock

   batch = ParallelMetricBatch([...])
   batch_lock = Lock()

   def update_metrics():
       with batch_lock:
           batch.update_node("metric", 100.0)

   def evaluate_metrics():
       with batch_lock:
           results = batch.evaluate()
   ```

3. **Per-thread batches (if possible):**
   ```python
   # ✅ SAFE - each thread has own batch
   import threading

   batch_local = threading.local()

   def get_batch():
       if not hasattr(batch_local, 'batch'):
           batch_local.batch = ParallelMetricBatch([...])
       return batch_local.batch

   # Each thread calls get_batch() → gets own instance
   ```

**Important:** 
- `evaluate()` is safe for GIL release; results not cached
- `update_node()` is NOT safe for concurrent calls
- Never mutate batch from multiple threads simultaneously

---

## Kubernetes Deployment Errors

### ERROR: `CrashLoopBackOff` or pod restarts continuously

**When:** Running NeuralBudget in Kubernetes  
**Root Cause:** Config file not found or invalid RBAC permissions

**Resolution:**
1. Verify ConfigMap created:
   ```bash
   kubectl get configmap neuralbudget-config
   kubectl describe configmap neuralbudget-config
   ```

2. Check if ConfigMap mounted correctly:
   ```bash
   kubectl exec <pod-name> -- ls -la /etc/neuralbudget/
   ```

3. Verify logs:
   ```bash
   kubectl logs <pod-name>
   kubectl logs <pod-name> --previous  # See logs before crash
   ```

4. Apply correct YAML:
   ```bash
   kubectl apply -f examples/kubernetes/configmap.yaml
   kubectl apply -f examples/kubernetes/deployment.yaml
   ```

---

### ERROR: Prometheus scrape endpoint returns 404

**When:** Prometheus cannot reach `/metrics` endpoint  
**Root Cause:** Service or port misconfigured

**Resolution:**
1. Verify service is running:
   ```bash
   kubectl get svc neuralbudget-exporter
   kubectl describe svc neuralbudget-exporter
   ```

2. Check port configuration:
   ```bash
   # In ServiceMonitor or scrape_config, verify port is 8080
   # In deployment, verify containerPort matches
   ```

3. Test endpoint locally:
   ```bash
   kubectl port-forward svc/neuralbudget-exporter 8080:8080
   curl http://localhost:8080/metrics
   ```

4. Verify Prometheus can reach pod:
   ```bash
   kubectl exec <prometheus-pod> -- curl http://<neuralbudget-pod>:8080/metrics
   ```

---

## Troubleshooting Tips

### General Debugging Strategy

**Step 1: Isolate the problem**
- Does it happen with a minimal example?
- Did it work before? What changed?

**Step 2: Check prerequisites**
- Is the package installed? (`pip list`)
- Is the config file valid? (`cat config.json | python -m json.tool`)
- Are dependencies available? (`python -c "import yaml"`)

**Step 3: Check logs**
- Python: Add print statements or use logging
- Kubernetes: `kubectl logs pod_name`
- Rust: Set `RUST_LOG=debug` environment variable

**Step 4: Verify data**
- Print metric_data before calling evaluate()
- Print config after loading
- Print intermediate results

**Step 5: Search existing solutions**
- Check PHASE1_AUDIT for similar errors
- Check troubleshooting sections in docs/guides/
- Check GitHub issues: https://github.com/pristley/NeuralBudget/issues

---

### Enable Debug Logging

**Python:**
```python
import logging

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger("neuralbudget")
logger.setLevel(logging.DEBUG)

# Now run your code
```

**Rust (if building from source):**
```bash
RUST_LOG=neuralbudget=debug cargo test --lib
```

---

### Common Quick Fixes

| Symptom | Fix |
|---------|-----|
| Import error | `pip install neuralbudget` |
| Config error | `python -m json.tool config.json` (validates JSON) |
| YAML error | `pip install pyyaml` |
| Out-of-order data | Sort data before pushing: `metrics.sort(key=lambda x: x[0])` |
| Thread crash | Use Lock: `with batch_lock: batch.update_node(...)` |
| Buffer full | Call `agg.prune(cutoff_timestamp)` |

---

## Still Stuck?

If none of these solutions work:

1. **Check the GitHub issues:** https://github.com/pristley/NeuralBudget/issues
2. **Review related documentation:**
   - [Getting Started](../guides/getting-started.md)
   - [User Guide](../guides/user-guide.md)
   - [API Reference](api.md)
3. **Open a new issue** with:
   - Error message (full traceback)
   - Minimal code that reproduces it
   - NeuralBudget version: `pip show neuralbudget`
   - Python version: `python --version`
   - Operating system

---

## See Also

- [Glossary](glossary.md) — Definitions of terms and acronyms
- [Getting Started](../guides/getting-started.md) — Quick tutorial
- [Troubleshooting Guide](../guides/troubleshooting.md) — Mode-specific troubleshooting
- [API Reference](api.md) — Complete API documentation
