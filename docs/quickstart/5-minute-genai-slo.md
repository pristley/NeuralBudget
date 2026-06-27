# Quick Start: GenAI SLO (5 Minutes)

Monitor LLM endpoints, RAG systems, and AI workloads in 5 minutes.

## Step 1: Install

```bash
pip install neuralbudget
```

## Step 2: Create GenAI SLO Config

Save as `genai-slo.json`:

```json
{
  "schema_version": 1,
  "mode": "genai",
  "profile": "strict_latency",
  "params": {
    "ttft_threshold_ms": 1000.0,
    "throughput_threshold_tokens_per_sec": 50.0,
    "availability_threshold": 0.999
  }
}
```

## Step 3: Run Your First Evaluation

Save as `evaluate.py`:

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient()
client.load_config("genai-slo.json")

# Simulate GenAI/LLM metrics
result = client.evaluate({
    "timestamp": 1624000000,
    "ttft_ms": 850.5,
    "throughput_tokens_per_sec": 65.3,
    "requests_total": 1000,
    "requests_success": 999,
    "requests_error": 1,
    "semantic_quality_score": 0.92,
    "model_name": "gpt-4-turbo"
})

print(f"✓ SLO Pass: {result['passed']}")
print(f"✓ Availability: {result.get('availability', 'N/A'):.4f}")
print(f"✓ TTFT: {result.get('ttft_ms', 'N/A')} ms")
print(f"✓ Throughput: {result.get('throughput_tokens_per_sec', 'N/A')} tokens/sec")
```

## Step 4: Run It

```bash
python evaluate.py
```

**Expected output:**
```
✓ SLO Pass: True
✓ Availability: 0.9990
✓ TTFT: 850.5 ms
✓ Throughput: 65.3 tokens/sec
```

## Next Steps

- **Learn more modes**: [User Guide](../guides/user-guide.md)
- **GenAI integrations**: [GenAI Connectors](../guides/genai_connectors.md)
- **Production deployment**: [Production Deployment](../guides/production-deployment.md)
- **Full API reference**: [API Reference](../reference/api.md)

---

See [Getting Started](../guides/getting-started.md) for complete setup and troubleshooting.
