# Quick Start: ML SLO (5 Minutes)

Monitor model performance, accuracy, and data drift in 5 minutes.

## Step 1: Install

```bash
pip install neuralbudget
```

## Step 2: Create ML SLO Config

Save as `ml-slo.json`:

```json
{
  "schema_version": 1,
  "mode": "ml",
  "profile": "strict_accuracy",
  "params": {
    "accuracy_threshold": 0.92,
    "latency_threshold_ms": 500.0,
    "drift_threshold": 0.05
  }
}
```

## Step 3: Run Your First Evaluation

Save as `evaluate.py`:

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient()
client.load_config("ml-slo.json")

# Simulate model performance metrics
result = client.evaluate({
    "timestamp": 1624000000,
    "accuracy": 0.945,
    "precision": 0.952,
    "recall": 0.938,
    "latency_ms": 245.5,
    "inference_count": 10000,
    "drift_score": 0.032,
    "model_version": "v1.2.3"
})

print(f"✓ SLO Pass: {result['passed']}")
print(f"✓ Accuracy: {result.get('accuracy', 'N/A'):.1%}")
print(f"✓ Drift Score: {result.get('drift_score', 'N/A'):.3f}")
print(f"✓ Latency: {result.get('latency_ms', 'N/A')} ms")
```

## Step 4: Run It

```bash
python evaluate.py
```

**Expected output:**
```
✓ SLO Pass: True
✓ Accuracy: 94.5%
✓ Drift Score: 0.032
✓ Latency: 245.5 ms
```

## Next Steps

- **Learn more modes**: [User Guide](../guides/user-guide.md)
- **Anomaly detection**: [Anomaly Detection](../reference/ANOMALY_DETECTION_IMPLEMENTATION.md)
- **Production deployment**: [Production Deployment](../guides/production-deployment.md)
- **Full API reference**: [API Reference](../reference/api.md)

---

See [Getting Started](../guides/getting-started.md) for complete setup and troubleshooting.
