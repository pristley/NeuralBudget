# NeuralBudget Quick Start Guides

**Welcome!** Choose your use case and get started in 5 minutes. Each guide includes copy-paste configuration, sample data, and expected output.

## 🚀 Pick Your Use Case

### 1. [HTTP Availability & Latency](http-slo.md) ⚡ **Fastest Path**

Monitor REST API uptime and response time.

```bash
# 1. Create slo.yaml (copy-paste from guide)
# 2. Create sample.json (copy-paste from guide)
# 3. neuralbudget eval slo.yaml sample.json
# Result: ✓ SLO PASS or ✗ SLO FAIL
```

**Metrics:**
- Availability: 99.9%
- P99 Latency: < 200ms
- Error Budget: 30-day window

**Time:** ~2 minutes

---

### 2. [ML Model Drift & Confidence](5-minute-ml-slo.md) 🤖

Monitor ML model performance, accuracy, and data drift.

```bash
# 1. slo.yaml with ml mode
# 2. sample.json with drift/confidence values
# 3. eval
```

**Metrics:**
- Accuracy: ≥92%
- Feature Drift: ≤15%
- Prediction Confidence: ≥80%

**Time:** ~2 minutes

---

### 3. [GenAI TPS + TTFT](5-minute-genai-slo.md) 🧠

Track LLM endpoints and AI workloads (TTFT = Time To First Token).

```bash
# 1. slo.yaml with genai mode
# 2. sample.json with tokens/latency
# 3. eval
```

**Metrics:**
- TTFT: < 1 second
- Throughput: ≥ 50 tokens/sec
- Availability: 99.9%
- Quality: ≥85%

**Time:** ~2 minutes

---

### 4. [Prometheus Integration](5-minute-http-slo.md) 📊

Generate and deploy Prometheus alerting rules from SLOs.

```bash
# 1. neuralbudget gen-rules slo.yaml > rules.yaml
# 2. kubectl apply -f rules.yaml
# 3. Watch PrometheusRules fire in Prometheus UI
```

**Includes:**
- Multi-window burn rate alerts
- Fast/medium/slow alert tiers
- Kubernetes PrometheusRule deployment

**Time:** ~3 minutes

---

### 5. [Python Notebook](../guides/getting-started.md) 🐍

Programmatic SLO evaluation using NeuralBudgetClient.

```python
from neuralbudget import NeuralBudgetClient
client = NeuralBudgetClient()
result = client.evaluate(sample)
print(f"SLO: {'PASS' if result['passed'] else 'FAIL'}")
```

**Includes:**
- Simple pass/fail checks
- Batch evaluation
- Alerting integration
- Config file loading

**Time:** ~2 minutes

---

### 6. [Composite DAG (Service Dependencies)](5-minute-composite-dag-slo.md) 🔗

Model inter-service dependencies and failure propagation with topological DAG evaluation.

```bash
# 1. slo.yaml with services and dependency edges
# 2. sample.json with per-service scores
# 3. neuralbudget eval → see cascading failures
```

**Metrics:**
- Per-service SLO scores (0.0-1.0)
- Dependency failure penalties
- Topological evaluation order
- Global system SLO

**Unique Value:**
- ✅ **Automatically propagates failures** (DB fails → API fails → Web fails)
- ✅ **Deterministic topological ordering** (dependencies evaluated first)
- ✅ **Global SLO reflects reality** (not misleading parallel aggregation)
- ✅ **Cycle detection** (prevents invalid topologies)

**Time:** ~2 minutes

---

## 📋 Examples Directory Structure

All examples are copy-paste ready:

