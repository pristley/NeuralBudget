# Composite SLO DAG Reference

This reference describes the Composite SLO dependency model used to evaluate a system-wide SLO from service-level objectives and dependency edges.

## 🎯 Unique Value Proposition

**Composite DAG is the only SLO mode that models inter-service dependencies and cascading failures.**

Unlike parallel SLO evaluation:

| Feature | Parallel SLOs | **Composite DAG** |
|---------|---------------|-------------------|
| **Per-service SLO** | ✅ Yes | ✅ Yes |
| **Dependency modeling** | ❌ No | ✅ **Yes** |
| **Failure propagation** | ❌ No | ✅ **Yes** |
| **Global SLO** | Manual aggregation | ✅ **Automatic** |
| **Topological ordering** | ❌ No | ✅ **Deterministic** |
| **Cycle detection** | ❌ No | ✅ **Automatic** |
| **Impact weighting** | ❌ No | ✅ **Per-service** |

### Why This Matters

**Without Composite DAG:**
```
Service A fails → "A: FAIL, B: PASS, C: PASS, Global: 67% PASS"
↑ Misleading: B & C actually degrade when A fails!
```

**With Composite DAG:**
```
Service A fails → "A: FAIL, B: DEGRADE (0.72), C: DEGRADE (0.58), Global: 42% FAIL"
↑ Accurate: Captures cascading impact across the system
```

## Real-World Use Cases

### 1. E-Commerce Checkout (Dependency Chain)
```
Payment API
    ↓ depends on
Fraud Detection Service
    ↓ depends on
ML Model Server
    ↓ depends on
Feature Store (Database)
```
**Value:** When Feature Store fails, Checkout fails—Composite DAG models this automatically.

### 2. Data Platform (Complex DAG)
```
              [Data Warehouse]
                    ↑
              [Transformation]
               ↙     ↑      ↘
        [Ingestion] [Analytics] [ML]
              ↑         ↑          ↑
     [Message Queue] [Cache] [ML Registry]
```
**Value:** Topological sort evaluates leaves first, preventing order-dependent bugs.

### 3. Multi-Region Deployment
```
[Global Load Balancer]
    ↙           ↓           ↘
[US-East]  [EU-West]  [Asia-Pacific]
   ↑          ↑           ↑
[DNS]  [Geo-Routing]  [Failover]
```
**Value:** Global SLO reflects which region failures impact users most (via impact_weight).

## Purpose

Use Composite SLO DAG evaluation when:

- service health is **dependency-aware**
- an upstream failure should **reduce or flag downstream SLO outcomes**
- you need a **single weighted global SLO** for the whole system
- you have **complex service topologies** requiring deterministic ordering
- you want **automatic cycle detection** and validation

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

## How It Works: Step-by-Step Evaluation

### Evaluation Pipeline

1. **Validate graph integrity** - Check for cycles, duplicates, unknown references
2. **Build topological order** - Deterministically order services (dependencies before dependents)
3. **Evaluate in topological order** - Process leaves first, propagate failures upward
4. **Apply failure penalties** - For each failed upstream, multiply downstream score
5. **Compute per-service status** - Each service gets local, effective, and pass/fail status
6. **Compute global SLO** - Weighted average of all effective scores
7. **Return global pass/fail** - Global SLO vs global_min_pass_score

### Failure Propagation Example

**Graph Setup:**
```
Database (db)
    ↑
API Server (api)  ← depends on db, penalty=0.25
    ↑
Web Gateway (gateway) ← depends on api, penalty=0.20
```

**Scenario 1: All Services Healthy**
```
Input:
  db: 0.98
  api: 0.95
  gateway: 0.92

Processing (topological order: db → api → gateway):
  db: local=0.98, effective=0.98, pass=✓
  api: local=0.95, no failed deps, effective=0.95, pass=✓
  gateway: local=0.92, no failed deps, effective=0.92, pass=✓

Result:
  Global: (0.98 + 0.95 + 0.92) / 3 = 0.95 ✓ PASS
```

**Scenario 2: Database Fails**
```
Input:
  db: 0.85 (below min_pass=0.90)
  api: 0.95
  gateway: 0.92

Processing (topological order: db → api → gateway):
  db: local=0.85, effective=0.85, pass=✗ FAIL

  api: local=0.95
       db failed! Apply penalty: 0.95 × (1.0 - 0.25) = 0.71
       effective=0.71, pass=✗ FAIL (0.71 < 0.90)

  gateway: local=0.92
           api failed! Apply penalty: 0.92 × (1.0 - 0.20) = 0.74
           effective=0.74, pass=✗ FAIL (0.74 < 0.90)

Result:
  Global: (0.85 + 0.71 + 0.74) / 3 = 0.77 ✗ FAIL
  
Key: ALL services fail due to database failure—this is the CORRECT behavior
that Composite DAG captures!
```

