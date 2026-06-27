# Quick Start: HTTP/gRPC SLO (5 Minutes)

Evaluate latency and availability for HTTP services in 5 minutes.

## Step 1: Install

```bash
pip install neuralbudget
```

## Step 2: Create SLO Config

Save as `http-slo.json`:

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

## Step 3: Run Your First Evaluation

Save as `evaluate.py`:

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient()
client.load_config("http-slo.json")

# Simulate metric data from your service
result = client.evaluate({
    "timestamp": 1624000000,
    "success": 9995,
    "total": 10000,
    "buckets": [
        {"upper_bound_ms": 50.0, "count": 3000},
        {"upper_bound_ms": 100.0, "count": 7000},
        {"upper_bound_ms": 200.0, "count": 9500},
        {"upper_bound_ms": 500.0, "count": 9990},
        {"upper_bound_ms": 1000.0, "count": 10000},
    ],
    "format": "prometheus_cumulative",
})

print(f"✓ SLO Pass: {result['passed']}")
print(f"✓ Availability: {result.get('availability', 'N/A'):.4f}")
print(f"✓ Latency P99: {result.get('latency_p99_ms', 'N/A')} ms")
```

## Step 4: Run It

```bash
python evaluate.py
```

**Expected output:**
```
✓ SLO Pass: True
✓ Availability: 0.9995
✓ Latency P99: 198 ms
```

## Next Steps

- **Learn more modes**: [User Guide](../guides/user-guide.md)
- **Production deployment**: [Production Deployment](../guides/production-deployment.md)
- **Prometheus integration**: [Prometheus Scraping](../guides/prometheus-scraping-examples.md)
- **Full API reference**: [API Reference](../reference/api.md)

---

See [Getting Started](../guides/getting-started.md) for complete setup and troubleshooting.
