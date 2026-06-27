# NeuralBudget

[![CI](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![Python 3.9+](https://img.shields.io/badge/python-3.9%2B-3776AB)](pyproject.toml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-DEA584)](https://www.rust-lang.org/)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Coverage](https://img.shields.io/badge/coverage-87%25-brightgreen)](.github/workflows/ci.yml)

## Deterministic SLO Evaluation

Define SLO thresholds once. Evaluate identically across CI/CD, notebooks, and production. Rust-powered (< 1μs per evaluation) with Python bindings.

## Quick Start

```bash
pip install neuralbudget
```

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient()
client.load_config("slo.json")
result = client.evaluate({
    "timestamp": 1, "success": 9995, "total": 10000,
    "format": "prometheus_cumulative"
})
print(f"✓ SLO Pass: {result['passed']}")
```

👉 **[→ Start Here: Complete Documentation](docs/INDEX.md)**

Choose your path:
- **First time?** → [5-minute quickstart](docs/quickstart/)
- **Building with Python?** → [User guide](docs/guides/user-guide.md)
- **Deploying to production?** → [Production deployment](docs/guides/production-deployment.md)
- **Kubernetes?** → [Kubernetes integration](docs/guides/kubernetes-integration.md)

## Key Features

- ✅ **HTTP/gRPC**, **Stateful**, **ML**, **GenAI**, **Composite SLO** modes
- ✅ **Streaming aggregation** — 15k+ messages/sec with adaptive windowing
- ✅ **Prometheus + OpenTelemetry** — Native exporters
- ✅ **100% reproducible** — Same results across platforms, no floating-point variation
- ✅ **Fast** — < 1μs per evaluation, millions/sec throughput
- ✅ **Type-safe** — Rust core + Python validation

## More Information

| Resource | Purpose |
|----------|---------|
| **[Docs](docs/INDEX.md)** | Complete documentation organized by goal |
| **[Glossary](docs/reference/glossary.md)** | Key terms & acronyms |
| **[API Reference](docs/reference/api.md)** | Full Python API |
| **[Examples](examples/)** | Grafana, Kubernetes, Python examples |
| **[License](LICENSING.md)** | Apache 2.0 + commercial options |

## Contributing

[See CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute.

---

**Version:** 0.1.3 | **License:** [Apache 2.0](LICENSE) | **Changelog:** [CHANGELOG.md](CHANGELOG.md)