**Scenario 3: Partial Degradation with Multiple Paths**
```
Graph:
  Cache (cache) [penalty from db: 0.15]
  DB (db)
  API (api) [depends on db (0.20) and cache (0.10)]
  Gateway (gateway) [depends on api (0.20)]

Input:
  cache: 0.98
  db: 0.82 (FAIL)
  api: 0.93
  gateway: 0.91

Processing (topological order: cache, db → api → gateway):
  cache: 0.98, pass=✓
  db: 0.82, pass=✗
  
  api: local=0.93
       db failed (penalty 0.20): 0.93 × 0.80 = 0.744
       cache ok, no penalty
       effective=0.744, pass=✗
  
  gateway: local=0.91
           api failed (penalty 0.20): 0.91 × 0.80 = 0.728
           effective=0.728, pass=✗

Result: Multi-dependency failures compound realistically
```

### Key Mechanics Explained

**Impact Weight:**
- Used in final global SLO calculation: `weighted_average = Σ(score × weight) / Σ(weight)`
- Higher weight = service failures more heavily impact global SLO
- Example: Payment service gets weight=3.0, Debug service gets weight=0.5

**Failure Penalty:**
- Applied when upstream dependency fails
- Range: 0.0 (no impact) to 1.0 (complete failure)
- Example: Cache failure (penalty=0.10) has less impact than Database failure (penalty=0.30)

**Min Pass Score:**
- Per-service threshold for pass/fail status
- Independent of global SLO threshold
- Allows different tiers (critical services require 0.95, non-critical require 0.85)

**Global Min Pass Score:**
- System-wide threshold
- Determines if entire system is considered healthy
- Example: 0.90 means global SLO must be ≥90% for PASS

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

### Python Example 1: E-Commerce Checkout Pipeline

```python
from neuralbudget import (
    CompositeServiceSlo,
    CompositeDependencyEdge,
    CompositeSloGraph,
    evaluate_composite_slo_graph,
)

# E-commerce checkout: Payment Gateway → Fraud Detection → ML Model Server → Feature Store
graph = CompositeSloGraph(
    services=[
        CompositeServiceSlo(
            service="feature_store",
            local_score=0.98,
            min_pass_score=0.95,
            impact_weight=3.0,  # Critical: everything depends on it
        ),
        CompositeServiceSlo(
            service="ml_model_server",
            local_score=0.96,
            min_pass_score=0.95,
            impact_weight=2.5,
        ),
        CompositeServiceSlo(
            service="fraud_detection",
            local_score=0.94,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="payment_gateway",
            local_score=0.99,
            min_pass_score=0.99,  # Strictest requirement
            impact_weight=4.0,  # Most critical
        ),
    ],
    dependencies=[
        CompositeDependencyEdge(
            dependency="feature_store",
            dependent="ml_model_server",
            failure_penalty=0.40,  # ML model severely degraded without features
        ),
        CompositeDependencyEdge(
            dependency="ml_model_server",
            dependent="fraud_detection",
            failure_penalty=0.35,
        ),
        CompositeDependencyEdge(
            dependency="fraud_detection",
            dependent="payment_gateway",
            failure_penalty=0.50,  # Can't process payments without fraud check
        ),
    ],
    global_min_pass_score=0.95,
)

result = evaluate_composite_slo_graph(graph)

print(f"Topological Order: {result.topological_order}")
# Output: ['feature_store', 'ml_model_server', 'fraud_detection', 'payment_gateway']

print(f"\nPer-Service Status:")
for svc in result.services:
    print(f"  {svc.service}:")
    print(f"    Local: {svc.local_score:.2f}")
    print(f"    Effective: {svc.effective_score:.2f}")
    print(f"    Pass: {svc.pass}")
    if svc.failed_dependencies:
        print(f"    Failed Deps: {svc.failed_dependencies}")

print(f"\nGlobal SLO: {result.global_slo:.2f}")
print(f"Global Pass: {result.global_pass}")
```

### Python Example 2: Degradation Simulation (Feature Store Fails)

