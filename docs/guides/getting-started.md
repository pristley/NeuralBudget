# NeuralBudget Getting Started

This guide is the fastest path to your first successful NeuralBudget evaluation.
It is designed for engineers who want a practical result in under 10 minutes.

## 1. Choose Your Interface

Use this decision guide:

- Use Python + `NeuralBudgetClient` if you are working in notebooks, CI jobs, or data pipelines.
- Use Python convenience helpers if you want one-shot evaluations with minimal setup.
- Use Rust APIs if you need strict typing and native integration in Rust services.

## 2. Install

### Python (recommended first path)

```bash
python3 -m pip install --upgrade pip maturin
maturin develop --release --manifest-path Cargo.toml
```

Optional for YAML configs:

```bash
python3 -m pip install pyyaml
```

### Rust

Add in `Cargo.toml`:

```toml
[dependencies]
neuralbudget = "0.1.2"
```

## 3. Create a Minimal Config

Create `slo.json`:

```json
{
  "schema_version": 1,
  "mode": "http",
  "profile": "strict_latency",
  "return_dataclass": false,
  "params": {
    "latency_threshold_ms": 200.0
  }
}
```

## 4. Run Your First Evaluation

Create `quick_eval.py`:

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
            {"upper_bound_ms": 200.0, "count": 9950},
            {"upper_bound_ms": 300.0, "count": 10000},
        ],
        "format": "prometheus_cumulative",
    }
)

print(result)
print("pass:", result.get("pass", result.get("global_pass")))
```

Run:

```bash
python3 quick_eval.py
```

## 5. Validate Local Quality Gates

Run the baseline checks:

```bash
cargo test --all-targets --all-features
python3 tests/python_convenience_tests.py
python3 tests/python_client_tests.py
```

For full release-grade validation:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo llvm-cov --workspace --all-features --lib --tests --summary-only
```

## 6. Common First-Run Errors

### `No config loaded. Call load_config(path) first.`

Call `load_config(...)` before `evaluate(...)`.

### `Unsupported config extension`

Use `.json`, `.yaml`, or `.yml`.

### `PyYAML is required to load YAML config files`

Install:

```bash
python3 -m pip install pyyaml
```

### `unknown ... preset`

Use a valid profile name from:

- `HTTP_PROFILE_PRESETS`
- `STATEFUL_PROFILE_PRESETS`
- `ML_PROFILE_PRESETS`
- `GENAI_PROFILE_PRESETS`

## 7. Where To Go Next

- Full walkthrough: [docs/guides/user-guide.md](user-guide.md)
- Production rollout: [docs/guides/production-deployment.md](production-deployment.md)
- Kubernetes operations: [docs/guides/kubernetes-integration.md](kubernetes-integration.md)
- Webhook alerting example: [examples/python/webhook_alerting.py](../../examples/python/webhook_alerting.py)
- Convenience API reference: [docs/reference/convenience-layer.md](../reference/convenience-layer.md)
- Composite DAG semantics: [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)