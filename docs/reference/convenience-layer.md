# Convenience Layer Reference

This document describes the Python convenience API in detail, including result dictionary schemas and profile presets.

## Design Goals

- Keep call sites ergonomic for notebooks and pipelines.
- Delegate numerical logic to the Rust-backed native module.
- Return dictionaries with consistent field names across all modes.

## Import Surface

Primary module:

- neuralbudget.convenience

Re-exported through package root:

- neuralbudget

## Result Models

All convenience functions return dictionaries with the following keys:

### AvailabilitySnapshotResult keys

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

### `get_http_profile_preset(name: str) -> HttpSloProfile`
### `get_stateful_profile_preset(name: str) -> StatefulSloProfile`
### `get_ml_profile_preset(name: str) -> MlSloProfile`
### `get_genai_profile_preset(name: str) -> GenAiSloProfile`

Get profile parameters by name.

**Parameters:**
- `name` (str) — Profile name (e.g., "default", "strict_latency", "aggressive")

**Return Type:**
- Profile dataclass with fields for that SLO mode (see Profile Models above)

**Preconditions:**
- `name` must match one of the built-in presets (see Built-in Presets section)
- Passing `None` resolves to "default" preset

**Raises:**
- `ValueError` — Unknown profile name (not in built-in presets)
- `KeyError` — Profile not found in lookup table

**Examples:**
```python
from neuralbudget import get_http_profile_preset

# Get a preset
profile = get_http_profile_preset("aggressive")
print(f"Latency threshold: {profile.latency_threshold_ms}ms")
print(f"Availability: {profile.availability_threshold:.1%}")

# Error handling
try:
    profile = get_http_profile_preset("unknown_profile")
except ValueError as e:
    print(f"ERROR: {e}")  # "Unknown profile: unknown_profile"

# Use with convenience functions
result = evaluate_http_histogram_once(
    sample={...},
    profile="strict_latency"  # Pass name directly
)
```

**Behavior:**

- Unknown names raise ValueError with helpful message
- Passing None to a profile argument resolves to the default preset

## Function Reference

### availability_snapshot

Purpose:

- Compute availability and error budget summary.

**Parameters:**
- `success` (int) — Number of successful operations
- `total` (int) — Total number of operations
- `slo_target` (float) — SLO target (0–100, percentage)
- `window_secs` (int) — Window size in seconds

**Return Type:**
```python
{
    "success": int,              # Successful operations
    "total": int,                # Total operations
    "availability": float,       # Availability (0.0–1.0)
    "slo_target": float,         # SLO target (0.0–100.0)
    "window_secs": int,          # Window size (seconds)
    "error_budget_seconds": float, # Remaining error budget (seconds)
    "target_met": bool           # Whether SLO target is met
}
```

**Raises:**
- `ValueError` — `total` ≤ 0 or `success` > `total`
- `ValueError` — `slo_target` outside range 0.0–100.0
- `ValueError` — `window_secs` ≤ 0
- `ZeroDivisionError` — `total` is 0

### evaluate_http_histogram_once

Purpose:

- Evaluate one histogram payload for latency and availability gates.

**Parameters:**
- `sample` (dict) — Histogram sample (same structure as HistogramSample)
- `latency_threshold_ms` (float, optional) — Override profile latency threshold
- `latency_percentile` (float, optional) — Override profile percentile
- `availability_threshold` (float, optional) — Override profile availability target
- `profile` (str, optional) — Profile name ("default", "strict_latency", "availability_first")

**Return Type:**
```python
{
    "timestamp": int,                   # Sample timestamp (seconds)
    "availability": float,              # Availability (0.0–1.0)
    "percentile_latency_ms": float,     # P-th percentile latency (ms)
    "evaluated_percentile": float,      # Which percentile was evaluated (0–100)
    "latency_pass": bool,               # Whether latency meets threshold
    "availability_pass": bool,          # Whether availability meets target
    "passed": bool                      # Whether both latency AND availability pass
}
```

**Raises:**
- `ValueError` — Invalid profile name
- `ValueError` — Histogram buckets not sorted by `upper_bound_ms`
- `ValueError` — Invalid threshold values (negative or >100)
- `KeyError` — Missing required fields in sample dict

