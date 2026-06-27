# Glossary of Terms

**Last Updated:** June 27, 2026

This glossary defines key terms, acronyms, and concepts used throughout NeuralBudget documentation. Use this reference to understand terminology consistently across all guides and API documentation.

---

## A

**Adaptive Windowing** — Automatic memory management for `StreamingAggregator` that removes old measurements when ingestion exceeds 15,000 samples per second, keeping memory under 4 MB. Activated automatically; no configuration required.

**Availability** — Percentage of requests that succeeded without error. Calculated as `(successful_requests / total_requests) * 100`. Example: 9,995 successful requests out of 10,000 total = 99.95% availability.

---

## B

**Burn Rate** — Speed at which an error budget is consumed over time. Calculated as `(errors_in_window / SLO_target_errors_for_window)`. Example: If SLO target is 99.9% availability and you have 1 error in 1 hour, burn rate is 1.0x (consuming budget at 1x speed). Higher burn rates trigger alerts.

**Budget Remaining** — The time window during which the SLO can continue to fail before violating the SLO target. Calculated as `error_budget_seconds / burn_rate`. If error budget is 3,600 seconds and burn rate is 2.0x, you have 1,800 seconds (30 minutes) before SLO violation.

---

## C

**Composite SLO** — Service dependency model that evaluates a DAG (directed acyclic graph) of services, propagating failures from dependencies to dependents. Unlike parallel evaluation, composite SLOs model inter-service relationships and failure impact. See `CompositeSloGraph`.

**CompositeSloGraph** — Rust class that evaluates service dependency DAGs with topological sort and failure propagation. Returns both individual service pass/fail status and global SLO status. Not available in Phase 2; documented for future reference.

**Config Schema** — YAML/JSON structure defining SLO evaluation parameters. Versioned to maintain backward compatibility. Current schema version is 1. See `docs/guides/user-guide.md` for config examples.

---

## D

**DAG** — Directed Acyclic Graph. A graph structure where edges flow in one direction and have no cycles. Used in Composite SLO evaluation to model service dependencies: nodes are services, edges represent dependencies.

**Determinism** — Property that identical inputs always produce identical outputs, regardless of language, runtime, or execution order. NeuralBudget is deterministic by design: all calculations are pure functions implemented in Rust.

**Drift** — Change in model behavior over time. In ML workloads, indicates the model's predictions no longer match real-world data. NeuralBudget detects drift using statistical tests (KS test, MAD). See `docs/reference/anomaly_drift_detection.md`.

---

## E

**Error Budget** — Allowable time window during which an SLO can fail before violating the target. Calculated as `(1 - SLO_target) * time_window`. Example: 99.9% SLO over 30 days = 0.1% * 30 days = 43.2 minutes of allowed downtime.

**Evaluation Result** — Output of SLO evaluation. Structure varies by SLO mode but always includes `pass` (boolean), `score` (0.0-1.0), and mode-specific fields. See `docs/reference/api.md` for mode-specific result structures.

---

## G

**GenAI Workload SLO** — SLO evaluation for large language models (LLMs) and generative AI systems. Measures throughput (TPS), responsiveness (TTFT), and semantic quality. See `docs/guides/genai_connectors.md`.

**GIL** — Global Interpreter Lock. Python's mechanism for thread synchronization that prevents true parallel execution of Python bytecode. NeuralBudget releases the GIL during `evaluate()` calls, allowing Rust code to run on multiple CPU cores while Python waits.

**Global SLO** — Overall service reliability status considering all dependencies in a Composite SLO DAG. Calculated as boolean: true if all services pass, false if any service fails.

---

## H

**HTTP SLO** — SLO evaluation for HTTP services. Combines availability (success rate) with latency percentiles. Supports latency thresholds at configurable percentiles (p50, p95, p99, etc.). See `docs/reference/api.md`.

---

## K

**KS Test** — Kolmogorov-Smirnov statistical test. Compares two probability distributions to detect when data has shifted significantly. Used in anomaly detection to identify drift. See `docs/reference/anomaly_drift_detection.md`.

---

## M

**MAD** — Median Absolute Deviation. Robust measure of variability that detects outliers. Calculated as median of absolute deviations from the median. More stable than standard deviation for non-normal distributions. Used in anomaly detection.

**ML Serving SLO** — SLO evaluation for machine learning model serving. Combines prediction latency, GPU utilization, model confidence, and data drift. See `docs/guides/genai_connectors.md`.

**Mode** — SLO evaluation type selected in config. Options: `http`, `stateful`, `ml`, `genai`, `composite` (future). Each mode has specific parameters and result formats.

---

## O

**OTLP** — OpenTelemetry Protocol. Standard format for collecting and exporting metrics, traces, and logs. NeuralBudget can ingest metrics from OpenTelemetry Collectors. See `docs/guides/prometheus-scraping-examples.md`.

---

## P

**ParallelMetricBatch** — Rust class for evaluating independent metrics in parallel. Each metric is evaluated independently against its threshold with no dependency modeling. Returns results as list of (id, value, threshold, pass, score) tuples. See `PARALLEL_SLO_API_REFERENCE.md`.

