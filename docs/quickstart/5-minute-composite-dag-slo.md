# 5-Minute Composite DAG SLO Guide

**⏱️ Time: 5 minutes | Level: Intermediate | Unique Value: Failure Propagation**

## What is Composite SLO DAG?

Composite DAG models **inter-service dependencies** and automatically propagates failures across your system. It answers: *"When Database fails, how does that impact my API, and ultimately, my global SLO?"*

```
Database fails (0.85) 
    ↓
API degrades (0.72 effective) 
    ↓
Web Gateway fails (0.58 effective)
    ↓
Global SLO: 58% FAIL ← Correct!
```

**Without Composite DAG:** You'd report "Database: FAIL, API: PASS, Gateway: PASS, Global: 67% PASS" — **misleading!**

---

## Quick Start (2 minutes)

### 1. Create `composite-slo.yaml`

```yaml
# Composite SLO configuration
services:
  - name: database
    local_score: 0.96
    min_pass_score: 0.95
    impact_weight: 3.0
    
  - name: api_server
    local_score: 0.95
    min_pass_score: 0.95
    impact_weight: 2.5
    
  - name: web_gateway
    local_score: 0.94
    min_pass_score: 0.90
    impact_weight: 2.0

dependencies:
  - dependency: database
    dependent: api_server
    failure_penalty: 0.25  # API degrades 25% if DB fails
    
  - dependency: api_server
    dependent: web_gateway
    failure_penalty: 0.20  # Gateway degrades 20% if API fails

global_min_pass_score: 0.95
```

### 2. Create `metrics.json` (Sample Data)

```json
{
  "timestamp": "2024-01-15T14:30:00Z",
  "services": [
    {
      "name": "database",
      "local_score": 0.96
    },
    {
      "name": "api_server",
      "local_score": 0.95
    },
    {
      "name": "web_gateway",
      "local_score": 0.94
    }
  ]
}
```

### 3. Run Evaluation (Python)

```python
from neuralbudget import (
    CompositeServiceSlo,
    CompositeDependencyEdge,
    CompositeSloGraph,
    evaluate_composite_slo_graph,
)

# Build graph
graph = CompositeSloGraph(
    services=[
        CompositeServiceSlo(
            service="database",
            local_score=0.96,
            min_pass_score=0.95,
            impact_weight=3.0,
        ),
        CompositeServiceSlo(
            service="api_server",
            local_score=0.95,
            min_pass_score=0.95,
            impact_weight=2.5,
        ),
        CompositeServiceSlo(
            service="web_gateway",
            local_score=0.94,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
    ],
    dependencies=[
        CompositeDependencyEdge(
            dependency="database",
            dependent="api_server",
            failure_penalty=0.25,
        ),
        CompositeDependencyEdge(
            dependency="api_server",
            dependent="web_gateway",
            failure_penalty=0.20,
        ),
    ],
    global_min_pass_score=0.95,
)

# Evaluate
result = evaluate_composite_slo_graph(graph)

# Output
print(f"Topological Order: {result.topological_order}")
# → ['database', 'api_server', 'web_gateway']

for svc in result.services:
    print(f"{svc.service}: {svc.effective_score:.2f} {'✓' if svc.pass else '✗'}")

print(f"Global SLO: {result.global_slo:.2f} {'✓ PASS' if result.global_pass else '✗ FAIL'}")
```

### Expected Output

```
Topological Order: ['database', 'api_server', 'web_gateway']

database: 0.96 ✓
api_server: 0.95 ✓
web_gateway: 0.94 ✓

Global SLO: 0.95 ✓ PASS
```

---

## Make It FAIL: Experiments

### Experiment 1: Database Degradation

**Change:** Database score from 0.96 → 0.80

```python
# Modify services list
services=[
    CompositeServiceSlo(
        service="database",
        local_score=0.80,  # ← DEGRADED
        min_pass_score=0.95,
        impact_weight=3.0,
    ),
    # ... rest same
]

result = evaluate_composite_slo_graph(graph)
```

