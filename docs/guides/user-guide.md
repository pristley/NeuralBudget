# NeuralBudget User Guide

This guide is the practical, end-to-end walkthrough for installing NeuralBudget,
choosing the right API surface, and applying SLO evaluation in notebooks,
services, and CI/CD pipelines.

## Who This Guide Is For

Use this guide if you are:

- building reliability guardrails for backend services
- monitoring model-serving and GenAI quality regressions
- adding SLO checks to pull-request or release pipelines
- running exploratory reliability analysis in Jupyter notebooks

## Installation

## Python (recommended for notebooks and pipelines)

Install from a built wheel:

```bash
python3 -m pip install neuralbudget
```

Install from source with `maturin`:

```bash
python3 -m pip install --upgrade pip maturin
maturin build --release --manifest-path Cargo.toml
python3 -m pip install --force-reinstall target/wheels/neuralbudget-*.whl
```

Editable local development install:

```bash
python3 -m pip install --upgrade pip maturin
maturin develop --release --manifest-path Cargo.toml
```

Optional YAML support for config files:

```bash
python3 -m pip install pyyaml
```

## Rust crate

Add this dependency in Cargo.toml:

```toml
[dependencies]
neuralbudget = "0.1.2"
```

## API Surface Map

NeuralBudget exposes three usage layers:

1. `NeuralBudgetClient` facade: best default for notebook and CI workflows.
2. `neuralbudget.convenience`: one-shot dict-friendly helpers with presets.
3. native Rust/PyO3 objects: maximum control and explicit model wiring.

If you are unsure where to start, use `NeuralBudgetClient`.

## Quick Start: Facade API

`NeuralBudgetClient` provides two stable steps:

1. `client.load_config("slo.yaml")`
2. `client.evaluate(metric_data)`

Minimal JSON configuration (`slo.json`):

```json
{
  "mode": "http",
  "profile": "strict_latency",
  "return_dataclass": false,
  "params": {
    "latency_threshold_ms": 180.0
  }
}
```

Python usage:

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient().load_config("slo.json")

result = client.evaluate(
    {
        "timestamp": 1,
        "success": 9995,
        "total": 10000,
        "buckets": [
            {"upper_bound_ms": 100.0, "count": 9700},
            {"upper_bound_ms": 220.0, "count": 10000}
        ],
        "format": "prometheus_cumulative"
    }
)

print(result["pass"], result["percentile_latency_ms"])
```

## Configuration Reference

Top-level config keys:

- `mode`: one of `http`, `stateful`, `ml`, `genai`, `composite`
- `profile`: optional preset name (non-composite modes)
- `return_dataclass`: optional bool for typed convenience returns
- `params`: optional overrides forwarded to the selected evaluator

YAML example (`slo.yaml`) for an ML gate:

```yaml
mode: ml
profile: drift_sensitive
params:
  min_pass_score: 0.92
```

## Mode Examples

## HTTP histogram mode

```python
client = NeuralBudgetClient().load_config("http_slo.yaml")
result = client.evaluate(
    {
        "timestamp": 120,
        "success": 4980,
        "total": 5000,
        "buckets": [
            [50.0, 4200],
            [100.0, 4700],
            [200.0, 5000]
        ],
        "format": "open_telemetry_delta"
    }
)
print(result["availability"], result["pass"])
```

## Stateful mode

```python
client = NeuralBudgetClient().load_config("stateful_slo.json")
result = client.evaluate(
    {
        "timestamp": 42,
        "replication_lag_ms": 180.0,
        "queue_depth": 700,
        "connection_pool_saturation": 0.72,
        "connection_wait_time_ms": 12.0
    }
)
print(result["score"], result["pass"])
```

## ML mode

```python
client = NeuralBudgetClient().load_config("ml_slo.json")
result = client.evaluate(
    {
        "timestamp": 7,
        "inference_latency_ms": 190.0,
        "gpu_utilization": 0.76,
        "feature_drift": 0.08,
        "prediction_confidence": 0.91
    }
)
print(result["hybrid_score"], result["pass"])
```

## GenAI mode

```python
client = NeuralBudgetClient().load_config("genai_slo.yaml")
result = client.evaluate(
    {
        "timestamp": 2,
        "tokens_generated": 350,
        "generation_duration_ms": 14000,
        "time_to_first_token_ms": 820,
        "reference_text": "The release passed reliability criteria.",
        "generated_text": "Reliability criteria were met for this release."
    }
)
print(result["tokens_per_second"], result["semantic_similarity"], result["pass"])
```

## Composite dependency DAG mode

Composite mode models upstream and downstream service impact.

```python
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient().load_config("composite_slo.json")

