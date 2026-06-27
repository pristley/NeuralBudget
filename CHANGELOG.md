# Changelog

All notable changes to this project will be documented in this file.

Release entries are maintained automatically by the CD workflow on tagged releases.

## [Unreleased]

### Added

- **Command-Line Interface (CLI Tool)**
  - Added `neuralbudget` binary with 4 subcommands: `eval`, `gen-rules`, `check`, `serve`
  - `eval` subcommand: Evaluate SLO against metrics with human-readable and JSON output
  - `gen-rules` subcommand: Generate Prometheus alerting rules in YAML and Kubernetes CRD formats
  - `check` subcommand: Validate SLO configurations with strict mode and detailed error reporting
  - `serve` subcommand: Placeholder for HTTP server mode (planned for future release)
  - Multi-platform builds: Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), Windows
  - Docker support: Multi-stage Dockerfile for optimized binary distribution
  - Comprehensive CLI tests: 13 integration test scenarios in `tests/cli_integration_tests.rs`
  - CI/CD automation: `.github/workflows/cli-build.yml` for cross-platform builds and releases

- **Documentation Organization**
  - Created `docs/cli/` directory with all CLI documentation
  - Moved `agentmap.md` to root for easy architecture discovery
  - Created `docs/cli/USER_GUIDE.md` with installation, commands, and workflows
  - Created `docs/cli/DEVELOPMENT.md` with building, testing, and cross-compilation guide
  - Created `docs/cli/IMPLEMENTATION_SUMMARY.md` with feature matrix and status

- **Phase 3: Adaptive Streaming & Parallel SLO Evaluation**
  - Added `StreamingAggregator` struct with velocity-based adaptive windowing for high-frequency metric ingestion
  - Implements automatic buffer pruning at >15,000 samples/sec with 5-second retention window
  - VecDeque-based zero-allocation push operations with O(1) amortized complexity
  - Added `ParallelMetricBatch` struct for parallel, independent metric evaluation using Rayon work-stealing threads
  - Explicit GIL release via `py.allow_threads()` for true multi-threaded evaluation from Python
  - Comprehensive unit tests for streaming aggregation and parallel graph evaluation

### Changed

- **Documentation Consolidation (Google Standards Audit)**
  - Consolidated 11 Phase 3 documentation files into 3 focused guides with active voice and user-focused framing
  - Created `PHASE3_GETTING_STARTED.md` with task-based introduction and concrete examples
  - Created `PARALLEL_SLO_API_REFERENCE.md` with complete API documentation and integration examples
  - Created `DEPLOYMENT_GUIDE.md` with pre/during/post deployment checklists and troubleshooting
  - Updated `README.md` with Phase 3 documentation section and audience targeting
  - Removed redundant documentation: ADAPTIVE_WINDOWING_COMPLETE.md, ADAPTIVE_WINDOWING_DEPLOYMENT.md, ADAPTIVE_WINDOWING_SUMMARY.md, PARALLEL_SLO_EVALUATION.md, PHASE3_COMPLETE_SUMMARY.md

- **Code Quality & CI/CD**
  - Fixed cargo fmt formatting violations in src/streaming.rs
  - Fixed test logic in test_slo_node_evaluation for correct pass/fail assertions
  - Added Rayon 1.7 to dependencies for parallel computation support
  - All 19 Python tests passing locally; CI/CD pipeline operational

### Dependencies

- Added `rayon = "1.7"` for data-level parallelism in SLO graph evaluation

## [0.1.3-r2] - 2026-06-26

### Changed

- release: flatten downloaded dist files for PyPI publish

## [0.1.3] - 2026-06-26

### Changed

- chore(release): bump version to 0.1.3
- docs: add production deployment, Kubernetes, and Prometheus guides
- docs: detail release automation and refresh changelog
- ci(cd): integrate cross-platform PyPI publish into release workflow
- docs: update changelog for v0.1.2 [skip ci]

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

## [0.1.2] - 2026-06-26

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

## [0.1.1] - 2026-06-26

### Changed

- Added the `calculate_availability(success, total)` API to the Rust and Python surfaces.
- Added `TimeWindow` support for rolling and calendar-aligned SLO window calculations.
- Added CI and CD GitHub Actions workflows for formatting, linting, testing, packaging, and tagged releases.
- Added a dedicated integration test suite alongside the library unit tests.
- Refined the README into a more complete user guide with badges, release references, and build status.

## [0.1.0] - 2026-06-26

### Added

- Initial Rust-first SLO foundation for availability, latency, and error-budget modeling.
- `SloConfig`, `ErrorBudget`, and `MetricPoint` data models.
- JSON and YAML serialization helpers for the core structs.
- Python wrappers for the core data structures via `PyO3`.
- `calculate_availability(success, total)` for classic SLI math.
- Unit coverage for Rust and Python-backed availability calculations.