**Result:**
```
database: 0.80 ✗ FAIL (below 0.95)
api_server: 0.71 ✗ FAIL (0.95 × 0.75 = 0.71, below 0.95)
web_gateway: 0.57 ✗ FAIL (0.71 × 0.80 = 0.57, below 0.90)

Global SLO: 0.69 ✗ FAIL
```

**Key:** Database failure cascaded to **ALL services** ↓ This is correct! Everything depends on the database.

### Experiment 2: API Degradation Only

**Change:** API score from 0.95 → 0.85 (but database still healthy)

```python
services=[
    CompositeServiceSlo(
        service="database",
        local_score=0.96,
        min_pass_score=0.95,
        impact_weight=3.0,
    ),
    CompositeServiceSlo(
        service="api_server",
        local_score=0.85,  # ← DEGRADED, but DB is fine
        min_pass_score=0.95,
        impact_weight=2.5,
    ),
    # ... gateway same
]
```

**Result:**
```
database: 0.96 ✓ PASS
api_server: 0.85 ✗ FAIL (own score, DB is fine)
web_gateway: 0.68 ✗ FAIL (0.94 × 0.80 = 0.75, but API failed → penalty applies)

Global SLO: 0.79 ✗ FAIL
```

**Key:** API failure propagates downstream, but database is isolated.

### Experiment 3: Web Gateway Degradation (Leaf Node)

**Change:** Gateway score from 0.94 → 0.80

```python
services=[
    CompositeServiceSlo(
        service="database",
        local_score=0.96,
        min_pass_score=0.95,
        impact_weight=3.0,
    ),
    CompositeServiceSlo(
        service="api_server",
        local_score=0.95,
        min_pass_score=0.95,
        impact_weight=2.5,
    ),
    CompositeServiceSlo(
        service="web_gateway",
        local_score=0.80,  # ← DEGRADED, but no one depends on it
        min_pass_score=0.90,
        impact_weight=2.0,
    ),
]
```

**Result:**
```
database: 0.96 ✓ PASS
api_server: 0.95 ✓ PASS
web_gateway: 0.80 ✗ FAIL

Global SLO: 0.90 ✓ PASS (barely)
```

**Key:** Gateway failure is **isolated** (it has no dependents). Only gateway itself fails, doesn't cascade.

---

## Key Concepts Explained

### Topological Order

Services are evaluated in dependency order (dependencies first). This ensures failure penalties are applied correctly.

```
dependencies:
  db → api
  api → gateway

→ Evaluation order: [db, api, gateway]
  Not: [api, db, gateway] ← Would be wrong!
```

### Failure Penalty

When an upstream service fails, multiply downstream score:

```
API local_score: 0.95
Database failed: Yes
Failure penalty: 0.25

Effective score = 0.95 × (1.0 - 0.25) = 0.95 × 0.75 = 0.71
```

**Penalty Interpretation:**
- 0.0 = No impact (cache failure might be 0.05)
- 0.25 = Medium impact
- 0.50 = Severe impact (API loses half its score)
- 1.0 = Total failure (no fallback)

### Impact Weight

Used for global SLO calculation. Higher weight = more critical.

```
Global SLO = (db_score × 3.0 + api_score × 2.5 + gateway_score × 2.0) / 7.5
           = (0.96 × 3.0 + 0.95 × 2.5 + 0.94 × 2.0) / 7.5
           = (2.88 + 2.375 + 1.88) / 7.5
           = 7.135 / 7.5
           = 0.95
```

---

## Real-World Examples

### E-Commerce Checkout
```
Payment API (weight 4.0)
    ↓ depends on
Fraud Detection (weight 2.0, penalty 0.50)
    ↓ depends on
ML Model (weight 2.5, penalty 0.35)
    ↓ depends on
Feature Store (weight 3.0, penalty 0.40)
```

**Scenario:** Feature store has stale data (0.78 SLO)
- Feature store: 0.78 ✗
- ML model: 0.92 × 0.60 = 0.55 ✗
- Fraud: 0.88 × 0.65 = 0.57 ✗
- Payment: 0.99 × 0.50 = 0.50 ✗
- **Result:** Cannot process payments! ✗

