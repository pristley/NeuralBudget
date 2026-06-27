# Generated Prometheus Rules Example

This document shows an example of Prometheus rules generated from `examples/slo_http.yaml`.

## Input SLO Configuration

```yaml
service: "payment-api"
target: 99.95
availability_threshold: 0.9995
latency_threshold_ms: 200

alerts:
  - window: "1h"
    threshold: 0.10
  - window: "6h"
    threshold: 0.05
  - window: "24h"
    threshold: 0.02
  - window: "3d"
    threshold: 0.01
```

## Generated Prometheus Rules YAML

```yaml
# Generated Prometheus recording and alerting rules for: payment-api
# Service: payment-api
# Target Availability: 99.95% (0.9995)
# Latency Threshold: 200ms (P99)
# Error Budget: 0.05%
# Generated with NeuralBudget SLO platform

groups:
  - name: "neuralbudget_payment-api_recording"
    interval: 30s
    rules:
      # Availability SLI: successful requests / total requests
      - record: "neuralbudget:slo:availability"
        expr: |
          100 * sum(rate(http_requests_total{job="payment-api", status=~"2.."}[1m])) /
          sum(rate(http_requests_total{job="payment-api"}[1m]))

      # Latency SLI: P99 latency in milliseconds
      - record: "neuralbudget:slo:latency_p99_ms"
        expr: |
          histogram_quantile(0.99,
            sum(rate(http_request_duration_seconds_bucket{job="payment-api"}[5m])) by (le)
          ) * 1000

      # Error rate: failed requests / total requests
      - record: "neuralbudget:slo:error_rate"
        expr: |
          sum(rate(http_requests_total{job="payment-api", status=~"5.."}[1m])) /
          sum(rate(http_requests_total{job="payment-api"}[1m]))

      # Error budget remaining (in error budget percentage points)
      - record: "neuralbudget:slo:error_budget_remaining"
        expr: |
          0.05 - (100 * (neuralbudget:slo:error_rate / (1 - 0.9995)))

      # Multi-window burn rate indicators
      - record: "neuralbudget:slo:burn_rate_1h"
        expr: |
          sum(rate(http_requests_total{job="payment-api", status=~"5.."}[1h])) / (1 - 0.9995)

      - record: "neuralbudget:slo:burn_rate_6h"
        expr: |
          sum(rate(http_requests_total{job="payment-api", status=~"5.."}[6h])) / (1 - 0.9995)

      - record: "neuralbudget:slo:burn_rate_24h"
        expr: |
          sum(rate(http_requests_total{job="payment-api", status=~"5.."}[24h])) / (1 - 0.9995)

      - record: "neuralbudget:slo:burn_rate_3d"
        expr: |
          sum(rate(http_requests_total{job="payment-api", status=~"5.."}[72h])) / (1 - 0.9995)

  - name: "neuralbudget_payment-api_alerts"
    interval: 1m
    rules:
      - alert: "SloErrorBudgetBurnRate1h"
        expr: |
          neuralbudget:slo:burn_rate_1h > 0.0005
        for: 1m
        labels:
          severity: warning
          slo: neuralbudget
        annotations:
          summary: "SLO error budget burning at {{ $value | humanizePercentage }} rate over 1h window"
          description: "Service {{ $labels.job }} is burning error budget at {{ $value | humanizePercentage }} rate over 1h window. Budget may be exhausted within days."
          runbook: "https://neuralbudget.io/runbooks/burn-rate-1h"

      - alert: "SloErrorBudgetBurnRate6h"
        expr: |
          neuralbudget:slo:burn_rate_6h > 0.00025
        for: 5m
        labels:
          severity: warning
          slo: neuralbudget
        annotations:
          summary: "SLO error budget burning at {{ $value | humanizePercentage }} rate over 6h window"
          description: "Service {{ $labels.job }} is burning error budget at {{ $value | humanizePercentage }} rate over 6h window. Budget may be exhausted within days."
          runbook: "https://neuralbudget.io/runbooks/burn-rate-6h"

      - alert: "SloErrorBudgetBurnRate24h"
        expr: |
          neuralbudget:slo:burn_rate_24h > 0.0001
        for: 15m
        labels:
          severity: warning
          slo: neuralbudget
        annotations:
          summary: "SLO error budget burning at {{ $value | humanizePercentage }} rate over 24h window"
          description: "Service {{ $labels.job }} is burning error budget at {{ $value | humanizePercentage }} rate over 24h window. Budget may be exhausted within days."
          runbook: "https://neuralbudget.io/runbooks/burn-rate-24h"

      - alert: "SloErrorBudgetBurnRate3d"
        expr: |
          neuralbudget:slo:burn_rate_3d > 0.00005
        for: 1h
        labels:
          severity: warning
          slo: neuralbudget
        annotations:
          summary: "SLO error budget burning at {{ $value | humanizePercentage }} rate over 3d window"
          description: "Service {{ $labels.job }} is burning error budget at {{ $value | humanizePercentage }} rate over 3d window. Budget may be exhausted within days."
          runbook: "https://neuralbudget.io/runbooks/burn-rate-3d"

      - alert: "SloLatencyExceeded"
        expr: neuralbudget:slo:latency_p99_ms > 200
        for: 5m
        labels:
          severity: warning
          slo: neuralbudget
        annotations:
          summary: "P99 latency {{ $value | humanize }}ms exceeds SLO target of 200ms"
          description: "Service performance is degraded. P99 latency is above acceptable threshold."

      - alert: "SloErrorBudgetExhausted"
        expr: neuralbudget:slo:error_budget_remaining <= 0
        for: 1m
        labels:
          severity: critical
          slo: neuralbudget
        annotations:
          summary: "SLO error budget has been exhausted"
          description: "Service {{ $labels.job }} has exhausted its monthly error budget. All requests should be treated as critical."
```