```
examples/quickstart/
├── http-slo/
│   ├── slo.yaml          # Copy-paste HTTP SLO config
│   ├── sample.json       # Copy-paste sample metrics
│   └── README.md         # Step-by-step guide
├── ml-slo/
│   ├── slo.yaml          # Copy-paste ML SLO config
│   ├── sample.json       # Copy-paste ML metrics
│   └── README.md
├── genai-slo/
│   ├── slo.yaml          # Copy-paste GenAI SLO config
│   ├── sample.json       # Copy-paste GenAI metrics
│   └── README.md
├── composite-dag/
│   ├── slo.yaml          # Copy-paste Composite DAG config
│   ├── sample.json       # Copy-paste dependency metrics
│   └── README.md         # Cascading failure guide
├── prometheus/
│   ├── README.md         # Prometheus integration guide
│   └── rules-template.yaml
└── notebook.ipynb        # Interactive Python notebook
```

---

## ⏱️ Time Estimates

| Guide | Setup | Run | Total |
|-------|-------|-----|-------|
| HTTP SLO | 1 min | 1 min | **2 min** |
| ML SLO | 1 min | 1 min | **2 min** |
| GenAI SLO | 1 min | 1 min | **2 min** |
| Composite DAG | 1 min | 1 min | **2 min** |
| Prometheus | 1 min | 2 min | **3 min** |
| Python | 1 min | 1 min | **2 min** |

---

## 🎯 Success Criteria

✅ New user can pick a use case
✅ Copy config and sample data
✅ Run evaluation in < 5 minutes
✅ See clear success (PASS/FAIL)
✅ All examples actually run
✅ Expected output matches guide

---

## 🔗 Quick Links

- **📖 Full Documentation:** [Getting Started](../guides/getting-started.md)
- **🚀 Production Deployment:** [Deployment Guide](../guides/production-deployment.md)
- **🤖 ML Integration:** [Anomaly Detection](../guides/anomaly_drift_detection.md)
- **🧠 GenAI Guide:** [GenAI Connectors](../guides/genai_connectors.md)
- **📊 Prometheus Guide:** [Prometheus Integration](../guides/kubernetes-integration.md)
- **🔌 API Reference:** [Full API](../reference/api.md)

---

## 💡 Tips

### Start Here (Recommended Order)

1. **New to SLOs?** → Start with [HTTP SLO](5-minute-http-slo.md)
2. **Have a microservices architecture?** → Try [Composite DAG](5-minute-composite-dag-slo.md)
3. **Have a Python service?** → Try [Getting Started](../guides/getting-started.md)
4. **Using Kubernetes?** → Check [Prometheus Integration](5-minute-http-slo.md)
5. **Running ML models?** → See [ML Drift](5-minute-ml-slo.md)
6. **Operating LLMs?** → Go to [GenAI SLO](5-minute-genai-slo.md)

### Common Issues

**"Can't find neuralbudget command"**
```bash
# Install Rust version
cargo install neuralbudget

# Or Python version
pip install neuralbudget
```

**"YAML parse error"**
- Check indentation (2 spaces, no tabs)
- Ensure all required fields present
- See [YAML Syntax Guide](../guides/user-guide.md#yaml-syntax)

**"Evaluation fails with 'no data'"**
- Verify sample.json has all required fields
- Check field names match SLO config
- See [Sample Format](../reference/api.md#sample-format)

---

## 🆘 Need Help?

- 🐛 [Report Issues](https://github.com/pristley/NeuralBudget/issues)
- 💬 [Discussions](https://github.com/pristley/NeuralBudget/discussions)
- 📧 [Email Support](mailto:support@example.com)
- 📚 [Full Docs](../guides/user-guide.md)

---

## 📊 What's an SLO?

**SLO (Service Level Objective)** = Target you commit to

```
Example SLO: "99.9% uptime, P99 latency < 200ms"
↓
Error Budget: "I can afford 0.1% downtime this month"
↓
Burn Rate: "If errors > 10%/hour, page me immediately"
↓
Action: "Fix the issue before error budget runs out"
```

**Why?** Helps you:
- ✅ Define what "good" means
- ✅ Track progress toward goals
- ✅ Alert when things break
- ✅ Make data-driven decisions

---

**Ready?** Pick your use case above and get started! 🚀
