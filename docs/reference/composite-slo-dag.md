# Composite SLO DAG Reference

This reference describes the Composite SLO dependency model used to evaluate a system-wide SLO from service-level objectives and dependency edges.

## Purpose

Use Composite SLO DAG evaluation when:

- service health is dependency-aware
- an upstream failure should reduce or flag downstream SLO outcomes
- you need a single weighted global SLO for the whole system

## Core Types

### CompositeServiceSlo

Per-service objective node.

Fields:

- `service: String`.
- `local_score: f64` in `[0.0, 1.0]` before dependency propagation.
- `min_pass_score: f64` in `[0.0, 1.0]` for node pass/fail.
- `impact_weight: f64` for global weighted aggregation.

### CompositeDependencyEdge

Directed dependency edge.

Fields:

- `dependency: String` (upstream service).
- `dependent: String` (downstream service).
- `failure_penalty: f64` in `[0.0, 1.0]` applied when upstream fails.

### CompositeSloGraph

Top-level graph input.

Fields:

- `services: Vec<CompositeServiceSlo>`.
- `dependencies: Vec<CompositeDependencyEdge>`.
- `global_min_pass_score: f64` in `[0.0, 1.0]`.

### CompositeSloEvaluation

Evaluation output.

Fields:

- `topological_order: Vec<String>`.
- `services: Vec<CompositeServiceSloEvaluation>`.
- `global_slo: f64` in `[0.0, 1.0]`.
- `global_pass: bool`.

### CompositeServiceSloEvaluation

Per-service evaluation output.

Fields:

- `service: String`.
- `local_score: f64`.
- `effective_score: f64` after dependency propagation.
- `min_pass_score: f64`.
- `dependency_adjusted: bool`.
- `failed_dependencies: Vec<String>`.
- `pass: bool`.

## Evaluation Semantics

1. Validate graph integrity.
2. Build deterministic topological order.
3. Evaluate services in order.
4. For each failed upstream dependency, multiply downstream score by `(1.0 - failure_penalty)`.
5. Compute per-service pass/fail using `effective_score >= min_pass_score`.
6. Compute `global_slo` via weighted average of effective scores using `impact_weight`.
7. Compute `global_pass` using `global_slo >= global_min_pass_score`.

## Error Conditions

Composite graph evaluation returns an error for:

- duplicate service names
- duplicate dependency edges
- unknown service references in dependency edges
- self-dependency edges
- cycles in dependency graph

## Rust Example

```rust
use neuralbudget::{
    evaluate_composite_slo, CompositeDependencyEdge, CompositeServiceSlo, CompositeSloGraph,
};

let graph = CompositeSloGraph {
    services: vec![
        CompositeServiceSlo {
            service: "service_a".to_string(),
            local_score: 0.72,
            min_pass_score: 0.9,
            impact_weight: 2.0,
        },
        CompositeServiceSlo {
            service: "service_b".to_string(),
            local_score: 0.97,
            min_pass_score: 0.9,
            impact_weight: 3.0,
        },
    ],
    dependencies: vec![CompositeDependencyEdge {
        dependency: "service_a".to_string(),
        dependent: "service_b".to_string(),
        failure_penalty: 0.2,
    }],
    global_min_pass_score: 0.85,
};

let result = evaluate_composite_slo(&graph).unwrap();
assert_eq!(result.services.len(), 2);
```

## Python API Surface (PyO3)

The native `neuralbudget` module exposes:

- `CompositeServiceSlo`
- `CompositeDependencyEdge`
- `CompositeSloGraph`
- `CompositeServiceSloEvaluation`
- `CompositeSloEvaluation`
- `evaluate_composite_slo_graph(...)`

These classes support dict conversion and JSON/YAML serialization like other native models.
