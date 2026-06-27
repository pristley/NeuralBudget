# Troubleshooting Guide

**Last Updated:** June 27, 2026

This guide helps you diagnose and resolve common issues when using NeuralBudget. For detailed error descriptions, see [Error Reference](../reference/errors.md).

---

## Quick Troubleshooting Decision Tree

```
Something's not working...
├─ Can't import NeuralBudget?
│  └─ → "Installation Issues" (below)
├─ Getting an error message?
│  └─ → "Error Reference" (docs/reference/errors.md)
├─ Code runs but results seem wrong?
│  └─ → "Evaluation Issues" (below)
├─ Performance is slow?
│  └─ → "Performance Issues" (below)
├─ App crashes or hangs?
│  └─ → "Thread Safety" (below)
└─ Running on Kubernetes?
   └─ → "Kubernetes Deployment" (below)
```

---

## Installation Issues

### Problem: `pip install` fails with compiler error

**Symptoms:**
```
error: Microsoft Visual C++ 14.0 or greater is required
```

**Diagnosis:**
- Building from source requires C++ compiler
- Pre-built wheels may not exist for your platform

**Solutions:**

**Option 1: Install from pre-built wheel (faster)**
```bash
# Use PyPI (recommended)
pip install neuralbudget --only-binary :all:
```

**Option 2: Use conda (pre-built for common platforms)**
```bash
conda install -c conda-forge neuralbudget
```

**Option 3: Install build tools (if building from source):**
```bash
# macOS
brew install rustup
rustup default stable

# Ubuntu/Debian
sudo apt-get install -y build-essential rust-dev

# Windows
# Download and install Visual Studio Build Tools
# https://visualstudio.microsoft.com/downloads/
```

---

### Problem: Wrong Python version

**Symptoms:**
```
ERROR: No matching distribution found for neuralbudget
```

**Diagnosis:**
- NeuralBudget requires Python 3.9+
- You may be using Python 2.7 or Python 3.8

**Solutions:**

Check your Python version:
```bash
python --version
python3 --version
```

If Python 3.9+ not installed:
```bash
# macOS
brew install python@3.11

# Ubuntu/Debian
sudo apt-get install python3.11

# Windows
# Download from python.org or use pyenv
```

Use explicit Python version:
```bash
python3.11 -m pip install neuralbudget
```

---

## Evaluation Issues

### Problem: Results always show `pass: false` even with good data

**Symptoms:**
```python
result = client.evaluate({...})
print(result)  # {'pass': False, 'score': 0.01, ...}
```

**Diagnosis:**

1. **Threshold too strict?**
   ```python
   # Check your thresholds
   params = config["params"]
   print(f"Latency threshold: {params['latency_threshold_ms']}ms")
   print(f"Success rate target: {params.get('success_rate_target', 0.999)}")
   ```

2. **Data values too low?**
   ```python
   # Print the actual values
   print(f"Success: {metric_data['success']} / {metric_data['total']}")
   print(f"Availability: {metric_data['success'] / metric_data['total']:.4f}")
   print(f"Target: {config['params']['success_rate_target']:.4f}")
   ```

3. **Wrong SLO mode?**
   ```python
   # Verify mode matches your metrics
   print(f"Config mode: {config['mode']}")
   print(f"Available modes: http, stateful, ml, genai")
   ```

**Solutions:**

**If threshold too strict:**
```json
{
  "schema_version": 1,
  "mode": "http",
  "profile": "loose_latency",
  "params": {
    "latency_threshold_ms": 500.0,
    "success_rate_target": 0.99
  }
}
```

**If data insufficient:**
Ensure you have real production data:
```python
# Good example
metric_data = {
    "timestamp": 1625097600000,
    "success": 9950,
    "total": 10000,  # 99.5% availability
    "buckets": [
        {"upper_bound_ms": 100.0, "count": 5000},
        {"upper_bound_ms": 200.0, "count": 9500},
        {"upper_bound_ms": 500.0, "count": 10000},
    ],
    "format": "prometheus_cumulative"
}
```

---

### Problem: Getting different results each time