Precedence:

- Explicit parameter values override profile values.
- Missing explicit parameters fall back to profile values.

### evaluate_stateful_once

Purpose:

- Evaluate one stateful service sample (db/queue style metrics).

**Parameters:**
- `sample` (dict) — Stateful service sample
- `replication_lag_threshold_ms` (float, optional) — Override profile threshold
- `queue_depth_threshold` (int, optional) — Override profile threshold
- `connection_pool_saturation_threshold` (float, optional) — Override profile threshold
- `profile` (str, optional) — Profile name ("default", "database_primary", "queue_hot_path")

**Return Type:**
```python
{
    "timestamp": int,                        # Sample timestamp (seconds)
    "score": float,                          # Overall composite score (0.0–1.0)
    "replication_lag_ok": bool,              # Whether replication lag acceptable
    "queue_depth_ok": bool,                  # Whether queue depth acceptable
    "connection_pool_ok": bool,              # Whether connection pool acceptable
    "connection_wait_penalized": bool,       # Whether wait time applied penalty
    "passed": bool                           # Whether all checks pass
}
```

**Raises:**
- `ValueError` — Invalid profile name
- `ValueError` — Threshold values out of valid range
- `KeyError` — Missing required fields in sample dict

Precedence:

- Explicit parameter values override profile values.
- Missing explicit parameters fall back to profile values.

### evaluate_ml_once

Purpose:

- Evaluate one ML serving sample with hybrid system + data scoring.

**Parameters:**
- `sample` (dict) — ML serving sample (inference latency, GPU utilization, drift, etc.)
- `max_inference_latency_ms` (float, optional) — Override profile latency threshold
- `max_gpu_utilization` (float, optional) — Override profile GPU threshold
- `max_feature_drift` (float, optional) — Override profile drift threshold
- `min_prediction_confidence` (float, optional) — Override profile confidence threshold
- `latency_weight` (float, optional) — Override profile latency weight in hybrid score
- `drift_weight` (float, optional) — Override profile drift weight in hybrid score
- `profile` (str, optional) — Profile name ("default", "latency_critical", "drift_sensitive")

**Return Type:**
```python
{
    "timestamp": int,                       # Sample timestamp (seconds)
    "inference_latency_score": float,       # Latency score (0.0–1.0)
    "gpu_utilization_score": float,         # GPU utilization score (0.0–1.0)
    "system_score": float,                  # System health score (0.0–1.0)
    "latency_score": float,                 # Latency component (0.0–1.0)
    "feature_drift_score": float,           # Drift component (0.0–1.0)
    "prediction_confidence_score": float,   # Confidence component (0.0–1.0)
    "drift_score": float,                   # Overall drift metric (0.0–1.0)
    "latency_weight": float,                # Weight for latency in hybrid score
    "drift_weight": float,                  # Weight for drift in hybrid score
    "hybrid_score": float,                  # Final score: (latency_weight * latency_score + drift_weight * drift_score)
    "passed": bool                          # Whether hybrid_score ≥ min_pass_score
}
```

**Raises:**
- `ValueError` — Invalid profile name
- `ValueError` — Threshold values out of valid range
- `ValueError` — Weights don't sum to valid distribution
- `KeyError` — Missing required fields in sample dict

Precedence:

- Explicit parameter values override profile values.
- Missing explicit parameters fall back to profile values.

### evaluate_genai_once

Purpose:

- Evaluate one GenAI serving sample for throughput, TTFT, and semantic similarity.

**Parameters:**
- `sample` (dict) — GenAI serving sample (tokens/sec, time-to-first-token, semantic similarity)
- `min_tokens_per_second` (float, optional) — Override profile throughput minimum
- `max_time_to_first_token_ms` (float, optional) — Override profile TTFT maximum
- `min_semantic_similarity` (float, optional) — Override profile semantic threshold
- `semantic_model_name` (str, optional) — Override profile semantic model
- `profile` (str, optional) — Profile name ("default", "latency_first", "quality_first")

