# Prometheus Rule Generation for SLOs

This guide explains how NeuralBudget automatically generates Prometheus recording and alerting rules from your SLO configuration, implementing the industry-standard multi-burn-rate alerting strategy.

## Overview

The `neuralbudget gen-rules` command transforms your SLO configuration into production-ready Prometheus rules that:

- **Record SLI metrics** (Availability, Latency, Error Rate)
- **Track error budget** in real-time
- **Generate multi-burn-rate alerts** to detect failures early
- **Support multiple output formats** (YAML, Kubernetes PrometheusRule CRD)

## Quick Start

```bash
# Generate plain Prometheus YAML rules
neuralbudget gen-rules examples/slo_http.yaml > rules.yaml

# Generate Kubernetes PrometheusRule CRD
neuralbudget gen-rules examples/slo_http.yaml --kubernetes --namespace monitoring > rules-crd.yaml

# Apply to Prometheus
kubectl apply -f rules-crd.yaml
```

## Configuration

Your SLO config file must include:

```yaml
service: "payment-api"
availability_threshold: 0.9995    # 99.95% availability target
latency_threshold_ms: 200         # P99 latency < 200ms
job_label: "payment-api"          # Prometheus job label (optional)

# Multi-window burn rate thresholds
alerts:
  - window: "1h"
    threshold: 0.10              # Alert if burning >10% of monthly budget/hour
  - window: "6h"
    threshold: 0.05              # Alert if burning >5% of monthly budget/hour
  - window: "24h"
    threshold: 0.02              # Alert if burning >2% of monthly budget/day
  - window: "3d"
    threshold: 0.01              # Alert if burning >1% of monthly budget/day
```

## Generated Recording Rules

The tool generates 4 recording rules for your SLO:

### 1. Availability SLI
```promql
neuralbudget:slo:availability
```
Calculates percentage of successful requests (HTTP 2xx status codes).

**Formula:**
```
100 * sum(rate(http_requests_total{job="payment-api", status=~"2.."}[1m]))
    / sum(rate(http_requests_total{job="payment-api"}[1m]))
```

### 2. Latency SLI (P99)
```promql
neuralbudget:slo:latency_p99_ms
```
Measures the 99th percentile request latency in milliseconds.

**Formula:**
```
histogram_quantile(0.99,
  sum(rate(http_request_duration_seconds_bucket{job="payment-api"}[5m])) by (le)
) * 1000
```

### 3. Error Rate
```promql
neuralbudget:slo:error_rate
```
Fraction of failed requests (HTTP 5xx status codes).

**Formula:**
```
sum(rate(http_requests_total{job="payment-api", status=~"5.."}[1m]))
  / sum(rate(http_requests_total{job="payment-api"}[1m]))
```

### 4. Error Budget Remaining
```promql
neuralbudget:slo:error_budget_remaining
```
Percentage of error budget still available (0-100%).

**Formula:**
```
error_budget_percent - (100 * (error_rate / allowed_error_rate))
```

For 99.95% SLO: `0.05% - (100 * (error_rate / 0.0005))`

### 5. Multi-Window Burn Rates
```promql
neuralbudget:slo:burn_rate_1h   # 1-hour window
neuralbudget:slo:burn_rate_6h   # 6-hour window
neuralbudget:slo:burn_rate_24h  # 24-hour window
neuralbudget:slo:burn_rate_3d   # 3-day window
```

These measure the rate at which error budget is consumed over different time horizons.

## Generated Alerting Rules

### Multi-Burn-Rate Alerts

The tool generates one alert per configured burn rate window. For 99.95% SLO:

#### 1h Window (Fast Burn)
```yaml
- alert: SloErrorBudgetBurnRate1h
  expr: neuralbudget:slo:burn_rate_1h > 0.0005
  for: 1m
  labels:
    severity: warning
  annotations:
    summary: "SLO error budget burning at 10x rate over 1h window"
```

**Triggers when:** Error rate exceeds 0.05% for 1 minute (0.0005 = 10% of 0.005% monthly budget)

**Action:** Investigate immediately; page on-call if needed

#### 6h Window (Medium Burn)
```yaml
- alert: SloErrorBudgetBurnRate6h
  expr: neuralbudget:slo:burn_rate_6h > 0.00025
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "SLO error budget burning at 2x rate over 6h window"
```

