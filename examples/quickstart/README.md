# NeuralBudget Quickstart Examples

> **Get started in 5 minutes** with copy-paste configurations and runnable examples.

## 🎯 Pick Your Use Case

### ⚡ [HTTP Availability & Latency](http-slo/) - **Fastest Path** (2 min)

Monitor REST API uptime and response time.

```bash
# Just copy-paste and run
cp slo.yaml sample.json .
neuralbudget eval slo.yaml sample.json
# Output: ✓ SLO PASS or ✗ SLO FAIL
```

**Files:**
- `slo.yaml` - SLO configuration
- `sample.json` - Sample metrics
- `README.md` - Complete guide

---

### 🤖 [ML Model Drift & Confidence](ml-slo/) (2 min)

Monitor model accuracy, precision, recall, and feature drift.

```bash
cp slo.yaml sample.json .
neuralbudget eval slo.yaml sample.json
```

**Includes:**
- Accuracy monitoring
- Drift detection
- Confidence thresholds
- GPU utilization tracking

---

### 🧠 [GenAI TPS + TTFT](genai-slo/) (2 min)

Track LLM endpoints and AI workloads (TTFT = Time To First Token).

```bash
cp slo.yaml sample.json .
neuralbudget eval slo.yaml sample.json
```

**Monitors:**
- TTFT (Time To First Token)
- Throughput (tokens/sec)
- Quality score
- Cost tracking

---

### 📊 [Prometheus Integration](prometheus/) (3 min)

Generate and deploy Prometheus alerting rules.

```bash
neuralbudget gen-rules slo.yaml > prometheus-rules.yaml
kubectl apply -f prometheus-rules.yaml -n monitoring
```

**Includes:**
- Multi-window burn rate alerts
- Kubernetes deployment guide
- Alert configuration examples

---

### 🐍 [Python Notebook](notebook.ipynb) (2 min)

Interactive examples using NeuralBudgetClient.

```python
from neuralbudget import NeuralBudgetClient
client = NeuralBudgetClient()
result = client.evaluate(sample)
print(f"SLO: {'PASS' if result['passed'] else 'FAIL'}")
```

---

## 📋 Quick Reference

| Use Case | Time | Files | Metrics |
|----------|------|-------|---------|
| HTTP | 2min | slo.yaml, sample.json | Availability, Latency |
| ML | 2min | slo.yaml, sample.json | Accuracy, Drift |
| GenAI | 2min | slo.yaml, sample.json | TTFT, Throughput |
| Prometheus | 3min | README.md | Alert Rules |
| Python | 2min | notebook.ipynb | Programmatic API |

---

## 🚀 Getting Started (30 Seconds)

### 1. Install NeuralBudget

```bash
# Rust (fastest)
cargo install neuralbudget

# Python
pip install neuralbudget
```

### 2. Pick a Use Case

Pick the folder that matches your use case above.

### 3. Copy Configuration

```bash
cd http-slo  # (or ml-slo, genai-slo, etc.)
# Copy files to your working directory
```

### 4. Run Evaluation

```bash
neuralbudget eval slo.yaml sample.json
```

### 5. See Results

```
✓ SLO PASS
  Availability: 99.90% ✓ (target: 99.90%)
  Latency P99: 187.5ms ✓ (threshold: 200ms)
```

Done! ✅

---

## 📚 Learning Path

### Beginner

1. Start with [HTTP example](http-slo/) - simplest case
2. Run the example and see ✓ PASS
3. Edit sample.json to trigger ✗ FAIL

### Intermediate

1. Try [ML example](ml-slo/) - more metrics
2. Or [GenAI example](genai-slo/) - modern use case
3. Experiment with different thresholds

### Advanced

1. Set up [Prometheus Integration](prometheus/) for production
2. Use [Python Notebook](notebook.ipynb) for custom automation
3. Read [Full Documentation](../guides/user-guide.md)

---

## 🎓 What Each Example Teaches

### HTTP SLO
- ✅ Basic SLO concepts
- ✅ Availability calculation
- ✅ Latency percentiles
- ✅ Error budget

### ML SLO
- ✅ Model quality metrics
- ✅ Drift detection
- ✅ Multi-metric SLOs
- ✅ Retraining triggers

### GenAI SLO
- ✅ LLM-specific metrics
- ✅ Cost tracking
- ✅ Quality scoring
- ✅ Streaming optimization

