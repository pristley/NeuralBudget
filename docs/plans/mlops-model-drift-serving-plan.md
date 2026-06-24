# MLOps Feature Plan: Model Drift and Serving SLO (`MlSlo`)

## Objective
Implement a hybrid SLO model for ML serving that combines system metrics and data-quality metrics into a single score with configurable weights.

Formula:

- `SLO_Score = (Latency_Score * 0.6) + (Drift_Score * 0.4)`
- Expose weight system to allow runtime policy tuning.

## Scope
- Add new Rust core model: `MlSlo`
- Add Rust telemetry sample and evaluation output models
- Add stream iterator support
- Expose API through PyO3 native module
- Add Python convenience helper
- Add unit and integration tests
- Add CI/CD job stages specifically validating MLOps hybrid SLO behavior

## Detailed Tasks

1. Core data model tasks
- Add `MlSample` with fields:
  - `timestamp`
  - `inference_latency_ms`
  - `gpu_utilization`
  - `feature_drift`
  - `prediction_confidence`
- Add `MlSlo` policy with thresholds and weights:
  - `max_inference_latency_ms`
  - `max_gpu_utilization`
  - `max_feature_drift`
  - `min_prediction_confidence`
  - `latency_weight`
  - `drift_weight`
  - `min_pass_score`
- Add `MlSloEvaluation` output model with component scores and hybrid score.

2. Scoring logic tasks
- Define per-signal normalization:
  - Lower-is-better signals (`latency`, `gpu_utilization`, `feature_drift`) score into `[0, 1]`.
  - Higher-is-better signal (`prediction_confidence`) score into `[0, 1]`.
- Build aggregate dimensions:
  - `Latency_Score` (system) from latency and GPU sub-scores.
  - `Drift_Score` (data) from feature-drift and confidence sub-scores.
- Apply exposed hybrid weights:
  - Normalize weights to avoid invalid sums.
  - Fallback to `0.6/0.4` when both weights are invalid.
- Apply pass/fail gate against configurable `min_pass_score`.

3. API and serialization tasks
- Add `JsonYamlExt` coverage for new structs.
- Add `MlSloIterator` for stream evaluation.
- Keep deterministic behavior across Rust and Python surfaces.

4. Python binding tasks
- Implement `FromPyObject` for `MlSample` and `MlSlo`.
- Add pyclasses:
  - `MlSample`
  - `MlSlo`
  - `MlSloEvaluation`
- Add pyfunctions:
  - `evaluate_ml_slo`
  - `evaluate_ml_slo_stream`
  - `coerce_ml_sample`
  - `coerce_ml_slo`
- Register all symbols in module init.

5. Python convenience tasks
- Add `evaluate_ml_once(sample, ...)` helper that:
  - accepts dictionary input
  - configures thresholds and weights
  - returns plain dictionary output for notebook and pipeline usage

6. Unit test tasks
- Add formula validation tests for exact weighted composition.
- Add weight normalization tests (non-unit sums and invalid values).
- Add edge-case tests for invalid thresholds and score clamping.
- Extend existing wrapper tests to include all new pyclass and pyfunction methods.

7. Integration test tasks
- Add end-to-end tests for:
  - default formula behavior
  - custom weight normalization
  - iterator pass/fail sequencing
  - JSON/YAML round-trip serialization

8. CI/CD tasks
- Add MLOps-focused test stage in CI and CD:
  - run `cargo test --all-features ml_slo -- --nocapture`
- Keep existing linting, formatting, full test, coverage, and wheel build gates.

## Acceptance Criteria
- `MlSlo` exists and is public in Rust API.
- Hybrid score follows weighted formula with exposed tunable weights.
- Rust and Python bindings produce consistent `hybrid_score` and `pass` fields.
- Dedicated unit and integration tests cover formula, normalization, and edge conditions.
- CI and CD workflows include explicit MLOps-targeted tests.

## Future Improvements (Backlog)
- Add optional EWMA smoothing for noisy feature drift.
- Add calibration score support (ECE/Brier) as additional data metric.
- Add configurable composition operators (weighted min, geometric mean).
- Add Prometheus/OpenTelemetry exporters for `MlSloEvaluation` fields.