```python
# Same graph, but feature_store scores 0.85 (below 0.95 min_pass_score)
graph_degraded = CompositeSloGraph(
    services=[
        CompositeServiceSlo(
            service="feature_store",
            local_score=0.85,  # DEGRADED
            min_pass_score=0.95,
            impact_weight=3.0,
        ),
        CompositeServiceSlo(
            service="ml_model_server",
            local_score=0.96,
            min_pass_score=0.95,
            impact_weight=2.5,
        ),
        CompositeServiceSlo(
            service="fraud_detection",
            local_score=0.94,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="payment_gateway",
            local_score=0.99,
            min_pass_score=0.99,
            impact_weight=4.0,
        ),
    ],
    dependencies=[
        # Same dependencies as above...
        CompositeDependencyEdge(
            dependency="feature_store",
            dependent="ml_model_server",
            failure_penalty=0.40,
        ),
        CompositeDependencyEdge(
            dependency="ml_model_server",
            dependent="fraud_detection",
            failure_penalty=0.35,
        ),
        CompositeDependencyEdge(
            dependency="fraud_detection",
            dependent="payment_gateway",
            failure_penalty=0.50,
        ),
    ],
    global_min_pass_score=0.95,
)

result = evaluate_composite_slo_graph(graph_degraded)

# Output shows cascading failure:
# feature_store: FAIL (0.85 < 0.95)
#   ↓ penalty 0.40
# ml_model_server: FAIL (0.96 × 0.60 = 0.576 < 0.95)
#   ↓ penalty 0.35
# fraud_detection: FAIL (0.94 × 0.65 = 0.611 < 0.90)
#   ↓ penalty 0.50
# payment_gateway: FAIL (0.99 × 0.50 = 0.495 < 0.99)
#
# Global: 0.39 ✗ FAIL (well below 0.95 threshold)

print(f"After Feature Store Failure:")
print(f"  Global SLO: {result.global_slo:.2f} (was 0.97)")
print(f"  Global Pass: {result.global_pass} (was True)")
print(f"  Failed Services: {sum(1 for s in result.services if not s.pass)} / 4")
```

### Python Example 3: Multi-Region Deployment

```python
# Global load balancer with three regional deployments
# Each region has DB, API, and Sentry (observability)

graph_multi_region = CompositeSloGraph(
    services=[
        # US-East Region
        CompositeServiceSlo(
            service="db_us_east",
            local_score=0.99,
            min_pass_score=0.95,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="api_us_east",
            local_score=0.98,
            min_pass_score=0.95,
            impact_weight=2.5,
        ),
        # EU-West Region
        CompositeServiceSlo(
            service="db_eu_west",
            local_score=0.97,
            min_pass_score=0.95,
            impact_weight=1.5,  # Lower traffic = lower impact
        ),
        CompositeServiceSlo(
            service="api_eu_west",
            local_score=0.96,
            min_pass_score=0.95,
            impact_weight=1.5,
        ),
        # Asia-Pacific Region
        CompositeServiceSlo(
            service="db_asia_pac",
            local_score=0.95,
            min_pass_score=0.95,
            impact_weight=1.0,
        ),
        CompositeServiceSlo(
            service="api_asia_pac",
            local_score=0.94,
            min_pass_score=0.90,
            impact_weight=1.0,
        ),
    ],
    dependencies=[
        # US-East
        CompositeDependencyEdge(
            dependency="db_us_east",
            dependent="api_us_east",
            failure_penalty=0.60,
        ),
        # EU-West
        CompositeDependencyEdge(
            dependency="db_eu_west",
            dependent="api_eu_west",
            failure_penalty=0.60,
        ),
        # Asia-Pacific
        CompositeDependencyEdge(
            dependency="db_asia_pac",
            dependent="api_asia_pac",
            failure_penalty=0.60,
        ),
    ],
    global_min_pass_score=0.90,
)

result = evaluate_composite_slo_graph(graph_multi_region)

# Global SLO reflects weighted average:
# (0.99×2.0 + 0.98×2.5 + 0.97×1.5 + 0.96×1.5 + 0.95×1.0 + 0.94×1.0) / 10.0
# = 9.56 / 10.0 = 0.956 ✓ PASS

print(f"Multi-Region Global SLO: {result.global_slo:.3f}")

# If US-East DB fails (0.75):
# api_us_east: 0.98 × 0.40 = 0.392 (FAIL)
# This weights heavily (2.5) in global calculation → significant impact
```

### Python Example 4: Dict/JSON Integration