### Prometheus
- ✅ Alert rule generation
- ✅ Kubernetes deployment
- ✅ Multi-window alerting
- ✅ Production monitoring

### Python Notebook
- ✅ Programmatic evaluation
- ✅ Batch processing
- ✅ Integration patterns
- ✅ Custom metrics

---

## 🔧 Common Tasks

### Make It FAIL

Edit `sample.json` to increase failures:

```json
{
  "requests": {
    "successful": 49800,  // Changed from 49950
    "total": 50000
  }
}
```

Then re-run:

```bash
neuralbudget eval slo.yaml sample.json
# Now shows: ✗ SLO FAIL
```

### Use Real Metrics

Replace sample.json with metrics from:
- **Prometheus**: `curl 'http://prometheus:9090/api/v1/query'`
- **CloudWatch**: `aws cloudwatch get-metric-statistics`
- **Datadog**: Export as JSON
- **New Relic**: Export as JSON

### Load from Python

```python
import json
from neuralbudget import NeuralBudgetClient

with open('sample.json') as f:
    sample = json.load(f)

client = NeuralBudgetClient()
result = client.evaluate(sample)
```

### Deploy to Kubernetes

```bash
# Generate Prometheus rules
neuralbudget gen-rules slo.yaml > prometheus-rules.yaml

# Deploy
kubectl apply -f prometheus-rules.yaml -n monitoring
```

### Set Up Alerting

```yaml
# Add to AlertManager config
routes:
  - match:
      alert: SLOAvailabilityBurnFast
    receiver: 'pagerduty'
  - match:
      alert: SLOLatencyBurnFast
    receiver: 'slack'
```

---

## ❓ FAQ

**Q: Which example should I start with?**
A: HTTP SLO - it's the simplest and fastest.

**Q: Can I combine multiple SLOs?**
A: Yes! Each service gets its own slo.yaml file.

**Q: How often should I evaluate?**
A: Every 5-10 minutes is typical. Adjust based on your use case.

**Q: What's error budget?**
A: The % of requests that can fail while still meeting your SLO. Once consumed, you're failing your SLO.

**Q: Can I use real metrics?**
A: Yes! Export from Prometheus, CloudWatch, Datadog, etc., and format as sample.json.

**Q: How do I set up production monitoring?**
A: Use the [Prometheus Integration](prometheus/) guide for Kubernetes deployment.

---

## 🔗 Next Steps

- **📖 Full Docs:** [User Guide](../guides/user-guide.md)
- **🚀 Production:** [Deployment Guide](../guides/production-deployment.md)
- **🤖 ML Guide:** [Anomaly Detection](../guides/anomaly_drift_detection.md)
- **🧠 GenAI Guide:** [GenAI Connectors](../guides/genai_connectors.md)
- **📊 Kubernetes:** [Kubernetes Integration](../guides/kubernetes-integration.md)
- **🔌 API Reference:** [Full API](../reference/api.md)

---

## 🐛 Troubleshooting

### "neuralbudget: command not found"

```bash
# Install it first
cargo install neuralbudget
# or
pip install neuralbudget
```

### "YAML parse error"

- Check indentation (2 spaces, no tabs)
- Ensure all required fields present
- See [YAML guide](../guides/user-guide.md#yaml-syntax)

### "Evaluation fails with 'no data'"

- Verify sample.json has all required fields
- Check field names match config
- See [Sample format](../reference/api.md#sample-format)

### "SLO always PASS even when metrics are bad"

- Check thresholds match your metrics
- Verify sample.json uses correct values
- Lower thresholds to test

---

## 📞 Get Help

- 💬 [GitHub Discussions](https://github.com/pristley/NeuralBudget/discussions)
- 🐛 [Report Issues](https://github.com/pristley/NeuralBudget/issues)
- 📚 [Full Documentation](../guides/)
- 🤝 [Contributing](../../CONTRIBUTING.md)

---

## ✅ Validation

All examples are validated:

- ✅ YAML syntax valid
- ✅ JSON syntax valid
- ✅ Required fields present
- ✅ Executable commands
- ✅ Documentation complete
- ✅ Weekly CI validation

See [GitHub Actions](../../.github/workflows/quickstart-validation.yml) for details.

---

**Ready?** Pick your use case above and get started! 🚀