**Pass Score** — Normalized score (0.0-1.0) representing how much a metric exceeds its threshold. Calculated as `min(value / threshold, 1.0)`. Score of 1.0 means metric exceeded or met threshold; 0.5 means metric is 50% of threshold.

**Percentile Latency** — Latency value at a specific percentile. Example: p99 latency = latency value at 99th percentile (99% of requests are faster). Higher percentiles capture tail latency (slower requests).

---

## S

**SLO** — Service Level Objective. A target reliability metric for a service. Example: "99.9% availability" or "p99 latency < 200ms". SLOs are measurable, actionable targets that define acceptable service behavior.

**Score** — Normalized metric evaluation (0.0-1.0). Represents how much a metric meets its threshold. For composite DAGs, global score is the mean of all individual service scores.

**Stateful Service SLO** — SLO evaluation for services with state (databases, caches, queues). Measures replication lag, queue depth, connection pool saturation. See `docs/reference/api.md`.

**StreamingAggregator** — Rust class for collecting high-frequency metrics (20,000+ samples/second) and computing windowed statistics. Provides automatic memory management and moving averages. See `docs/reference/streaming-aggregator.md`.

---

## T

**Threshold** — Target value for an SLO metric. If metric value >= threshold, the metric passes. Example: latency threshold of 200ms means requests under 200ms pass.

**Topological Sort** — Ordering of DAG nodes such that all dependencies precede dependents. Used in Composite SLO evaluation to ensure failure propagation flows in correct direction.

**TTL** — Time To Live. Duration before data expires or is automatically removed. In `StreamingAggregator`, measurements older than TTL are pruned during high-load periods.

**TTFT** — Time To First Token. Latency from request submission to first token in LLM response. Key SLO metric for generative AI workloads. See `docs/guides/genai_connectors.md`.

---

## W

**Windowed Average** — Mean value computed over a time window. `StreamingAggregator.get_moving_average(timestamp, window_ms)` returns windowed average within the specified time window. Used to smooth out spiky metrics.

**Window Size** — Duration of time window (in milliseconds) for computing moving averages. Larger windows smooth noise; smaller windows capture recent changes. Common window sizes: 100ms, 1s, 5s.

---

## Acronyms Reference

| Acronym | Full Name | Where Used |
|---------|-----------|-----------|
| **SLO** | Service Level Objective | Throughout all documentation |
| **DAG** | Directed Acyclic Graph | Composite SLO evaluation |
| **OTLP** | OpenTelemetry Protocol | Prometheus integration |
| **GIL** | Global Interpreter Lock | Python performance discussion |
| **TTL** | Time To Live | Streaming aggregator |
| **TTFT** | Time To First Token | GenAI workload SLOs |
| **KS test** | Kolmogorov-Smirnov test | Anomaly detection |
| **MAD** | Median Absolute Deviation | Anomaly detection |
| **TPS** | Throughput (tokens per second) | GenAI workload SLOs |
| **P99** | 99th Percentile | Latency SLOs |
| **P95** | 95th Percentile | Latency SLOs |
| **P50** | 50th Percentile (Median) | Latency SLOs |

---

## SLO Modes Reference

| Mode | Purpose | Use When | Key Parameters |
|------|---------|----------|---|
| **http** | HTTP service reliability | Evaluating API availability + latency | `latency_threshold_ms`, `percentile`, `success_rate_target` |
| **stateful** | Stateful service reliability | Evaluating databases, caches, queues | `replication_lag_ms`, `queue_depth`, `pool_saturation` |
| **ml** | ML model serving reliability | Evaluating ML inference systems | `model_latency_ms`, `gpu_utilization`, `confidence_threshold`, `drift_threshold` |
| **genai** | GenAI workload reliability | Evaluating LLMs and generative AI | `ttft_ms`, `throughput_tps`, `quality_score` |
| **composite** | Service dependency DAGs | Evaluating multi-service systems | `service_graph` (YAML format) |

---

## Key Concepts

### Reliability Hierarchy
1. **Availability** (most basic) — % of requests that succeeded
2. **Latency SLO** (more specific) — % of requests under a latency threshold
3. **Error Budget** (business-level) — Allowable time to miss SLO before penalty
4. **Burn Rate** (operational) — Speed of error budget consumption

### SLO Evaluation Flow
1. **Config** (define SLO params) → 
2. **Metrics** (collect data) → 
3. **Evaluation** (compute pass/fail) → 
4. **Alert** (notify if violated)

### Scoring Models
- **Pass Score** — How much does metric exceed threshold? (0.0-1.0)
- **Composite Score** — How do multiple services combine? (mean score of all services)
- **Effective Score** — After failure propagation in DAG? (depends on dependencies)

---

## See Also

- [API Reference](api.md) — Complete function signatures and return types
- [Error Reference](errors.md) — Error codes, root causes, and solutions
- [Getting Started](../guides/getting-started.md) — Quick tutorial
- [User Guide](../guides/user-guide.md) — Comprehensive feature guide