**Symptoms:**
```python
result1 = client.evaluate(metric_data)
result2 = client.evaluate(metric_data)  # Different result!
```

**Diagnosis:**
- NeuralBudget is deterministic; same input → same output
- This usually means input is changing

**Solutions:**

1. **Check if metric_data is modified:**
   ```python
   import json
   
   metric_data = {...}
   
   # Debug: verify input consistency
   print("Input hash:", hash(json.dumps(metric_data, sort_keys=True)))
   result1 = client.evaluate(metric_data)
   
   print("Input hash:", hash(json.dumps(metric_data, sort_keys=True)))
   result2 = client.evaluate(metric_data)
   ```

2. **Verify config hasn't changed:**
   ```python
   # If reloading config between evaluations
   client.load_config("slo.json")  # Config 1
   result1 = client.evaluate(...)
   
   client.load_config("slo.json")  # Config 2 (might differ!)
   result2 = client.evaluate(...)
   ```

3. **Check for floating-point precision:**
   ```python
   # If using very small values
   result = client.evaluate({
       "timestamp": 1,
       "success": 999999999,
       "total": 1000000000,  # 99.9999999% → rounds
   })
   ```

---

## Performance Issues

### Problem: Evaluation is slow (>1 second for 1,000 metrics)

**Symptoms:**
```python
import time

start = time.time()
result = client.evaluate({...})  # Takes 1+ seconds
elapsed = time.time() - start
print(f"Evaluation took {elapsed:.2f}s")
```

**Diagnosis:**

1. **Using ParallelMetricBatch without parallel evaluation?**
   ```python
   # Without GIL release
   batch = ParallelMetricBatch([...])
   for item in batch.nodes:
       # Sequential Python loop - slow!
       pass
   ```

2. **Running on single-core machine?**
   ```bash
   grep -c processor /proc/cpuinfo
   ```

3. **Locks contending in multi-threaded app?**

**Solutions:**

**Use `evaluate()` which releases GIL:**
```python
# ✅ FAST - releases GIL, uses all cores
batch = ParallelMetricBatch([...])
results = batch.evaluate()  # Sub-millisecond for 1k metrics

# ❌ SLOW - holds GIL, single core
for node in batch.nodes:
    result = node.evaluate()
```

**Check CPU core count:**
```python
import multiprocessing
print(f"CPU cores: {multiprocessing.cpu_count()}")

# If only 1 core, can't parallelize
# Consider using multi-process or distributed setup
```

**For 10k+ metrics, batch them:**
```python
def evaluate_large_batch(all_metrics, batch_size=10000):
    for i in range(0, len(all_metrics), batch_size):
        batch = all_metrics[i:i + batch_size]
        batch_obj = ParallelMetricBatch(batch)
        results = batch_obj.evaluate()
        yield results
```

---

### Problem: Memory usage grows unbounded

**Symptoms:**
```
Memory: 100 MB → 500 MB → 1 GB (keeps growing)
```

**Diagnosis:**

1. **StreamingAggregator not pruning?**
   ```python
   size = agg.len()
   if size > 1000000:
       print("WARNING: Large buffer, call prune()")
   ```

2. **Keeping old batch objects?**
   ```python
   # ❌ WRONG - batches accumulate in memory
   batches = []
   for i in range(10000):
       batch = ParallelMetricBatch([...])
       batches.append(batch)  # Keeps old batches
   
   # ✅ RIGHT - reuse or discard
   for i in range(10000):
       batch = ParallelMetricBatch([...])
       results = batch.evaluate()
       del batch  # Explicitly free
   ```

**Solutions:**

**For StreamingAggregator:**
```python
agg = StreamingAggregator()

# Periodic pruning
import time

while True:
    agg.push(time.time(), value)
    
    # Prune every 60 seconds
    if time.time() % 60 < 1:
        cutoff = time.time() - 3600000  # Keep last hour
        agg.prune(int(cutoff * 1000))
```

