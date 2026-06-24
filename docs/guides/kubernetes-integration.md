# Kubernetes Integration Guide

This guide expands on the baseline manifests in `examples/kubernetes/` and focuses on production deployment patterns.

## Prerequisites

- Kubernetes cluster access (`kubectl` configured)
- Namespace for NeuralBudget workloads
- Container image that exposes `/health/live`, `/health/ready`, and `/metrics`

## 1. Create Namespace

```bash
kubectl create namespace neuralbudget
```

## 2. Deploy Baseline Manifests

```bash
kubectl apply -n neuralbudget -f examples/kubernetes/configmap.yaml
kubectl apply -n neuralbudget -f examples/kubernetes/deployment.yaml
kubectl apply -n neuralbudget -f examples/kubernetes/service.yaml
```

## 3. Verify Pod and Service Health

```bash
kubectl get pods -n neuralbudget -l app=neuralbudget-evaluator
kubectl get svc -n neuralbudget neuralbudget-evaluator
kubectl describe deploy -n neuralbudget neuralbudget-evaluator
```

Expected checks:

- Pods are `Ready`.
- Service has endpoints.
- Readiness and liveness probes are passing.

## 4. Validate Runtime Config

The deployment mounts policy from ConfigMap at `/etc/neuralbudget/slo.yaml` using `NB_CONFIG_PATH`.

Confirm mounted config:

```bash
kubectl exec -n neuralbudget deploy/neuralbudget-evaluator -- cat /etc/neuralbudget/slo.yaml
```

## 5. Safe Policy Updates

Update policy without image rebuild:

1. Update `examples/kubernetes/configmap.yaml`.
2. Apply config.
3. Restart deployment to reload mounted policy.

```bash
kubectl apply -n neuralbudget -f examples/kubernetes/configmap.yaml
kubectl rollout restart -n neuralbudget deploy/neuralbudget-evaluator
kubectl rollout status -n neuralbudget deploy/neuralbudget-evaluator
```

## 6. Progressive Delivery (Canary)

Use a second deployment for canary policy or image validation.

- `neuralbudget-evaluator-stable`: current production
- `neuralbudget-evaluator-canary`: candidate release

Route a small percentage of traffic to canary and compare:

- failure ratio
- global score trends
- evaluation latency

Promote canary after sustained success over multiple evaluation windows.

## 7. Capacity and Autoscaling

Start with explicit requests and limits (already in `deployment.yaml`) and tune from observed usage.

If autoscaling is needed, add an HPA using CPU and/or custom metrics.

Operational guidance:

- Keep at least 2 replicas.
- Avoid CPU starvation for evaluation loops.
- Scale based on request rate and evaluation latency.

## 8. Security Hardening

Recommended hardening for production clusters:

- Run non-root user in container.
- Add `securityContext` with dropped Linux capabilities.
- Restrict egress using NetworkPolicy.
- Store secrets in Kubernetes Secret, not ConfigMap.

## 9. Rollback Procedure

If policy or image rollout degrades SLO behavior:

```bash
kubectl rollout undo -n neuralbudget deploy/neuralbudget-evaluator
kubectl rollout status -n neuralbudget deploy/neuralbudget-evaluator
```

Then inspect:

- deployment events
- pod logs
- Prometheus pass/fail trend for rollback window

## 10. Quick Troubleshooting

### Pods not ready

- Verify `/health/ready` handler and probe timings.
- Increase `initialDelaySeconds` for slow startup.

### Config updates not reflected

- Confirm ConfigMap is updated in namespace.
- Restart deployment after config apply.

### Uneven behavior across replicas

- Confirm all pods run same image digest.
- Confirm all pods mount same policy revision.
