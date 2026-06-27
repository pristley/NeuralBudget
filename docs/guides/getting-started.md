# NeuralBudget Getting Started

Use this guide to run your first NeuralBudget evaluation in about 10 minutes.

## What You Do

1. Install NeuralBudget.
2. Create a minimal SLO config file.
3. Run one evaluation.
4. Confirm that the evaluation passes.
5. Run local quality checks.

## Prerequisites

- Python 3.9 or later
- Rust toolchain (only if you build from source)
- A shell environment on Linux, macOS, or Windows

## 1. Install NeuralBudget

Choose one path.

### Path A: Install from PyPI (fastest)

~~~bash
python3 -m pip install --upgrade pip
python3 -m pip install neuralbudget
~~~

### Path B: Build from source (development path)

~~~bash
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget
python3 -m pip install --upgrade pip maturin
maturin develop --release --manifest-path Cargo.toml
~~~

Optional: install YAML support if you use YAML configs.

~~~bash
python3 -m pip install pyyaml
~~~

## 2. Create a Minimal Config

Create a file named slo.json.

~~~json
{
  "schema_version": 1,
  "mode": "http",
  "profile": "strict_latency",
  "params": {
    "latency_threshold_ms": 200.0
  }
}
~~~

## 3. Run Your First Evaluation

Create a file named quick_eval.py.

~~~python
from neuralbudget import NeuralBudgetClient
import sys

try:
    # Step 1: Create client and load config
    client = NeuralBudgetClient()
    client.load_config("slo.json")
    
    # Step 2: Prepare metric data
    metric_data = {
        "timestamp": 1,
        "success": 9995,
        "total": 10000,
        "buckets": [
            {"upper_bound_ms": 100.0, "count": 9700},
            {"upper_bound_ms": 200.0, "count": 9950},
            {"upper_bound_ms": 300.0, "count": 10000}
        ],
        "format": "prometheus_cumulative"
    }
    
    # Step 3: Evaluate
    result = client.evaluate(metric_data)
    
    # Step 4: Display results
    is_pass = result.get("pass", result.get("global_pass"))
    print("✓ Evaluation succeeded")
    print("result:", result)
    print("pass:", bool(is_pass))

except FileNotFoundError:
    print("✗ ERROR: Config file 'slo.json' not found", file=sys.stderr)
    print("  → Create slo.json first (see Step 2 above)", file=sys.stderr)
    sys.exit(1)

except ValueError as e:
    print(f"✗ ERROR: Invalid config or metrics: {e}", file=sys.stderr)
    print("  → Check slo.json format and metric_data structure", file=sys.stderr)
    sys.exit(1)

except RuntimeError as e:
    print(f"✗ ERROR: Evaluation failed: {e}", file=sys.stderr)
    sys.exit(1)
~~~

Run it:

~~~bash
python3 quick_eval.py
~~~

Expected outcome:
- The script prints an evaluation object.
- The pass line prints True for this sample input.

**If you see an error:**
- `✗ ERROR: Config file 'slo.json' not found` → Make sure you created slo.json in Step 2
- `✗ ERROR: Invalid config or metrics` → Check your slo.json and metric_data match the format above
- `✗ ERROR: Evaluation failed` → See [Troubleshooting Guide](troubleshooting.md) for help

## 4. Validate Local Quality Gates

Run baseline checks:

~~~bash
cargo test --all-targets --all-features
python3 tests/python_convenience_tests.py
python3 tests/python_client_tests.py
~~~

Run release-grade checks:

~~~bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo llvm-cov --workspace --all-features --lib --tests --summary-only
~~~

Expected outcome:
- All commands exit successfully.
- No formatting, lint, or test failures remain.

## 5. Fix Common First-Run Errors

### Error: No config loaded. Call load_config(path) first.
Fix:
Call load_config before evaluate.

### Error: Unsupported config extension
Fix:
Use one of these config file extensions:
- .json
- .yaml
- .yml

### Error: PyYAML is required to load YAML config files
Fix:

~~~bash
python3 -m pip install pyyaml
~~~

### Error: unknown preset
Fix:
Use a valid preset name for your selected mode.

## 6. Next Documentation

- Full workflow and mode guidance: [docs/guides/user-guide.md](user-guide.md)
- Production rollout: [docs/guides/production-deployment.md](production-deployment.md)
- Kubernetes deployment: [docs/guides/kubernetes-integration.md](kubernetes-integration.md)
- Convenience API reference: [docs/reference/convenience-layer.md](../reference/convenience-layer.md)
- Composite DAG behavior: [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)