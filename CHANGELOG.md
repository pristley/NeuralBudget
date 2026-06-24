# Changelog

All notable changes to this project will be documented in this file.

Release entries are maintained automatically by the CD workflow on tagged releases.

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