**For batch evaluation:**
```python
# Process one batch at a time, don't accumulate
def process_metrics_streaming(metric_stream, batch_size=10000):
    batch_data = []
    
    for metric in metric_stream:
        batch_data.append(metric)
        
        if len(batch_data) >= batch_size:
            batch = ParallelMetricBatch(batch_data)
            results = batch.evaluate()
            yield results
            batch_data = []  # Reset for next batch
            # old batch is garbage collected

    # Process remainder
    if batch_data:
        batch = ParallelMetricBatch(batch_data)
        results = batch.evaluate()
        yield results
```

---

## Thread Safety Issues

### Problem: App crashes with "Data corruption detected" or random panics

**Symptoms:**
```
thread 'pool-3' panicked at 'assertion failed: ...'
SIGABRT received
RuntimeError: Data corruption
```

**Diagnosis:**
- `ParallelMetricBatch` is not thread-safe for concurrent mutations
- Multiple threads calling `update_node()` or `evaluate()` simultaneously

**Solutions:**

**Option 1: Single-threaded use (simplest)**
```python
# ✅ SAFE
batch = ParallelMetricBatch([...])
batch.update_node("metric", 100.0)
results = batch.evaluate()

# Works fine when everything happens in one thread
```

**Option 2: Protect with Lock**
```python
# ✅ SAFE
from threading import Lock

batch = ParallelMetricBatch([...])
batch_lock = Lock()

def update_metrics(data):
    with batch_lock:
        for metric_id, value in data.items():
            batch.update_node(metric_id, value)

def get_results():
    with batch_lock:
        return batch.evaluate()

# Now multiple threads can safely call these functions
```

**Option 3: Per-thread batches**
```python
# ✅ SAFE
import threading

batch_local = threading.local()

def get_thread_batch():
    if not hasattr(batch_local, 'batch'):
        batch_local.batch = ParallelMetricBatch([...])
    return batch_local.batch

def worker_thread():
    batch = get_thread_batch()
    batch.evaluate()  # Safe - no shared state

# Each thread has its own batch
```

---

## Kubernetes Deployment

### Problem: Pod in `CrashLoopBackOff`

**Symptoms:**
```bash
kubectl get pods
# neuralbudget-xxx   0/1   CrashLoopBackOff
```

**Diagnosis:**

1. **Check pod logs:**
   ```bash
   kubectl logs deployment/neuralbudget
   kubectl logs deployment/neuralbudget --previous
   ```

2. **Common causes:**
   - Config not mounted
   - Invalid permissions
   - Port already in use

**Solutions:**

**Step 1: Verify ConfigMap:**
```bash
kubectl get configmap neuralbudget-config
kubectl describe configmap neuralbudget-config
```

**Step 2: Check if mounted:**
```bash
kubectl exec <pod-name> -- ls -la /etc/neuralbudget/
```

**Step 3: Re-apply manifests:**
```bash
# Apply in order
kubectl apply -f examples/kubernetes/configmap.yaml
kubectl apply -f examples/kubernetes/deployment.yaml
kubectl apply -f examples/kubernetes/service.yaml
```

**Step 4: View detailed error:**
```bash
kubectl describe pod <pod-name>
# Look for "Last State" → "Reason" → "Message"
```

---

### Problem: Prometheus can't scrape metrics

**Symptoms:**
```
Prometheus UI: "Target unreachable"
OR
No metrics appear in Prometheus
```

**Diagnosis:**

1. **Check service is running:**
   ```bash
   kubectl get svc neuralbudget-exporter
   kubectl get endpoints neuralbudget-exporter
   ```

2. **Verify port configuration:**
   ```bash
   # Port in Service should match deployment containerPort (8080)
   kubectl describe svc neuralbudget-exporter
   ```

**Solutions:**

**Step 1: Test endpoint locally:**
```bash
# Port forward to local machine
kubectl port-forward svc/neuralbudget-exporter 8080:8080

# In another terminal
curl http://localhost:8080/metrics
# Should return: # HELP, # TYPE, metric names...
```

**Step 2: Test from Prometheus pod:**
```bash
kubectl exec <prometheus-pod> -- \
  curl http://neuralbudget-exporter.default.svc:8080/metrics
```

