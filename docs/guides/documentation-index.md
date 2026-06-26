# Documentation Index

This index groups documentation by audience and workflow.

## Start Here

- [README.md](../../README.md): Project overview, installation, and high-level examples.
- [docs/guides/getting-started.md](getting-started.md): Fastest path to first successful evaluation.
- [docs/guides/user-guide.md](user-guide.md): End-to-end guide for modes, config, CI/CD, and troubleshooting.

## Read By Goal

- I need a first working example: [docs/guides/getting-started.md](getting-started.md)
- I need interface and mode selection help: [docs/guides/user-guide.md](user-guide.md)
- I need production rollout guidance: [docs/guides/production-deployment.md](production-deployment.md)
- I need Kubernetes runbooks: [docs/guides/kubernetes-integration.md](kubernetes-integration.md)
- I need Prometheus scrape and alert patterns: [docs/guides/prometheus-scraping-examples.md](prometheus-scraping-examples.md)
- I need high-frequency metric collection: [../../PHASE3_GETTING_STARTED.md](../../PHASE3_GETTING_STARTED.md)
- I need adaptive windowing details: [../../ADAPTIVE_WINDOWING_DESIGN.md](../../ADAPTIVE_WINDOWING_DESIGN.md)
- I need streaming & performance implementation details: [../../PHASE3_STREAMING_IMPLEMENTATION.md](../../PHASE3_STREAMING_IMPLEMENTATION.md)
- I need ready-made Grafana dashboards: [examples/grafana/README.md](../../examples/grafana/README.md)
- I need the complete Python API reference: [docs/reference/api.md](../reference/api.md)
- I need convenience-layer API details: [docs/reference/convenience-layer.md](../reference/convenience-layer.md)
- I need webhook/incident alerting setup: [examples/python/webhook_alerting.py](../../examples/python/webhook_alerting.py)
- I need composite DAG schema and scoring semantics: [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)
- I need release changes: [CHANGELOG.md](../../CHANGELOG.md)

## Feature Plans

- [docs/plans/mlops-model-drift-serving-plan.md](../plans/mlops-model-drift-serving-plan.md): Detailed implementation plan for MlSlo hybrid scoring.

## Phase 3: Streaming & Performance (Live)

- **Overview**: [../../PHASE3_GETTING_STARTED.md](../../PHASE3_GETTING_STARTED.md) — Streaming aggregators and parallel evaluation walkthrough
- **Streaming Implementation**: [../../PHASE3_STREAMING_IMPLEMENTATION.md](../../PHASE3_STREAMING_IMPLEMENTATION.md) — Architecture and design decisions
- **Adaptive Windowing**: [../../ADAPTIVE_WINDOWING_DESIGN.md](../../ADAPTIVE_WINDOWING_DESIGN.md) — Memory-bounded high-frequency ingestion
- **Deployment Guide**: [../../DEPLOYMENT_GUIDE.md](../../DEPLOYMENT_GUIDE.md) — Production rollout patterns for Phase 3 features

## API References

- [docs/reference/api.md](../reference/api.md): Complete reference for NeuralBudget Python API, including native extension classes, NeuralBudgetClient, convenience functions, alert dispatching, data models, type hints, and examples.
- [docs/reference/streaming-aggregator.md](../reference/streaming-aggregator.md): StreamingAggregator API, usage patterns, performance characteristics, and adaptive windowing behavior.
- [docs/reference/convenience-layer.md](../reference/convenience-layer.md): Detailed reference for the Python convenience layer, typed dataclass returns, and profile presets.
- [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md): Reference for Composite SLO DAG schemas, dependency propagation, global score semantics, and errors.

## Testing and Quality Gates

- CI workflow: [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
- CD workflow: [.github/workflows/cd.yml](../../.github/workflows/cd.yml)
- Release workflow: [.github/workflows/release.yml](../../.github/workflows/release.yml)
- Coverage command: `cargo llvm-cov --workspace --all-features --lib --tests --summary-only`
- Property tests: [src/tests.rs](../../src/tests.rs) (`proptest` suites)
- Python convenience tests: [tests/python_convenience_tests.py](../../tests/python_convenience_tests.py)
- Python client facade tests: [tests/python_client_tests.py](../../tests/python_client_tests.py)

## Deployment Examples

- [examples/kubernetes/configmap.yaml](../../examples/kubernetes/configmap.yaml)
- [examples/kubernetes/deployment.yaml](../../examples/kubernetes/deployment.yaml)
- [examples/kubernetes/service.yaml](../../examples/kubernetes/service.yaml)
- [examples/kubernetes/servicemonitor.yaml](../../examples/kubernetes/servicemonitor.yaml)
- [examples/kubernetes/prometheus-additional-scrape-config.yaml](../../examples/kubernetes/prometheus-additional-scrape-config.yaml)
- [examples/grafana/README.md](../../examples/grafana/README.md)
- [examples/grafana/dashboards/http-slo-dashboard.json](../../examples/grafana/dashboards/http-slo-dashboard.json)
- [examples/grafana/dashboards/stateful-slo-dashboard.json](../../examples/grafana/dashboards/stateful-slo-dashboard.json)
- [examples/grafana/dashboards/ml-slo-dashboard.json](../../examples/grafana/dashboards/ml-slo-dashboard.json)
- [examples/grafana/dashboards/genai-slo-dashboard.json](../../examples/grafana/dashboards/genai-slo-dashboard.json)
- [examples/grafana/dashboards/composite-slo-dashboard.json](../../examples/grafana/dashboards/composite-slo-dashboard.json)
- [examples/python/webhook_alerting.py](../../examples/python/webhook_alerting.py)
- [examples/python/webhook_alerting_config.json](../../examples/python/webhook_alerting_config.json)

## Suggested Reading Order

1. [README.md](../../README.md)
2. [docs/guides/getting-started.md](getting-started.md)
3. [docs/guides/user-guide.md](user-guide.md)
4. [docs/guides/production-deployment.md](production-deployment.md)
5. [docs/guides/kubernetes-integration.md](kubernetes-integration.md)
6. [docs/guides/prometheus-scraping-examples.md](prometheus-scraping-examples.md)
7. [docs/reference/api.md](../reference/api.md)
8. [docs/reference/convenience-layer.md](../reference/convenience-layer.md)
9. [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)
10. [CHANGELOG.md](../../CHANGELOG.md)
11. [docs/plans/mlops-model-drift-serving-plan.md](../plans/mlops-model-drift-serving-plan.md)
