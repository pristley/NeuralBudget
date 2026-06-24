# NeuralBudget

[![CI](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml)
[![CD](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml)
[![Release](https://img.shields.io/github/v/release/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![Tag](https://img.shields.io/github/v/tag/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/tags)
[![Last Commit](https://img.shields.io/github/last-commit/pristley/NeuralBudget/main)](https://github.com/pristley/NeuralBudget/commits/main)
[![Changelog](https://img.shields.io/badge/changelog-keep%20a%20changelog-blue)](CHANGELOG.md)
[![Docs](https://img.shields.io/badge/docs-reference%20index-blue)](docs/guides/documentation-index.md)
[![Coverage Gate](https://img.shields.io/badge/coverage%20gate-89%25-brightgreen)](https://github.com/pristley/NeuralBudget/blob/main/.github/workflows/ci.yml)
[![Rust 2021](https://img.shields.io/badge/rust-2021-DEA584)](https://www.rust-lang.org/)
[![Python 3.11+](https://img.shields.io/badge/python-3.11%2B-3776AB)](pyproject.toml)
[![PyO3](https://img.shields.io/badge/pyo3-0.22-orange)](https://pyo3.rs)
[![Benchmarks](https://img.shields.io/badge/benchmarks-criterion-informational)](benches/composite_slo_dag.rs)
[![License](https://img.shields.io/badge/license-source--available-lightgrey)](LICENSE)

NeuralBudget is a Rust-first SLO toolkit for deterministic reliability analytics across service, ML, and GenAI workloads.
It combines a strongly typed Rust core, PyO3-native Python bindings, and convenience helpers for notebook and pipeline workflows.

Core capabilities:

- availability and error-budget math
- burn-rate tracking over metric streams
- stateless HTTP/gRPC histogram SLO evaluation
- stateful service SLO evaluation (replication, queue depth, pool saturation, wait penalties)
- ML serving + drift hybrid SLO (`MlSlo`)
- GenAI qualitative SLO (`GenAiSlo`) with TPS, TTFT, and semantic-similarity placeholder scoring
- Composite dependency DAG SLO evaluation with propagated impact and System Global SLO

## Table of Contents

- [Why NeuralBudget](#why-neuralbudget)
- [What Is New](#what-is-new)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [User Guide](#user-guide)
- [Comprehensive User Guide](#comprehensive-user-guide)
- [Composite SLOs (Dependencies)](#composite-slos-dependencies)
- [Convenience Layer Guide](#convenience-layer-guide)
- [Examples](#examples)
- [Development and CI/CD](#development-and-cicd)
- [PyPI Publishing](#pypi-publishing)
- [Changelog and Documentation](#changelog-and-documentation)
- [License](#license)

## Why NeuralBudget

NeuralBudget is designed for teams that need reliability math to be reproducible, inspectable, and language-agnostic.

Use it when you need to:

- enforce SLO policies in CI/CD or release gates
- run consistent reliability analytics in notebooks and Python pipelines
- evaluate stateless + stateful workloads with clear pass/fail semantics
- track ML and GenAI quality signals with deterministic scoring rules

## What Is New

Recent additions include:

- `MlSlo` with weighted hybrid formula:
  - `Hybrid Score = (Latency Score * latency_weight) + (Drift Score * drift_weight)`
- `GenAiSlo` for LLM workloads:
  - throughput (`tokens_per_second`)
  - responsiveness (`time_to_first_token_ms`)
  - qualitative score (`semantic_similarity`) via sentence-transformers placeholder + lexical fallback
- convenience-layer profile presets and optional dataclass returns
- convenience helper for GenAI one-shot evaluations
- expanded Python convenience tests and CI coverage for convenience workflows
- Composite SLO DAG runner with cycle detection and weighted global score calculation

## Installation

### Rust crate

Add to Cargo.toml:

```toml
[dependencies]
neuralbudget = "0.1.1"
```

### Python extension (local build)

```bash
python3 -m pip install --upgrade pip maturin
maturin build --release --manifest-path Cargo.toml
python3 -m pip install --force-reinstall target/wheels/neuralbudget-*.whl
```

### Python extension (editable development install)

```bash
python3 -m pip install --upgrade pip maturin
maturin develop --release --manifest-path Cargo.toml
```

## Quick Start

### Rust: availability and budget

```rust
use neuralbudget::{calculate_availability, calculate_error_budget};

let availability = calculate_availability(9_995, 10_000);
let error_budget_seconds = calculate_error_budget(0.999, 3_600);

assert_eq!(availability, 0.9995);
assert_eq!(error_budget_seconds, 3.6);
```

### Python: one-shot convenience evaluation

```python
from neuralbudget.convenience import availability_snapshot

snapshot = availability_snapshot(success=9_995, total=10_000, slo_target=0.999)
print(snapshot["availability"], snapshot["target_met"])
```

### Python facade: notebook and pipeline entrypoint

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient().load_config("slo.yaml")

result = client.evaluate(
    {
        "timestamp": 1,
        "success": 9995,
        "total": 10000,
        "buckets": [
            {"upper_bound_ms": 100.0, "count": 9700},
            {"upper_bound_ms": 220.0, "count": 10000},
        ],
        "format": "prometheus_cumulative",
    }
)

print(result)
```

## User Guide

This README contains quick-reference usage.

For a full walkthrough with installation paths, mode-by-mode examples,
Jupyter and CI/CD recipes, and troubleshooting, see:

- [docs/guides/user-guide.md](docs/guides/user-guide.md)

## Comprehensive User Guide

- [NeuralBudget User Guide](docs/guides/user-guide.md)

### Core primitives

NeuralBudget exposes baseline reliability primitives:

- `calculate_availability(success, total)`
- `calculate_error_budget(slo_target, window_seconds)`
- `calculate_burn_rate(metric_stream, window_seconds)`
- `TimeWindow::rolling(...)` and `TimeWindow::calendar_aligned(...)`

Rolling vs calendar windows are useful for alerting and compliance windows.

```rust
use neuralbudget::TimeWindow;

let rolling = TimeWindow::rolling(3_600);
assert!(rolling.contains(1_699_999_999, 1_700_000_000));

let calendar = TimeWindow::calendar_aligned(86_400, 18_000);
assert!(calendar.contains(68_400, 90_000));
```

### Stateless HTTP/gRPC histogram SLO

`HttpSlo` evaluates histogram samples on two gates:

- latency percentile threshold
- availability threshold

Supported histogram formats:

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
print(evaluation.pass, evaluation.percentile_latency_ms)
```

### Stateful service SLO

`StatefulSlo` evaluates:

- replication lag
- queue depth
- connection pool saturation
- connection wait time penalty

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

### ML SLO (`MlSlo`)

`MlSlo` combines system and model quality signals.

- latency/system subscore from inference latency + GPU utilization
- drift/quality subscore from feature drift + prediction confidence
- weighted hybrid pass/fail threshold

Formula:

`Hybrid Score = (Latency Score * latency_weight) + (Drift Score * drift_weight)`

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

print(result["hybrid_score"], result["pass"])
```

### GenAI qualitative SLO (`GenAiSlo`)

`GenAiSlo` targets LLM-serving reliability and quality.

It checks:

- `tokens_per_second` against minimum throughput
- `time_to_first_token_ms` against maximum latency
- `semantic_similarity` against minimum qualitative threshold

Current semantic similarity behavior:

- tries sentence-transformers via Python interop where available
- falls back to lexical approximation for portability in constrained runtime environments

```python
from neuralbudget.convenience import evaluate_genai_once

result = evaluate_genai_once(
    {
        "timestamp": 1,
        "tokens_generated": 420,
        "generation_duration_ms": 14000,
        "time_to_first_token_ms": 850,
        "reference_text": "NeuralBudget is a deterministic SLO scoring toolkit.",
        "generated_text": "NeuralBudget provides deterministic reliability scoring.",
    },
    profile="default",
)

print(
    result["tokens_per_second"],
    result["semantic_similarity"],
    result["pass"],
)
```

### Composite SLOs (Dependencies)

Use `evaluate_composite_slo` when a service graph has upstream/downstream dependencies and you need propagated impact.

Behavior:

- traverses dependency graph in topological order
- rejects invalid graphs with cycles
- when a dependency fails, dependent services are automatically adjusted by edge penalty
- marks dependency impact per service (`dependency_adjusted`, `failed_dependencies`)
- computes weighted `global_slo` and evaluates `global_pass` against `global_min_pass_score`

```rust
use neuralbudget::{
    evaluate_composite_slo, CompositeDependencyEdge, CompositeServiceSlo, CompositeSloGraph,
};

let graph = CompositeSloGraph {
    services: vec![
        CompositeServiceSlo {
            service: "service_a".to_string(),
            local_score: 0.72,
            min_pass_score: 0.9,
            impact_weight: 2.0,
        },
        CompositeServiceSlo {
            service: "service_b".to_string(),
            local_score: 0.97,
            min_pass_score: 0.9,
            impact_weight: 3.0,
        },
    ],
    dependencies: vec![CompositeDependencyEdge {
        dependency: "service_a".to_string(),
        dependent: "service_b".to_string(),
        failure_penalty: 0.2,
    }],
    global_min_pass_score: 0.85,
};

let result = evaluate_composite_slo(&graph).unwrap();
assert_eq!(result.services.len(), 2);
assert!(result
    .services
    .iter()
    .any(|entry| entry.service == "service_b" && entry.dependency_adjusted));
```

Python (native extension API):

```python
import neuralbudget

graph = neuralbudget.CompositeSloGraph(
    services=[
        neuralbudget.CompositeServiceSlo("service_a", 0.72, 0.9, 2.0),
        neuralbudget.CompositeServiceSlo("service_b", 0.97, 0.9, 3.0),
    ],
    dependencies=[
        neuralbudget.CompositeDependencyEdge("service_a", "service_b", 0.2),
    ],
    global_min_pass_score=0.85,
)

evaluation = neuralbudget.evaluate_composite_slo_graph(graph)
print(evaluation.global_slo, evaluation.global_pass)
```

## Convenience Layer Guide

Import path:

```python
from neuralbudget import convenience
```

Major convenience functions:

- `availability_snapshot(...)`
- `metric_stream(...)`
- `burn_rate_from_values(...)`
- `evaluate_http_histogram_once(...)`
- `evaluate_stateful_once(...)`
- `evaluate_ml_once(...)`
- `evaluate_genai_once(...)`

### Profile presets

Preset registries:

- `HTTP_PROFILE_PRESETS`
- `STATEFUL_PROFILE_PRESETS`
- `ML_PROFILE_PRESETS`
- `GENAI_PROFILE_PRESETS`

Lookup helpers:

- `get_http_profile_preset(name)`
- `get_stateful_profile_preset(name)`
- `get_ml_profile_preset(name)`
- `get_genai_profile_preset(name)`

### Dataclass return mode

All one-shot convenience evaluators support:

- `return_dataclass=False` (default): dictionary output
- `return_dataclass=True`: typed dataclass output

```python
from neuralbudget.convenience import evaluate_genai_once

typed = evaluate_genai_once(
    {
        "timestamp": 2,
        "tokens_generated": 260,
        "generation_duration_ms": 10000,
        "time_to_first_token_ms": 920,
        "reference_text": "Latency and quality should both be tracked.",
        "generated_text": "Track both quality and latency for LLM systems.",
    },
    profile="quality_first",
    return_dataclass=True,
)

print(typed.passed, typed.semantic_similarity)
```

## Examples

Runnable Python examples are in:

- [examples/python/availability_budget.py](examples/python/availability_budget.py)
- [examples/python/http_slo_histogram.py](examples/python/http_slo_histogram.py)
- [examples/python/stateful_slo.py](examples/python/stateful_slo.py)
- [examples/python/tiered_stateful_profiles.py](examples/python/tiered_stateful_profiles.py)
- [examples/python/ml_slo_drift_serving.py](examples/python/ml_slo_drift_serving.py)
- [examples/python/convenience_layer.py](examples/python/convenience_layer.py)

Run examples:

```bash
python3 examples/python/convenience_layer.py
python3 examples/python/ml_slo_drift_serving.py
```

## Development and CI/CD

### Local validation commands

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
python3 tests/python_convenience_tests.py
```

### Performance benchmarking

To compile and run performance benchmarks for composite DAG evaluation:

```bash
cargo bench --no-run
cargo bench composite_slo_dag
```

The benchmark target includes representative chain-graph sizes (`100`, `1_000`, `5_000`) and is intended for tracking evaluation cost trends as features evolve.

### Pipeline policy

- CI validates format, lint, tests, coverage gate, and packaging checks.
- CD re-validates and publishes artifacts on tagged release workflows.
- Coverage floor is enforced at 89% line coverage.

## PyPI Publishing

NeuralBudget publishes pre-built wheels for major platforms using:

- [`.github/workflows/cd.yml`](.github/workflows/cd.yml)

Release artifacts include:

- Linux (`manylinux`, `x86_64`)
- macOS (`x86_64` and `aarch64`)
- Windows (`x86_64`)
- source distribution (`sdist`)

### One-time repository setup

1. In PyPI, create a Trusted Publisher for this GitHub repository.
2. In GitHub, keep the workflow environment named `pypi`.
3. Ensure your project name on PyPI is `neuralbudget`.

### Release flow

1. Bump version in `pyproject.toml` and crate metadata as needed.
2. Create and push a tag like `v0.1.2`.
3. Publish a GitHub Release for that tag.
5. The `CD` workflow builds cross-platform wheels and uploads to PyPI.

## Changelog and Documentation

- Release history: [CHANGELOG.md](CHANGELOG.md)
- Documentation index: [docs/guides/documentation-index.md](docs/guides/documentation-index.md)
- Comprehensive user guide: [docs/guides/user-guide.md](docs/guides/user-guide.md)
- Convenience reference: [docs/reference/convenience-layer.md](docs/reference/convenience-layer.md)
- Composite DAG reference: [docs/reference/composite-slo-dag.md](docs/reference/composite-slo-dag.md)
- ML plan: [docs/plans/mlops-model-drift-serving-plan.md](docs/plans/mlops-model-drift-serving-plan.md)

## License

This repository is published under the NeuralBudget Source-Available License 1.0.
See [LICENSE](LICENSE) for terms.

