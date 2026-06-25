# Contributing to NeuralBudget

Thank you for your interest in contributing to NeuralBudget! This guide will help you get started with development, testing, and submitting pull requests.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Pull Requests](#submitting-pull-requests)
- [Code Style & Standards](#code-style--standards)
- [Reporting Issues](#reporting-issues)

---

## Code of Conduct

This project is committed to providing a welcoming, inclusive environment for all contributors. Please:

- **Be respectful** — Treat all contributors with professionalism
- **Be constructive** — Provide helpful feedback and solutions
- **Be inclusive** — Welcome perspectives from diverse backgrounds
- **Report violations** — Use GitHub's reporting tools for conduct issues

---

## Getting Started

### Prerequisites

- **Rust**: 2021 edition
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```

- **Python**: 3.9+ with pip
  ```bash
  python3 --version  # Verify 3.9+
  ```

- **Build tools** (for native extension builds):
  ```bash
  pip install maturin
  ```

- **Git**: For version control

### Clone the Repository

```bash
git clone https://github.com/pristley/NeuralBudget.git
cd NeuralBudget
```

---

## Development Setup

### 1. Initialize Development Environment

```bash
# Set up Rust toolchain with required components
rustup update
rustup component add rustfmt clippy llvm-tools-preview

# Set up Python virtual environment
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install development dependencies
pip install --upgrade pip maturin
```

### 2. Build the Project

```bash
# Verify Rust compilation
cargo build

# Build the Python extension (editable)
maturin develop

# Verify Python import
python3 -c "import neuralbudget; print(neuralbudget.__version__ if hasattr(neuralbudget, '__version__') else 'OK')"
```

### 3. Validate Your Setup

```bash
# Run all tests
cargo test --all-features
python3 tests/python_convenience_tests.py
python3 tests/python_client_tests.py

# Run linting
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Making Changes

### Workflow

1. **Create a branch** for your feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/issue-number-description
   ```

2. **Make your changes** following the guidelines below

3. **Test locally** before committing

4. **Commit with clear messages**:
   ```bash
   git commit -m "feat: add new SLO type for batch processing"
   git commit -m "fix: resolve race condition in composite DAG"
   git commit -m "docs: update API reference with examples"
   ```

### Commit Message Format

Use conventional commits for clarity:

- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation only
- `test:` — Tests or test infrastructure
- `refactor:` — Code refactoring without behavior change
- `perf:` — Performance improvements
- `ci:` — CI/CD workflow changes
- `chore:` — Build tooling, dependencies, etc.

**Example:**
```
feat: add WeekdayAligned time window type

Implements calendar-aligned windows that respect business hours and 
timezone transitions for SLOs spanning multiple weeks.

- Add WeekdayAligned variant to TimeWindow enum
- Implement day-of-week boundary detection
- Add comprehensive unit tests
- Update convenience layer with preset

Closes #42
```

---

## Testing

### Run All Tests

```bash
# Full test suite with coverage
cargo test --all-features
python3 tests/python_convenience_tests.py
python3 tests/python_client_tests.py
```

### Run Specific Tests

```bash
# Rust: run a specific test
cargo test test_name

# Rust: filter tests by keyword
cargo test http_slo

# Rust: run integration tests only
cargo test --tests

# Python: run with verbose output
python3 -m pytest tests/python_client_tests.py -v
```

### Coverage Testing

```bash
# Generate coverage report (requires cargo-llvm-cov)
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --lib --tests --html

# View report
open target/llvm-cov/html/index.html  # macOS
xdg-open target/llvm-cov/html/index.html  # Linux
```

### Writing Tests

**Rust:** Add tests in the same file or use `tests/` directory:

```rust
#[test]
fn test_new_feature_behavior() {
    let input = SloConfig::new(99.9, "7d");
    let result = input.evaluate_sample(&sample);
    assert!(result.passed);
}
```

**Python:** Use unittest or pytest conventions:

```python
def test_new_feature_behavior(self):
    client = NeuralBudgetClient()
    result = client.evaluate(metric_data)
    self.assertTrue(result['passed'])
```

---

## Code Style & Standards

### Rust

Follow Rust conventions enforced by CI:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all --check

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

**Guidelines:**
- Use meaningful variable names
- Add doc comments (`///`) for public items
- Prefer `match` over nested `if` for exhaustive checks
- Keep functions under 100 lines where possible
- Use `Result<T>` for fallible operations

### Python

Follow PEP 8 with these preferences:

```bash
# Format code (uses black)
pip install black
black python/

# Run linting
pip install pylint
pylint python/
```

**Guidelines:**
- Use type hints on function signatures
- Add docstrings to all public functions
- Prefer explicit imports over star imports
- Keep functions focused on a single responsibility
- Use dataclasses for value objects

### Documentation

- Use Markdown for all documentation files
- Add code examples where helpful
- Keep lines under 100 characters
- Use consistent formatting for tables and code blocks

---

## Submitting Pull Requests

### Before Submitting

1. **Update your branch** with latest `main`:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

2. **Run full test suite** locally:
   ```bash
   cargo test --all-features
   cargo fmt --all --check
   cargo clippy --all-targets --all-features -- -D warnings
   python3 tests/python_*.py
   ```

3. **Verify coverage** hasn't decreased:
   ```bash
   cargo llvm-cov --all-features --lib --tests --summary-only --fail-under-lines 87
   ```

### PR Guidelines

**Title:** Use conventional commit format
```
feat: add composite SLO with weighted dependencies
fix: resolve calendar window timezone calculation
docs: add troubleshooting section to README
```

**Description:** Include:
- **What** — Clear description of changes
- **Why** — Motivation and problem being solved
- **How** — Implementation approach
- **Testing** — How you tested the changes
- **Breaking changes** — Any backward compatibility concerns

**Template:**
```markdown
## Description
Brief explanation of the change.

## Problem Statement
Why is this change needed?

## Solution
How does this PR address the problem?

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Checklist
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] No new warnings introduced
- [ ] Changelog entry added (if applicable)
```

### After Submitting

- **Respond to feedback** promptly and constructively
- **Make requested changes** in new commits (don't force-push)
- **Reference issues** using `Closes #123` or `Fixes #456`
- **Allow maintainers to merge** once approved

---

## Reporting Issues

### Security Vulnerabilities

**Do not open public issues for security vulnerabilities.**

Instead, email `security@example.com` with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

### Bug Reports

Include:
1. **Version** of NeuralBudget (`cargo tree | grep neuralbudget`)
2. **Environment** (OS, Python/Rust versions)
3. **Minimal reproducer** (code or config that triggers the bug)
4. **Expected vs. actual behavior**
5. **Error logs or stack trace**

**Template:**
```markdown
## Describe the bug
Clear description of what went wrong.

## Steps to reproduce
1. ...
2. ...
3. ...

## Expected behavior
What should happen.

## Actual behavior
What actually happened.

## Environment
- OS: macOS 14.0
- Rust: 1.75
- Python: 3.11
- NeuralBudget: 0.1.3

## Error logs
```
Stack trace or error output here
```
```

### Feature Requests

Provide:
1. **Use case** — Why you need this feature
2. **Proposed solution** — How it should work
3. **Alternatives** — Other approaches you've considered
4. **Context** — Related issues or discussions

---

## Project Structure for Contributors

See [`agentmap.md`](agentmap.md) for detailed architecture overview.

**Key directories:**
- `src/` — Rust core implementation
- `python/` — Python bindings and convenience layer
- `tests/` — Test suites
- `docs/` — User documentation
- `examples/` — Example scripts and configurations

---

## Getting Help

- **Documentation**: Check [`docs/guides/`](docs/guides/documentation-index.md)
- **Examples**: See [`examples/`](examples/)
- **Discussions**: Use GitHub Discussions
- **Issues**: Search existing issues before creating new ones
- **Maintainers**: Tag `@pristley` in discussions

---

## Recognition

Contributors are recognized in:
1. **CHANGELOG.md** — For significant changes
2. **GitHub Contributors** — Automatically updated
3. **Release notes** — For major releases

Thank you for contributing to NeuralBudget! 🎉