**Step 3: Check ServiceMonitor (if using)**
```bash
kubectl get servicemonitor
kubectl describe servicemonitor neuralbudget-exporter
```

---

## Integration Issues

### Problem: Slack/Email alerts not sending

**Symptoms:**
```python
manager.dispatch_to_slack(webhook_url, message)
# No error, but no Slack message received
```

**Diagnosis:**

1. **Webhook URL invalid?**
   ```python
   # Verify webhook from Slack settings
   # Should be https://hooks.slack.com/services/...
   ```

2. **Network blocked?**
   ```python
   import urllib.request
   try:
       urllib.request.urlopen(webhook_url)
   except Exception as e:
       print(f"Cannot reach webhook: {e}")
   ```

**Solutions:**

**Test webhook locally:**
```python
import json
import urllib.request

webhook_url = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
message = {
    "text": "Test from NeuralBudget",
    "attachments": [
        {
            "color": "good",
            "text": "SLO passed"
        }
    ]
}

try:
    req = urllib.request.Request(
        webhook_url,
        data=json.dumps(message).encode('utf-8'),
        headers={'Content-Type': 'application/json'}
    )
    with urllib.request.urlopen(req) as response:
        print(f"Status: {response.status}")
except Exception as e:
    print(f"Error: {e}")
```

---

## Debugging Techniques

### Enable Verbose Logging

```python
import logging

logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

# NeuralBudget logs
neuralbudget_logger = logging.getLogger('neuralbudget')
neuralbudget_logger.setLevel(logging.DEBUG)

# Now run your code with debug output
client = NeuralBudgetClient()
client.load_config("slo.json")
result = client.evaluate({...})
```

### Print Config After Loading

```python
import json

client = NeuralBudgetClient()
client.load_config("slo.json")

# Debug: print loaded config
print("Loaded config:")
print(json.dumps(client.config, indent=2))
```

### Validate Input Before Evaluating

```python
def validate_and_evaluate(client, metric_data):
    # Validate structure
    required_fields = ["timestamp", "success", "total", "buckets", "format"]
    for field in required_fields:
        if field not in metric_data:
            raise ValueError(f"Missing required field: {field}")
    
    # Validate types
    assert isinstance(metric_data["timestamp"], int)
    assert isinstance(metric_data["success"], int)
    assert isinstance(metric_data["total"], int)
    assert isinstance(metric_data["buckets"], list)
    
    # Validate values
    assert metric_data["success"] <= metric_data["total"], \
        "Success cannot exceed total"
    
    # Now safe to evaluate
    return client.evaluate(metric_data)
```

---

## When All Else Fails

### Collect Diagnostic Information

```bash
# Version information
python --version
pip show neuralbudget

# System information
uname -a
df -h

# NeuralBudget logs (if saved to file)
tail -100 neuralbudget.log

# If Kubernetes
kubectl version
kubectl describe pod <pod-name>
```

### Report an Issue

When reporting a bug, include:

1. **Error message** (full traceback)
   ```python
   import traceback
   try:
       # your code
   except Exception as e:
       traceback.print_exc()
   ```

2. **Minimal code to reproduce**
   ```python
   from neuralbudget import NeuralBudgetClient
   
   client = NeuralBudgetClient()
   client.load_config("slo.json")
   result = client.evaluate({
       "timestamp": 1,
       "success": 99,
       "total": 100,
       ...
   })
   ```

3. **Environment**
   - NeuralBudget version: `pip show neuralbudget`
   - Python version: `python --version`
   - OS: `uname -a`
   - Kubernetes version (if relevant): `kubectl version`

4. **Steps to reproduce**
   - What did you do?
   - What did you expect?
   - What happened instead?

---

## Related Documentation

- [Error Reference](../reference/errors.md) — Detailed error messages and solutions
- [Glossary](../reference/glossary.md) — Term definitions
- [Getting Started](../guides/getting-started.md) — Quick tutorial
- [User Guide](../guides/user-guide.md) — Feature guide
- [Kubernetes Integration](../guides/kubernetes-integration.md) — Deployment guide
