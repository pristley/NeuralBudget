# NeuralBudget

NeuralBudget is a Rust-first foundation for Service Level Objective engineering across software, MLOps, and AI systems. The project is focused on building a dependency-light statistical core that can calculate availability, latency, and error-budget health while exposing a clean interface to Python consumers.

## Current Scope

The repository currently provides the Day 1 foundation for the statistical engine:

- `SloConfig` for SLO target and evaluation window configuration
- `ErrorBudget` for remaining budget and burn velocity tracking
- `MetricPoint` for timestamped measurements with optional labels
- JSON and YAML serialization helpers using `serde`
- Python binding entrypoints using `PyO3` so Python callers can pass dictionaries into Rust-backed types

This is the backbone layer only. Calculation routines for availability, latency, and error-rate rollups are the next logical step.

## Why This Exists

Most SLO stacks are designed around traditional web services and assume a single runtime, a single telemetry shape, or heavyweight dependencies. NeuralBudget is intended to stay small and predictable:

- Rust for deterministic, high-performance model and math code
- Minimal external dependencies for a tighter operational surface
- Python interop for notebooks, analytics pipelines, and ML-adjacent workflows
- Plain JSON and YAML support for easy integration with config files and APIs

## Project Layout

```text
.
├── .github/
│   └── workflows/
│       ├── cd.yml
│       └── ci.yml
├── src/
│   └── lib.rs
├── Cargo.toml
└── README.md
```

## Core Models

### `SloConfig`

Represents the target objective and evaluation window.

```rust
pub struct SloConfig {
		pub target: f64,
		pub window: String,
}
```

Example:

```json
{
	"target": 99.9,
	"window": "30d"
}
```

### `ErrorBudget`

Represents the remaining error budget and the rate at which it is being consumed.

```rust
pub struct ErrorBudget {
		pub remaining: f64,
		pub velocity: f64,
}
```

Example:

```yaml
remaining: 0.42
velocity: 1.7
```

### `MetricPoint`

Represents a timestamped metric sample with optional labels.

```rust
pub struct MetricPoint {
		pub timestamp: i64,
		pub value: f64,
		pub labels: HashMap<String, String>,
}
```

Example:

```json
{
	"timestamp": 1719220000,
	"value": 0.998,
	"labels": {
		"service": "inference",
		"region": "use1"
	}
}
```

If `labels` is omitted during deserialization, it defaults to an empty map.

## Serialization Support

All core models derive `Serialize` and `Deserialize` via `serde`. The library currently exposes helper methods for:

- parsing from JSON strings
- serializing to JSON strings
- parsing from YAML strings
- serializing to YAML strings

That makes the models usable from config files, CLI inputs, test fixtures, and service integrations without additional translation code.

## Python Interop

The crate uses `PyO3` to define Python-facing wrapper types:

- `PySloConfig`
- `PyErrorBudget`
- `PyMetricPoint`

These wrappers support:

- direct construction from typed arguments
- `from_dict(...)` conversion from Python dictionaries
- `to_dict()` conversion back into Python-native structures
- `to_json()` and `to_yaml()` export helpers

The Rust extraction layer also supports Python dictionaries being coerced directly into the Rust structs for future binding functions.

Illustrative Python usage:

```python
payload = {
		"target": 99.95,
		"window": "7d",
}

config = SloConfig.from_dict(payload)
print(config.to_json())
```

## Local Development

### Prerequisites

- Rust stable toolchain
- Cargo
- Python development headers if you plan to extend or package the `PyO3` bindings

### Install Rust

If Rust is not already installed:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y
. "$HOME/.cargo/env"
```

### Build and Test

```bash
cargo test
```

At the moment, the crate is configured as an `rlib` so the model layer and binding definitions can be compiled and tested reliably in environments where Python embedding is not available as a shared library.

## CI/CD

This repository includes two GitHub Actions workflows:

### CI

File: `.github/workflows/ci.yml`

Runs on pushes and pull requests targeting `main` and performs:

- repository checkout
- Rust toolchain setup
- dependency fetch
- formatting verification with `cargo fmt --check`
- linting with `cargo clippy`
- test execution with `cargo test`

This is the gate that keeps the foundation compileable and reviewable.

### CD

File: `.github/workflows/cd.yml`

Runs on pushes to `main` after CI-relevant checks and performs:

- repository checkout
- Rust toolchain setup
- test verification
- `cargo package --allow-dirty --no-verify` to assemble a distributable source package
- artifact upload of the generated package metadata

This is a lightweight delivery pipeline suitable for the current repository stage. Once there is a real deployment target, Python wheel packaging, crate publishing, or release automation can be layered on top of it without replacing the CI baseline.

## Roadmap

Near-term priorities:

1. Parse and validate SLO windows into typed time intervals.
2. Implement availability, latency, and error-rate aggregation logic.
3. Add burn-rate and multi-window error budget calculations.
4. Introduce packaging for Python extension distribution.
5. Add benchmark coverage for high-volume metric ingestion.

## Repository Status

This repository is still in its early foundation phase. The current code is intentionally small and focused so that the statistical engine can evolve from stable data contracts rather than ad hoc interfaces.

## License

This repository is published under the custom NeuralBudget Source-Available License 1.0. The source is publicly available, but organizational use requires prior written permission from the copyright holder.

In practical terms:

- individuals may review, fork, and modify the code for personal, educational, and evaluation purposes
- organizations may not use, deploy, modify, redistribute internally, or incorporate this project into business or institutional workflows without permission

See the full terms in the LICENSE file.

