# NeuralBudget

[![CI](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![Python 3.9+](https://img.shields.io/badge/python-3.9%2B-3776AB)](pyproject.toml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-DEA584)](https://www.rust-lang.org/)
[![Coverage](https://img.shields.io/badge/coverage-87%25-brightgreen)](.github/workflows/ci.yml)

## What You Build With NeuralBudget

Monitor service reliability with deterministic, reproducible SLO evaluations. Define thresholds once. Evaluate identically across CI/CD pipelines, notebooks, and production systems.

**Real examples:**
- Gate deployments on quantified reliability: "Block this build if availability < 99.9%"
- Track ML model quality: "Alert when prediction drift exceeds 5% or latency > 200ms"
- Monitor LLM endpoints: "Fail SLO if token throughput drops or semantic quality degrades"
- Composite service health: "Evaluate entire microservice graph; fail if critical path SLO breaches"

## Install (2 minutes)

```bash
pip install neuralbudget
```

## Evaluate Your First SLO (5 minutes)

```python
try:
    from neuralbudget import NeuralBudgetClient

    client = NeuralBudgetClient()
    client.load_config("slo.json")
    result = client.evaluate({
        "timestamp": 1,
        "success": 9995,
        "total": 10000,
        "format": "prometheus_cumulative",
    })
    print(f"✓ SLO Pass: {result['passed']}")
except FileNotFoundError:
    print("✗ Create slo.json first. See docs/guides/getting-started.md")
except ValueError as e:
    print(f"✗ Invalid config: {e}")
```

See [getting-started.md](docs/guides/getting-started.md) for step-by-step walkthrough.

## Key Capabilities

| Capability | Purpose | Use Case |
|---|---|---|
| **HTTP/gRPC Histogram Evaluation** | Latency percentile + availability gates | Service SLOs |
| **Stateful Service Metrics** | Replication lag, queue depth, saturation | Distributed system health |
| **ML Serving SLOs** | Latency + GPU utilization + model drift | Model monitoring |
| **GenAI Workload SLOs** | Throughput (TPS) + responsiveness + semantic quality | LLM endpoint monitoring |
| **Composite DAG Evaluation** | Service graph with failure propagation | System-wide health scoring |
| **Streaming Aggregation** | High-frequency windowed metrics | Production ingestion (15k+ msgs/sec) |
| **Prometheus & OpenTelemetry Integration** | Native exporters and scraping | Observability platforms |

## Why NeuralBudget?

**Deterministic:** All calculations are pure functions. Run the same metrics on Windows, Linux, or macOS; get identical results. No floating-point variation, no GC pauses.

**Type-Safe:** Rust compile-time guarantees at the core + Python TypedDict validation at the boundary. Catch errors before deployment.

**Fast:** Single evaluations complete in < 1 microsecond. Composite DAGs with 50+ services: < 100 microseconds. Evaluate millions of metrics per second on standard hardware.

**Reproducible:** Single source of truth in Rust. Python bindings are thin FFI wrappers. Fix a bug once; it fixes everywhere.

## Architecture

NeuralBudget pairs a **Rust core** (deterministic, fast, type-safe) with **Python bindings** (ergonomic, notebook-friendly):

- **Rust Core** (`src/core.rs`, `src/exporter.rs`): All SLO models and evaluation logic
- **Python Bindings** (`src/python.rs`): PyO3 FFI exposing Rust types to Python
- **Python Convenience Layer** (`python/neuralbudget/`): High-level functions for common patterns

Design rationale: Data scientists need Python notebooks; production systems need compiled performance. Both teams get deterministic guarantees.

See [docs/reference/architecture.md](docs/reference/architecture.md) for visual architecture diagrams and detailed design decisions.

## Choose Your Path

| Path | For | Start With |
|---|---|---|
| **Python** | Notebooks, scripts, CI/CD | [docs/guides/getting-started.md](docs/guides/getting-started.md) |
| **Rust** | Production binaries, embedded systems | [README](docs/reference/api.md) in crates.io |
| **Kubernetes** | Sidecar collectors, exporters | [docs/guides/kubernetes-integration.md](docs/guides/kubernetes-integration.md) |
| **Grafana** | Visual dashboards | [examples/grafana/README.md](examples/grafana/README.md) |

## Documentation

- **[Getting Started](docs/guides/getting-started.md)** — First evaluation in 10 minutes
- **[User Guide](docs/guides/user-guide.md)** — All SLO modes and config patterns
- **[API Reference](docs/reference/api.md)** — Complete Python API with examples
- **[Architecture](docs/reference/architecture.md)** — Design rationale and module interactions
- **[Glossary](docs/reference/glossary.md)** — Key terms and acronyms
- **[Troubleshooting](docs/guides/troubleshooting.md)** — Common errors and solutions
- **[Full Index](docs/guides/documentation-index.md)** — All docs organized by goal

## Version

Current: **0.1.3** | [Changelog](CHANGELOG.md) | [Contributing](CONTRIBUTING.md)

License: [Source-Available](LICENSE)
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