```python
import json
from neuralbudget import CompositeServiceSlo, CompositeDependencyEdge, CompositeSloGraph

# Build from dicts
services_dict = [
    {
        "service": "cache",
        "local_score": 0.98,
        "min_pass_score": 0.85,
        "impact_weight": 1.5,
    },
    {
        "service": "database",
        "local_score": 0.96,
        "min_pass_score": 0.95,
        "impact_weight": 3.0,
    },
]

dependencies_dict = [
    {
        "dependency": "cache",
        "dependent": "database",
        "failure_penalty": 0.25,
    }
]

services = [CompositeServiceSlo(**s) for s in services_dict]
dependencies = [CompositeDependencyEdge(**d) for d in dependencies_dict]

graph = CompositeSloGraph(
    services=services,
    dependencies=dependencies,
    global_min_pass_score=0.90,
)

# Evaluate and convert result back to dict
result = evaluate_composite_slo_graph(graph)
result_dict = {
    "topological_order": result.topological_order,
    "global_slo": float(result.global_slo),
    "global_pass": result.global_pass,
    "services": [
        {
            "service": s.service,
            "local_score": float(s.local_score),
            "effective_score": float(s.effective_score),
            "pass": s.pass,
            "dependency_adjusted": s.dependency_adjusted,
            "failed_dependencies": s.failed_dependencies,
        }
        for s in result.services
    ],
}

# Print as JSON
print(json.dumps(result_dict, indent=2))
```

## Advanced Patterns

### Pattern 1: Conditional Failures (Simulating Errors)

```python
# Simulate different failure scenarios by varying scores
def simulate_failure_impact(graph, failed_service: str, degradation: float):
    """
    Simulate what happens if a service score drops.
    degradation: 0.0-1.0 where 1.0 = total failure
    """
    modified_services = []
    for svc in graph.services:
        if svc.service == failed_service:
            new_score = svc.local_score * (1.0 - degradation)
        else:
            new_score = svc.local_score
        modified_services.append(
            CompositeServiceSlo(
                service=svc.service,
                local_score=new_score,
                min_pass_score=svc.min_pass_score,
                impact_weight=svc.impact_weight,
            )
        )
    
    modified_graph = CompositeSloGraph(
        services=modified_services,
        dependencies=graph.dependencies,
        global_min_pass_score=graph.global_min_pass_score,
    )
    return evaluate_composite_slo_graph(modified_graph)

# Test what happens if database degrades by 30%
result = simulate_failure_impact(graph, "database", 0.30)
print(f"Database 30% degradation → Global SLO: {result.global_slo:.2f}")

# Test what happens if cache fails completely
result = simulate_failure_impact(graph, "cache", 1.0)
print(f"Cache 100% failure → Global SLO: {result.global_slo:.2f}")
```

### Pattern 2: Dependency Weighting for Severity

```python
# Different penalty strategies based on service criticality
FAILURE_PENALTIES = {
    "database": 0.50,      # Severe penalty - everything depends on it
    "cache": 0.10,         # Light penalty - can degrade gracefully
    "logging": 0.02,       # Minimal penalty - not user-facing
    "payment_processor": 0.70,  # Severe - revenue impact
    "analytics": 0.01,     # Minimal - non-critical
}

# Use this to build dynamic dependency edges
def build_dependency_edges(dependencies_spec: list[dict]) -> list[CompositeDependencyEdge]:
    edges = []
    for dep_spec in dependencies_spec:
        service = dep_spec["dependency"]
        penalty = FAILURE_PENALTIES.get(service, 0.25)  # default 0.25
        edges.append(
            CompositeDependencyEdge(
                dependency=dep_spec["dependency"],
                dependent=dep_spec["dependent"],
                failure_penalty=penalty,
            )
        )
    return edges
```

### Pattern 3: Monitoring Integration

```python
# Fetch real SLO scores from monitoring system and evaluate
def evaluate_from_metrics(service_metrics: dict) -> CompositeSloGraph:
    """
    Build graph from live metrics.
    service_metrics: {"service_name": 0.95, ...}
    """
    services = [
        CompositeServiceSlo(
            service=name,
            local_score=score,
            min_pass_score=0.90,  # or from service config
            impact_weight=1.0,    # or from topology
        )
        for name, score in service_metrics.items()
    ]
    
    # Dependencies would typically come from a config or service registry
    # This is a simplified example
    return CompositeSloGraph(
        services=services,
        dependencies=[],  # Load from your service mesh/registry
        global_min_pass_score=0.90,
    )

# Example: Get metrics from Prometheus-like system
live_metrics = {
    "database": 0.97,
    "cache": 0.99,
    "api": 0.95,
}

graph = evaluate_from_metrics(live_metrics)
result = evaluate_composite_slo_graph(graph)
```