**Triggers when:** Error rate exceeds 0.025% for 5 minutes

**Action:** Investigate; non-urgent follow-up

#### 24h Window (Slow Burn)
```yaml
- alert: SloErrorBudgetBurnRate24h
  expr: neuralbudget:slo:burn_rate_24h > 0.0001
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "SLO error budget running out over 24h window"
```

**Triggers when:** Error rate exceeds 0.01% for 15 minutes

**Action:** Track; schedule investigation

#### 3d Window (Critical Burn)
```yaml
- alert: SloErrorBudgetBurnRate3d
  expr: neuralbudget:slo:burn_rate_3d > 0.00005
  for: 1h
  labels:
    severity: warning
  annotations:
    summary: "SLO error budget will be exhausted in 3 days"
```

**Triggers when:** Error rate exceeds 0.005% for 1 hour

**Action:** Post-mortem scheduled; process review

### Latency Alert
```yaml
- alert: SloLatencyExceeded
  expr: neuralbudget:slo:latency_p99_ms > 200
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "P99 latency exceeds SLO target of 200ms"
```

### Error Budget Exhaustion Alert
```yaml
- alert: SloErrorBudgetExhausted
  expr: neuralbudget:slo:error_budget_remaining <= 0
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "SLO error budget has been exhausted"
```

## Burn Rate Math

The multi-burn-rate approach is based on Google's SRE practices. Here's how the thresholds work:

### Calculate Monthly Error Budget
```
Monthly error budget % = 100 × (1 - target)
For 99.95% SLO:  0.05% (= 21.6 minutes/month)
```

### Calculate Hourly Budget (4 weeks)
```
Hourly budget = Monthly budget / (30 * 24)
For 99.95% SLO: 0.05% / 720 = 0.000069% per hour
```

### Burn Rate Thresholds
```
1h window @ 10% burn = 0.0005        (0.000069 × 10 × 0.7)
6h window @ 5% burn  = 0.00025       (0.000069 × 5 × 0.7)
24h window @ 2% burn = 0.0001        (0.000069 × 2 × 0.7)
3d window @ 1% burn  = 0.00005       (0.000069 × 1 × 0.7)

Note: 0.7 factor accounts for 30-day vs actual month length
```

## Usage Examples

### Example 1: Payment API (99.95% SLO)

**Config:** `examples/slo_http.yaml`

**Generated Recording Rules:**
- `neuralbudget:slo:availability` → tracks % of successful payments
- `neuralbudget:slo:latency_p99_ms` → tracks payment processing latency
- `neuralbudget:slo:error_budget_remaining` → shows budget available (max 21.6 min/month)

**Generated Alerts:**
- **1h**: Fires if payment failure rate > 0.05% for 1 minute
- **6h**: Fires if payment failure rate > 0.025% for 5 minutes
- **24h**: Fires if payment failure rate > 0.01% for 15 minutes
- **3d**: Fires if payment failure rate > 0.005% for 1 hour

### Example 2: Custom SLO with Different Windows

```yaml
service: "ml-inference"
availability_threshold: 0.99         # 99%
latency_threshold_ms: 5000          # 5s for ML models

alerts:
  - window: "1h"
    threshold: 0.20                 # Aggressive burn alert
  - window: "24h"
    threshold: 0.05
```

## Output Formats

### Plain Prometheus YAML
```bash
neuralbudget gen-rules config.yaml > rules.yaml
```

Output includes:
```yaml
groups:
  - name: "neuralbudget_service_recording"
    interval: 30s
    rules:
      - record: "neuralbudget:slo:availability"
        expr: "..."
      # ... more recording rules ...

  - name: "neuralbudget_service_alerts"
    interval: 1m
    rules:
      - alert: "SloErrorBudgetBurnRate1h"
        expr: "..."
      # ... more alerting rules ...
```

### Kubernetes PrometheusRule CRD
```bash
neuralbudget gen-rules config.yaml --kubernetes --namespace monitoring > rules-crd.yaml
```

Output includes:
```yaml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: "service-slo"
  namespace: "monitoring"
  labels:
    app: neuralbudget
spec:
  groups:
    - name: "neuralbudget_service_recording"
      # ... rules ...
```

Apply with:
```bash
kubectl apply -f rules-crd.yaml
```

