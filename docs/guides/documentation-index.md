# Documentation Index

This index helps you find the right guide for your task. Start with your goal and follow the recommended path.

## Start Here (Pick One)

New to NeuralBudget? Choose your path:

- **I want to evaluate my first SLO** → [Getting Started](getting-started.md) (10 minutes)
- **I'm building with Python** → [User Guide](user-guide.md) (mode selection, config patterns, CI/CD)
- **I need production deployment help** → [Production Deployment](production-deployment.md) (Kubernetes, monitoring, alerts)

## Find Docs By Goal

### Setup & First Steps

- **I'm getting installation errors** → [Troubleshooting: Installation](troubleshooting.md#installation-issues)
- **I need to understand terminology** → [Glossary](../reference/glossary.md)
- **I'm seeing error codes** → [Error Reference](../reference/errors.md) (all errors with solutions)

### Using NeuralBudget

**Choose your SLO mode:**
- HTTP/gRPC histogram SLOs: [User Guide: HTTP Mode](user-guide.md#http-slo-evaluation)
- Stateful service SLOs: [User Guide: Stateful Mode](user-guide.md#stateful-service-slos)
- ML serving SLOs: [User Guide: ML Mode](user-guide.md#ml-serving-slos)
- GenAI workload SLOs: [User Guide: GenAI Mode](user-guide.md#genai-workload-slos)
- Composite service graphs: [User Guide: Composite DAG](user-guide.md#composite-dependency-dags)

**API & Configuration:**
- Complete Python API reference: [API Reference](../reference/api.md)
- Convenience layer functions: [Convenience Functions](../reference/convenience-layer.md)
- Composite DAG schema: [Composite SLO DAG](../reference/composite-slo-dag.md)
- Streaming aggregator API: [Streaming Aggregator](../reference/streaming-aggregator.md)

### Advanced Features

- **High-frequency metric collection** → [Phase 3: Streaming](../internal/phases/PHASE3_GETTING_STARTED.md)
- **Parallel metric evaluation** → [Parallel API Reference](PARALLEL_SLO_API_REFERENCE.md)
- **Adaptive memory management** → [Adaptive Windowing Design](../internal/design/ADAPTIVE_WINDOWING_DESIGN.md)
- **Anomaly detection & drift** → [Anomaly Detection](../reference/ANOMALY_DETECTION_IMPLEMENTATION.md)
- **Burn-rate alerting** → [Burn-Rate Forecasting](burn-rate-forecasting.md)
- **Advanced alert dispatch** → [Advanced Alert Dispatch](../reference/ADVANCED_ALERT_DISPATCH_SUMMARY.md)
- **GenAI integrations** → [GenAI Connectors](../reference/GENAI_CONNECTORS_PHASE4_SUMMARY.md)
- **Dashboard & CLI monitoring** → [Dashboard CLI](../reference/DASHBOARD_CLI_README.md)

### Production & Operations

- **Deploy to Kubernetes** → [Kubernetes Integration](kubernetes-integration.md)
- **Configure Prometheus** → [Prometheus Scraping Guide](prometheus-scraping-examples.md)
- **Generate Prometheus rules** → [Prometheus Rule Generation](prometheus-rule-generation.md)
  - Multi-burn-rate alerting strategy
  - Recording rules for SLI tracking
  - Generated rules examples- **Multi-burn-rate alerting** → [Multi-Burn-Rate Alerting](multi-burn-rate-alerting.md)
  - Google SRE error budget patterns
  - 4-window alerting configuration
  - Real-world incident examples
  - Tuning guidelines by service type- **Build Grafana dashboards** → [Grafana Dashboards](../../examples/grafana/README.md)
- **Set up webhooks & alerting** → [Webhook Alerting](../../examples/python/webhook_alerting.py)
- **Troubleshoot issues** → [Troubleshooting Guide](troubleshooting.md)

### Understanding the System

- **How does NeuralBudget work?** → [Architecture](../reference/architecture.md) (with diagrams)
- **Why Rust + Python?** → [Architecture: Design Rationale](../reference/architecture.md#why-rust-first-architecture)
- **What's the performance?** → [Architecture: Performance Characteristics](../reference/architecture.md#performance-characteristics)
- **How are scores calculated?** → [Composite SLO Scoring](../reference/composite-slo-dag.md#scoring)

### Contributing & Development

- **How do I contribute?** → [Contributing](../../CONTRIBUTING.md)
- **What changed in this release?** → [Changelog](../../CHANGELOG.md)
- **What's the roadmap?** → [Feature Plans](../plans/mlops-model-drift-serving-plan.md)

## Documentation Paths (By Role)

### Data Scientists / ML Engineers
1. [Getting Started](getting-started.md)
2. [User Guide: ML Mode](user-guide.md#ml-serving-slos)
3. [Anomaly Detection](../reference/ANOMALY_DETECTION_IMPLEMENTATION.md)
4. [Troubleshooting](troubleshooting.md)

### Backend / SRE Engineers
1. [Getting Started](getting-started.md)
2. [User Guide: HTTP Mode](user-guide.md#http-slo-evaluation)
3. [Kubernetes Integration](kubernetes-integration.md)
4. [Prometheus Rule Generation](prometheus-rule-generation.md)
5. [Multi-Burn-Rate Alerting](multi-burn-rate-alerting.md)
6. [Prometheus Scraping](prometheus-scraping-examples.md)
7. [Burn-Rate Alerting](burn-rate-forecasting.md)

### DevOps / Platform Teams
1. [Production Deployment](production-deployment.md)
2. [Kubernetes Integration](kubernetes-integration.md)
3. [Prometheus Scraping](prometheus-scraping-examples.md)
4. [Advanced Alert Dispatch](../reference/ADVANCED_ALERT_DISPATCH_SUMMARY.md)
5. [Grafana Dashboards](../../examples/grafana/README.md)

### API Users / Library Integrators
1. [API Reference](../reference/api.md)
2. [Convenience Functions](../reference/convenience-layer.md)
3. [Error Reference](../reference/errors.md)
4. [Architecture](../reference/architecture.md)

## How to Use This Index

1. **Scan the "Start Here" section** — Most users pick one link here
2. **Find your goal in "By Goal"** — Section headers match common tasks
3. **Use role-based paths** — Jump to your role's recommended sequence
4. **Check the glossary first** — If you're confused by terminology, read [Glossary](../reference/glossary.md)
5. **Search the error reference** — If you see an error, find it in [Errors](../reference/errors.md)

## All Docs Overview

| Category | Purpose | Files |
|---|---|---|
| **Guides** | Step-by-step tutorials for tasks | getting-started, user-guide, production-deployment, kubernetes, ... |
| **Reference** | Complete API and design docs | api, architecture, glossary, errors, composite-dag, ... |
| **Examples** | Runnable code snippets | [examples/](../../examples/) directory |
| **Plans** | Feature roadmaps | mlops-model-drift-serving-plan |
| **Archives** | Internal reports | docs/internal/ (audits, verifications) |
