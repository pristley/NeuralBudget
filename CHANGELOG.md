# Changelog

All notable changes to this project will be documented in this file.

Release entries are maintained automatically by the CD workflow on tagged releases.

## [Unreleased]

### Added

- Core error budget and burn-rate helpers for SLO analysis.
- `calculate_error_budget(slo_target, time_window_seconds)` for the absolute budget in seconds.
- `calculate_burn_rate(metric_stream, window_secs)` for 5-minute and 1-hour burn-rate comparisons.
- `calculate_mad(values)` for robust Median Absolute Deviation calculations.
- `filter_statistical_outliers(metric_stream, mad_threshold, min_samples)` for configurable anomaly filtering.
- Web API SLO framework with `WebApiRequest`, `WebApiSloPolicy`, `WebApiSloReport`, and `calculate_web_api_slo`.
- Stateless HTTP/gRPC SLO histogram evaluation with `HttpSlo`, `HttpSloIterator`, `HistogramSample`, and `HistogramBucket`.
- Histogram percentile evaluation support for both Prometheus cumulative and OpenTelemetry delta bucket formats.
- Stateful software SLO evaluation with `StatefulSlo`, `StatefulSample`, `StatefulSloEvaluation`, and `StatefulSloIterator`.
- Stateful scoring model for replication lag, queue depth, connection pool saturation, and connection-wait-time penalties.
- Weighted policy profiles for database and queue tiers through `StatefulPolicyProfile` and `StatefulPolicyProfileSet`.
- Python wheel packaging support with `pyproject.toml` and `maturin` for the Rust-backed `neuralbudget` module.
- Modularized the Rust library by splitting `src/lib.rs` into `src/core.rs`, `src/python.rs`, and `src/tests.rs` with a thin re-export facade.
- Added Python example scripts for availability/budget primitives, stateless HTTP histogram SLOs, and stateful database/queue SLO flows.
- Added coverage-gate and Rust version badges to the README for clearer project health signaling.

### Changed

- CI/CD now run documentation tests via `cargo test --doc --all-features`.
- CI/CD now run explicit all-target unit tests via `cargo test --all-targets --all-features` in addition to library and integration suites.
- CI/CD continue to enforce a practical 89% line-coverage floor with `cargo llvm-cov` after modular coverage accounting changes.
- README release section now includes a professional `v0.1.1` release-notes summary and refreshed pipeline documentation.
- Release-note automation now emits categorized and more readable notes for tagged releases.

## [0.1.1] - 2026-06-24

### Changed

- Added the `calculate_availability(success, total)` API to the Rust and Python surfaces.
- Added `TimeWindow` support for rolling and calendar-aligned SLO window calculations.
- Added CI and CD GitHub Actions workflows for formatting, linting, testing, packaging, and tagged releases.
- Added a dedicated integration test suite alongside the library unit tests.
- Refined the README into a more complete user guide with badges, release references, and build status.

## [0.1.0] - 2026-06-24

### Added

- Initial Rust-first SLO foundation for availability, latency, and error-budget modeling.
- `SloConfig`, `ErrorBudget`, and `MetricPoint` data models.
- JSON and YAML serialization helpers for the core structs.
- Python wrappers for the core data structures via `PyO3`.
- `calculate_availability(success, total)` for classic SLI math.
- Unit coverage for Rust and Python-backed availability calculations.