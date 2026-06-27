# HTTP Availability & Latency SLO - Quick Start (5 Minutes)

Monitor REST API uptime and response time. This is the **fastest path** to getting started with NeuralBudget.

## ⏱️ Time: ~2 Minutes

## What You'll Do

1. ✅ Copy `slo.yaml` configuration
2. ✅ Copy `sample.json` metrics  
3. ✅ Run `neuralbudget eval`
4. ✅ See **✓ SLO PASS** or **✗ SLO FAIL**

## 📋 Prerequisites

- NeuralBudget installed:
  ```bash
  cargo install neuralbudget
  # or
  pip install neuralbudget
  ```

## Step 1: Copy SLO Configuration

Create a file named `slo.yaml`:

```yaml
# HTTP/gRPC SLO Configuration - Copy & Paste Ready
service: "quickstart-api"
description: "Quick start example for HTTP SLO evaluation"

# SLO Target: 99.9% availability
target: 99.9

# Measurement window
window: "30d"

# HTTP-specific thresholds
latency_threshold_ms: 200     # P99 latency must be < 200ms
availability_threshold: 0.999  # Availability must be > 99.9%
latency_percentile: 0.99       # Use P99 percentile

# Multi-window burn rate alerts (recommended by Google SRE)
alerts:
  - window: "1h"
    threshold: 0.10  # Fast burn (1hr)
  - window: "6h"
    threshold: 0.05  # Medium burn (6hr)
  - window: "24h"
    threshold: 0.02  # Slow burn (24hr)

# Optional: Custom outlier filtering
outlier_filter:
  enabled: true
  mad_threshold: 3.5
  min_samples: 3

# Service tags
tags:
  mode: "http"
  tier: "prod"
```

## Step 2: Copy Sample Metrics

Create a file named `sample.json`:

```json
{
  "timestamp": 1704067200,
  "service": "quickstart-api",
  "measurement_window": "5m",
  "requests": {
    "total": 50000,
    "successful": 49950,
    "failed": 50
  },
  "latency": {
    "p50_ms": 45.2,
    "p90_ms": 125.8,
    "p99_ms": 187.5,
    "p99_9_ms": 250.3,
    "max_ms": 892.1
  },
  "errors": {
    "connection_refused": 5,
    "timeout": 0,
    "service_unavailable": 35,
    "auth_failed": 10,
    "other": 0
  },
  "annotations": {
    "deployment_window": false,
    "load_spike": false,
    "network_incident": false
  }
}
```

## Step 3: Evaluate

```bash
neuralbudget eval slo.yaml sample.json
```

## Expected Output: PASS

```
✓ SLO PASS
  Availability: 99.90% ✓ (target: 99.90%)
  Latency P99: 187.5ms ✓ (threshold: 200ms)
  Error Budget Used: 0.001% (30d window)
```

### Metrics Breakdown

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Availability | 99.90% | ≥99.9% | ✓ PASS |
| P99 Latency | 187.5ms | <200ms | ✓ PASS |
| Error Budget | 99.999% | >0% | ✓ PASS |

## Experiment: Make It FAIL

To understand how failures work, edit `sample.json` and increase failures:

```json
"successful": 49800,   // Changed from 49950
```

Then run:
```bash
neuralbudget eval slo.yaml sample.json
```

**Expected output:**
```
✗ SLO FAIL
  Availability: 99.60% ✗ (target: 99.90%)
  Latency P99: 187.5ms ✓
  Error Budget: Warning! Alert triggered
  - Fast burn rate: 10%/hour (threshold: 10%)
```

## 🎯 Understanding the Configuration

### Key Fields in `slo.yaml`

| Field | Meaning | Example |
|-------|---------|---------|
| `target` | Availability percentage | `99.9` = 99.9% uptime |
| `window` | Error budget period | `30d` = monthly budget |
| `latency_threshold_ms` | P99 latency limit | `200` = must be < 200ms |
| `availability_threshold` | Success rate needed | `0.999` = 99.9% |
| `alerts[].window` | Alert time window | `1h`, `6h`, `24h` |
| `alerts[].threshold` | Alert burn rate | `0.10` = 10%/hour |

### Sample Data Fields

**Requests:**
- `total`: Total requests in window
- `successful`: Successful requests
- `failed`: Failed requests (total - successful)

