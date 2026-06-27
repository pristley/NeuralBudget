# Applying Generated Prometheus Rules

This guide shows practical examples of how to use `neuralbudget gen-rules` to generate and apply Prometheus rules to different monitoring systems.

## Quick Examples

### Generate and Apply to Local Prometheus

```bash
# Generate YAML rules
neuralbudget gen-rules examples/slo_http.yaml > rules.yaml

# Add to Prometheus configuration
sudo tee -a /etc/prometheus/rules/slo.yaml < rules.yaml

# Reload Prometheus
curl -X POST http://localhost:9090/-/reload

# Verify rules loaded
curl http://localhost:9090/api/v1/rules | jq '.data.groups[] | select(.name | contains("neuralbudget"))'
```

### Generate and Apply to Kubernetes

```bash
# Generate Kubernetes CRD
neuralbudget gen-rules examples/slo_http.yaml --kubernetes --namespace monitoring > slo-rules.yaml

# Apply to cluster
kubectl apply -f slo-rules.yaml

# Verify
kubectl get PrometheusRule -n monitoring
kubectl describe PrometheusRule payment-api-slo -n monitoring

# View alert rules
kubectl get PrometheusRule payment-api-slo -n monitoring -o yaml | grep -A 5 "alert:"
```

### Inline Application (Recommended for CI/CD)

```bash
# Generate and apply in one command
neuralbudget gen-rules examples/slo_http.yaml --kubernetes --namespace monitoring | kubectl apply -f -

# Verify in Prometheus UI
kubectl port-forward -n monitoring svc/prometheus 9090:9090
# Open http://localhost:9090 in browser
# Navigate to: Alerts tab
```

## Real-World Scenarios

### Scenario 1: Multiple Services

```bash
#!/bin/bash
# Apply SLO rules for entire microservices architecture

SERVICES=(
  "payment-api"
  "user-service"
  "recommendation-engine"
  "ml-inference"
)

for service in "${SERVICES[@]}"; do
  echo "Deploying rules for $service..."
  neuralbudget gen-rules "config/$service/slo.yaml" \
    --kubernetes \
    --namespace monitoring | kubectl apply -f -
done

# Verify all rules deployed
kubectl get PrometheusRule -n monitoring | grep slo
```

### Scenario 2: Environment-Specific Rules

```bash
#!/bin/bash
# Different SLO targets for dev, staging, production

ENV=$1  # dev, staging, prod
CONFIG="config/slo-${ENV}.yaml"

neuralbudget gen-rules "$CONFIG" \
  --kubernetes \
  --namespace "monitoring-${ENV}" | kubectl apply -f -

echo "✓ SLO rules deployed to monitoring-${ENV}"
```

### Scenario 3: GitOps with ArgoCD

```yaml
# argocd-app.yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: neuralbudget-rules
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/myorg/slo-configs
    targetRevision: HEAD
    path: prometheus-rules/
    # Pre-generate rules during sync
    plugin:
      name: neuralbudget-generator
      env:
        - name: SLO_CONFIG
          value: "slo.yaml"
  destination:
    server: https://kubernetes.default.svc
    namespace: monitoring
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
```

### Scenario 4: Terraform Integration

```hcl
# terraform/prometheus.tf

resource "null_resource" "generate_slo_rules" {
  triggers = {
    slo_config = file("${path.module}/../config/slo.yaml")
  }

  provisioner "local-exec" {
    command = "neuralbudget gen-rules ${path.module}/../config/slo.yaml --kubernetes > ${path.module}/slo-rules.yaml"
  }
}

resource "kubernetes_manifest" "slo_rules" {
  depends_on = [null_resource.generate_slo_rules]
  
  manifest = yamldecode(file("${path.module}/slo-rules.yaml"))
}
```

### Scenario 5: CI/CD Pipeline Validation

```yaml
# .github/workflows/validate-slo-rules.yaml
name: Validate SLO Rules

on:
  pull_request:
    paths:
      - 'config/slo*.yaml'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install NeuralBudget
        run: cargo build --release --bin neuralbudget
      
      - name: Check SLO Config
        run: ./target/release/neuralbudget check config/slo.yaml --strict
      
      - name: Generate Rules
        run: ./target/release/neuralbudget gen-rules config/slo.yaml > /tmp/rules.yaml
      
      - name: Validate Prometheus YAML
        run: |
          yamllint /tmp/rules.yaml
          promtool check rules /tmp/rules.yaml
      
      - name: Preview Generated Rules
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const rules = fs.readFileSync('/tmp/rules.yaml', 'utf-8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: '```yaml\n' + rules + '\n```'
            });
```

### Scenario 6: Monitoring Dashboard Integration