## Kubernetes PrometheusRule CRD Example

```yaml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: "payment-api-slo"
  namespace: "monitoring"
  labels:
    app: neuralbudget
    service: payment-api
spec:
  groups:
    - name: "neuralbudget_payment-api_recording"
      interval: 30s
      rules:
        # Availability SLI: successful requests / total requests
        - record: "neuralbudget:slo:availability"
          expr: |
            100 * sum(rate(http_requests_total{job="payment-api", status=~"2.."}[1m])) /
            sum(rate(http_requests_total{job="payment-api"}[1m]))

        # Latency SLI: P99 latency in milliseconds
        - record: "neuralbudget:slo:latency_p99_ms"
          expr: |
            histogram_quantile(0.99,
              sum(rate(http_request_duration_seconds_bucket{job="payment-api"}[5m])) by (le)
            ) * 1000

        # Error rate: failed requests / total requests
        - record: "neuralbudget:slo:error_rate"
          expr: |
            sum(rate(http_requests_total{job="payment-api", status=~"5.."}[1m])) /
            sum(rate(http_requests_total{job="payment-api"}[1m]))

        # Error budget remaining (in error budget percentage points)
        - record: "neuralbudget:slo:error_budget_remaining"
          expr: |
            0.05 - (100 * (neuralbudget:slo:error_rate / (1 - 0.9995)))

        # Multi-window burn rate indicators
        - record: "neuralbudget:slo:burn_rate_1h"
          expr: |
            sum(rate(http_requests_total{job="payment-api", status=~"5.."}[1h])) / (1 - 0.9995)

        - record: "neuralbudget:slo:burn_rate_6h"
          expr: |
            sum(rate(http_requests_total{job="payment-api", status=~"5.."}[6h])) / (1 - 0.9995)

        - record: "neuralbudget:slo:burn_rate_24h"
          expr: |
            sum(rate(http_requests_total{job="payment-api", status=~"5.."}[24h])) / (1 - 0.9995)

        - record: "neuralbudget:slo:burn_rate_3d"
          expr: |
            sum(rate(http_requests_total{job="payment-api", status=~"5.."}[72h])) / (1 - 0.9995)

    - name: "neuralbudget_payment-api_alerts"
      interval: 1m
      rules:
        - alert: "SloErrorBudgetBurnRate1h"
          expr: |
            neuralbudget:slo:burn_rate_1h > 0.0005
          for: 1m
          labels:
            severity: warning
            slo: neuralbudget
            service: payment-api
          annotations:
            summary: "SLO error budget burning at high rate over 1h window"
            description: "Service {{ $labels.job }} has error rate of {{ $value | humanizePercentage }} over 1h window, consuming error budget at {{ $value }}x rate. Budget exhaustion in ~{{ div 1 $value | humanizeDuration }}."
            dashboard: "https://neuralbudget.io/dashboard?service={{ $labels.job }}"

        - alert: "SloErrorBudgetBurnRate6h"
          expr: |
            neuralbudget:slo:burn_rate_6h > 0.00025
          for: 5m
          labels:
            severity: warning
            slo: neuralbudget
            service: payment-api
          annotations:
            summary: "SLO error budget burning at high rate over 6h window"
            description: "Service {{ $labels.job }} has error rate of {{ $value | humanizePercentage }} over 6h window, consuming error budget at {{ $value }}x rate. Budget exhaustion in ~{{ div 1 $value | humanizeDuration }}."
            dashboard: "https://neuralbudget.io/dashboard?service={{ $labels.job }}"

        - alert: "SloErrorBudgetBurnRate24h"
          expr: |
            neuralbudget:slo:burn_rate_24h > 0.0001
          for: 15m
          labels:
            severity: warning
            slo: neuralbudget
            service: payment-api
          annotations:
            summary: "SLO error budget burning at high rate over 24h window"
            description: "Service {{ $labels.job }} has error rate of {{ $value | humanizePercentage }} over 24h window, consuming error budget at {{ $value }}x rate. Budget exhaustion in ~{{ div 1 $value | humanizeDuration }}."
            dashboard: "https://neuralbudget.io/dashboard?service={{ $labels.job }}"

        - alert: "SloErrorBudgetBurnRate3d"
          expr: |
            neuralbudget:slo:burn_rate_3d > 0.00005
          for: 1h
          labels:
            severity: warning
            slo: neuralbudget
            service: payment-api
          annotations:
            summary: "SLO error budget burning at high rate over 3d window"
            description: "Service {{ $labels.job }} has error rate of {{ $value | humanizePercentage }} over 3d window, consuming error budget at {{ $value }}x rate. Budget exhaustion in ~{{ div 1 $value | humanizeDuration }}."
            dashboard: "https://neuralbudget.io/dashboard?service={{ $labels.job }}"

        - alert: "SloLatencyExceeded"
          expr: neuralbudget:slo:latency_p99_ms > 200
          for: 5m
          labels:
            severity: warning
            slo: neuralbudget
          annotations:
            summary: "P99 latency exceeds SLO target of 200ms"
            description: "Service {{ $labels.job }} P99 latency is {{ $value | humanize }}ms (target: 200ms). Performance is degraded."
            dashboard: "https://neuralbudget.io/dashboard?service={{ $labels.job }}"

        - alert: "SloErrorBudgetExhausted"
          expr: neuralbudget:slo:error_budget_remaining <= 0
          for: 1m
          labels:
            severity: critical
            slo: neuralbudget
          annotations:
            summary: "SLO error budget has been exhausted"
            description: "Service {{ $labels.job }} has exhausted its monthly error budget ({{ $value | humanizePercentage }} remaining). All requests are now at risk of violating SLO."
            dashboard: "https://neuralbudget.io/dashboard?service={{ $labels.job }}"
```

