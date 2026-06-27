# GenAI/LLM SLO Quick Start (5 Minutes)

Monitor LLM endpoints, RAG systems, and AI workloads in 5 minutes.

## ⚡ What You'll Do

1. Copy `slo.yaml` and `sample.json`
2. Run one command
3. See LLM performance metrics ✓ PASS or ✗ FAIL

## 📋 Step-by-Step

### Step 1: Copy the Files

```bash
cp slo.yaml ./
cp sample.json ./
```

### Step 2: Install neuralbudget (if needed)

```bash
cargo install neuralbudget
# or
pip install neuralbudget
```

### Step 3: Evaluate GenAI Performance

```bash
neuralbudget eval slo.yaml sample.json
```

### Expected Output

```
✓ SLO PASS
  Availability: 99.90% ✓ (target: 99.90%)
  TTFT: 850.5ms ✓ (threshold: <1000ms)
  Throughput: 65.3 tokens/sec ✓ (target: ≥50)
  Quality Score: 92% ✓ (target: ≥85%)
  Latency P99: 950.2ms ✓
  Token Cost: $7.00 ✓
```

### Step 4: Trigger a Failure - High TTFT

Edit `sample.json` to simulate slow response:

```json
"latency": {
  "ttft_ms": 1200.5,  // Change from 850.5
  "ttft_p99_ms": 1350.2
}
```

Re-run:

```bash
neuralbudget eval slo.yaml sample.json
```

Expected:
```
✗ SLO FAIL
  TTFT: 1200.5ms ✗ (threshold: <1000ms)
  ⚠️ Action: Investigate model latency
  ⚠️ Consider: Increase replicas or use smaller model
```

### Step 5: Trigger a Failure - Quality Degradation

Edit `sample.json` quality metrics:

```json
"quality": {
  "semantic_quality_score": 0.78,  // Drop from 0.92
  "hallucination_rate": 0.08       // Increase from 0.02
}
```

Re-run:

```bash
neuralbudget eval slo.yaml sample.json
```

Expected:
```
✗ SLO FAIL
  Quality Score: 78% ✗ (target: ≥85%)
  Hallucination Rate: 8% ✗
  ⚠️ Action: Review model output quality
  ⚠️ Consider: Prompt engineering or model update
```

### Step 6: Monitor Costs

Check token usage and costs:

```json
"tokens": {
  "total_tokens": 700000,
  "estimated_cost_usd": 7.00
}
```

This helps track:
- Total cost over time
- Cost per request
- Budget consumption

## 🎯 Key Metrics Explained

| Metric | Meaning |
|--------|---------|
| `TTFT` | Time for first token (user perception) |
| `Throughput` | Tokens generated per second |
| `Semantic Quality` | Output relevance and correctness |
| `Hallucination Rate` | Made-up facts (should be low) |
| `Token Cost` | Monetary cost of API calls |

## 💡 Common LLM Use Cases

### RAG (Retrieval-Augmented Generation)
```yaml
ttft_threshold_ms: 2000  # Allow more time for retrieval
semantic_quality_threshold: 0.90  # High quality needed
```

### Real-time Chat
```yaml
ttft_threshold_ms: 500   # User expects fast response
throughput_threshold_tokens_per_sec: 80  # Smooth streaming
```

### Batch Processing
```yaml
ttft_threshold_ms: 5000  # Less strict on latency
throughput_threshold_tokens_per_sec: 100 # Max efficiency
```

## 📚 Next Steps

- **GenAI Integrations**: [GenAI Connectors](../../guides/genai_connectors.md)
- **Burn Rate Forecasting**: [Burn Rate Forecasting](../../guides/burn-rate-forecasting.md)
- **Production Setup**: [Production Deployment](../../guides/production-deployment.md)
- **Advanced Alerting**: [Advanced Alert Dispatch](../../guides/advanced_alert_dispatch.md)

## 🔗 Learn More

- [Full SLO Guide](../../guides/user-guide.md)
- [API Reference](../../reference/api.md)
- [Troubleshooting](../../guides/troubleshooting.md)