## Prometheus Metrics Requirements

Your application must export metrics in Prometheus format:

```
# Required metrics:
http_requests_total{job="payment-api", status="200"} counter
http_requests_total{job="payment-api", status="500"} counter
http_request_duration_seconds_bucket{job="payment-api", le="0.1"} histogram
http_request_duration_seconds_bucket{job="payment-api", le="0.2"} histogram
# ... more buckets to cover latency threshold ...
```

### Instrumentation Examples

**Go (with Prometheus client):**
```go
import "github.com/prometheus/client_golang/prometheus"

requestDuration := prometheus.NewHistogramVec(
  prometheus.HistogramOpts{
    Name: "http_request_duration_seconds",
    Buckets: []float64{.001, .01, .05, .1, .2, .5, 1},
  },
  []string{"method", "status"},
)
```

**Python (with prometheus_client):**
```python
from prometheus_client import Histogram, Counter

request_duration = Histogram(
  'http_request_duration_seconds',
  'HTTP request latency',
  ['method', 'status'],
  buckets=(.001, .01, .05, .1, .2, .5, 1),
)

requests_total = Counter(
  'http_requests_total',
  'HTTP requests total',
  ['status'],
)
```

## Customization

### Change Default Windows

Edit the SLO config:
```yaml
alerts:
  - window: "30m"
    threshold: 0.50       # Custom 30m window
  - window: "2h"
    threshold: 0.10
```

### Filter by Service

Use the `job_label` field:
```yaml
job_label: "api-gateway"  # Must match Prometheus job label
```

### Multiple SLOs per Service

Create separate config files:
```
slo_http.yaml        → gen-rules → http-rules.yaml
slo_grpc.yaml        → gen-rules → grpc-rules.yaml
slo_databases.yaml   → gen-rules → db-rules.yaml
```

Apply all:
```bash
kubectl apply -f *-rules.yaml
```

## Troubleshooting

### Alert Not Firing

1. Check metrics are present in Prometheus:
   ```
   http_requests_total{job="payment-api"}
   http_request_duration_seconds_bucket{job="payment-api"}
   ```

2. Verify job label matches config:
   ```promql
   http_requests_total{job="payment-api"}
   ```

3. Check recording rules evaluate:
   ```promql
   neuralbudget:slo:availability
   ```

4. Verify alert threshold calculation:
   ```promql
   # For 99.95% SLO, 1h window with 10% burn:
   (1 - 0.9995) * 0.1 = 0.0005
   ```

### Invalid PromQL

If the generated rules have syntax errors:

1. Validate with Prometheus query editor
2. Check metric names match your instrumentation
3. Verify histogram buckets cover the latency threshold

### Too Many Alerts

Adjust thresholds in config:
```yaml
alerts:
  - window: "1h"
    threshold: 0.30       # Increased from 0.10
```

## Integration with Alertmanager

Forward alerts to notification channels:

```yaml
# alertmanager.yml
route:
  receiver: default
  routes:
    - match:
        alertname: SloErrorBudgetBurnRate1h
      receiver: pagerduty
      group_wait: 0s
      repeat_interval: 5m
    
    - match:
        alertname: SloErrorBudgetBurnRate6h
      receiver: slack
      group_wait: 5m
      repeat_interval: 1h

receivers:
  - name: pagerduty
    pagerduty_configs:
      - service_key: "${PAGERDUTY_KEY}"
  
  - name: slack
    slack_configs:
      - webhook_url: "${SLACK_WEBHOOK_URL}"
```

## Next Steps

1. **Generate Rules:** `neuralbudget gen-rules your-slo.yaml`
2. **Deploy Rules:** `kubectl apply -f rules.yaml`
3. **Configure Alerting:** Update Alertmanager routing
4. **Test Alerts:** Simulate failures to verify alerts fire
5. **Monitor Budget:** Check dashboard for error budget trends

## References

- [Google SRE Book - SLOs](https://sre.google/books/)
- [Google SRE Workbook - Error Budget](https://sre.google/workbook/)
- [Prometheus Recording Rules](https://prometheus.io/docs/prometheus/latest/configuration/recording_rules/)
- [Prometheus Alerting Rules](https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/)
- [Sloth - SLO Generator](https://sloth.dev/)
