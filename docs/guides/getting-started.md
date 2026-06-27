# Getting Started: Run Your First SLO Evaluation

Complete your first NeuralBudget evaluation in 10 minutes. By the end, you evaluate a service against an availability threshold and see a pass/fail result.

## Complete These 5 Steps

### 1. Install NeuralBudget

Run one command:

```bash
pip install neuralbudget
```

**Verify:** Run this to confirm the install:
```bash
python3 -c "import neuralbudget; print('✓ NeuralBudget ready')"
```

### 2. Create an SLO Config File

Create `slo.json` in your current directory:

```json
{
  "schema_version": 1,
  "mode": "http",
  "profile": "balanced",
  "params": {
    "latency_threshold_ms": 200.0,
    "availability_threshold": 0.999
  }
}
```

**What this does:** Defines an SLO for an HTTP service with a 200ms latency threshold and 99.9% availability target.

### 3. Create a Python Script

Create `evaluate_slo.py`:

```python
try:
    from neuralbudget import NeuralBudgetClient

    client = NeuralBudgetClient()
    client.load_config("slo.json")
    
    result = client.evaluate({
        "timestamp": 1,
        "success": 9995,
        "total": 10000,
        "buckets": [
            {"upper_bound_ms": 100.0, "count": 9700},
            {"upper_bound_ms": 220.0, "count": 10000},
        ],
        "format": "prometheus_cumulative",
    })
    
    print(f"✓ SLO Pass: {result['passed']}")
    print(f"✓ Score: {result['score']:.3f}")
    print(f"✓ Availability: {result['availability']:.4f}")

except FileNotFoundError:
    print("✗ slo.json not found")
    print("  → Create slo.json in your current directory (see Step 2)")
except ValueError as e:
    print(f"✗ Invalid metrics: {e}")
    print("  → Check that bucket counts are ordered correctly")
```

### 4. Run the Evaluation

Execute the script:

```bash
python3 evaluate_slo.py
```

**Expect to see:**
```
✓ SLO Pass: True
✓ Score: 0.978
✓ Availability: 0.9995
```

If you see an error, check [docs/guides/troubleshooting.md](troubleshooting.md) or [docs/reference/errors.md](../reference/errors.md).

### 5. Modify and Re-Evaluate

Change the availability threshold in `slo.json` to 0.9999 (99.99%), then re-run the script:

```bash
python3 evaluate_slo.py
```

**Expect to see:**
```
✗ SLO Pass: False
```

This shows you how NeuralBudget evaluates thresholds. Lower the threshold back to 0.999 and verify it passes again.

## Next Steps

- **Run more modes:** [User Guide](user-guide.md) covers ML serving, stateful services, and composite DAGs
- **Use in CI/CD:** [Production Deployment](production-deployment.md) shows integration patterns
- **Understand SLO config:** [Full reference](../reference/api.md) documents all fields
- **Get help:** [Troubleshooting](troubleshooting.md) answers common questions
- Composite DAG behavior: [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)