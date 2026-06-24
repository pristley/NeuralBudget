# NeuralBudget

[![Build Status](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml)
[![CD Status](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml)
[![Latest Release](https://img.shields.io/github/v/release/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![Release Tag](https://img.shields.io/github/v/tag/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![License](https://img.shields.io/badge/license-source--available-lightgrey)](LICENSE)

NeuralBudget is a Rust-first SLO foundation for availability, latency, and error-budget analysis with Python interoperability. It provides a deterministic core for stateless and stateful service health calculations while keeping data models simple enough for notebooks, pipelines, and operational tooling.

## At a Glance

- Rust library with `serde`-based models for config and telemetry data
- Python-facing wrappers built with `PyO3`
- Availability, latency, and budget calculations for web APIs and stateful systems
- JSON and YAML serialization support
- Iterator-first evaluation models for streaming telemetry

## User Guide

### What this project is for

Use NeuralBudget when you want a compact core for SLO-related calculations without pulling in a large observability framework. The current scope focuses on pragmatic building blocks:

- define SLO configuration objects
- serialize and deserialize core data structures
- evaluate HTTP/gRPC SLOs from histogram telemetry
- evaluate database and queue SLOs from stateful operational signals
- expose core logic to Python consumers

### Quick Start

#### Rust

```rust
use neuralbudget::calculate_availability;

let availability = calculate_availability(995, 1000);
assert_eq!(availability, 0.995);
```

#### Python

```python
import neuralbudget

availability = neuralbudget.calculate_availability(995, 1000)
print(availability)
```

### Core API

- `SloConfig`: target and evaluation window metadata
- `ErrorBudget`: remaining budget and burn velocity
- `MetricPoint`: timestamped observations with optional labels
- `WebApiRequest`: timestamped request metrics (`latency_ms`, `status_code`, labels)
- `WebApiSloPolicy`: policy for availability/latency targets, window size, and outlier filtering
- `WebApiSloReport`: complete report including availability, latency SLI, burn rates, and budget
- `HistogramBucket`, `HistogramSample`, and `HistogramFormat`: histogram telemetry structures for stateless SLO checks
- `HttpSlo` and `HttpSloIterator`: p99 latency + availability pass/fail evaluator for HTTP/gRPC streams
- `StatefulSample`, `StatefulSlo`, and `StatefulSloIterator`: stateful database/queue SLO evaluation with connection-wait penalties
- `StatefulSloEvaluation`: per-sample output with gate status, score, and pass/fail
- `calculate_availability(success, total)`: classic SLI ratio, returned as a `float`
- `calculate_error_budget(slo_target, window_secs)`: remaining budget in seconds for an SLO target and time window
- `calculate_burn_rate(metric_stream, window_secs)`: normalized burn rate for a stream of budget-consuming samples
- `calculate_mad(values)`: Median Absolute Deviation for robust spike detection
- `filter_statistical_outliers(metric_stream, mad_threshold, min_samples)`: configurable outlier filtering
- `calculate_web_api_slo(requests, policy, now)`: end-to-end SLO calculation for web API streams

### Serialization

The core Rust models support JSON and YAML conversion through `serde` helpers. This makes the library suitable for:

- config files
- CLI input and output
- reproducible test fixtures
- Python-driven analytics workflows

## Time Window Calculus

NeuralBudget supports two SLO window styles:

- Rolling windows, which measure backward from the current evaluation time.
- Calendar-aligned windows, which snap to fixed boundaries in a timezone-aware local clock.

The current implementation uses `TimeWindow::contains(timestamp, now)` in Rust and `neuralbudget.is_timestamp_in_window(...)` in Python. Calendar-aligned windows accept a timezone offset in seconds so the same logic works across UTC and local schedules.

Example:

```rust
use neuralbudget::TimeWindow;

let rolling = TimeWindow::rolling(3_600);
assert!(rolling.contains(1_699_999_999, 1_700_000_000));

let calendar = TimeWindow::calendar_aligned(86_400, 18_000);
assert!(calendar.contains(68_400, 90_000));
```

## Error Budget and Burn Rate

The budget formula is the SLO target gap multiplied by the time window in seconds:

```rust
use neuralbudget::calculate_error_budget;

let budget = calculate_error_budget(0.999, 3_600);
assert_eq!(budget, 3.6);
```

Burn rate works over a stream of timestamped samples. In this repository, samples with a value above `0.0` are treated as budget-consuming events. That makes it easy to compare the last 5 minutes against the last hour by calling `calculate_burn_rate(metric_stream, 300)` and `calculate_burn_rate(metric_stream, 3_600)`.

## Web API SLO Framework

NeuralBudget now includes a generic web API SLO framework for request-level streams.

- Availability uses successful requests (`status_code < 500`) over total requests.
- Latency SLI uses `latency_threshold_ms` with optional MAD-based outlier filtering.
- Error budget uses `calculate_error_budget` over the configured window.
- Burn rates are reported for both 5-minute and 1-hour windows.

Example:

```rust
use neuralbudget::{
	calculate_web_api_slo, OutlierFilterConfig, WebApiRequest, WebApiSloPolicy,
};

let requests = vec![
	WebApiRequest {
		timestamp: 1,
		latency_ms: 120.0,
		status_code: 200,
		labels: Default::default(),
	},
	WebApiRequest {
		timestamp: 2,
		latency_ms: 4000.0,
		status_code: 200,
		labels: Default::default(),
	},
];

let policy = WebApiSloPolicy {
	availability_target: 0.99,
	latency_threshold_ms: 250.0,
	time_window_seconds: 60,
	outlier_filter: OutlierFilterConfig {
		enabled: true,
		mad_threshold: 3.5,
		min_samples: 2,
	},
};

let report = calculate_web_api_slo(&requests, &policy, 2);
assert!(report.total_requests >= 1);
```

## Stateless HTTP/gRPC SLO (Histogram Iterator)

`HttpSlo` evaluates each histogram sample with two gates:

- latency gate: p99 latency must be below `200ms` (or your configured threshold)
- availability gate: success rate must be above `99.9%` (or your configured threshold)

Each sample passes only when both gates pass.

The iterator accepts both histogram modes:

- `prometheus_cumulative`: cumulative bucket counts
- `open_telemetry_delta`: per-bucket delta counts

Example:

```rust
use neuralbudget::{
	HistogramBucket, HistogramFormat, HistogramSample, HttpSlo, HttpSloIterator,
};

let slo = HttpSlo::default();
let stream = vec![HistogramSample {
	timestamp: 1,
	success: 9_995,
	total: 10_000,
	buckets: vec![
		HistogramBucket {
			upper_bound_ms: 100.0,
			count: 9_700,
		},
		HistogramBucket {
			upper_bound_ms: 150.0,
			count: 200,
		},
		HistogramBucket {
			upper_bound_ms: 220.0,
			count: 100,
		},
	],
	format: HistogramFormat::OpenTelemetryDelta,
}];

let evaluations: Vec<_> = HttpSloIterator::new(slo, stream.into_iter()).collect();
assert_eq!(evaluations.len(), 1);
assert!(evaluations[0].pass);
```

## Stateful Database/Queue SLO

`StatefulSlo` evaluates sample streams using four signals:

- replication lag
- queue depth
- connection pool saturation
- connection wait time

If connection wait time exceeds the configured threshold, a score penalty is applied before pass/fail is decided.

Default policy behavior:

- replication lag must be less than or equal to `250ms`
- queue depth must be less than or equal to `1000`
- connection pool saturation must be less than or equal to `0.8`
- connection wait time above `20ms` reduces score using `connection_wait_penalty_weight`

Example:

```rust
use neuralbudget::{StatefulSample, StatefulSlo, StatefulSloIterator};

let slo = StatefulSlo::default();
let samples = vec![
	StatefulSample {
		timestamp: 1,
		replication_lag_ms: 180.0,
		queue_depth: 700,
		connection_pool_saturation: 0.7,
		connection_wait_time_ms: 8.0,
	},
	StatefulSample {
		timestamp: 2,
		replication_lag_ms: 200.0,
		queue_depth: 800,
		connection_pool_saturation: 0.75,
		connection_wait_time_ms: 60.0,
	},
];

let evaluations: Vec<_> = StatefulSloIterator::new(slo, samples.into_iter()).collect();
assert!(evaluations[0].pass);
assert!(evaluations[1].connection_wait_penalized);
assert!(!evaluations[1].pass);
```

## Releases

The project is currently at `v0.1.1`. That version represents the foundation layer: core models, serialization helpers, Python wrappers, the first availability calculation primitive, and the initial CI/CD pipeline polish.

Release artifacts and tags will appear in the GitHub Releases page as the project evolves.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history. The CD workflow keeps this file synchronized for tagged releases.

## Build Status

Continuous integration runs on GitHub Actions using [CI](.github/workflows/ci.yml) and [CD](.github/workflows/cd.yml).

- CI runs formatting, linting, and tests on every push and pull request targeting `main`.
- CD reruns validation on `main`, packages the crate, and publishes release artifacts for tagged builds.
- Both pipelines also run `cargo llvm-cov` and enforce a 90% line-coverage floor.

The badges above reflect the current state of both workflows.

## Coverage Policy

NeuralBudget uses line coverage as a practical quality gate instead of pretending every private branch and wrapper shim must be driven to 100%.

- Coverage is measured with `cargo llvm-cov --workspace --all-features`.
- CI/CD fail if total line coverage drops below 90%.
- The policy intentionally treats PyO3 wrapper glue and similar boilerplate as normal maintenance code, not as a reason to chase zero-value tests for every branch.
- The current validated coverage is above the enforced floor, so future changes must stay above that bar.

## Project Status

This repository is still in an early foundation phase. The current codebase is intentionally small so the statistical engine can grow from stable data contracts rather than ad hoc interfaces.

### Current Scope

- Rust data models for SLO configuration and telemetry samples
- JSON and YAML support for the public structs
- Python bindings for ergonomic interop
- Classic availability, error budget, burn-rate, and web API SLO calculations
- Stateless histogram-based `HttpSlo` iterator for HTTP/gRPC pass-fail evaluation
- Stateful database/queue `StatefulSlo` iterator with connection-wait penalization
- Weighted policy profiles for database and queue tiers

### Near-Term Roadmap

1. Add packaging support for Python distribution.

## Development

### Requirements

- Rust stable toolchain
- Cargo
- Python development headers if you plan to package or extend the PyO3 bindings

### Run Tests

```bash
cargo test
```

The repository keeps test coverage in three tiers:

- unit tests in the library source
- integration tests in [tests/integration_tests.rs](tests/integration_tests.rs)
- functional tests in [tests/functional_tests.rs](tests/functional_tests.rs)

CI and CD run all three tiers separately.

### CI/CD Flow

Every change should follow the same path that the repository automation enforces:

1. Push or open a pull request to `main`.
2. Let CI run formatting and linting, then execute unit, integration, and functional test suites.
3. Merge to `main` once checks pass.
4. Let CD re-run validation, package artifacts, and publish tagged releases.

Equivalent local commands:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --all-features
cargo test --doc --all-features
cargo test --tests --all-features
```

### Formatting and Linting

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
```

## Repository Layout

```text
.
├── .github/
│   └── workflows/
│       ├── cd.yml
│       └── ci.yml
├── src/
│   └── lib.rs
├── tests/
│   ├── functional_tests.rs
│   └── integration_tests.rs
├── CHANGELOG.md
├── Cargo.toml
└── README.md
```

## License

This repository is published under the custom NeuralBudget Source-Available License 1.0. See [LICENSE](LICENSE) for the full terms.

