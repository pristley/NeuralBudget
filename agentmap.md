# NeuralBudget Architecture & Module Map

This document provides a comprehensive overview of the NeuralBudget project structure, module responsibilities, and service interactions.

## Project Overview

**NeuralBudget** is a Rust-first SLO (Service Level Objective) toolkit with Python interoperability. It provides deterministic reliability analytics for service, ML, and GenAI workloads through:
- A strongly-typed Rust core (`src/`)
- Native Python bindings via PyO3 (`python/neuralbudget/`)
- Convenience helpers for notebooks and pipelines
- Integration with Prometheus, OpenTelemetry, and webhook alerting systems

---

## Directory Structure

```
NeuralBudget/
├── src/                          # Rust core implementation
│   ├── lib.rs                    # Module declarations and public exports
│   ├── core.rs                   # Core SLO models and calculations
│   ├── exporter.rs               # Prometheus metrics exporter
│   ├── otlp.rs                   # OpenTelemetry Protocol integration
│   ├── python.rs                 # Python/PyO3 FFI bindings
│   └── bin/                      # CLI binary
│       ├── main.rs               # CLI entry point
│       └── commands/             # CLI subcommands
│
├── python/
│   ├── neuralbudget/             # Native extension package
│   │   ├── __init__.py           # Package exports
│   │   ├── client.py             # High-level NeuralBudgetClient facade
│   │   ├── convenience.py        # Ergonomic helper layer
│   │   └── alerting.py           # Webhook alert dispatcher
│   │
│   └── neuralbudget_convenience/ # (Python-only convenience module)
│
├── tests/                        # Test suites
│   ├── functional_tests.rs       # Rust functional tests
│   ├── integration_tests.rs      # Rust integration tests
│   ├── cli_integration_tests.rs  # CLI integration tests
│   ├── python_client_tests.py    # NeuralBudgetClient tests
│   ├── python_alerting_tests.py  # Alerting integration tests
│   └── python_convenience_tests.py # Convenience layer tests
│
├── benches/                      # Performance benchmarks
│   └── composite_slo_dag.rs      # DAG evaluation performance tests
│
├── examples/                     # Example configurations and scripts
│   ├── python/                   # Python usage examples
│   ├── kubernetes/               # K8s deployment manifests
│   ├── grafana/                  # Grafana dashboard templates
│   ├── slo_http.yaml             # HTTP service SLO example
│   ├── slo_ml.yaml               # ML service SLO example
│   └── sample_http.json          # Sample metrics JSON
│
├── docs/                         # Project documentation
│   ├── guides/                   # User guides and walkthroughs
│   ├── reference/                # API and component references
│   ├── cli/                      # CLI documentation
│   ├── internal/                 # Internal design docs
│   └── plans/                    # Project planning
│
├── Cargo.toml                    # Rust dependencies and build config
├── pyproject.toml                # Python package metadata
└── Dockerfile                    # Docker build for CLI
```

---

## Module Responsibilities

### Rust Core (`src/`)

#### `lib.rs` — Module Orchestration
- **Purpose**: Declares all submodules and re-exports public API
- **Responsibility**: Maintains the public interface contract
- **Exports**: All types and functions needed by Python bindings and end-users

#### `core.rs` — SLO Primitives & Data Models
- **Purpose**: Defines all fundamental SLO types and calculation logic
- **Key Types**:
  - `SloConfig` — SLO metadata (target, window, schema versioning)
  - `ErrorBudget` — Budget remaining and burn velocity
  - `TimeWindow` — Rolling vs. calendar-aligned time windows
  - `HistogramSample`, `HistogramBucket` — Latency distribution samples
  - `HttpSlo` — Stateless HTTP/gRPC SLO (latency percentile + availability)
  - `StatefulSlo` — Stateful service evaluation (replication lag, queue depth, pool saturation)
  - `MlSlo` — ML serving hybrid SLO (latency score + drift score with weighted formula)
  - `GenAiSlo` — LLM workload SLO (throughput, TTFT, semantic similarity)
  - `CompositeSlo` — Dependency DAG evaluation with failure propagation
- **Responsibility**: Stateless computation; all business logic lives here
- **Key Properties**:
  - Deterministic (same inputs = same outputs, every time)
  - No external I/O dependencies
  - Thoroughly tested for correctness

#### `exporter.rs` — Prometheus Metrics Export
- **Purpose**: Converts SLO evaluation results into Prometheus exposition format
- **Responsibility**: 
  - Renders SLO metrics as Prometheus TEXT format
  - Handles label mapping and metric naming conventions
- **Use Cases**: Direct scraping by Prometheus, custom Prometheus client integration

