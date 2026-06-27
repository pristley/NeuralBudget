# GenAI Token-Per-Second & TTFT - Quick Start (5 Minutes)

Monitor LLM endpoints, RAG systems, and AI workloads. Track TTFT (Time To First Token), throughput, and quality.

## ⏱️ Time: ~2 Minutes

## What You'll Do

1. ✅ Copy GenAI `slo.yaml` configuration
2. ✅ Copy `sample.json` with TTFT and throughput metrics
3. ✅ Run `neuralbudget eval`
4. ✅ See LLM health status

## 📋 Prerequisites

- NeuralBudget installed (see [HTTP guide](5-minute-http-slo.md))

## Step 1: Copy GenAI SLO Configuration

Create `genai-slo.yaml`:

```yaml
# GenAI/LLM SLO Configuration
service: "genai-api-quickstart"
description: "Quick start example for GenAI SLO evaluation"

# SLO Target: 99.9% availability
target: 99.9

# Measurement window
window: "7d"

# GenAI/LLM-specific thresholds
ttft_threshold_ms: 1000                # Time to first token < 1 second
throughput_threshold_tokens_per_sec: 50  # Min 50 tokens/sec
availability_threshold: 0.999          # Must be available 99.9%
semantic_quality_threshold: 0.85       # Quality score >= 85%

# Token limits and cost tracking
token_limits:
  max_tokens_per_request: 4000
  max_context_length: 8192
  cost_per_1k_tokens: 0.01

# Model-specific settings
model:
  name: "gpt-4-turbo"
  version: "2024-01"
  max_retries: 3
  timeout_sec: 30
  temperature_stability: true

# Burn rate alerts
alerts:
  - window: "30m"
    threshold: 0.15  # Fast burn
  - window: "4h"
    threshold: 0.08  # Medium burn
  - window: "24h"
    threshold: 0.02  # Slow burn

tags:
  mode: "genai"
  tier: "prod"
```

## Step 2: Copy GenAI Sample Metrics

Create `genai-sample.json`:

```json
{
  "timestamp": 1704067200,
  "service": "gpt-api-endpoint",
  "measurement_window": "5m",
  "requests": {
    "total": 1000,
    "successful": 999,
    "failed": 1,
    "rate_limited": 0
  },
  "latency": {
    "ttft_ms": 850.5,
    "ttft_p99_ms": 950.2,
    "total_time_to_completion_ms": 2450.3,
    "mean_ttft_ms": 780.1
  },
  "throughput": {
    "tokens_per_sec": 65.3,
    "mean_tokens_per_sec": 62.1,
    "min_tokens_per_sec": 45.8,
    "p99_tokens_per_sec": 70.2
  },
  "tokens": {
    "total_input_tokens": 450000,
    "total_output_tokens": 250000,
    "total_tokens": 700000,
    "estimated_cost_usd": 7.00
  },
  "quality": {
    "semantic_quality_score": 0.92,
    "hallucination_rate": 0.02,
    "relevance_score": 0.91
  },
  "errors": {
    "timeout": 0,
    "rate_limit": 0,
    "authentication": 1,
    "service_unavailable": 0,
    "other": 0
  },
  "model_info": {
    "model_name": "gpt-4-turbo",
    "model_version": "2024-01",
    "deployment_id": "prod-001",
    "region": "us-east-1"
  }
}
```

## Step 3: Evaluate

```bash
neuralbudget eval genai-slo.yaml genai-sample.json
```

## Expected Output: PASS

```
✓ SLO PASS - LLM endpoint performing well
  Availability: 99.90% ✓ (target: 99.90%)
  TTFT: 850.5ms ✓ (threshold: <1000ms)
  Throughput: 65.3 tokens/sec ✓ (target: ≥50)
  Quality Score: 92% ✓ (target: ≥85%)
  Latency P99: 950.2ms ✓
  Token Cost: $7.00 ✓
```

## Experiment 1: Trigger High TTFT

Edit `genai-sample.json`:

```json
"latency": {
  "ttft_ms": 1200.5,    // Changed from 850.5
  "ttft_p99_ms": 1350.2
}
```

Re-run:
```bash
neuralbudget eval genai-slo.yaml genai-sample.json
```

**Expected:**
```
✗ SLO FAIL
  TTFT: 1200.5ms ✗ (threshold: <1000ms)
  ⚠️ Action: Investigate model latency
  ⚠️ Consider: Increase replicas or use smaller model
```

## Experiment 2: Trigger Quality Degradation

Edit:
```json
"quality": {
  "semantic_quality_score": 0.78,  // Drop from 0.92
  "hallucination_rate": 0.08       // Increase from 0.02
}
```

Re-run:
```bash
neuralbudget eval genai-slo.yaml genai-sample.json
```

**Expected:**
```
✗ SLO FAIL
  Quality Score: 78% ✗ (target: ≥85%)
  Hallucination Rate: 8% ✗
  ⚠️ Action: Review model output quality
  ⚠️ Consider: Prompt engineering or model update
```