result = client.evaluate(
    {
        "services": [
            {
                "service": "gateway",
                "local_score": 0.94,
                "min_pass_score": 0.9,
                "impact_weight": 2.0
            },
            {
                "service": "checkout",
                "local_score": 0.88,
                "min_pass_score": 0.9,
                "impact_weight": 3.0
            }
        ],
        "dependencies": [
            {
                "dependency": "gateway",
                "dependent": "checkout",
                "failure_penalty": 0.15
            }
        ],
        "global_min_pass_score": 0.9
    }
)

print(result["global_slo"], result["global_pass"])
```

## Jupyter Notebook Workflow

Typical notebook pattern:

1. Load one config per experiment or environment.
2. Evaluate each interval or slice.
3. Convert result dicts to DataFrame for plotting.

```python
import pandas as pd
from neuralbudget import NeuralBudgetClient

client = NeuralBudgetClient().load_config("http_slo.yaml")
rows = []

for sample in metric_samples:
    rows.append(client.evaluate(sample))

df = pd.DataFrame(rows)
df[["timestamp", "availability", "percentile_latency_ms", "pass"]].tail()
```

## CI/CD Pipeline Workflow

Use NeuralBudget as a release gate by exiting non-zero when score criteria fail.

```python
#!/usr/bin/env python3

import json
import sys
from pathlib import Path

from neuralbudget import NeuralBudgetClient

config_path = Path("slo.yaml")
metrics_path = Path("metrics.json")

client = NeuralBudgetClient().load_config(config_path)
metric_data = json.loads(metrics_path.read_text(encoding="utf-8"))
result = client.evaluate(metric_data)

passed = bool(result.get("pass", result.get("global_pass", False)))
print(json.dumps(result, indent=2, sort_keys=True))

if not passed:
    sys.exit("SLO gate failed")
```

Example GitHub Actions step:

```yaml
- name: Run SLO gate
  run: python3 scripts/slo_gate.py
```

## Release and Distribution Automation

Release packaging and publishing are integrated in `.github/workflows/release.yml`.

For tagged releases (`v*`), CD performs:

- validation gates (format, lint, tests, coverage)
- crate packaging (`.crate`)
- source distribution build (`sdist`)
- cross-platform wheel builds:
    - Linux `x86_64` (`manylinux`)
    - Windows `x86_64`
    - macOS `aarch64`
- GitHub Release creation with all generated artifacts
- PyPI publish through trusted publishing (`pypa/gh-action-pypi-publish`)

### Trusted Publisher setup checklist

1. In PyPI, create a Trusted Publisher for repository `pristley/NeuralBudget`.
2. Set workflow path to `.github/workflows/release.yml`.
3. Use environment name `pypi`.
4. Ensure the same `pypi` environment exists in GitHub repository settings.

### Release execution checklist

1. Update versions in `Cargo.toml` and `pyproject.toml`.
2. Push the release commit to `main`.
3. Create and push a tag (`vX.Y.Z`).
4. Publish a GitHub Release for that tag.
5. Watch CD workflow jobs until publish completes.

## Applications and Use Cases

## Service reliability governance

- enforce API latency/availability contracts pre-release
- prevent regressions from being promoted to production

## Platform capacity and stateful readiness

- catch queue-depth and pool-saturation issues before incidents
- score readiness of data stores and event-driven workers

## MLOps quality gates

- combine serving latency with drift and confidence in one score
- evaluate rollout safety for new model versions

## GenAI response quality and responsiveness

- track throughput and first-token latency for user experience
- monitor qualitative similarity as a deterministic policy signal

## Dependency-aware global health modeling

- propagate upstream failures across service DAGs
- compute one weighted global SLO for executive and release dashboards

## Troubleshooting

## "No config loaded" error

Call `load_config` before `evaluate`.

## YAML file not loading

Install PyYAML:

```bash
python3 -m pip install pyyaml
```

## Unsupported config extension

Use `.json`, `.yaml`, or `.yml`.

## Unknown profile name

Use one of the preset names from:

- `HTTP_PROFILE_PRESETS`
- `STATEFUL_PROFILE_PRESETS`
- `ML_PROFILE_PRESETS`
- `GENAI_PROFILE_PRESETS`

## Additional References

- README: project overview and quick examples
- docs/guides/production-deployment.md: production rollout, Kubernetes, and Prometheus integration
- docs/reference/convenience-layer.md: convenience helper details
- docs/reference/composite-slo-dag.md: composite DAG schema and semantics
- examples/python/: runnable scripts