#### `otlp.rs` — OpenTelemetry Integration
- **Purpose**: Ingests OpenTelemetry Protocol histogram samples
- **Responsibility**:
  - Parses OTLP JSON payloads (delta and cumulative formats)
  - Converts OTLP histograms to NeuralBudget HistogramSample
  - Handles format conversions (delta ↔ cumulative)
- **Use Cases**: Direct integration with OTLP collectors, Grafana Loki, observability platforms

#### `python.rs` — PyO3 FFI Bridge
- **Purpose**: Exposes Rust types and functions to Python
- **Responsibility**:
  - PyO3 type conversions (Rust ↔ Python objects)
  - Error mapping from Rust to Python exceptions
  - Helper extraction functions for dict/TypedDict conversion
  - Python wrapper classes (e.g., `PySloConfig`, `PyTimeWindow`)
- **Design**: Thin FFI layer; all logic remains in Rust

#### `bin/main.rs` — CLI Entry Point
- **Purpose**: Command-line interface for SLO operations
- **Subcommands**:
  - `eval` — Evaluate SLO against sample metrics
  - `gen-rules` — Generate Prometheus alerting rules
  - `check` — Validate SLO configurations
  - `serve` — HTTP server (future)
- **Design**: Modular subcommand architecture with error handling

---

### Python Bindings & Convenience Layer (`python/neuralbudget/`)

#### `client.py` — NeuralBudgetClient Facade
- **Purpose**: Stable, ergonomic entry point for notebooks and CI/CD pipelines
- **Key Methods**:
  - `NeuralBudgetClient.load_config(path)` — Load YAML/JSON SLO configs
  - `client.evaluate(metric_data)` — Evaluate metrics against loaded config
- **Responsibilities**:
  - Config parsing and validation
  - Mode dispatch (http, stateful, ml, genai, composite)
  - Optional return dataclass conversion
  - Alert dispatching on SLO violations
- **Design Pattern**: Configuration object → evaluation strategy dispatch

#### `convenience.py` — Ergonomic Helpers
- **Purpose**: Thin, low-boilerplate helpers around native API
- **Key Functions**:
  - `availability_snapshot()` — One-shot availability check
  - `evaluate_http_once()` — Single HTTP histogram evaluation
  - `evaluate_stateful_once()` — Single stateful service evaluation
  - `evaluate_ml_once()` — Single ML SLO evaluation
  - `evaluate_genai_once()` — Single GenAI SLO evaluation
- **Design Philosophy**: 
  - Minimal logic; delegates calculations to Rust
  - Preserves deterministic behavior
  - Dataclass result types for type safety
- **Profile System**: Pre-configured parameter sets (e.g., "aggressive", "conservative") for common scenarios

#### `alerting.py` — Webhook Alert Dispatcher
- **Purpose**: Sends SLO violation notifications to external systems
- **Supported Providers**:
  - **Slack**: Incoming webhooks with rich formatting
  - **PagerDuty**: Events API v2 for incident creation
  - **Opsgenie**: Alerts API v2 for multi-channel routing
- **Key Features**:
  - Private network detection (prevents leaking internal metrics)
  - Payload size limits (64 KB cap)
  - Structured error reporting
- **Security**: Environment-variable-based credential management

#### `__init__.py` — Package Initialization
- **Purpose**: Re-exports public API from submodules
- **Exports**: `NeuralBudgetClient`, convenience functions, alert dispatcher

---

## Key Service Interactions

### Data Flow: Evaluation Pipeline

```
┌──────────────────────────────────┐
│  Metric Input (YAML/JSON/Dict)   │
│  - Prometheus histogram           │
│  - OTLP payload                   │
│  - Raw request/stateful metrics   │
└──────────────────┬───────────────┘
                   │
                   ▼
        ┌──────────────────────┐
        │  NeuralBudgetClient  │
        │  (config dispatch)   │
        └──────────┬───────────┘
                   │
         ┌─────────┴─────────┐
         │                   │
         ▼                   ▼
    ┌────────────┐    ┌──────────────┐
    │Convenience │    │Native (Rust) │
    │ Layer      │    │ Core API     │
    └────────────┘    └──────────────┘
         │                   │
         └─────────┬─────────┘
                   │
                   ▼
        ┌──────────────────────┐
        │   SLO Calculation    │
        │  (core.rs logic)     │
        └──────────┬───────────┘
                   │
                   ▼
        ┌──────────────────────┐
        │  Evaluation Result   │
        │  (dataclass/dict)    │
        └──────────┬───────────┘
                   │
         ┌─────────┴─────────┐
         │                   │
         ▼                   ▼
    ┌──────────┐       ┌────────────┐
    │ Return   │       │ Alert      │
    │ to User  │       │ Dispatch   │
    │          │       │ (alerting) │
    └──────────┘       └────────────┘
```

### Config Loading & Schema Versioning