## 🎯 Understanding GenAI Metrics

### Latency Metrics

| Metric | Meaning | Impact |
|--------|---------|--------|
| `TTFT` | Time to first token | User perception of responsiveness |
| `Total latency` | End-to-end response time | Streaming quality |
| `P99 latency` | 99th percentile | Tail latency SLO |

### Throughput

- **Tokens per second**: Generation speed
- **Target:** ≥50 tokens/sec (typical for GPT-4)
- **Low throughput:** Indicates bottleneck or throttling

### Quality Metrics

| Metric | Meaning | Target |
|--------|---------|--------|
| `Semantic Quality` | Relevance & correctness | ≥85% |
| `Hallucination Rate` | Made-up facts | <5% |
| `Relevance Score` | Answer matches question | ≥85% |

### Cost Tracking

- `Total tokens`: Sum of input + output
- `Estimated cost`: Based on pricing model
- **Budget:** Monitor spend against monthly limit

## 💡 Common LLM Use Cases

### RAG (Retrieval-Augmented Generation)

```yaml
ttft_threshold_ms: 2000  # Allow more time for retrieval
semantic_quality_threshold: 0.90  # High quality needed
throughput_threshold_tokens_per_sec: 30  # Quality over speed
```

**Why:** Retrieval adds latency; quality is critical.

### Real-Time Chat

```yaml
ttft_threshold_ms: 500   # User expects fast response
throughput_threshold_tokens_per_sec: 80  # Smooth streaming
availability_threshold: 0.999  # High uptime
```

**Why:** Users perceive TTFT; streaming must be smooth.

### Batch Processing

```yaml
ttft_threshold_ms: 5000  # Less strict on latency
throughput_threshold_tokens_per_sec: 100 # Max efficiency
availability_threshold: 0.95  # Less critical
```

**Why:** Batch jobs tolerate higher latency; throughput matters.

### Content Generation

```yaml
ttft_threshold_ms: 1500
semantic_quality_threshold: 0.92  # Very high quality
token_limits:
  max_tokens_per_request: 8000  # Long-form content
```

**Why:** Quality is paramount; users wait for good output.

## 🚨 Alert Scenarios

### Scenario 1: TTFT Spike

- **Cause:** Model overload, deployment issue
- **Action:** Page on-call, check metrics
- **Timeline:** < 2 minutes

### Scenario 2: Quality Drop

- **Cause:** Model update bug, prompt issue
- **Action:** Rollback model or adjust prompt
- **Timeline:** 10-30 minutes

### Scenario 3: Throughput Degradation

- **Cause:** Token limit hit, rate limiting
- **Action:** Increase quota or use smaller model
- **Timeline:** Hours

### Scenario 4: Cost Spike

- **Cause:** Bug creating large requests, more traffic
- **Action:** Investigate, add request limits
- **Timeline:** Immediate review

## 🔄 Integration with GenAI Platforms

### OpenAI / Azure OpenAI

```python
# Track metrics from API response
response = openai.ChatCompletion.create(...)
metrics = {
    "ttft_ms": response["time_to_first_token"],
    "tokens_per_sec": response["tokens"] / response["duration"],
    "total_tokens": response["usage"]["total_tokens"]
}
```

### Anthropic Claude

```python
# Parse from response metadata
response = client.messages.create(...)
metrics = {
    "ttft_ms": response.stop_reason == "end_turn" and 850 or None,
    "tokens_per_sec": len(response.content[0].text.split()) / duration
}
```

## ❓ FAQs

**Q: What TTFT should I target?**
A: 500-1000ms for real-time, 1-5s for batch. User research shows <500ms feels instant.

**Q: How do I reduce TTFT?**
A: Use smaller models, enable streaming, increase replicas, optimize prompts.

**Q: What's a good hallucination rate?**
A: <2% is excellent, <5% is acceptable, >10% is problematic.

**Q: How do I track costs?**
A: Monitor tokens × price. Set budgets and alerts.

## 🔗 Next Steps

- [GenAI Integrations](../../guides/genai_connectors.md)
- [Burn Rate Forecasting](../../guides/burn-rate-forecasting.md)
- [Production Deployment](../../guides/production-deployment.md)
- [Advanced Alerting](../../guides/advanced_alert_dispatch.md)
- [Try Prometheus Integration](../examples/quickstart/prometheus/README.md)

## 🔗 Full Resources

- **🧠 GenAI Guide:** [GenAI Connectors](../../guides/genai_connectors.md)
- **📊 Prometheus:** [Prometheus Integration](../../guides/prometheus-scraping-examples.md)
- **📈 Forecasting:** [Burn Rate Forecasting](../../guides/burn-rate-forecasting.md)
- **🔌 API Reference:** [Full API](../../reference/api.md)

---

**Questions?** [Ask on GitHub Discussions](https://github.com/pristley/NeuralBudget/discussions)
