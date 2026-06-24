# Changelog

All notable changes to this project will be documented in this file.

Release entries are maintained automatically by the CD workflow on tagged releases.

## [0.1.2] - 2026-06-24

### Changed

- chore(release): bump version to 0.1.2
- ci(pypi): publish cross-platform wheels via GitHub release workflow
- docs: add comprehensive user guide and improve client API docs
- feat(python): add NeuralBudgetClient facade for notebook and CI workflows
- docs: refresh README badges and project summary
- fix: apply rustfmt to satisfy CI/CD format checks
- ci: make coverage gate deterministic for lib and tests
- feat: add composite DAG python API, benchmarking, and docs refresh
- feat: add composite DAG SLO runner with global scoring
- feat: add GenAI convenience helper and first-class user guide docs
- feat: add convenience dataclass returns, presets, tests, and pipeline gates
- docs: add detailed documentation index and convenience reference
- feat: add MlSlo hybrid drift-serving SLO and pipeline coverage
- docs(readme): expand professional Python user guide
- feat(python): add convenience layer for one-shot SLO workflows
- docs: professionalize release notes and refresh badges
- Add Python examples for SLO workflows
- Modularize lib.rs into core and python modules
- Document wheel build in README
- Add Python wheel packaging support
- Add weighted stateful policy profiles
- Enforce coverage gate and update docs
- Add StatefulSlo evaluation and refresh project documentation
- Expand HttpSlo test coverage and align CI/CD docs
- Add web API SLO framework with MAD outlier filtering
- Update README for budget and functional pipeline
- Add budget algorithms and full test tiers
- Automate release notes and badges
- Document time windows and test coverage
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
- Added a small pure-Python convenience layer (`neuralbudget.convenience`) for dictionary-oriented snapshots and one-shot SLO evaluations.
- Added GenAI convenience support with `evaluate_genai_once`, `GenAiSloProfile`, and `GENAI_PROFILE_PRESETS`.
- Added convenience exports for GenAI helper and preset lookup via `get_genai_profile_preset`.
- Added Composite SLO DAG support with `CompositeSloGraph`, `CompositeDependencyEdge`, `evaluate_composite_slo`, and weighted System Global SLO output.
- Added dependency-impact propagation so downstream services are automatically adjusted and flagged when upstream dependencies fail.
- Added PyO3 Composite DAG bindings with `CompositeServiceSlo`, `CompositeDependencyEdge`, `CompositeSloGraph`, `CompositeServiceSloEvaluation`, `CompositeSloEvaluation`, and `evaluate_composite_slo_graph(...)`.
- Added Criterion benchmark target `benches/composite_slo_dag.rs` for large DAG evaluations (`cargo bench`).

### Documentation

- Expanded README into a detailed first-class user guide with:
	- complete installation paths for Rust and Python
	- end-to-end examples for HTTP, Stateful, ML, GenAI, and Composite DAG SLO workflows
	- convenience layer usage for presets and dataclass return mode
	- explicit CI/CD and coverage policy sections
	- direct changelog and documentation navigation badges/links
- Added dedicated Composite DAG reference docs at `docs/reference/composite-slo-dag.md` and linked it from the docs index and README.

### Changed

- CI/CD now run documentation tests via `cargo test --doc --all-features`.
- CI/CD now run explicit all-target unit tests via `cargo test --all-targets --all-features` in addition to library and integration suites.
- CI/CD continue to enforce a practical 89% line-coverage floor with `cargo llvm-cov` after modular coverage accounting changes.
- README release section now includes a professional `v0.1.1` release-notes summary and refreshed pipeline documentation.
- Release-note automation now emits categorized and more readable notes for tagged releases.
- Composite DAG evaluator refactored into modular internal stages (graph indexing, deterministic topological ordering, service evaluation, global aggregation).
- Composite DAG traversal now has deterministic ordering and duplicate-edge validation for reproducible outputs.

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