1. **User provides YAML/JSON config**
2. **Client parses config → `ClientConfigFile` TypedDict**
3. **Schema version validation** (prevents breaking changes)
4. **Mode-specific parameter extraction**
5. **Rust core initialized with extracted config**
6. **Evaluation ready**

### Composite SLO DAG Evaluation

1. **Service nodes** provide local SLO scores
2. **Dependency edges** define failure propagation
3. **DAG runner** (in `core.rs`):
   - Performs topological sort with cycle detection
   - Propagates failures down dependency chain with weighted penalties
   - Computes global System SLO score
4. **Result**: Service-level and system-level pass/fail decisions

---

## Dependency Graph

### Rust → Python
- **PyO3 (0.24.2)**: Language binding framework
- **Serde + serde_yaml/serde_json**: Serialization for configs
- **Criterion**: Benchmarking framework (dev-only)
- **Clap**: CLI argument parsing
- **Anyhow**: Error handling for CLI

### Python → Rust
- `neuralbudget` (native extension built from Rust core)
- `pathlib`, `dataclasses`, `json`, `urllib` (stdlib)

### External Services (Optional)
- **Prometheus**: For metrics scraping and alerting
- **OpenTelemetry**: For distributed trace context and histogram ingestion
- **Slack/PagerDuty/Opsgenie**: For webhook-based alerting
- **Docker**: For containerized distribution

---

## Testing Architecture

| Test Suite | Location | Purpose |
|------------|----------|---------|
| Functional (Rust) | `tests/functional_tests.rs` | Core SLO calculation correctness |
| Integration (Rust) | `tests/integration_tests.rs` | Multi-component interaction |
| CLI Integration (Rust) | `tests/cli_integration_tests.rs` | CLI subcommand behavior |
| Client (Python) | `tests/python_client_tests.py` | Config loading and dispatch |
| Convenience (Python) | `tests/python_convenience_tests.py` | Helper ergonomics and results |
| Alerting (Python) | `tests/python_alerting_tests.py` | Webhook dispatch behavior |
| Benchmarks (Rust) | `benches/composite_slo_dag.rs` | Composite DAG performance |

---

## Design Principles

### 1. **Determinism First**
- All calculations are pure functions
- No randomness, no external state
- Identical inputs always produce identical outputs

### 2. **Type Safety Across Language Boundary**
- Rust `derive(Serialize, Deserialize)` for schema
- Python `TypedDict` for config validation
- Comprehensive type hints in Python API

### 3. **Minimal Convenience Layer**
- Python convenience functions are thin wrappers
- All heavy lifting stays in Rust
- Simplifies maintenance and preserves correctness

### 4. **Configuration as Code**
- YAML/JSON configs version-controlled
- Schema versioning prevents silent incompatibilities
- Environment-variable interpolation for secrets (alerting)

### 5. **Clear Separation of Concerns**
- **Core** = Calculation logic
- **Exporter** = Format conversion (Prometheus)
- **OTLP** = Input format conversion (OpenTelemetry)
- **CLI** = Command-line interface
- **Python** = Language binding
- **Client** = Facade and orchestration
- **Convenience** = Ergonomics and profiles
- **Alerting** = Webhook dispatch

---

## Common Development Tasks

### Adding a New SLO Type
1. Define struct in `core.rs` with `#[derive(Serialize, Deserialize)]`
2. Implement evaluation method
3. Add PyO3 wrapper in `python.rs`
4. Add convenience helper in `convenience.py`
5. Add test case in appropriate test file

### Extending Alert Providers
1. Add provider logic to `alerting.py` `AlertDispatcher.send_violation()`
2. Define payload format for the provider's API
3. Add tests to `python_alerting_tests.py`
4. Document in user guide

### Adding Composite DAG Features
1. Extend `CompositeSlo` model in `core.rs`
2. Update cycle detection and topological sort if needed
3. Update result types in `convenience.py`
4. Add benchmark to `benches/composite_slo_dag.rs`

### Adding CLI Subcommands
1. Define new variant in `Commands` enum in `src/bin/main.rs`
2. Create new module in `src/bin/commands/`
3. Implement subcommand logic
4. Add integration tests to `tests/cli_integration_tests.rs`
5. Document in `docs/cli/USER_GUIDE.md`

---

## Further Reading

- **User Guide**: `docs/guides/getting-started.md`
- **CLI Guide**: `docs/cli/USER_GUIDE.md`
- **CLI Development**: `docs/cli/DEVELOPMENT.md`
- **Production Deployment**: `docs/guides/production-deployment.md`
- **API References**:
  - **Python API**: `docs/reference/api.md` — Complete Python extension and convenience layer reference
  - **Composite SLO DAG**: `docs/reference/composite-slo-dag.md` — Dependency graph schemas and scoring
  - **Convenience Layer**: `docs/reference/convenience-layer.md` — Helper functions and profile presets