### Multi-Region Platform
```
US-East (weight 3.0): [db → api]
EU-West (weight 1.5): [db → api]
Asia-Pac (weight 1.0): [db → api]
```

**Scenario:** US-East DB fails
- US-East impacts global SLO more (weight 3.0) than Asia-Pac (weight 1.0)
- System still healthy if other regions are up

---

## Unique Value vs. Alternatives

| Need | Solution |
|------|----------|
| Single per-service SLOs? | Use `StreamingSloEvaluator` |
| Parallel services (no dependencies)? | Use `SloGraph` |
| **Dependencies + cascading failures?** | ✅ **Use Composite DAG** |
| Compare with multiple scenarios? | Combine with failure simulation (see examples) |

---

## Quick Reference

### Configuration Keys

```yaml
services:
  - name: <service_id>
    local_score: 0.0-1.0 (current health)
    min_pass_score: 0.0-1.0 (pass threshold)
    impact_weight: 0.0-10.0 (importance in global calc)

dependencies:
  - dependency: <upstream_service>
    dependent: <downstream_service>
    failure_penalty: 0.0-1.0 (impact of upstream failure)

global_min_pass_score: 0.0-1.0 (system health threshold)
```

### Output Fields

```python
result.topological_order  # [service1, service2, ...]
result.global_slo         # 0.0-1.0 global score
result.global_pass        # True/False system health
result.services[]
  .service                # Service name
  .local_score            # Original score
  .effective_score        # After dependency adjustments
  .pass                   # Pass/fail vs min_pass_score
  .dependency_adjusted    # Was effective_score changed?
  .failed_dependencies    # [upstream services that failed]
```

---

## Error Scenarios

Composite DAG validates the graph. These will fail:

```
✗ Duplicate service names
✗ Duplicate dependency edges
✗ Reference to non-existent service
✗ Self-dependency (service depends on itself)
✗ Cycles (a → b → c → a)
✓ Multiple dependencies on same service (a → c, b → c is OK)
✓ Multiple independent branches (a → c, b → d is OK)
```

---

## Next Steps

1. **Run Examples:** `python examples/python/composite_slo_dag_examples.py`
2. **Integration:** Load your service topology from service mesh/config
3. **Monitoring:** Fetch live scores from Prometheus/DataDog, evaluate continuously
4. **Alerting:** Alert when `global_slo` falls below threshold

---

## FAQ

**Q: Why is my global SLO different from manual average?**
A: Composite DAG uses weighted average. Check `impact_weight` values. Formula: `Σ(score × weight) / Σ(weight)`

**Q: Can a service fail without its upstream failing?**
A: Yes! Services have their own `local_score` and `min_pass_score`. Both matter.

**Q: What if I have circular dependencies?**
A: Composite DAG detects cycles and returns an error. Fix your dependency model.

**Q: Can I use partial scores (not boolean pass/fail)?**
A: Yes! Scores are 0.0-1.0. Even if effective_score < min_pass_score, the score is still used in global calculation.

**Q: How do I determine failure penalties?**
A: Think about user impact. Cache failure (0.10) ≠ Database failure (0.50). Start conservative, adjust based on incidents.

**Q: Can I have multiple upstream dependencies on one service?**
A: Yes! The penalties compound. If service A has dependencies on B (penalty 0.25) and C (penalty 0.30), and both fail:
  - Effective = local_score × (1 - 0.25) × (1 - 0.30) = local_score × 0.525

---

## Resources

- **Full Reference:** [docs/reference/composite-slo-dag.md](../reference/composite-slo-dag.md)
- **Python Examples:** [examples/python/composite_slo_dag_examples.py](../../examples/python/composite_slo_dag_examples.py)
- **Benchmarks:** [benches/composite_slo_dag.rs](../../benches/composite_slo_dag.rs) (tested up to 5000 services)
- **Tests:** [tests/functional_end_to_end_tests.rs](../../tests/functional_end_to_end_tests.rs)
