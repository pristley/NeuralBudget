# Development Guide

This guide covers local development, testing strategies, debugging, and contribution workflows for NeuralBudget developers.

## Table of Contents

- [Development Environment Setup](#development-environment-setup)
- [Testing Strategy](#testing-strategy)
- [Debugging](#debugging)
- [Performance Profiling](#performance-profiling)
- [Common Development Tasks](#common-development-tasks)
- [CI/CD Pipelines](#cicd-pipelines)
- [Troubleshooting Development Issues](#troubleshooting-development-issues)

---

## Development Environment Setup

### One-Time Setup

```bash
# Clone repository
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Update Rust and install components
rustup update
rustup component add rustfmt clippy llvm-tools-preview

# Create Python virtual environment
python3 -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# Install Python development tools
pip install --upgrade pip
pip install maturin black pylint pytest
```

### Build for Development

```bash
# Build Rust core
cargo build

# Build and install Python extension in editable mode
maturin develop

# Verify build
python3 -c "import neuralbudget; print('OK')"
```

### Environment Variables

Useful development variables:

```bash
# Verbose build output
export CARGO_VERBOSE=true

# Enable backtrace on panics
export RUST_BACKTRACE=full

# Run tests with output
export RUST_LOG=debug
```

---

## Testing Strategy

### Test Organization

| Test Type | Location | Purpose | Coverage |
|-----------|----------|---------|----------|
| **Unit** | `src/*.rs` inline tests | Individual function correctness | Core logic |
| **Functional** | `tests/functional_tests.rs` | SLO calculation correctness | End-to-end flows |
| **Integration** | `tests/integration_tests.rs` | Multi-component interaction | Full workflows |
| **Python** | `tests/python_*.py` | Python binding behavior | API surface |
| **Benchmarks** | `benches/` | Performance regression | Composite DAG |

### Running Tests

```bash
# All tests with all features
cargo test --all-features
python3 tests/python_convenience_tests.py
python3 tests/python_client_tests.py

# Single test file
cargo test --test integration_tests

# Filter by name
cargo test http_slo

# With output
cargo test -- --nocapture

# Generate coverage report
cargo llvm-cov --all-features --lib --tests --html
open target/llvm-cov/html/index.html
```

### Writing Tests

**Property-based tests** for invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_availability_bounded(success in 0u64..1000, total in 1u64..1000) {
        let avail = calculate_availability(success, total);
        prop_assert!((0.0..=1.0).contains(&avail));
    }
}
```

**Example-based tests** for behavior:

```rust
#[test]
fn test_http_slo_passes_within_threshold() {
    let slo = HttpSlo {
        latency_threshold_ms: 100.0,
        latency_percentile: 99.0,
        availability_threshold: 99.9,
    };
    
    let sample = HistogramSample {
        timestamp: 1000,
        success: 999,
        total: 1000,
        buckets: vec![],
        format: HistogramFormat::PrometheusHistogram,
    };
    
    let result = slo.evaluate_sample(&sample);
    assert!(result.passed);
}
```

**Python tests** with fixtures:

```python
def setUp(self):
    self.client = NeuralBudgetClient()
    self.config_path = "examples/http_slo.yaml"
    self.client.load_config(self.config_path)

def test_client_dispatches_correctly(self):
    result = self.client.evaluate(self.metric_data)
    self.assertIsInstance(result, dict)
    self.assertIn("passed", result)
```

### Maintaining Coverage

The project enforces **87% line coverage**:

```bash
# Check if coverage meets gate
cargo llvm-cov --all-features --lib --tests --summary-only --fail-under-lines 87

# Generate detailed report
cargo llvm-cov --all-features --lib --tests --html --output-path coverage

# Identify uncovered branches
cargo llvm-cov report --html
```

**When adding code:**
- Write tests first (TDD encouraged)
- Aim for 90%+ on new code
- Document why coverage doesn't reach untested paths (e.g., "defensive check for invariant")

---

## Debugging

### Rust Debugging

**Enable backtraces:**
```bash
RUST_BACKTRACE=full cargo test test_name
RUST_BACKTRACE=1 cargo run
```

**Use `dbg!` macro for quick inspection:**
```rust
let config = SloConfig::new(99.9, "7d");
dbg!(&config);  // Prints debug output to stderr
```

**Debug-print in tests:**
```rust
#[test]
fn test_with_debug_output() {
    let result = evaluate_sample(&sample);
    println!("Result: {:#?}", result);  // Pretty-printed
    assert!(result.passed);
}
```

**Using lldb (macOS) or gdb (Linux):**
```bash
# Compile with debug symbols
cargo build

# Start debugger
lldb target/debug/neuralbudget
(lldb) run
(lldb) breakpoint set -f src/core.rs -l 123
```

### Python Debugging

**Print debugging:**
```python
from pprint import pprint
result = client.evaluate(metric_data)
pprint(result)  # Pretty-print
```

**Using pdb (Python Debugger):**
```python
import pdb; pdb.set_trace()

# Or in Python 3.7+:
breakpoint()
```

**Type checking:**
```bash
# Install type checker
pip install mypy

# Check Python code
mypy python/
```

### PyO3 Debugging

**Check Python ↔ Rust type conversions:**
```python
import neuralbudget
config = neuralbudget.SloConfig(99.9, "7d")
print(type(config))  # <class 'neuralbudget.SloConfig'>
```

**Enable PyO3 debug output:**
```bash
PYTHONPATH=target/debug:$PYTHONPATH python3 -c "import neuralbudget; ..."
```

---

## Performance Profiling

### Benchmark Composite DAG Evaluation

```bash
# Run default benchmarks
cargo bench --bench composite_slo_dag

# With specific filters
cargo bench -- --bench-filter "chain_100"

# Generate comparison report
cargo bench -- --baseline main
```

**Benchmark results location:** `target/criterion/`

### CPU Profiling with Flamegraph

```bash
# Install flamegraph tool
cargo install flamegraph

# Profile a test
cargo flamegraph --test integration_tests -- test_name

# View result
open flamegraph.svg
```

### Memory Profiling

```bash
# Valgrind (Linux)
valgrind --leak-check=full cargo test --lib

# Instruments (macOS)
# Use Xcode's Instruments.app with the llvm-tools binary
```

### Latency Profiling

```rust
use std::time::Instant;

#[test]
fn profile_evaluation_latency() {
    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = slo.evaluate_sample(&sample);
    }
    let elapsed = start.elapsed();
    println!("10k evaluations: {:?}", elapsed);  // Should be < 100ms
}
```

---

## Common Development Tasks

### Adding a New SLO Type

1. **Define the type** in `src/core.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct NewSlo {
       pub threshold: f64,
       pub config: String,
   }
   
   impl NewSlo {
       pub fn evaluate_sample(&self, sample: &NewSample) -> NewSloEvaluation {
           // Implementation
       }
   }
   ```

2. **Add PyO3 wrapper** in `src/python.rs`:
   ```rust
   #[pyclass(name = "NewSlo")]
   pub struct PyNewSlo(pub NewSlo);
   
   #[pymethods]
   impl PyNewSlo {
       #[new]
       fn new(threshold: f64, config: String) -> Self {
           PyNewSlo(NewSlo { threshold, config })
       }
   }
   ```

3. **Add convenience helper** in `python/neuralbudget/convenience.py`:
   ```python
   def evaluate_new_once(sample: dict) -> NewEvaluationResult:
       """Evaluate a new SLO once."""
       slo = _native.NewSlo(...)
       return NewEvaluationResult(**slo.evaluate_sample(sample))
   ```

4. **Write tests** for all three layers
5. **Update documentation** with examples

### Extending the Python Client

1. **Add mode** to `EvaluationMode` TypedDict in `client.py`
2. **Implement dispatch** in `NeuralBudgetClient.evaluate()`
3. **Add convenience integration** in `convenience.py`
4. **Write integration tests** in `tests/python_client_tests.py`

### Adding an Alert Provider

1. **Implement sender** in `python/neuralbudget/alerting.py`:
   ```python
   def _send_myprovider(self, config, mode, profile, result):
       payload = json.dumps({
           "alert": result.get("passed") == False
       })
       # Send to provider API
   ```

2. **Add to dispatch** in `AlertDispatcher.send_violation()`
3. **Write tests** in `tests/python_alerting_tests.py`
4. **Document** in user guide

---

## CI/CD Pipelines

### GitHub Actions Workflows

| Workflow | File | Trigger | Purpose |
|----------|------|---------|---------|
| **CI** | `.github/workflows/ci.yml` | Push to main, PRs | Lint, test, coverage |
| **CD** | `.github/workflows/cd.yml` | Push to main | Build artifacts, upload |
| **Release** | `.github/workflows/release.yml` | Tag v* | Publish to PyPI, create release |

### Local CI Simulation

Run the full CI pipeline locally:

```bash
#!/bin/bash
set -e

echo "=== Formatting Check ==="
cargo fmt --all --check

echo "=== Linting Check ==="
cargo clippy --all-targets --all-features -- -D warnings

echo "=== Running Tests ==="
cargo test --lib --all-features
cargo test --all-targets --all-features
cargo test --doc --all-features
cargo test --tests --all-features

echo "=== Python Tests ==="
python3 tests/python_convenience_tests.py
python3 tests/python_client_tests.py

echo "=== Coverage Gate ==="
cargo llvm-cov --workspace --all-features --lib --tests --summary-only --fail-under-lines 87

echo "=== Wheel Build ==="
maturin build --release

echo "✅ All checks passed!"
```

Save as `.local-ci.sh`, then `bash .local-ci.sh` before pushing.

---

## Troubleshooting Development Issues

### Common Issues

**Issue: `cargo build` fails with PyO3 errors**
```
Solution: Ensure Python development headers are installed
  Ubuntu: sudo apt install python3-dev
  macOS: brew install python@3.11
  Windows: Python installer includes headers
```

**Issue: `maturin develop` fails**
```
Solution: Ensure maturin version matches requirements
  pip install --upgrade maturin
  maturin --version  # Should be 1.8+
```

**Issue: Tests fail with `thread panicked`**
```
Solution: Run with full backtrace
  RUST_BACKTRACE=full cargo test test_name
```

**Issue: Python tests import neuralbudget but it's not found**
```
Solution: Rebuild the extension
  maturin develop
  python3 -c "import neuralbudget"  # Verify
```

**Issue: Coverage report missing or incomplete**
```
Solution: Reinstall llvm-cov and clear cache
  cargo install --force cargo-llvm-cov
  rm -rf target/llvm-cov-target/
  cargo llvm-cov clean
  cargo llvm-cov --all-features --lib --tests --html
```

### Getting Help

1. **Check existing issues** on GitHub
2. **Read error messages carefully** — they often contain solutions
3. **Search documentation** in `docs/guides/`
4. **Ask in GitHub Discussions**
5. **Tag maintainers** for urgent issues

---

## Development Checklist

Before submitting a PR, verify:

- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test --all-features`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] Linting passes (`cargo clippy --all-targets --all-features -- -D warnings`)
- [ ] Coverage meets gate (≥87%)
- [ ] Documentation updated (README, guides, or comments)
- [ ] Commit messages follow conventional commits
- [ ] No unintended files committed
- [ ] Branch is up-to-date with main

---

**Happy coding! 🚀**
