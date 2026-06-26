# NeuralBudget

[![CI](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml)
[![CD](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml)
[![Release](https://img.shields.io/github/v/release/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![Tag](https://img.shields.io/github/v/tag/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/tags)
[![Last Commit](https://img.shields.io/github/last-commit/pristley/NeuralBudget/main)](https://github.com/pristley/NeuralBudget/commits/main)
[![Changelog](https://img.shields.io/badge/changelog-keep%20a%20changelog-blue)](CHANGELOG.md)
[![Docs](https://img.shields.io/badge/docs-reference%20index-blue)](docs/guides/documentation-index.md)
[![Coverage](https://img.shields.io/badge/coverage-87%25-brightgreen)](https://github.com/pristley/NeuralBudget/blob/main/.github/workflows/ci.yml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-DEA584)](https://www.rust-lang.org/)
[![Python 3.9+](https://img.shields.io/badge/python-3.9%2B-3776AB)](pyproject.toml)
[![PyO3](https://img.shields.io/badge/pyo3-0.24-orange)](https://pyo3.rs)
[![License](https://img.shields.io/badge/license-source--available-lightgrey)](LICENSE)

## Overview

**NeuralBudget** is a Rust-first SLO (Service Level Objective) toolkit for deterministic reliability analytics across service, ML, and GenAI workloads. It provides a strongly-typed Rust core with native Python bindings, enabling teams to run reproducible, inspectable reliability calculations across notebooks, CI/CD pipelines, and production systems.

### Core Capabilities

- **Availability & Error Budget** — Calculate remaining budget and burn velocity
- **HTTP/gRPC Histogram SLOs** — Stateless latency percentile + availability evaluation
- **Stateful Service SLOs** — Evaluate replication lag, queue depth, pool saturation, and connection wait penalties
- **ML Serving SLOs** — Hybrid scoring combining latency, GPU utilization, drift, and prediction confidence
- **GenAI Workload SLOs** — Track throughput (TPS), responsiveness (TTFT), and semantic quality
- **Composite Dependency DAGs** — Service graph evaluation with failure propagation and global SLO calculation
- **Prometheus & OpenTelemetry** — Native exporters and ingestion for observability integration

### Why NeuralBudget?

Use NeuralBudget when you need to:

- **Enforce policies** — Gate CI/CD or releases on quantified reliability metrics
- **Notebook analytics** — Run deterministic SLO calculations in Jupyter, notebooks, or Python scripts
- **Multi-workload evaluation** — Assess services, ML models, and LLM systems with unified SLO framework
- **Reproducible results** — Guarantee identical outputs across languages and environments

## Table of Contents

- [Getting Started](#getting-started)
- [Architecture & Design](#architecture--design)
- [Key Dependencies](#key-dependencies)
- [Quick Start](#quick-start)
- [Core Features](#core-features)
- [Integration Examples](#integration-examples)
- [Contribution Guidelines](#contribution-guidelines)
- [Documentation](#documentation)
- [License](#license)

---

## Getting Started

### Why NeuralBudget?

Choose NeuralBudget when you need:

- **Reproducible SLO calculations** — Run the same metrics through different environments and get identical results
- **Multi-workload evaluation** — Handle services, ML models, and LLM systems with one framework
- **Policy enforcement** — Gate CI/CD deployments on quantified, deterministic reliability metrics
- **Development-time validation** — Evaluate SLOs in notebooks or local scripts before production
- **Python + Rust stack** — Get Rust's performance with Python's productivity

### Prerequisites

- **Rust**: 2021 edition (for crate usage)
- **Python**: 3.9+ (for Python extension)
- **Build tools**: For local extension builds (Maturin, pip, compiler toolchain)

### Installation

#### Rust Crate

Add to `Cargo.toml`:

```toml
[dependencies]
neuralbudget = "0.1.3"
```

#### Python Extension from PyPI

The recommended way for Python users:

```bash
pip install neuralbudget
```

#### Python Extension from Source

For development or custom builds:

```bash
# Clone and install build tools
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget
pip install --upgrade pip maturin

# Build release wheels
maturin build --release

# Install from built wheel
pip install --force-reinstall target/wheels/neuralbudget-*.whl
```

#### Development Install (Editable)

For active development with auto-rebuild:

```bash
maturin develop --release
```

---

## Architecture & Design

### Design Philosophy

NeuralBudget follows three core principles:

1. **Determinism First** — All calculations are pure functions; identical inputs always produce identical outputs regardless of language, runtime, or execution order.

2. **Type Safety Across Boundaries** — Rust compile-time types + Python TypedDict validation ensure correctness at the language boundary.

3. **Minimal Convenience Layer** — Python helpers are thin wrappers around Rust logic, keeping all heavy lifting in the compiled core for correctness and performance.

### Why Rust-First Architecture?

**Performance**: Rust's zero-cost abstractions and compiled execution mean SLO evaluations complete in microseconds, not milliseconds.

**Correctness**: Strong type system prevents entire classes of bugs at compile-time. All business logic lives in one place (Rust core), ensuring identical behavior across bindings.

**Determinism**: Rust's no-GC, no-runtime model guarantees reproducible results. The same metric always produces the same SLO evaluation, whether called from Python, CLI, or Kubernetes sidecar.

**Interoperability**: PyO3 enables native Python bindings without wrapper overhead. Call Rust code from Python as if it were Python, but with Rust's guarantees.

### Why This Architecture?

The Rust-core + Python-bindings design addresses fundamental challenges in reliability engineering:

#### Problem 1: Reproducibility Across Environments
**Challenge**: SLO calculations must be identical whether evaluated in CI/CD pipelines, production sidecars, or analytics notebooks.  
**Solution**: By implementing all calculation logic in a compiled Rust core, NeuralBudget guarantees bit-for-bit reproducibility. There's no floating-point runtime variation, no GC pauses, no JIT compilation artifacts. Python developers get that guarantee automatically.

#### Problem 2: Bridging Data Science and Systems Engineering
**Challenge**: Data scientists need notebooks and Python; infrastructure teams need compiled reliability and performance. A pure Python implementation would be too slow for production; a pure Rust-only library frustrates notebook users.  
**Solution**: Rust provides the deterministic calculation engine; Python provides the ergonomic interface. Users choose their tool, get Rust's correctness guarantees either way.

#### Problem 3: Correctness at Scale
**Challenge**: SLO evaluation mistakes can lead to under-provisioning, false escalations, and eroded customer trust. Type systems alone aren't enough.  
**Solution**: Strong types in Rust + TypedDict validation in Python + schema versioning in YAML configs create multiple layers of defense:
- Compile-time checks catch API mismatches in Rust
- Runtime TypedDict validation catches malformed configs
- Schema versioning prevents silent incompatibilities across versions

#### Problem 4: Performance Without Compromise
**Challenge**: Composite DAG evaluation can be expensive (topological sort, failure propagation). In CI/CD, we need sub-millisecond latency; in notebooks, we need interactivity.  
**Solution**: Rust's zero-cost abstractions mean no overhead. Evaluating a 50-node composite SLO graph completes in microseconds, making it feasible to:
- Gate CI/CD on SLO metrics without slowing builds
- Interactively explore SLO scenarios in notebooks
- Run thousands of evaluations in seconds for analysis

#### Problem 5: Minimal Maintenance Burden
**Challenge**: Keeping multiple implementations in sync (Python-only, Node-only, Go-only) is a maintainability nightmare. Bugs fixed in one don't reach others; features diverge; APIs drift.  
**Solution**: Single source of truth in Rust. Python bindings are thin FFI wrappers. Testing focuses on the core. When you update business logic, it updates everywhere automatically.

### Project Structure

| Component | Purpose | Language |
|-----------|---------|----------|
| `src/core.rs` | SLO models and calculation logic | Rust |
| `src/exporter.rs` | Prometheus metrics rendering | Rust |
| `src/otlp.rs` | OpenTelemetry format conversion | Rust |
| `src/python.rs` | PyO3 FFI bindings | Rust |
| `python/neuralbudget/` | High-level facade and helpers | Python |
| `tests/` | Unit, integration, and property-based tests | Rust + Python |

For detailed module responsibilities and interactions, see [agentmap.md](agentmap.md).

---

## Key Dependencies

| Dependency | Version | Purpose | Why |
|-----------|---------|---------|-----|
| **pyo3** | 0.24.2 | Python ↔ Rust interop | Enable native extension bindings |
| **serde** | 1.0 | Serialization framework | Config schema versioning and portability |
| **serde_yaml** | 0.9 | YAML support | User-friendly config files |
| **serde_json** | 1.0 | JSON support | Alternative config format + OTLP ingestion |
| **criterion** | 0.5 | Benchmarking (dev) | Track composite DAG performance trends |
| **proptest** | 1.6 | Property-based testing (dev) | Verify invariants across input spaces |

**Optional Runtime Dependencies** (for alerting):
- Slack, PagerDuty, Opsgenie webhook APIs (via stdlib `urllib`)

**External Services** (optional integration):
- Prometheus (metrics scraping + alerting)
- OpenTelemetry Collector (OTLP ingestion)

---

## Quick Start

### Python: Basic Availability Check

```python
from neuralbudget.convenience import availability_snapshot

snapshot = availability_snapshot(success=9_995, total=10_000, slo_target=0.999)
print(f"Availability: {snapshot['availability']:.4f}")
print(f"SLO Met: {snapshot['target_met']}")
```

### Python: HTTP SLO Evaluation

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient().load_config("slo.yaml")

result = client.evaluate({
    "timestamp": 1,
    "success": 9995,
    "total": 10000,
    "buckets": [
        {"upper_bound_ms": 100.0, "count": 9700},
        {"upper_bound_ms": 220.0, "count": 10000},
    ],
    "format": "prometheus_cumulative",
})

print(f"SLO Pass: {result['passed']}")
```

### Rust: Availability & Error Budget

```rust
use neuralbudget::{calculate_availability, calculate_error_budget};

let availability = calculate_availability(9_995, 10_000);
let error_budget_seconds = calculate_error_budget(0.999, 3_600);

assert_eq!(availability, 0.9995);
assert_eq!(error_budget_seconds, 3.6);
```

### YAML Configuration Example

```yaml
schema_version: 1
mode: http
profile: balanced
params:
  latency_threshold_ms: 200.0
  availability_threshold: 0.999
alerts:
  slack:
    webhook_url: "${SLACK_WEBHOOK_URL}"
```

---

## Core Features

### Stateless HTTP/gRPC SLOs

Evaluate histogram samples on two gates: latency percentile threshold and availability threshold.

**Supported formats**:
- `prometheus_cumulative`
- `open_telemetry_delta`

```python
import neuralbudget

slo = neuralbudget.HttpSlo(
    latency_threshold_ms=200.0,
    latency_percentile=0.99,
    availability_threshold=0.999,
)

sample = neuralbudget.HistogramSample(
    timestamp=1,
    success=9995,
    total=10000,
    buckets=[
        neuralbudget.HistogramBucket(100.0, 9700),
        neuralbudget.HistogramBucket(150.0, 200),
        neuralbudget.HistogramBucket(220.0, 100),
    ],
    format="open_telemetry_delta",
)

evaluation = slo.evaluate_stream([sample])[0]
print(f"Pass: {evaluation.pass}, P99 Latency: {evaluation.percentile_latency_ms}ms")
```

### Stateful Service SLOs

Evaluate service health indicators: replication lag, queue depth, connection pool saturation, and wait time penalties.

```rust
use neuralbudget::{StatefulSample, StatefulSlo};

let slo = StatefulSlo::default();
let sample = StatefulSample {
    timestamp: 1,
    replication_lag_ms: 190.0,
    queue_depth: 650,
    connection_pool_saturation: 0.74,
    connection_wait_time_ms: 10.0,
};

let eval = slo.evaluate_sample(sample);
assert!(eval.pass);
```

### ML Serving SLOs

Combine system performance (latency, GPU utilization) with model quality (drift, confidence) using weighted hybrid scoring.

**Formula**: `Hybrid Score = (Latency Score × latency_weight) + (Drift Score × drift_weight)`

```python
from neuralbudget.convenience import evaluate_ml_once

result = evaluate_ml_once(
    {
        "timestamp": 1,
        "inference_latency_ms": 185.0,
        "gpu_utilization": 0.72,
        "feature_drift": 0.07,
        "prediction_confidence": 0.93,
    },
    latency_weight=0.6,
    drift_weight=0.4,
)

print(f"Hybrid Score: {result['hybrid_score']:.3f}, Pass: {result['pass']}")
```

### GenAI Workload SLOs

Evaluate LLM serving reliability across throughput (TPS), responsiveness (TTFT), and semantic quality.

```python
from neuralbudget.convenience import evaluate_genai_once

result = evaluate_genai_once(
    {
        "timestamp": 1,
        "tokens_generated": 420,
        "generation_duration_ms": 14000,
        "time_to_first_token_ms": 850,
        "reference_text": "NeuralBudget is a deterministic SLO toolkit.",
        "generated_text": "NeuralBudget provides deterministic reliability scoring.",
    },
    profile="default",
)

print(
    f"TPS: {result['tokens_per_second']:.1f}, "
    f"Quality: {result['semantic_similarity']:.2f}, "
    f"Pass: {result['pass']}"
)
```

### Composite Dependency DAGs

Evaluate service dependency graphs with automatic failure propagation and weighted global SLO calculation.

```python
import neuralbudget

graph = neuralbudget.CompositeSloGraph(
    services=[
        neuralbudget.CompositeServiceSlo("api-gateway", 0.95, 0.9, 2.0),
        neuralbudget.CompositeServiceSlo("auth-service", 0.98, 0.9, 1.5),
        neuralbudget.CompositeServiceSlo("payment-service", 0.92, 0.9, 3.0),
    ],
    dependencies=[
        neuralbudget.CompositeDependencyEdge("auth-service", "api-gateway", 0.15),
        neuralbudget.CompositeDependencyEdge("payment-service", "api-gateway", 0.25),
    ],
    global_min_pass_score=0.85,
)

evaluation = neuralbudget.evaluate_composite_slo_graph(graph)
print(f"Global SLO: {evaluation.global_slo:.3f}, System Pass: {evaluation.global_pass}")
```

---

## Integration Examples

---

## Documentation

Complete documentation organized by use case:

### Core SLO Framework

| Document | Audience | Purpose |
|----------|----------|---------|
| [Getting Started](docs/guides/getting-started.md) | New users | First successful run walkthrough |
| [User Guide](docs/guides/user-guide.md) | Developers | Comprehensive configuration and API reference |
| [Architecture Map](agentmap.md) | Architects | Module responsibilities and service interactions |
| [Production Deployment](docs/guides/production-deployment.md) | Operations | Rollout patterns and best practices |
| [Kubernetes Integration](docs/guides/kubernetes-integration.md) | Platform engineers | K8s manifests and ServiceMonitor setup |
| [Prometheus Integration](docs/guides/prometheus-scraping-examples.md) | SREs | Scrape configs and alert rules |
| [Convenience Layer API](docs/reference/convenience-layer.md) | Python users | Helper functions and profile presets |
| [Composite DAG Reference](docs/reference/composite-slo-dag.md) | Advanced users | Dependency graph evaluation semantics |
| [Grafana Dashboards](examples/grafana/README.md) | Operators | Pre-built visualization templates |

### Phase 3: High-Performance Streaming and Parallel Evaluation

| Document | Audience | Purpose |
|----------|----------|---------|
| [Getting Started](PHASE3_GETTING_STARTED.md) | Python developers | Task-based walkthrough with code examples |
| [API Reference](PARALLEL_SLO_API_REFERENCE.md) | API users | Complete method signatures and performance characteristics |
| [Deployment Guide](DEPLOYMENT_GUIDE.md) | Operations/DevOps | Production configuration, monitoring, and troubleshooting |

### Runnable Examples

See [examples/](examples/) for Python, Kubernetes, and Prometheus configurations:

```bash
python3 examples/python/availability_budget.py
python3 examples/python/http_slo_histogram.py
python3 examples/python/ml_slo_drift_serving.py
python3 examples/python/webhook_alerting.py
```

---

## Contribution Guidelines

### Reporting Issues

Found a bug or have a feature request?

1. **Check existing issues** — Search [GitHub Issues](https://github.com/pristley/NeuralBudget/issues) first
2. **Provide context** — Include:
   - What you tried (code example)
   - What you expected
   - What actually happened
   - Environment (OS, Python/Rust version, NeuralBudget version)
3. **Minimal reproduction** — A small, self-contained example helps us fix faster

### Submitting Pull Requests

We welcome contributions! Here's how:

1. **Fork and branch** — Create a feature branch from `main`
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Follow the style guide** — Code quality checks run on all PRs:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   python3 -m black tests/ python/
   ```

3. **Add tests** — All changes should have test coverage:
   ```bash
   cargo test --all-targets --all-features
   python3 tests/python_*_tests.py
   ```

4. **Update docs** — If adding features, update relevant documentation in `docs/`

5. **Push and open PR** — Reference any related issues in the description

### Development Setup

```bash
# Clone
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget

# Install dependencies
pip install --upgrade pip maturin
cargo update

# Local validation (matching CI)
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
python3 tests/python_*.py

# Build extension for development
maturin develop --release
```

### Code Quality Standards

- **Test coverage**: Minimum 87% line coverage (verified in CI)
- **Formatting**: `cargo fmt` (Rust), `black` (Python)
- **Linting**: `cargo clippy` (Rust) with no warnings
- **Documentation**: Public APIs must have docstrings/comments
- **Performance**: Composite DAG evaluations benchmarked in `benches/`

### Commit Message Convention

Keep commits clear and descriptive:

```
[type]: Brief description

Longer explanation if needed.

- Bullet points for multiple changes
- Or separate commits for clarity
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`

Example:
```
feat: Add timeout parameter to composite SLO evaluation

Allows callers to abort long-running DAG evaluations.
Useful for strict latency requirements in CI/CD.

Resolves #123
```

---

## License & Attribution

This repository is published under the **NeuralBudget Source-Available License 1.0**.

### Terms Summary

- **Use**: You may use this code for evaluation, internal development, and non-commercial purposes
- **Modification**: You may modify the code for internal use
- **Distribution**: Commercial use or redistribution requires a license agreement
- **Attribution**: Attribution required; preserve license notices

See [LICENSE](LICENSE) for full legal text.

---

## Quick Links

- **GitHub**: https://github.com/pristley/NeuralBudget
- **PyPI**: https://pypi.org/project/neuralbudget/
- **Issues**: https://github.com/pristley/NeuralBudget/issues
- **Releases**: https://github.com/pristley/NeuralBudget/releases


---

## Advanced Integrations

### OpenTelemetry Protocol (OTLP) Ingestion

NeuralBudget can ingest OTLP metric payloads directly, converting them to native samples.

```python
import neuralbudget

payload = """{
    "resourceMetrics": [{
        "scopeMetrics": [{
            "metrics": [{
                "name": "http.server.duration",
                "histogram": {
                    "dataPoints": [{
                        "timeUnixNano": "1700000000000000000",
                        "count": "100",
                        "bucketCounts": ["70", "25", "5"],
                        "explicitBounds": [100.0, 250.0]
                    }]
                }
            }]
        }]
    }]
}"""

slo = neuralbudget.HttpSlo(200.0, 0.99, 0.95)
evaluations = neuralbudget.evaluate_http_slo_otlp(payload, "http.server.duration", slo)
print(f"Pass: {evaluations[0].pass}")
```

**Supported helpers**:
- `ingest_otlp_histogram()` — Convert OTLP histogram to HistogramSample
- `ingest_otlp_numeric()` — Convert OTLP gauge/sum to MetricPoint
- `evaluate_http_slo_otlp()` — Evaluate HTTP SLO directly from OTLP payload

### Prometheus Metrics Export

NeuralBudget renders evaluation results as Prometheus text exposition format for scraping:

```python
import neuralbudget

slo = neuralbudget.HttpSlo(200.0, 0.99, 0.999)
sample = neuralbudget.HistogramSample(
    timestamp=1,
    success=100,
    total=100,
    buckets=[neuralbudget.HistogramBucket(100.0, 100)],
    format="prometheus_cumulative",
)
evaluation = slo.evaluate_histogram(sample)

# Reusable exporter
exporter = neuralbudget.PrometheusExporter("neuralbudget")
exporter.set_static_label("env", "prod")
exporter.observe_http_slo("api-gateway", evaluation)
print(exporter.render())
```

---

## Development

### Local Setup

Clone and setup development environment:

```bash
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget

# Install build tools
pip install --upgrade pip maturin
cargo update

# Run local validation (matching CI)
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
python3 tests/python_*.py

# Build extension
maturin develop --release
```

### Testing & Coverage

NeuralBudget uses property-based tests (via Proptest) and deterministic unit tests:

```bash
# Run all tests with coverage
cargo llvm-cov --workspace --all-features --lib --tests --summary-only

# Run property-based tests (in src/tests.rs)
cargo test --lib core -- --nocapture

# Benchmark composite DAG evaluation
cargo bench composite_slo_dag
```

**Coverage Requirements**:
- Minimum: 87% line coverage (CI gate)
- Target: 95%+ for release confidence

### Release Process

NeuralBudget uses GitHub Actions for cross-platform builds and PyPI publishing:

1. **Update version** in `pyproject.toml` and `Cargo.toml`
2. **Commit and push** to `main`
3. **Create a git tag**: `git tag v0.1.3 && git push origin v0.1.3`
4. **Create GitHub Release** — CD automatically:
   - Validates all checks (fmt, clippy, tests, coverage)
   - Builds wheels for Linux, macOS, Windows
   - Publishes to PyPI

Published artifacts:
- Linux x86_64 (manylinux2014)
- macOS aarch64
- Windows x86_64
- Python source distribution (sdist)

---

## Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| **HTTP SLO Evaluation** | <1ms | Per histogram sample |
| **Availability Calculation** | <100μs | Pure arithmetic |
| **Composite DAG (100 services)** | <10ms | Including topological sort |
| **Alert Dispatch** | 100-1000ms | Network I/O dependent |

**Memory**: ~1MB per client instance (Python or Rust)

**Throughput**: Thousands of SLO evaluations per second on modern hardware

---

## Troubleshooting

### Python Import Issues

**Problem:** `ImportError: No module named 'neuralbudget'`

**Solution:**
```bash
# Reinstall the extension
pip install --force-reinstall neuralbudget

# Or rebuild from source
maturin develop
```

### Configuration Validation Errors

**Problem:** `ValueError: unknown variant in 'mode'`

**Solution:** Ensure config has a valid mode (`http`, `stateful`, `ml`, `genai`, `composite`):
```yaml
mode: http  # Must be one of the valid types
params: {...}
```

### Coverage Gate Failures

**Problem:** CI fails with "coverage below 87%"

**Solution:** Write tests for new code:
```bash
# Check current coverage
cargo llvm-cov --all-features --lib --tests --summary-only

# View detailed report
cargo llvm-cov --all-features --html
open target/llvm-cov/html/index.html
```

### Wheel Build Failures

**Problem:** `maturin build` fails with compiler errors

**Solution:**
```bash
# Update maturin
pip install --upgrade maturin

# Clean and rebuild
cargo clean
maturin build --release
```

### PyO3 Deprecation Warnings

**Note:** Some warnings during development are acceptable. They're tracked in [AUDIT_REPORT.md](AUDIT_REPORT.md) and don't affect functionality.

For more troubleshooting, see [docs/guides/development.md](docs/guides/development.md#troubleshooting-development-issues).

---

## Documentation

- **[Architecture Map](agentmap.md)** — Module responsibilities and interactions
- **[User Guide](docs/guides/user-guide.md)** — Comprehensive usage walkthrough
- **[API Reference](docs/reference/api.md)** — Python API with examples
- **[Development Guide](docs/guides/development.md)** — Setup, testing, debugging
- **[Contributing](CONTRIBUTING.md)** — PR guidelines and code standards
- **[Audit Report](AUDIT_REPORT.md)** — Code quality assessment and recommendations
- **[Production Deployment](docs/guides/production-deployment.md)** — Deployment topology and operations

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history and [docs/guides/documentation-index.md](docs/guides/documentation-index.md) for complete documentation index.