## Alert Behavior Visualization

### Burn Rate Timeline

```
Error Budget: 0.05% (≈ 21.6 minutes/month for 99.95% SLO)

Timeline:
Hour 1:   Normal operations
Hour 2:   0.06% error rate → 1h window FIRES (>0.05% = 10x burn)
Hour 8:   Error continues    → 6h window FIRES (>0.025% = 5x burn)
Day 2:    Error rate drops   → Alerts resolve
Day 25:   Low burn rate      → 3d window FIRES (<1% burn over 3d)
Day 30:   Budget exhausted   → CRITICAL alert fires
```

### Multi-Window Coverage

The combination of 4 burn rate windows provides:

| Window | Frequency | Confidence | Action |
|--------|-----------|-----------|--------|
| 1h @ 10% | 5-15 min | High (immediate anomaly) | Page on-call |
| 6h @ 5% | 1-5 hours | High (sustained degradation) | Investigate |
| 24h @ 2% | 4-24 hours | Medium (slow burn) | Track trend |
| 3d @ 1% | 24 hours+ | Low (worst-case scenario) | Post-mortem |

## Using Generated Rules

### Apply to Local Prometheus

```bash
# Generate rules
neuralbudget gen-rules examples/slo_http.yaml > /etc/prometheus/rules/payment-api.yaml

# Reload Prometheus
curl -X POST http://localhost:9090/-/reload
```

### Apply to Kubernetes Prometheus Operator

```bash
# Generate CRD
neuralbudget gen-rules examples/slo_http.yaml --kubernetes --namespace monitoring > payment-api-rules.yaml

# Apply to cluster
kubectl apply -f payment-api-rules.yaml

# Verify
kubectl get PrometheusRule -n monitoring
```

### Verify Rules Load

```bash
# Check PrometheusRule CRD
kubectl describe PrometheusRule payment-api-slo -n monitoring

# Query Prometheus for recording rules
promtool check rules /etc/prometheus/rules/payment-api.yaml
```

## Testing Generated Alerts

### Simulate High Error Rate

```bash
# Using synthetic metrics or by stopping the service:
curl -X POST http://localhost:9999/metrics \
  -d 'http_requests_total{job="payment-api",status="500"} 1000'

# Within 1 minute, check alerts:
curl http://localhost:9093/api/v1/alerts | jq '.data[] | select(.labels.alertname=="SloErrorBudgetBurnRate1h")'
```

### Verify Alert Thresholds

Query in Prometheus:
```promql
# Check current availability
neuralbudget:slo:availability

# Check current burn rate over 1h
neuralbudget:slo:burn_rate_1h

# Check error budget remaining
neuralbudget:slo:error_budget_remaining

# Compare with threshold (0.0005 for 1h window)
neuralbudget:slo:burn_rate_1h > 0.0005
```

## Next Steps

1. Generate rules: `neuralbudget gen-rules your-slo.yaml --kubernetes`
2. Deploy to Kubernetes: `kubectl apply -f rules.yaml`
3. Configure Alertmanager routing for notifications
4. Set up dashboards to visualize SLI metrics
5. Create runbooks for each alert type