**Latency:**
- `p50_ms`: Median latency
- `p90_ms`: 90th percentile
- `p99_ms`: 99th percentile (used in SLO)
- `p99_9_ms`: 99.9th percentile

**Errors:**
- Count of each error type
- Used to categorize failures

## 📊 Interpreting Results

### SLO PASS
✓ All thresholds met
✓ Error budget available
✓ Service is healthy

**Next:** Monitor continuously, update as needed

### SLO FAIL
✗ One or more thresholds exceeded
✗ Error budget consumed or running out
✗ Immediate action needed

**Next Steps:**
1. Investigate root cause
2. Page on-call engineer
3. Fix issue to restore budget
4. Post-mortem to prevent recurrence

## 🔄 Using Real Metrics

### From Prometheus

```bash
# Query Prometheus for last 5 minutes
curl 'http://prometheus:9090/api/v1/query?query=requests_total'
```

Then format as sample.json and evaluate:

```bash
neuralbudget eval slo.yaml prometheus_sample.json
```

### From CloudWatch (AWS)

```bash
# Export CloudWatch metrics
aws cloudwatch get-metric-statistics \
  --metric-name Requests \
  --namespace AWS/ApplicationELB \
  --statistics Sum
```

Then format and evaluate.

### From Datadog/New Relic

Export as JSON metrics and evaluate locally.

## 🚨 Alert Configuration Examples

### Aggressive (Page immediately)
```yaml
alerts:
  - window: "30m"
    threshold: 0.20  # 20%/30min
```

### Balanced (Recommended)
```yaml
alerts:
  - window: "1h"
    threshold: 0.10
  - window: "6h"
    threshold: 0.05
  - window: "24h"
    threshold: 0.02
```

### Relaxed (Post-mortem only)
```yaml
alerts:
  - window: "3d"
    threshold: 0.01
```

## 📚 Common Patterns

### E-Commerce Site

```yaml
target: 99.95            # High availability needed
latency_threshold_ms: 100  # Users expect fast checkouts
```

### Internal Tool

```yaml
target: 95               # More relaxed for internal tools
latency_threshold_ms: 500  # Latency less critical
```

### Real-Time Chat

```yaml
target: 99.9
latency_threshold_ms: 50   # Very strict on latency
```

## ❓ FAQs

**Q: What if my P99 latency is exactly at threshold?**
A: It fails. The threshold is an upper bound (must be <, not ≤).

**Q: How often should I evaluate?**
A: Every 5-10 minutes is typical. Adjust window size in sample data accordingly.

**Q: Can I have multiple SLOs?**
A: Yes! Create separate slo.yaml files per service.

**Q: What does error budget mean?**
A: The percentage of failures you can "afford" while still meeting your SLO. Once consumed, you're failing the SLO.

## 🔗 Next Steps

### Option A: Learn More HTTP Patterns
- [Advanced HTTP SLOs](../../guides/user-guide.md#http-section)
- [Production Deployment](../../guides/production-deployment.md)

### Option B: Try Another Use Case
- [ML Model Drift](5-minute-ml-slo.md)
- [GenAI Monitoring](5-minute-genai-slo.md)

### Option C: Set Up Kubernetes
- [Prometheus Integration](../examples/quickstart/prometheus/README.md)
- [Kubernetes Guide](../../guides/kubernetes-integration.md)

### Option D: Programmatic Usage
- [Python Notebook](../examples/quickstart/notebook.ipynb)
- [Python API](../../reference/api.md#python-client)

## 🔗 Full Resources

- **📖 Complete User Guide:** [User Guide](../../guides/user-guide.md)
- **🚀 Production Setup:** [Production Deployment](../../guides/production-deployment.md)
- **📊 Prometheus:** [Prometheus Integration](../../guides/prometheus-scraping-examples.md)
- **🔌 API Reference:** [Full API](../../reference/api.md)
- **🐛 Troubleshooting:** [Troubleshooting Guide](../../guides/troubleshooting.md)

---

**Questions?** 
- 💬 [Ask on GitHub Discussions](https://github.com/pristley/NeuralBudget/discussions)
- 🐛 [Report Issues](https://github.com/pristley/NeuralBudget/issues)
- 📚 [Browse Examples](../examples/quickstart/http-slo)
