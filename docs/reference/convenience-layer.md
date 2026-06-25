# Convenience Layer Reference

This document describes the Python convenience API in detail, including typed dataclass results and profile presets.

## Design Goals

- Keep call sites ergonomic for notebooks and pipelines.
- Delegate numerical logic to the Rust-backed native module.
- Preserve backward compatibility by returning dictionaries by default.
- Offer optional typed return values for stronger editor and runtime ergonomics.

## Import Surface

Primary module:

- neuralbudget.convenience

Re-exported through package root:

- neuralbudget

## Result Models

Set return_dataclass=True to receive these dataclasses.

### AvailabilitySnapshotResult

Fields:

- success: int
- total: int
- availability: float
- slo_target: float
- window_secs: int
- error_budget_seconds: float
- target_met: bool

### HttpHistogramEvaluationResult

Fields:

- timestamp: int
- availability: float
- percentile_latency_ms: float
- evaluated_percentile: float
- latency_pass: bool
- availability_pass: bool
- passed: bool

### StatefulEvaluationResult

Fields:

- timestamp: int
- score: float
- replication_lag_ok: bool
- queue_depth_ok: bool
- connection_pool_ok: bool
- connection_wait_penalized: bool
- passed: bool

### MlEvaluationResult

Fields:

- timestamp: int
- inference_latency_score: float
- gpu_utilization_score: float
- system_score: float
- latency_score: float
- feature_drift_score: float
- prediction_confidence_score: float
- drift_score: float
- latency_weight: float
- drift_weight: float
- hybrid_score: float
- passed: bool

### GenAiEvaluationResult

Fields:

- timestamp: int
- tokens_per_second: float
- time_to_first_token_ms: float
- semantic_similarity: float
- tokens_per_second_ok: bool
- time_to_first_token_ok: bool
- semantic_similarity_ok: bool
- passed: bool

## Profile Models

Profile objects provide named policy defaults while still allowing per-call overrides.

### HttpSloProfile

Fields:

- latency_threshold_ms
- latency_percentile
- availability_threshold

### StatefulSloProfile

Fields:

- replication_lag_threshold_ms
- queue_depth_threshold
- connection_pool_saturation_threshold
- connection_wait_time_threshold_ms
- connection_wait_penalty_weight
- min_pass_score

### MlSloProfile

Fields:

- max_inference_latency_ms
- max_gpu_utilization
- max_feature_drift
- min_prediction_confidence
- latency_weight
- drift_weight
- min_pass_score

### GenAiSloProfile

Fields:

- min_tokens_per_second
- max_time_to_first_token_ms
- min_semantic_similarity
- semantic_model_name

## Built-in Presets

### HTTP_PROFILE_PRESETS

- default
- strict_latency
- availability_first

### STATEFUL_PROFILE_PRESETS

- default
- database_primary
- queue_hot_path

### ML_PROFILE_PRESETS

- default
- latency_critical
- drift_sensitive

### GENAI_PROFILE_PRESETS

- default
- latency_first
- quality_first

## Preset Lookup Helpers

- get_http_profile_preset(name)
- get_stateful_profile_preset(name)
- get_ml_profile_preset(name)
- get_genai_profile_preset(name)

Behavior:

- Unknown names raise ValueError.
- Passing None to a profile argument resolves to the default preset.

## Function Reference

### availability_snapshot

Purpose:

- Compute availability and error budget summary.

Key parameters:

- success
- total
- slo_target
- window_secs
- return_dataclass

Returns:

- dict by default, AvailabilitySnapshotResult if return_dataclass=True.

### evaluate_http_histogram_once

Purpose:

- Evaluate one histogram payload for latency and availability gates.

Key parameters:

- sample
- latency_threshold_ms
- latency_percentile
- availability_threshold
- profile
- return_dataclass

Precedence:

- Explicit parameter values override profile values.
- Missing explicit parameters fall back to profile values.

Returns:

- dict by default, HttpHistogramEvaluationResult if return_dataclass=True.

### evaluate_stateful_once

Purpose:

- Evaluate one stateful service sample (db/queue style metrics).

Key parameters:

- sample
- threshold and penalty arguments
- profile
- return_dataclass

Precedence:

- Explicit parameter values override profile values.

Returns:

- dict by default, StatefulEvaluationResult if return_dataclass=True.

### evaluate_ml_once

Purpose:

- Evaluate one ML serving sample with hybrid system + data scoring.

Key parameters:

- sample
- threshold and hybrid weight arguments
- profile
- return_dataclass

Precedence:

- Explicit parameter values override profile values.

Returns:

- dict by default, MlEvaluationResult if return_dataclass=True.

### evaluate_genai_once

Purpose:

- Evaluate one GenAI serving sample for throughput, TTFT, and semantic similarity.

Key parameters:

- sample
- min_tokens_per_second
- max_time_to_first_token_ms
- min_semantic_similarity
- semantic_model_name
- profile
- return_dataclass

Precedence:

- Explicit parameter values override profile values.

Returns:

- dict by default, GenAiEvaluationResult if return_dataclass=True.

## Client Facade Integration

If you want config-driven execution, use `NeuralBudgetClient` from `neuralbudget`.

Typical flow:

1. `client.load_config("slo.yaml")`
2. `client.evaluate(metric_data)`

Use convenience functions directly when:

- you want one-shot calls without external config files
- you are composing custom Python orchestration logic

Use `NeuralBudgetClient` when:

- you want a stable config contract across notebook, CI, and service workflows
- you want schema validation and mode selection from JSON/YAML

## Usage Patterns

### Backward-compatible dictionary flow

Use defaults and dictionary output for quick scripts.

### Typed workflow flow

Use return_dataclass=True when:

- You want attribute access instead of dictionary keys.
- You want clearer type hints and safer refactors.

### Profile-driven operations flow

Use profile names when:

- Teams need consistent SLO policy baselines.
- You want simple switching between policy modes.

Use explicit overrides when:

- You need temporary adjustments without redefining a profile.

## Testing Strategy

The convenience layer test file uses a native-module stub to keep tests fast and deterministic without requiring a compiled extension.

Test file:

- tests/python_convenience_tests.py

Coverage includes:

- Dataclass return behavior
- Preset retrieval and override behavior
- Unknown preset error paths

## Minimal GenAI Example

```python
from neuralbudget.convenience import evaluate_genai_once

result = evaluate_genai_once(
    {
        "timestamp": 1,
        "tokens_generated": 420,
        "generation_duration_ms": 14000,
        "time_to_first_token_ms": 850,
        "reference_text": "NeuralBudget is deterministic.",
        "generated_text": "NeuralBudget provides deterministic reliability checks.",
    },
    profile="default",
)

print(result["tokens_per_second"], result["semantic_similarity"], result["pass"])
```
