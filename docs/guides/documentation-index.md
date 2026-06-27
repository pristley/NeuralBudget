# Documentation Index

This index groups documentation by audience and workflow.

## Start Here

- [README.md](../../README.md): Project overview, installation, and high-level examples.
- [docs/guides/getting-started.md](getting-started.md): Fastest path to first successful evaluation.
- [docs/guides/user-guide.md](user-guide.md): End-to-end guide for modes, config, CI/CD, and troubleshooting.

## Read By Goal

- I don't know where to start: [docs/guides/getting-started.md](getting-started.md)
- I need to understand key terms: [docs/reference/glossary.md](../reference/glossary.md)
- I'm getting an error: [docs/reference/errors.md](../reference/errors.md)
- I need troubleshooting help: [docs/guides/troubleshooting.md](troubleshooting.md)
- I need to understand the architecture: [docs/reference/architecture.md](../reference/architecture.md)
- I need interface and mode selection help: [docs/guides/user-guide.md](user-guide.md)
- I need production rollout guidance: [docs/guides/production-deployment.md](production-deployment.md)
- I need Kubernetes runbooks: [docs/guides/kubernetes-integration.md](kubernetes-integration.md)
- I need Prometheus scrape and alert patterns: [docs/guides/prometheus-scraping-examples.md](prometheus-scraping-examples.md)
- I need SRE workbook burn-rate alerting: [docs/guides/burn-rate-forecasting.md](burn-rate-forecasting.md)
- I need high-frequency metric collection: [../../PHASE3_GETTING_STARTED.md](../../PHASE3_GETTING_STARTED.md)
- I need adaptive windowing details: [../../ADAPTIVE_WINDOWING_DESIGN.md](../../ADAPTIVE_WINDOWING_DESIGN.md)
- I need streaming & performance implementation details: [../../PHASE3_STREAMING_IMPLEMENTATION.md](../../PHASE3_STREAMING_IMPLEMENTATION.md)
- I need ready-made Grafana dashboards: [examples/grafana/README.md](../../examples/grafana/README.md)
- I need the complete Python API reference: [docs/reference/api.md](../reference/api.md)
- I need convenience-layer API details: [docs/reference/convenience-layer.md](../reference/convenience-layer.md)
- I need webhook/incident alerting setup: [examples/python/webhook_alerting.py](../../examples/python/webhook_alerting.py)
- I need advanced alert dispatch with retry/dedup/escalation: [docs/guides/advanced_alert_dispatch.md](advanced_alert_dispatch.md)
- I need a lightweight dashboard without Grafana: [docs/guides/dashboard_cli.md](dashboard_cli.md)
- I need terminal-based SLO monitoring: [docs/guides/dashboard_cli.md](dashboard_cli.md#cli-tui)
- I need GenAI telemetry integration (OpenAI, Anthropic, vLLM, Triton): [docs/guides/genai_connectors.md](genai_connectors.md)
- I need GenAI connector API reference: [docs/reference/genai_connectors.md](../reference/genai_connectors.md)
- I need composite DAG schema and scoring semantics: [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)
- I need burn-rate API reference: [docs/reference/burn-rate-forecasting.md](../reference/burn-rate-forecasting.md)
- I need dashboard and CLI TUI API details: [docs/reference/dashboard_cli.md](../reference/dashboard_cli.md)
- I need release changes: [CHANGELOG.md](../../CHANGELOG.md)

## Feature Plans

- [docs/plans/mlops-model-drift-serving-plan.md](../plans/mlops-model-drift-serving-plan.md): Detailed implementation plan for MlSlo hybrid scoring.

## Phase 3: Streaming & Performance (Live)

- **Overview**: [../../PHASE3_GETTING_STARTED.md](../../PHASE3_GETTING_STARTED.md) — Streaming aggregators and parallel evaluation walkthrough
- **Streaming Implementation**: [../../PHASE3_STREAMING_IMPLEMENTATION.md](../../PHASE3_STREAMING_IMPLEMENTATION.md) — Architecture and design decisions
- **Adaptive Windowing**: [../../ADAPTIVE_WINDOWING_DESIGN.md](../../ADAPTIVE_WINDOWING_DESIGN.md) — Memory-bounded high-frequency ingestion
- **Deployment Guide**: [../../DEPLOYMENT_GUIDE.md](../../DEPLOYMENT_GUIDE.md) — Production rollout patterns for Phase 3 features

## API References & Reference Materials

**Getting Help:**
- [docs/reference/glossary.md](../reference/glossary.md): Glossary of key terms, acronyms, and concepts used throughout NeuralBudget.
- [docs/reference/errors.md](../reference/errors.md): Error reference guide with root causes, solutions, and debugging techniques.
- [docs/guides/troubleshooting.md](troubleshooting.md): Consolidated troubleshooting guide with decision trees and solutions.
- [docs/reference/architecture.md](../reference/architecture.md): System architecture, design decisions, and module interactions with Mermaid diagrams.

**API Documentation:**
- [docs/reference/api.md](../reference/api.md): Complete reference for NeuralBudget Python API, including native extension classes, NeuralBudgetClient, convenience functions, alert dispatching, data models, type hints, and examples.
- [docs/reference/streaming-aggregator.md](../reference/streaming-aggregator.md): StreamingAggregator API, usage patterns, performance characteristics, and adaptive windowing behavior.
- [docs/reference/convenience-layer.md](../reference/convenience-layer.md): Detailed reference for the Python convenience layer, typed dataclass returns, and profile presets.
- [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md): Reference for Composite SLO DAG schemas, dependency propagation, global score semantics, and errors.
- [docs/reference/burn-rate-forecasting.md](../reference/burn-rate-forecasting.md): Burn-rate forecasting API, multi-window alerts, SRE workbook patterns, and TTEE calculations.
- [docs/reference/advanced_alert_dispatch.md](../reference/advanced_alert_dispatch.md): Advanced alert dispatch API reference, retry policies, deduplication, circuit breaker, and escalation.
- [docs/reference/dashboard_cli.md](../reference/dashboard_cli.md): Dashboard and CLI TUI API reference, endpoints, data models, and integration patterns.
- [docs/reference/genai_connectors.md](../reference/genai_connectors.md): GenAI telemetry connectors API reference, data models, connector implementations, and integration examples.
- [docs/reference/anomaly_drift_detection.md](../reference/anomaly_drift_detection.md): Anomaly detection and drift analysis API reference.

**Additional References:**
- [docs/reference/parallel-slo-api.md](../reference/parallel-slo-api.md): ParallelMetricBatch API reference (Phase 3 streaming SLO evaluation).

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
- [examples/python/dashboard_cli_examples.py](../../examples/python/dashboard_cli_examples.py)
- [examples/python/genai_connector_examples.py](../../examples/python/genai_connector_examples.py)

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
