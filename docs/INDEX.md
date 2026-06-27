# NeuralBudget Documentation

Welcome! This is your starting point for NeuralBudget documentation. Use the quick links below to find what you need.

## 🚀 Start Here

**Never used NeuralBudget before?** Pick one:

- [5-Minute HTTP SLO](quickstart/5-minute-http-slo.md) — Evaluate latency & availability for services
- [5-Minute ML SLO](quickstart/5-minute-ml-slo.md) — Monitor model performance & data drift
- [5-Minute GenAI SLO](quickstart/5-minute-genai-slo.md) — Track LLM endpoints & RAG systems

Or start with the [Getting Started Guide](guides/getting-started.md) for a complete walkthrough.

---

## 📚 Documentation by Goal

### Learning & Tutorials

| Goal | Document | Time |
|------|----------|------|
| First evaluation | [Getting Started](guides/getting-started.md) | 10 min |
| Understand concepts | [Glossary](reference/glossary.md) | 5 min |
| Learn by mode | [User Guide](guides/user-guide.md) | 30 min |
| Troubleshoot issues | [Troubleshooting](guides/troubleshooting.md) | varies |

### Implementation & Integration

| Goal | Document | Audience |
|------|----------|----------|
| Python API reference | [API Reference](reference/api.md) | Developers |
| Convenience functions | [Convenience Layer](reference/convenience-layer.md) | Python users |
| Composite DAGs | [Composite SLO DAG](reference/composite-slo-dag.md) | Advanced users |
| Streaming aggregator | [Streaming Aggregator](reference/streaming-aggregator.md) | High-frequency use |
| Error solutions | [Error Reference](reference/errors.md) | Troubleshooting |

### Production & Operations

| Goal | Document |
|------|----------|
| Deploy to production | [Production Deployment](guides/production-deployment.md) |
| Kubernetes setup | [Kubernetes Integration](guides/kubernetes-integration.md) |
| Prometheus integration | [Prometheus Scraping](guides/prometheus-scraping-examples.md) |
| Grafana dashboards | [Grafana Setup](../examples/grafana/README.md) |
| Alert configuration | [Alert Dispatch](guides/advanced_alert_dispatch.md) |
| Burn-rate alerting | [Burn-Rate Forecasting](guides/burn-rate-forecasting.md) |

### Advanced Topics

| Goal | Document |
|------|----------|
| Architecture & design | [Architecture](reference/architecture.md) |
| Anomaly detection | [Anomaly Detection](reference/ANOMALY_DETECTION_IMPLEMENTATION.md) |
| GenAI connectors | [GenAI Integration](guides/genai_connectors.md) |
| Dashboard CLI | [Dashboard & CLI](guides/dashboard_cli.md) |
| Development setup | [Development Guide](guides/development.md) |

---

## 📂 Documentation Structure

```
docs/
├── INDEX.md                    ← You are here
├── quickstart/                 ← Copy-paste ready examples
│   ├── 5-minute-http-slo.md
│   ├── 5-minute-ml-slo.md
│   └── 5-minute-genai-slo.md
├── guides/                     ← Step-by-step tutorials
│   ├── getting-started.md
│   ├── user-guide.md
│   ├── production-deployment.md
│   ├── kubernetes-integration.md
│   └── ... (8 more guides)
├── reference/                  ← Complete API & design docs
│   ├── api.md
│   ├── architecture.md
│   ├── glossary.md
│   ├── errors.md
│   └── ... (10 more references)
└── internal/                   ← Internal & archived docs
    ├── phases/                 ← Phase 3 architecture
    ├── audits/                 ← Quality audits & reports
    ├── design/                 ← Design decisions
    ├── ci/                     ← CI/CD verification
    └── architecture/           ← System architecture
```

---

## 🎯 Recommended Reading Order

1. **New users**: [Getting Started](guides/getting-started.md) → Pick a quickstart → [User Guide](guides/user-guide.md)
2. **Python developers**: [API Reference](reference/api.md) → [Error Reference](reference/errors.md) → [Troubleshooting](guides/troubleshooting.md)
3. **Operations teams**: [Production Deployment](guides/production-deployment.md) → [Kubernetes](guides/kubernetes-integration.md) → [Prometheus](guides/prometheus-scraping-examples.md)
4. **ML/Data scientists**: [5-Minute ML SLO](quickstart/5-minute-ml-slo.md) → [Anomaly Detection](reference/ANOMALY_DETECTION_IMPLEMENTATION.md) → [User Guide ML Mode](guides/user-guide.md#ml-serving-slos)
5. **Contributors**: [CONTRIBUTING](../CONTRIBUTING.md) → [Development](guides/development.md) → [Architecture](reference/architecture.md)

---

## 🔗 Other Important Docs

- **[Licensing](internal/LICENSING.md)** — Apache 2.0 open source + commercial options
- **[Contributing](../CONTRIBUTING.md)** — How to contribute
- **[Changelog](internal/CHANGELOG.md)** — Release history
- **[README](../README.md)** — Project overview

---

## 🏗️ Internal Documentation

Internal docs are archived in `docs/internal/` for reference:

- **[docs/internal/phases/](internal/phases/)** — Phase 3 streaming & performance (archived)
- **[docs/internal/audits/](internal/audits/)** — Quality audits & reports
- **[docs/internal/design/](internal/design/)** — Design decisions & rationale
- **[docs/internal/ci/](internal/ci/)** — CI/CD verification reports
- **[docs/internal/architecture/](internal/architecture/)** — System architecture reference

---

## ❓ Can't Find Something?

- **Need help?** → [Troubleshooting](guides/troubleshooting.md)
- **Getting an error?** → [Error Reference](reference/errors.md)
- **Don't know the term?** → [Glossary](reference/glossary.md)
- **Want to contribute?** → [Contributing](../CONTRIBUTING.md)
- **Need commercial support?** → [Licensing](internal/LICENSING.md)

---

## 📋 Documentation Quick Links

**By Role:**
- Backend/SRE: [Production Deployment](guides/production-deployment.md), [Kubernetes](guides/kubernetes-integration.md), [Prometheus](guides/prometheus-scraping-examples.md)
- ML/Data Science: [ML SLO Mode](guides/user-guide.md#ml-serving-slos), [Anomaly Detection](reference/ANOMALY_DETECTION_IMPLEMENTATION.md)
- DevOps: [Kubernetes](guides/kubernetes-integration.md), [Grafana](../examples/grafana/README.md), [Alert Dispatch](guides/advanced_alert_dispatch.md)
- API Users: [API Reference](reference/api.md), [Convenience Layer](reference/convenience-layer.md)

**By Format:**
- Quick starts: [Quickstart guides](quickstart/)
- Tutorials: [Getting Started](guides/getting-started.md), [User Guide](guides/user-guide.md)
- API docs: [API Reference](reference/api.md), [Convenience Layer](reference/convenience-layer.md)
- Examples: [examples/](../examples/)
- Runbooks: [Production Deployment](guides/production-deployment.md), [Kubernetes](guides/kubernetes-integration.md)

---

**Last updated:** June 27, 2026 | **License:** [Apache 2.0](../LICENSE)
