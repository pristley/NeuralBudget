# Prometheus Scraping Examples

This guide provides practical scraping, relabeling, recording-rule, and alert examples for NeuralBudget-powered services.

## Scrape Target Expectations

Your service should expose metrics on:

- path: `/metrics`
- protocol: HTTP
- service port name: `http`

The repository includes:

- `examples/kubernetes/servicemonitor.yaml`
- `examples/kubernetes/prometheus-additional-scrape-config.yaml`

## Option A: Prometheus Operator (ServiceMonitor)

Apply ServiceMonitor:

```bash
kubectl apply -f examples/kubernetes/servicemonitor.yaml
```

Verify target discovery in Prometheus UI under `Status -> Targets`.

Recommended tweaks for high-scale clusters:

- Set `namespaceSelector` to specific namespaces instead of `any: true`.
- Use tighter label selectors to avoid accidental scrape fan-out.
- Keep scrape interval aligned with evaluation cadence.

## Option B: Vanilla Prometheus Additional Scrape Config

Use the sample in `examples/kubernetes/prometheus-additional-scrape-config.yaml` and merge it into your Prometheus config.

Example with stricter relabel filters:

```yaml
- job_name: neuralbudget-evaluator
  metrics_path: /metrics
  kubernetes_sd_configs:
    - role: endpoints
  relabel_configs:
    - source_labels: [__meta_kubernetes_namespace]
      action: keep
      regex: neuralbudget
    - source_labels: [__meta_kubernetes_service_label_app]
      action: keep
      regex: neuralbudget-evaluator
    - source_labels: [__meta_kubernetes_endpoint_port_name]
      action: keep
      regex: http
```

## Recommended Metric Naming

Use low-cardinality labels and predictable names:

- `neuralbudget_eval_total{mode,profile}`
- `neuralbudget_eval_pass_total{mode,profile}`
- `neuralbudget_eval_fail_total{mode,profile}`
- `neuralbudget_eval_duration_seconds_bucket{mode,profile}`
- `neuralbudget_global_slo{mode,profile}`

Avoid per-request labels (request ID, user ID, payload hash).

## Recording Rules

Precompute high-value indicators to simplify dashboards and alerts:

```yaml
groups:
  - name: neuralbudget-recording
    interval: 30s
    rules:
      - record: neuralbudget:eval_fail_ratio_5m
        expr: |
          sum(rate(neuralbudget_eval_fail_total[5m]))
          /
          clamp_min(sum(rate(neuralbudget_eval_total[5m])), 1)
      - record: neuralbudget:global_slo_avg_5m
        expr: avg_over_time(neuralbudget_global_slo[5m])
      - record: neuralbudget:eval_p95_duration_5m
        expr: |
          histogram_quantile(
            0.95,
            sum(rate(neuralbudget_eval_duration_seconds_bucket[5m])) by (le)
          )
```

## Alert Examples

```yaml
groups:
  - name: neuralbudget-alerts
    rules:
      - alert: NeuralBudgetHighFailureRatio
        expr: neuralbudget:eval_fail_ratio_5m > 0.05
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: NeuralBudget failure ratio is above 5%
      - alert: NeuralBudgetGlobalSloLow
        expr: neuralbudget:global_slo_avg_5m < 0.90
        for: 15m
        labels:
          severity: critical
        annotations:
          summary: NeuralBudget global SLO average is below target
      - alert: NeuralBudgetEvaluatorSlow
        expr: neuralbudget:eval_p95_duration_5m > 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: NeuralBudget evaluator p95 duration is high
```

## Dashboard Panels to Start With

- Evaluation pass/fail ratio (5m and 1h)
- Global SLO rolling average
- Evaluation latency p50/p95
- Per-profile evaluation throughput

## Validation Checklist

1. Target appears as `UP` in Prometheus.
2. `neuralbudget_eval_total` increases over time.
3. Recording rules evaluate without errors.
4. Synthetic failure triggers expected alerts.
5. Alert routing reaches destination (PagerDuty/Slack/etc.).

## Troubleshooting

### Target missing

- Service label selector does not match pods.
- Wrong endpoint port name.
- Namespace selector mismatch.

### Metrics empty or sparse

- Application not exposing `/metrics`.
- Scrape interval too long for evaluation frequency.
- Relabel rules over-filtering endpoints.

### Alert noise

- Use `for` windows to reduce flapping.
- Add recording rules and threshold smoothing.
- Split warning vs critical thresholds.