```bash
#!/bin/bash
# Generate rules and create dashboard links

# Generate rules
neuralbudget gen-rules slo.yaml --kubernetes > rules.yaml

# Extract service name
SERVICE=$(grep "service:" rules.yaml | head -1 | awk '{print $2}')

# Apply rules
kubectl apply -f rules.yaml

# Create dashboard ConfigMap
kubectl create configmap slo-dashboard-${SERVICE} \
  --from-literal=dashboard-url="https://grafana.example.com/d/slo-${SERVICE}" \
  -n monitoring

echo "✓ Rules deployed"
echo "✓ Dashboard: https://grafana.example.com/d/slo-${SERVICE}"
```

## Troubleshooting Applied Rules

### Verify Rules Are Loaded

```bash
# Check in Prometheus
curl http://localhost:9090/api/v1/rules | jq '.data.groups'

# Check in Kubernetes
kubectl get PrometheusRule -A | grep slo
```

### Debug Rule Evaluation

```bash
# Connect to Prometheus
kubectl port-forward -n monitoring svc/prometheus 9090:9090

# Query the recording rules (in Prometheus UI)
neuralbudget:slo:availability
neuralbudget:slo:latency_p99_ms
neuralbudget:slo:burn_rate_1h

# Check alert status
curl http://localhost:9090/api/v1/alerts | jq '.data.alerts[] | select(.labels.alertname | contains("SloErrorBudgetBurnRate"))'
```

### Verify Metrics Are Available

```bash
# Check that required metrics exist
curl http://localhost:9090/api/v1/series \
  -G --data-urlencode 'match[]=http_requests_total{job="payment-api"}'

curl http://localhost:9090/api/v1/series \
  -G --data-urlencode 'match[]=http_request_duration_seconds_bucket{job="payment-api"}'
```

## Integration with Alert Routing

### Alertmanager Configuration

```yaml
# alertmanager.yml
global:
  resolve_timeout: 5m

route:
  receiver: 'default'
  group_by: ['alertname', 'cluster', 'service']
  routes:
    # Fast burn alerts → page immediately
    - match:
        alertname: 'SloErrorBudgetBurnRate1h'
      receiver: 'pagerduty-critical'
      group_wait: 0s
      group_interval: 1m
      repeat_interval: 5m
    
    # Medium burn alerts → notification
    - match:
        alertname: 'SloErrorBudgetBurnRate6h'
      receiver: 'slack-sre'
      group_wait: 5m
      group_interval: 1h
      repeat_interval: 2h
    
    # Slow burn alerts → logged only
    - match:
        alertname: 'SloErrorBudgetBurnRate24h'
      receiver: 'datadog'
      group_wait: 15m
      group_interval: 6h
      repeat_interval: 6h
    
    # Budget exhaustion → immediate page
    - match:
        alertname: 'SloErrorBudgetExhausted'
      receiver: 'pagerduty-critical'
      group_wait: 0s
      repeat_interval: 1h

receivers:
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '${PAGERDUTY_SERVICE_KEY}'
        details:
          service: 'SLO Alert'
          severity: 'critical'

  - name: 'slack-sre'
    slack_configs:
      - api_url: '${SLACK_WEBHOOK_URL}'
        channel: '#slo-alerts'
        title: 'SLO Alert'

  - name: 'datadog'
    webhook_configs:
      - url: 'https://alerts.datadoghq.com/api/v1/validate'
        send_resolved: true
```

## Customization Examples

### Change Burn Rate Windows

Create a custom SLO config:

```yaml
# config/slo-custom-windows.yaml
service: "critical-api"
availability_threshold: 0.999
latency_threshold_ms: 100

alerts:
  - window: "30m"      # Very fast burn
    threshold: 0.50
  - window: "2h"       # Fast burn
    threshold: 0.20
  - window: "24h"      # Slow burn
    threshold: 0.05
  - window: "7d"       # Critical
    threshold: 0.01
```

Then apply:

```bash
neuralbudget gen-rules config/slo-custom-windows.yaml --kubernetes | kubectl apply -f -
```

### Multiple SLOs for One Service

```bash
# Payment processing (strict SLO)
neuralbudget gen-rules config/payment-api/slo-strict.yaml --kubernetes | kubectl apply -f -

# Payment webhooks (lenient SLO)
neuralbudget gen-rules config/payment-api/slo-lenient.yaml --kubernetes | kubectl apply -f -

# Different recording rule namespaces will prevent conflicts
# Rules: neuralbudget:slo:availability (from each config)
```

## Performance Considerations

### Recording Rule Interval

Default: 30 seconds

For high-volume services, adjust:

```yaml
# Increase interval to reduce load
interval: 60s  # Evaluate every 60s instead of 30s
```

### Alert Evaluation Window

Default: 1 minute

For noisy environments:

```bash
# Increase evaluation window to reduce alert chatter
neuralbudget gen-rules slo.yaml --kubernetes  # Uses defaults

# Manually adjust generated YAML:
#   for: 2m  # instead of 1m for 1h window
```

## Next Steps

1. [Prometheus Rule Generation Guide](prometheus-rule-generation.md)
2. [Alertmanager Setup](https://prometheus.io/docs/alerting/latest/overview/)
3. [Grafana SLO Dashboards](../../examples/grafana/README.md)