**Return Type:**
```python
{
    "timestamp": int,                   # Sample timestamp (seconds)
    "tokens_per_second": float,         # Throughput (tokens/sec)
    "time_to_first_token_ms": float,    # TTFT (milliseconds)
    "semantic_similarity": float,       # Semantic similarity (0.0–1.0)
    "tokens_per_second_ok": bool,       # Whether throughput meets minimum
    "time_to_first_token_ok": bool,     # Whether TTFT meets maximum
    "semantic_similarity_ok": bool,     # Whether semantic similarity meets minimum
    "passed": bool                      # Whether ALL three checks pass
}
```

**Raises:**
- `ValueError` — Invalid profile name
- `ValueError` — Threshold values out of valid range
- `ValueError` — Invalid semantic_model_name
- `KeyError` — Missing required fields in sample dict

Precedence:

- Explicit parameter values override profile values.
- Missing explicit parameters fall back to profile values.

## Client Facade Integration

If you want config-driven execution, use `NeuralBudgetClient` from `neuralbudget`.

**Typical flow:**

1. Create client: `client = NeuralBudgetClient()`
2. Load config: `client.load_config("slo.yaml")`
3. Evaluate metrics: `result = client.evaluate(metric_data)`

**When to use `NeuralBudgetClient`:**

- You want a stable config contract across notebook, CI, and service workflows
- You want schema validation and mode selection from JSON/YAML
- You need to reload configurations dynamically
- You're integrating with infrastructure (Kubernetes, Terraform, etc.)

**When to use convenience functions directly:**

- You want one-shot calls without external config files
- You are composing custom Python orchestration logic
- You need programmatic control over evaluation parameters
- You're building a custom UI or dashboard

## Usage Patterns

### Pattern 1: Config-driven with NeuralBudgetClient

**For production services and CI/CD:**

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient()

try:
    client.load_config("slo.yaml")
except FileNotFoundError:
    print("ERROR: Config file not found")
    exit(1)
except ValueError as e:
    print(f"ERROR: Invalid config: {e}")
    exit(1)

metric_data = {
    "timestamp": 1686326400,
    "success": 999,
    "total": 1000,
    "buckets": [...]
}

try:
    result = client.evaluate(metric_data)
    
    if result['passed']:
        print("✅ SLO PASS")
    else:
        print("❌ SLO FAIL")
        print(f"  Reason: {result}")
        
except ValueError as e:
    print(f"ERROR: Invalid metric data: {e}")
except RuntimeError as e:
    print(f"ERROR: Evaluation failed: {e}")
```

### Pattern 2: One-shot evaluations with convenience functions

**For notebooks and quick scripts:**

```python
from neuralbudget import (
    availability_snapshot,
    evaluate_http_histogram_once,
    get_http_profile_preset,
)

# Simple availability check
result = availability_snapshot(
    success=999,
    total=1000,
    slo_target=99.9,
    window_secs=604800
)
print(f"Availability: {result['availability']:.2%}")
print(f"Budget remaining: {result['error_budget_seconds']:.0f}s")

# Histogram evaluation with override
result = evaluate_http_histogram_once(
    sample={
        "timestamp": 1686326400,
        "success": 9950,
        "total": 10000,
        "buckets": [
            {"upper_bound_ms": 100.0, "count": 9850},
            {"upper_bound_ms": 500.0, "count": 9950},
            {"upper_bound_ms": 1000.0, "count": 10000},
        ],
    },
    latency_threshold_ms=150.0,  # Override default
    profile="aggressive"
)

if not result['latency_pass']:
    print(f"⚠️  Latency too high: {result['percentile_latency_ms']:.1f}ms")
```

### Pattern 3: Profile-aware with parameter override

**For flexible policy evaluation:**

```python
from neuralbudget import (
    evaluate_ml_once,
    get_ml_profile_preset,
)

# Get profile and inspect defaults
profile = get_ml_profile_preset("latency_critical")
print(f"Default latency threshold: {profile.max_inference_latency_ms}ms")

# Override specific parameters
result = evaluate_ml_once(
    sample={
        "timestamp": 1686326400,
        "inference_latency_ms": 45.0,
        "gpu_utilization": 0.65,
        "feature_drift": 0.02,
        "prediction_confidence": 0.92,
    },
    max_inference_latency_ms=50.0,  # Override to 50ms
    max_feature_drift=0.05,          # Override to 5%
    profile="latency_critical"       # Use preset for other params
)

