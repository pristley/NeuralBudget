# Prometheus Rules - Quick Start (5 Minutes)

Generate and deploy Prometheus alerting rules from SLO definitions in 5 minutes.

## ⚡ What You'll Do

1. Use an existing SLO definition
2. Generate Prometheus rules
3. Deploy to Kubernetes or Prometheus

## 📋 Step-by-Step

### Step 1: Create SLO Definition (or use existing)

```bash
# Using the HTTP SLO from quickstart
cp examples/quickstart/http-slo/slo.yaml ./my-slo.yaml
```

### Step 2: Generate Prometheus Rules

```bash
neuralbudget gen-rules my-slo.yaml > prometheus-rules.yaml
```

### Step 3: View Generated Rules

```bash
cat prometheus-rules.yaml
```

Expected output (sample):
```yaml
groups:
- name: neuralbudget_slo_rules
  interval: 30s
  rules:
  - alert: SLOAvailabilityBurnFast
    expr: |
      (1 - (rate(requests_success_total[1h]) / rate(requests_total[1h]))) > 0.001
    for: 5m
    labels:
      severity: critical
      slo: quickstart-api
    annotations:
      summary: "{{ $labels.service }} availability burning too fast"
      description: "Availability burn rate > 10% at 1h window"

  - alert: SLOLatencyBurnFast
    expr: |
      (histogram_quantile(0.99, rate(request_duration_bucket[1h])) / 0.200) > 1.0
    for: 5m
    labels:
      severity: warning
      slo: quickstart-api
```

### Step 4: Deploy to Kubernetes

```bash
# Create PrometheusRule resource
kubectl apply -f prometheus-rules.yaml -n monitoring
```

### Step 5: Verify Alerts in Prometheus

```bash
# Port-forward to Prometheus
kubectl port-forward -n monitoring svc/prometheus 9090:9090

# Open in browser: http://localhost:9090
# Go to: Alerts tab
# Look for: SLOAvailabilityBurnFast, SLOLatencyBurnFast
```

Expected:
- Green (inactive): SLO is passing ✓
- Red (firing): SLO is failing ✗

## 📊 Generated Rules Explained

### Burn Rate Rules

Each SLO generates multiple burn rate alerts:

| Rule | Window | Threshold | Purpose |
|------|--------|-----------|---------|
| `BurnFast` | 1h | 10% | Rapid degradation (wake-up alert) |
| `BurnMedium` | 6h | 5% | Sustained issues (investigate) |
| `BurnSlow` | 24h | 2% | Slow trend (plan maintenance) |

### Metric Requirements

These Prometheus metrics must exist:

**For HTTP SLOs:**
```
- requests_total
- requests_success_total
- request_duration_bucket
```

**For ML SLOs:**
```
- model_accuracy
- model_precision
- model_recall
- inference_latency_ms
```

**For GenAI SLOs:**
```
- llm_request_ttft_ms
- llm_tokens_generated_total
- llm_request_total
- llm_request_success_total
```

## 🔧 Customization

### Generate Rules with Custom Alert Severity

```bash
neuralbudget gen-rules \
  --fast-burn-severity critical \
  --medium-burn-severity warning \
  --slow-burn-severity info \
  my-slo.yaml > prometheus-rules.yaml
```

### Generate Rules with Custom Notification Channels

Modify alerts in generated YAML:

```yaml
  - alert: SLOAvailabilityBurnFast
    # ... 
    annotations:
      slack_channel: "#platform-alerts"
      pagerduty_key: "{{ secrets.pagerduty_key }}"
```

### Add Custom Labels

```bash
neuralbudget gen-rules \
  --labels team=platform,env=prod \
  my-slo.yaml > prometheus-rules.yaml
```

## 📚 Next Steps

### Option A: Manual Prometheus

1. Copy rules to Prometheus config directory:
   ```bash
   cp prometheus-rules.yaml /etc/prometheus/rules/
   ```

2. Reload Prometheus:
   ```bash
   curl -X POST http://localhost:9090/-/reload
   ```

### Option B: Kubernetes + PrometheusOperator

```bash
# Apply rules as PrometheusRule CRD
kubectl apply -f prometheus-rules.yaml -n monitoring

# Update Prometheus CR to include the rules
kubectl patch prometheus prometheus-operator \
  --type merge \
  -p '{"spec":{"ruleSelector":{"matchLabels":{"prometheus":"slo"}}}}'
```

### Option C: Ansible/Terraform

```bash
# Generate rules and use in IaC
neuralbudget gen-rules my-slo.yaml | \
  terraform apply -var-file=- -f prometheus-stack.tf
```

## 🚨 Viewing Alerts

### In Prometheus UI

1. Navigate to `http://prometheus:9090/alerts`
2. Look for alerts matching your SLO
3. Click alert to see queries

### In AlertManager

1. Configure AlertManager to receive Prometheus alerts
2. Set up notification routes:
   ```yaml
   routes:
   - match:
       severity: critical
     receiver: 'pagerduty'
   - match:
       severity: warning
     receiver: 'slack'
   ```

### In Grafana

1. Create dashboard:
   ```
   Dashboard → New → Import
   Search: "SLO Burn Rate"
   ```

2. Or create custom dashboard with queries:
   ```
   rate(requests_success_total[5m]) / rate(requests_total[5m])
   ```

## 🐛 Troubleshooting

### Rules Not Evaluating

**Problem:** Alerts show "pending" for long time

**Solution:** Check Prometheus scrape interval vs alert evaluation:
```bash
# In prometheus.yml
scrape_interval: 15s
evaluation_interval: 30s
```

### Metrics Not Found

**Problem:** Rules fire with "no data"

**Solution:** Verify metrics are being scraped:
```bash
# In Prometheus query interface
{job="your-service"}
```

### Alert Storms

**Problem:** Too many alerts firing

**Solution:** Adjust thresholds in SLO:
```yaml
alerts:
  - window: "1h"
    threshold: 0.15  # Increase from 0.10
```

## 🔗 Learn More

- [Prometheus Documentation](https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/)
- [Full SLO Guide](../../guides/user-guide.md)
- [Production Deployment](../../guides/production-deployment.md)
- [Kubernetes Integration](../../guides/kubernetes-integration.md)
