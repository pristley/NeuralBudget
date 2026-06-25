# Python API Reference

Complete reference for the NeuralBudget Python API, including the native extension and convenience layers.

## Table of Contents

- [Installation](#installation)
- [Core Classes](#core-classes)
- [NeuralBudgetClient](#neuralbudgetclient)
- [Convenience Functions](#convenience-functions)
- [Alert Dispatching](#alert-dispatching)
- [Data Models](#data-models)
- [Examples](#examples)

---

## Installation

```bash
# From PyPI
pip install neuralbudget

# From source
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget
pip install maturin
maturin develop
```

---

## Core Classes

These classes come from the native Rust extension and represent SLO models and configurations.

### `SloConfig`

Configuration for an SLO target and evaluation window.

**Attributes:**
- `target: float` — SLO target (0.0–100.0, percentage)
- `window: str` — Time window ("7d", "30d", "1h", etc.)

**Methods:**
```python
config = neuralbudget.SloConfig(target=99.9, window="7d")
config.to_dict()   # Convert to dictionary
config.to_json()   # Convert to JSON string
config.to_yaml()   # Convert to YAML string
```

**Example:**
```python
import neuralbudget

slo = neuralbudget.SloConfig(99.9, "7d")
print(f"Target: {slo.target}%, Window: {slo.window}")
```

### `ErrorBudget`

Represents remaining error budget and burn rate.

**Attributes:**
- `target: float` — SLO target percentage
- `window_seconds: int` — Window size in seconds
- `error_seconds_remaining: float` — Remaining error budget (seconds)
- `burn_rate_1h: float` — 1-hour burn rate
- `burn_rate_5m: float` — 5-minute burn rate

**Methods:**
```python
budget = neuralbudget.ErrorBudget(target=99.9, window_seconds=604800)
print(f"Remaining: {budget.error_seconds_remaining}s")
```

### `TimeWindow`

Time window calculator for rolling and calendar-aligned windows.

**Types:**
- `rolling(seconds)` — Sliding window from now
- `calendar_aligned(period_seconds, timezone_offset_seconds)` — Aligned to calendar boundaries

**Methods:**
```python
# Rolling window (last 7 days)
tw_rolling = neuralbudget.TimeWindow.rolling(604800)

# Calendar-aligned (UTC+0)
tw_calendar = neuralbudget.TimeWindow.calendar_aligned(604800, 0)

# Calendar-aligned (US Eastern, UTC-5 = -18000 seconds)
tw_eastern = neuralbudget.TimeWindow.calendar_aligned(604800, -18000)

# Check if timestamp is in window
is_in_window = tw_rolling.is_timestamp_in_window(1686326400)
```

### `HistogramSample`

Latency distribution sample from Prometheus or application metrics.

**Attributes:**
- `timestamp: int` — Timestamp (seconds since epoch)
- `success: int` — Successful requests/operations
- `total: int` — Total requests/operations
- `buckets: list[HistogramBucket]` — Latency distribution buckets
- `format: HistogramFormat` — Format identifier

**Methods:**
```python
sample = neuralbudget.HistogramSample(
    timestamp=1686326400,
    success=999,
    total=1000,
    buckets=[
        {"upper_bound_ms": 100.0, "count": 950},
        {"upper_bound_ms": 500.0, "count": 45},
        {"upper_bound_ms": 1000.0, "count": 5},
    ],
    format="prometheus_histogram"
)
```

### SLO Models

#### `HttpSlo`

Evaluates HTTP/gRPC request SLOs (latency + availability).

**Attributes:**
- `latency_threshold_ms: float` — P-th percentile threshold (ms)
- `latency_percentile: float` — Percentile to check (0–100)
- `availability_threshold: float` — Availability threshold (0–100, percentage)

**Methods:**
```python
http_slo = neuralbudget.HttpSlo(
    latency_threshold_ms=100.0,
    latency_percentile=99.0,
    availability_threshold=99.9
)
result = http_slo.evaluate_sample(histogram_sample)
```

#### `StatefulSlo`

Evaluates stateful service SLOs (replication lag, queue depth, pool saturation).

**Attributes:**
- `replication_lag_threshold_ms: float`
- `queue_depth_threshold: int`
- `connection_pool_saturation_threshold: float`
- `connection_wait_time_threshold_ms: float`
- `connection_wait_penalty_weight: float`
- `min_pass_score: float`

#### `MlSlo`

Evaluates ML serving SLOs (latency + system health + drift).

**Attributes:**
- `max_inference_latency_ms: float`
- `max_gpu_utilization: float`
- `max_feature_drift: float`
- `min_prediction_confidence: float`
- `latency_weight: float` — Weight for latency in hybrid score
- `drift_weight: float` — Weight for drift in hybrid score

#### `GenAiSlo`

Evaluates GenAI workload SLOs (throughput, TTFT, semantic similarity).

**Attributes:**
- `min_tokens_per_second: float`
- `max_time_to_first_token_ms: float`
- `min_semantic_similarity: float`

#### `CompositeSlo`

Evaluates composite service SLOs with dependency graphs.

**Methods:**
```python
composite = neuralbudget.CompositeSlo()
result = composite.evaluate_dependencies({
    "services": [
        {"service": "api", "local_score": 0.999, "min_pass_score": 0.99, "impact_weight": 1.0},
        {"service": "db", "local_score": 0.995, "min_pass_score": 0.99, "impact_weight": 0.8},
    ],
    "dependencies": [
        {"dependency": "db", "dependent": "api", "failure_penalty": 0.1}
    ],
    "global_min_pass_score": 0.99
})
```

---

## NeuralBudgetClient

High-level facade for configuration-driven workflows.

### Constructor

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient()
```

### Methods

#### `load_config(path: str | Path) -> None`

Load SLO configuration from YAML or JSON file.

**Parameters:**
- `path` — Path to config file (.yaml, .json, or .yml)

**Raises:**
- `FileNotFoundError` — Config file not found
- `ValueError` — Invalid config schema or format

**Example:**
```python
client.load_config("slo.yaml")
client.load_config("config/http_slo.json")
```

#### `evaluate(metric_data: dict | list | Any) -> dict | Any`

Evaluate metrics against loaded configuration.

**Parameters:**
- `metric_data` — Metric payload matching the SLO mode

**Returns:**
- Evaluation result as dataclass (if available) or dict

**Raises:**
- `RuntimeError` — No config loaded
- `ValueError` — Invalid metric data format

**Example:**
```python
metric = {
    "timestamp": 1686326400,
    "success": 999,
    "total": 1000,
    "buckets": [...]
}
result = client.evaluate(metric)
print(f"Passed: {result['passed']}")
```

### Configuration Format

#### YAML Example

```yaml
schema_version: 1
mode: http
profile: aggressive
params:
  latency_threshold_ms: 50.0
  latency_percentile: 99.0
  availability_threshold: 99.95
alerts:
  slack:
    webhook_url: "https://hooks.slack.com/..."
  pagerduty:
    integration_key: "pagerduty-key"
```

#### JSON Example

```json
{
  "schema_version": 1,
  "mode": "stateful",
  "profile": "conservative",
  "params": {
    "replication_lag_threshold_ms": 200.0,
    "queue_depth_threshold": 1000
  }
}
```

---

## Convenience Functions

Thin wrappers around native API for one-shot evaluations and profiles.

### Availability

#### `availability_snapshot(success: int, total: int, slo_target: float, window_secs: int) -> AvailabilitySnapshotResult`

Single-call availability check.

**Parameters:**
- `success` — Successful operations
- `total` — Total operations
- `slo_target` — SLO target (0–100, percentage)
- `window_secs` — Window size in seconds

**Returns:**
```python
@dataclass
class AvailabilitySnapshotResult:
    success: int
    total: int
    availability: float
    slo_target: float
    window_secs: int
    error_budget_seconds: float
    target_met: bool
```

**Example:**
```python
from neuralbudget import availability_snapshot

result = availability_snapshot(
    success=999, 
    total=1000, 
    slo_target=99.9, 
    window_secs=604800
)
print(f"Availability: {result.availability:.2%}")
print(f"Budget: {result.error_budget_seconds:.1f}s")
```

### HTTP SLO

#### `evaluate_http_histogram_once(sample: dict, profile: str = "standard") -> HttpHistogramEvaluationResult`

Evaluate HTTP histogram against a preset profile.

**Parameters:**
- `sample` — Histogram sample dict (same as HistogramSample fields)
- `profile` — Profile name ("standard", "aggressive", "conservative")

**Returns:**
```python
@dataclass
class HttpHistogramEvaluationResult:
    timestamp: int
    availability: float
    percentile_latency_ms: float
    evaluated_percentile: float
    latency_pass: bool
    availability_pass: bool
    passed: bool
```

**Example:**
```python
from neuralbudget import evaluate_http_histogram_once

result = evaluate_http_histogram_once(
    sample={
        "timestamp": 1686326400,
        "success": 9950,
        "total": 10000,
        "buckets": [{"upper_bound_ms": 100.0, "count": 9950}],
    },
    profile="aggressive"
)
```

### Profiles

#### `HTTP_PROFILE_PRESETS`

Available HTTP SLO profiles:
- `standard` — 99% availability, 100ms p99
- `aggressive` — 99.9% availability, 50ms p99
- `conservative` — 99% availability, 500ms p99

#### `get_http_profile_preset(name: str) -> HttpSloProfile`

Get profile parameters by name.

```python
from neuralbudget import get_http_profile_preset

profile = get_http_profile_preset("aggressive")
print(f"Latency threshold: {profile.latency_threshold_ms}ms")
print(f"Availability: {profile.availability_threshold:.1%}")
```

### ML SLO

#### `evaluate_ml_once(sample: dict, profile: str = "standard") -> MlEvaluationResult`

Evaluate ML serving SLO.

**Parameters:**
- `sample` — ML metrics dict
- `profile` — Profile name ("standard", "production", "experimental")

**Returns:**
```python
@dataclass
class MlEvaluationResult:
    timestamp: int
    inference_latency_score: float
    gpu_utilization_score: float
    system_score: float
    latency_score: float
    feature_drift_score: float
    prediction_confidence_score: float
    drift_score: float
    latency_weight: float
    drift_weight: float
    hybrid_score: float
    passed: bool
```

### GenAI SLO

#### `evaluate_genai_once(sample: dict, profile: str = "standard") -> GenAiEvaluationResult`

Evaluate GenAI workload SLO.

**Parameters:**
- `sample` — GenAI metrics dict
- `profile` — Profile name ("standard", "aggressive", "responsive")

**Returns:**
```python
@dataclass
class GenAiEvaluationResult:
    timestamp: int
    tokens_per_second: float
    time_to_first_token_ms: float
    semantic_similarity: float
    tokens_per_second_ok: bool
    time_to_first_token_ok: bool
    semantic_similarity_ok: bool
    passed: bool
```

---

## Alert Dispatching

### `AlertDispatcher`

Send SLO violation notifications.

**Methods:**

#### `send_violation(*, mode: str, profile: str, metric_data: dict, result: dict, alerts_config: dict) -> AlertDispatchSummary`

Send alerts to configured providers.

**Parameters:**
- `mode` — SLO mode (http, ml, genai, stateful, composite)
- `profile` — Profile name (e.g., "standard")
- `metric_data` — Original metric input
- `result` — Evaluation result
- `alerts_config` — Alert provider configuration

**Returns:**
```python
@dataclass
class AlertDispatchSummary:
    attempted: int
    succeeded: int
    failed: int
    results: list[AlertDispatchResult]
```

**Example:**
```python
from neuralbudget import AlertDispatcher

dispatcher = AlertDispatcher()
result = dispatcher.send_violation(
    mode="http",
    profile="aggressive",
    metric_data=metric,
    result=evaluation,
    alerts_config={
        "slack": {
            "webhook_url": "https://hooks.slack.com/..."
        }
    }
)
print(f"Sent to {result.succeeded} providers")
```

### Supported Providers

#### Slack

```yaml
alerts:
  slack:
    webhook_url: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
```

#### PagerDuty

```yaml
alerts:
  pagerduty:
    integration_key: "your-pagerduty-integration-key"
```

#### Opsgenie

```yaml
alerts:
  opsgenie:
    api_key: "your-opsgenie-api-key"
    region: "us"  # or "eu"
```

---

## Data Models

### Result Dataclasses

All evaluation results are frozen dataclasses:

```python
@dataclass(frozen=True)
class HttpHistogramEvaluationResult:
    timestamp: int
    availability: float
    percentile_latency_ms: float
    evaluated_percentile: float
    latency_pass: bool
    availability_pass: bool
    passed: bool
```

Frozen dataclasses are immutable and hashable:

```python
result = evaluate_http_histogram_once(sample)
result.passed = False  # Raises FrozenInstanceError
hash(result)  # Can be used in sets/dicts
```

---

## Examples

### Example 1: Simple HTTP SLO Check

```python
from neuralbudget import evaluate_http_histogram_once

# Typical Prometheus histogram sample
sample = {
    "timestamp": 1686326400,
    "success": 9950,
    "total": 10000,
    "buckets": [
        {"upper_bound_ms": 50.0, "count": 8500},
        {"upper_bound_ms": 100.0, "count": 1200},
        {"upper_bound_ms": 500.0, "count": 200},
        {"upper_bound_ms": 1000.0, "count": 50},
    ]
}

result = evaluate_http_histogram_once(sample, profile="aggressive")
print(f"Availability: {result.availability:.2%}")
print(f"P99 Latency: {result.percentile_latency_ms:.1f}ms")
print(f"SLO Met: {result.passed}")
```

### Example 2: Configuration-Driven Workflow

```python
from neuralbudget import NeuralBudgetClient, AlertDispatcher
from pathlib import Path

client = NeuralBudgetClient()
client.load_config(Path("config/slo.yaml"))

# Evaluate
metric = {...}
result = client.evaluate(metric)

# Alert on violation
if not result["passed"]:
    dispatcher = AlertDispatcher()
    dispatcher.send_violation(
        mode="http",
        profile="aggressive",
        metric_data=metric,
        result=result,
        alerts_config={
            "slack": {
                "webhook_url": "https://hooks.slack.com/..."
            }
        }
    )
```

### Example 3: Composite Service SLO

```python
from neuralbudget import CompositeSlo

composite = CompositeSlo()

result = composite.evaluate_dependencies({
    "services": [
        {"service": "api", "local_score": 0.999, "min_pass_score": 0.99, "impact_weight": 1.0},
        {"service": "cache", "local_score": 0.98, "min_pass_score": 0.95, "impact_weight": 0.5},
        {"service": "db", "local_score": 0.995, "min_pass_score": 0.99, "impact_weight": 0.8},
    ],
    "dependencies": [
        {"dependency": "db", "dependent": "api", "failure_penalty": 0.05},
        {"dependency": "cache", "dependent": "api", "failure_penalty": 0.02},
    ],
    "global_min_pass_score": 0.99
})

print(f"System SLO: {result['global_score']:.3f}")
print(f"System Pass: {result['global_passed']}")
```

---

## Type Hints

All public APIs use type hints for IDE support:

```python
from neuralbudget import (
    NeuralBudgetClient,
    evaluate_http_histogram_once,
    HttpHistogramEvaluationResult,
)
from pathlib import Path

client: NeuralBudgetClient = NeuralBudgetClient()
client.load_config(Path("slo.yaml"))

metric: dict = {...}
result: HttpHistogramEvaluationResult = evaluate_http_histogram_once(metric)

if result.passed:
    print("SLO Met!")
```

---

## Error Handling

All exceptions inherit from Python built-ins:

```python
from neuralbudget import NeuralBudgetClient
import json

client = NeuralBudgetClient()

try:
    client.load_config("missing.yaml")
except FileNotFoundError:
    print("Config file not found")

try:
    result = client.evaluate({"invalid": "data"})
except ValueError as e:
    print(f"Invalid metric: {e}")
except RuntimeError:
    print("No config loaded - call load_config() first")
```

---

## Performance Characteristics

- **Evaluation latency**: <1ms per SLO check (Rust core)
- **Memory overhead**: <1MB per client instance
- **Alert dispatch**: Network I/O dependent (typically <1s)
- **Composite DAG evaluation**: O(n + m) where n=services, m=dependencies

---

**See also:**
- [User Guide](../guides/user-guide.md)
- [Development Guide](../guides/development.md)
- [Architecture Map](../../agentmap.md)
