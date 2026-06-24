# Production Deployment Guide

This guide describes how to run NeuralBudget-powered evaluation services in production,
with Kubernetes deployment patterns and Prometheus scraping integration.

## Scope and Deployment Model

NeuralBudget is a library, so production deployment typically means shipping an internal
service that embeds NeuralBudget to evaluate SLO snapshots, policy checks, or streaming windows.

A common architecture:

1. App/API receives metric payloads (or reads from a stream).
2. App calls `NeuralBudgetClient` or native bindings.
3. App returns policy verdicts and exposes Prometheus metrics.
4. CI/CD and runtime alerting consume those metrics.

## Container Image Pattern

For Python-backed services, a typical container shape is:

- Python runtime with your app code
- installed `neuralbudget` wheel
- mounted SLO config (`slo.yaml` or `slo.json`)
- `/metrics` endpoint for Prometheus

The Kubernetes examples in [examples/kubernetes](examples/kubernetes) assume this runtime model.

## Kubernetes Integration

Use these baseline manifests:

- [examples/kubernetes/configmap.yaml](examples/kubernetes/configmap.yaml): SLO config injection
- [examples/kubernetes/deployment.yaml](examples/kubernetes/deployment.yaml): app deployment with probes and scrape annotations
- [examples/kubernetes/service.yaml](examples/kubernetes/service.yaml): stable service endpoint

Apply in order:

```bash
kubectl apply -f examples/kubernetes/configmap.yaml
kubectl apply -f examples/kubernetes/deployment.yaml
kubectl apply -f examples/kubernetes/service.yaml
```

### Operational Recommendations

- Set resource requests/limits to avoid noisy-neighbor latency spikes.
- Use at least two replicas for steady availability checks.
- Keep readiness and liveness probes separate (`/health/ready`, `/health/live`).
- Externalize SLO config into ConfigMap/Secret for zero-image policy changes.

## Prometheus Scraping

NeuralBudget itself does not expose metrics directly; your service should export metrics that include:

- evaluation count
- pass/fail counters
- score histograms (`global_slo`, `hybrid_score`, latency buckets)
- evaluation latency

### Option A: Prometheus Operator (recommended)

Use [examples/kubernetes/servicemonitor.yaml](examples/kubernetes/servicemonitor.yaml).

```bash
kubectl apply -f examples/kubernetes/servicemonitor.yaml
```

This integrates with kube-prometheus-stack style installations and targets the `http` service port.

### Option B: Vanilla Prometheus additional scrape config

Use [examples/kubernetes/prometheus-additional-scrape-config.yaml](examples/kubernetes/prometheus-additional-scrape-config.yaml)
and inject it into your Prometheus config or Helm values.

## Suggested Metric Names

For production dashboards and alerts, emit application-level metrics such as:

- `neuralbudget_eval_total{mode,profile}`
- `neuralbudget_eval_pass_total{mode,profile}`
- `neuralbudget_eval_fail_total{mode,profile}`
- `neuralbudget_eval_duration_seconds_bucket`
- `neuralbudget_global_slo`

If evaluating composite DAGs, include service-level labels where cardinality is controlled.

## Alerting Examples

Use simple policy alerts first:

- High failure ratio in the last 5 minutes
- Global SLO below threshold for 3 consecutive windows
- Evaluation pipeline latency saturation

Example PromQL signal:

```promql
sum(rate(neuralbudget_eval_fail_total[5m]))
/
sum(rate(neuralbudget_eval_total[5m])) > 0.05
```

## CI/CD Production Gates

For releases, combine runtime observability with build-time gates:

1. CI: test and lint pass
2. CD: packaged artifacts and published wheels
3. pre-prod: run canary workload and SLO checks
4. prod promotion: require pass ratio and score thresholds

You can reuse the script pattern in [docs/guides/user-guide.md](docs/guides/user-guide.md)
for fail-fast SLO gating.

## Security and Reliability Notes

- Avoid embedding credentials in ConfigMaps.
- Use Secrets for tokens, API keys, and private endpoints.
- Pin image digests for deterministic rollouts.
- Keep scrape intervals aligned with evaluation cadence.
- Cap label cardinality in metrics to avoid TSDB bloat.

## Troubleshooting

### No metrics visible in Prometheus

- Verify service selector labels match deployment labels.
- Confirm `/metrics` responds on the expected container port.
- Check ServiceMonitor namespace selector and labels.

### Frequent probe restarts

- Ensure probes reflect real app startup time.
- Increase `initialDelaySeconds` for warm-up heavy models.

### Inconsistent SLO policy behavior across pods

- Confirm all pods mount the same config revision.
- Roll deployment after ConfigMap policy updates.
