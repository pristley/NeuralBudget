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

## [Unreleased]

### Added

- Added production deployment guide at `docs/guides/production-deployment.md` covering deployment topology, runtime operations, alerting, and troubleshooting.
- Added Kubernetes example manifests under `examples/kubernetes/` for ConfigMap, Deployment, and Service wiring.
- Added Prometheus integration examples under `examples/kubernetes/` for ServiceMonitor (Operator) and additional scrape configs (vanilla Prometheus).

### Changed

- Integrated cross-platform wheel builds directly into CD release path (`.github/workflows/cd.yml`) for Linux, Windows, and macOS.
- Integrated PyPI trusted publishing into CD using OIDC (`id-token: write`) and environment `pypi`.
- Consolidated release automation by removing standalone `.github/workflows/pypi-release.yml` and using CD as the single release orchestration workflow.

### Documentation

- Updated README with release automation details, trusted publisher prerequisites, and corrected release flow steps.
- Updated user guide with a detailed release and distribution automation section and trusted publisher checklist.
- Updated documentation index to include changelog-first release history navigation and refreshed CI/CD pointers.
- Expanded README, user guide, and documentation index with production deployment navigation and Kubernetes/Prometheus example references.

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