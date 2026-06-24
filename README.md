# NeuralBudget

[![Build Status](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/ci.yml)
[![CD Status](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml/badge.svg)](https://github.com/pristley/NeuralBudget/actions/workflows/cd.yml)
[![Latest Release](https://img.shields.io/github/v/release/pristley/NeuralBudget)](https://github.com/pristley/NeuralBudget/releases)
[![Crate Version](https://img.shields.io/badge/crate-v0.1.1-blue)](https://github.com/pristley/NeuralBudget/blob/main/Cargo.toml)
[![License](https://img.shields.io/badge/license-source--available-lightgrey)](LICENSE)

NeuralBudget is a Rust-first SLO foundation for availability, latency, and error-budget analysis with Python interoperability. It provides a small, deterministic core for service health calculations while keeping the data model simple enough for notebooks, pipelines, and operational tooling.

## At a Glance

- Rust library with `serde`-based models for config and telemetry data
- Python-facing wrappers built with `PyO3`
- Availability calculation helpers for classic SLI math
- JSON and YAML serialization support
- Lightweight, dependency-conscious design

## User Guide

### What this project is for

Use NeuralBudget when you want a compact core for SLO-related calculations without pulling in a large observability framework. The current scope is intentionally narrow:

- define SLO configuration objects
- serialize and deserialize core data structures
- calculate basic availability from good and total events
- expose the same logic to Python consumers

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
- `calculate_availability(success, total)`: classic SLI ratio, returned as a `float`

### Serialization

The core Rust models support JSON and YAML conversion through `serde` helpers. This makes the library suitable for:

- config files
- CLI input and output
- reproducible test fixtures
- Python-driven analytics workflows

## Releases

The project is currently at `v0.1.1`. That version represents the foundation layer: core models, serialization helpers, Python wrappers, the first availability calculation primitive, and the initial CI/CD pipeline polish.

Release artifacts and tags will appear in the GitHub Releases page as the project evolves.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history.

## Build Status

Continuous integration runs on GitHub Actions using [CI](.github/workflows/ci.yml) and [CD](.github/workflows/cd.yml).

- CI runs formatting, linting, and tests on every push and pull request targeting `main`.
- CD reruns validation on `main`, packages the crate, and publishes release artifacts for tagged builds.

The badges above reflect the current state of both workflows.

## Project Status

This repository is still in an early foundation phase. The current codebase is intentionally small so the statistical engine can grow from stable data contracts rather than ad hoc interfaces.

### Current Scope

- Rust data models for SLO configuration and metric points
- JSON and YAML support for the public structs
- Python bindings for ergonomic interop
- Classic availability calculation logic

### Near-Term Roadmap

1. Expand SLO calculations beyond the classic availability ratio.
2. Add latency and error-rate aggregation helpers.
3. Introduce richer release notes as new versions ship.
4. Add packaging support for Python distribution.

## Development

### Requirements

- Rust stable toolchain
- Cargo
- Python development headers if you plan to package or extend the PyO3 bindings

### Run Tests

```bash
cargo test
```

### CI/CD Flow

Every change should follow the same path that the repository automation enforces:

1. Push or open a pull request to `main`.
2. Let CI run `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-targets --all-features`.
3. Merge to `main` once checks pass.
4. Tag a release version such as `v0.1.1` to produce release artifacts through CD.

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
├── CHANGELOG.md
├── Cargo.toml
└── README.md
```

## License

This repository is published under the custom NeuralBudget Source-Available License 1.0. See [LICENSE](LICENSE) for the full terms.