print(f"Hybrid score: {result['hybrid_score']:.4f}")
print(f"Passed: {result['passed']}")
```

## Dictionary-based workflow

Use defaults and dictionary output for quick scripts and integration into existing tools.

### Profile-driven operations flow

Use profile names when:

- Teams need consistent SLO policy baselines.
- You want simple switching between policy modes.

Use explicit overrides when:

- You need temporary adjustments without redefining a profile.

## Error Handling Guide

All convenience functions and the client facade can raise exceptions. Here's how to handle them:

### Common Exceptions

| Exception | When It Occurs | How to Handle |
|---|---|---|
| `ValueError` | Invalid profile name, out-of-range thresholds, missing fields | Check input parameters; see error message for details |
| `KeyError` | Required field missing in sample dict | Verify sample dict has all required fields for that SLO mode |
| `TypeError` | Wrong data type (e.g., success is string not int) | Cast or validate input types before calling |
| `FileNotFoundError` | Config file not found | Verify path is correct; check file exists |
| `RuntimeError` | No config loaded before evaluate() | Call load_config() first |
| `ZeroDivisionError` | total=0 in availability_snapshot | Ensure total > 0 before calling |

### Example: Robust Error Handling

```python
from neuralbudget import NeuralBudgetClient
import sys

def safe_evaluate(config_path, metric_data):
    """Evaluate with comprehensive error handling."""
    client = NeuralBudgetClient()
    
    # Step 1: Load config with error handling
    try:
        client.load_config(config_path)
    except FileNotFoundError:
        print(f"ERROR: Config not found at {config_path}", file=sys.stderr)
        return None
    except ValueError as e:
        print(f"ERROR: Invalid config: {e}", file=sys.stderr)
        return None
    except KeyError as e:
        print(f"ERROR: Missing required config field: {e}", file=sys.stderr)
        return None
    
    # Step 2: Validate metric data
    required_fields = ["timestamp", "success", "total", "buckets"]
    missing = [f for f in required_fields if f not in metric_data]
    if missing:
        print(f"ERROR: Missing required fields: {missing}", file=sys.stderr)
        return None
    
    # Step 3: Type check
    if not isinstance(metric_data["total"], int) or metric_data["total"] <= 0:
        print("ERROR: 'total' must be positive integer", file=sys.stderr)
        return None
    
    if not isinstance(metric_data["success"], int) or metric_data["success"] > metric_data["total"]:
        print("ERROR: 'success' must be ≤ 'total'", file=sys.stderr)
        return None
    
    # Step 4: Evaluate with error handling
    try:
        result = client.evaluate(metric_data)
        return result
    except ValueError as e:
        print(f"ERROR: Evaluation failed (invalid data): {e}", file=sys.stderr)
        return None
    except RuntimeError as e:
        print(f"ERROR: Evaluation failed (internal error): {e}", file=sys.stderr)
        return None

# Usage
if __name__ == "__main__":
    result = safe_evaluate("slo.yaml", {
        "timestamp": 1686326400,
        "success": 999,
        "total": 1000,
        "buckets": [...]
    })
    
    if result:
        print(f"SLO {'PASS' if result['passed'] else 'FAIL'}")
    else:
        sys.exit(1)
```

### Profile Lookup Error Handling

```python
from neuralbudget import get_http_profile_preset

# Method 1: Try-except
try:
    profile = get_http_profile_preset("unknown")
except ValueError as e:
    print(f"Profile not found: {e}")
    profile = get_http_profile_preset("default")

# Method 2: Provide list of valid profiles
valid_profiles = ["default", "strict_latency", "availability_first"]
user_profile = input(f"Choose profile {valid_profiles}: ")

if user_profile not in valid_profiles:
    print(f"Invalid profile. Using 'default'")
    user_profile = "default"

profile = get_http_profile_preset(user_profile)
```

## Testing Strategy

The convenience layer test file uses a native-module stub to keep tests fast and deterministic without requiring a compiled extension.

Test file:

- tests/python_convenience_tests.py

Coverage includes:

- Dataclass return behavior
- Preset retrieval and override behavior
- Unknown preset error paths
- Parameter validation and error raising

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
