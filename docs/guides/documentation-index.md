# Documentation Index

This index groups documentation by audience and workflow.

## Start Here

- README: Project overview, installation, examples, and high-level API surface.
- docs/guides/user-guide.md: End-to-end installation, usage, examples, applications, and troubleshooting.
- docs/guides/user-guide.md (Native Prometheus Exporter section): How to expose SLO evaluation results as Prometheus metrics.
- docs/guides/user-guide.md (OpenTelemetry ingestion section): How to ingest OTLP JSON metrics directly and evaluate SLOs.
- docs/guides/production-deployment.md: Production rollout patterns, Kubernetes manifests, and Prometheus scraping setup.
- docs/guides/kubernetes-integration.md: Kubernetes rollout runbook for config updates, canary strategy, and rollback.
- docs/guides/prometheus-scraping-examples.md: Prometheus Operator and vanilla scrape examples with recording rules and alerts.
- CHANGELOG.md: Versioned release history and categorized change entries.

## Feature Plans

- docs/plans/mlops-model-drift-serving-plan.md: Detailed implementation plan for MlSlo hybrid scoring.

## API References

- docs/reference/convenience-layer.md: Detailed reference for the Python convenience layer, typed dataclass returns, and profile presets.
- docs/reference/composite-slo-dag.md: Reference for Composite SLO DAG schemas, dependency propagation, global score semantics, and errors.

## Testing and Quality Gates

- CI workflow: .github/workflows/ci.yml
- CD workflow: .github/workflows/release.yml (validation + release + distribution)
- PyPI release process: .github/workflows/release.yml (trusted publishing)
- Coverage command: cargo llvm-cov --workspace --all-features --lib --tests --summary-only
- Property tests: src/tests.rs (`proptest` suites)
- Python convenience tests: tests/python_convenience_tests.py
- Python client facade tests: tests/python_client_tests.py

## Deployment Examples

- examples/kubernetes/configmap.yaml
- examples/kubernetes/deployment.yaml
- examples/kubernetes/service.yaml
- examples/kubernetes/servicemonitor.yaml
- examples/kubernetes/prometheus-additional-scrape-config.yaml

## Suggested Reading Order

1. README
2. docs/guides/user-guide.md
3. docs/guides/production-deployment.md
4. docs/guides/kubernetes-integration.md
5. docs/guides/prometheus-scraping-examples.md
6. CHANGELOG.md
7. docs/reference/convenience-layer.md
8. docs/reference/composite-slo-dag.md
9. docs/plans/mlops-model-drift-serving-plan.md
