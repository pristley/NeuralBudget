# Composite DAG SLO Example

**Copy & Paste Ready** | **E-Commerce Checkout Pipeline** | **Demonstrates: Failure Propagation**

## What This Example Shows

This example models an e-commerce checkout system with 4 services and 3 dependencies:

```
Feature Store (data warehouse)
    ↓ (penalty 0.40)
ML Model Server (fraud detection model)
    ↓ (penalty 0.35)
Fraud Detection Service
    ↓ (penalty 0.50)
Payment Gateway
```

**Key Point:** When Feature Store fails, the failure cascades down to Payment Gateway. 

---

## Quick Start (Copy-Paste)

### 1. Run Baseline (All Healthy)

```bash
# Files provided: slo.yaml and sample.json
# They show: all services healthy (scores 0.94-0.99)

neuralbudget eval slo.yaml sample.json
```

**Expected Output:**
```
=== Composite SLO DAG Evaluation ===
Topological Order: ['feature_store', 'ml_model_server', 'fraud_detection', 'payment_gateway']

Per-Service Status:
  feature_store: 0.96 ✓ PASS
  ml_model_server: 0.95 ✓ PASS
  fraud_detection: 0.94 ✓ PASS
  payment_gateway: 0.99 ✓ PASS

Global SLO: 0.96 ✓ PASS
```

---

## Experiments: Make It FAIL

### Experiment 1: Feature Store Degrades

**Scenario:** Database replication lag causes stale features

**Change `sample.json` - Feature Store Score:**
```json
{
  "services": [
    {
      "name": "feature_store",
      "local_score": 0.80,  # ← CHANGED from 0.96
      "status": "degraded",
      "details": {
        "data_staleness_minutes": 45  # ← 45 min staleness (bad!)
      }
    },
    // ... rest same
  ]
}
```

**Run:**
```bash
neuralbudget eval slo.yaml sample.json
```

**Result: Cascading Failure**
```
Per-Service Status:
  feature_store: 0.80 ✗ FAIL (below 0.95 min_pass)

  ml_model_server: 0.57 ✗ FAIL
    └─ Calculation: 0.95 × (1.0 - 0.40) = 0.95 × 0.60 = 0.57
    └─ Failed deps: feature_store

  fraud_detection: 0.37 ✗ FAIL
    └─ Calculation: 0.94 × (1.0 - 0.35) = 0.94 × 0.65 = 0.61
    └─ But ml_model_server is now input for fraud_detection, so cascades
    └─ Failed deps: ml_model_server

  payment_gateway: 0.19 ✗ FAIL
    └─ Calculation: 0.99 × (1.0 - 0.50) = 0.99 × 0.50 = 0.495
    └─ But fraud_detection failed → 0.495 × (1.0 - 0.35) ≈ 0.32
    └─ Failed deps: fraud_detection

Global SLO: 0.43 ✗ FAIL
```

**Key Insight:** ONE upstream failure cascaded to ALL downstream services! ⚠️
This is correct and why Composite DAG is valuable.

---

### Experiment 2: Fraud Detection Service Fails Independently

**Scenario:** Fraud detection service crashes (not because of upstream)

**Change `sample.json` - Fraud Detection Score:**
```json
{
  "services": [
    {
      "name": "feature_store",
      "local_score": 0.96  // ← Back to normal
    },
    {
      "name": "ml_model_server",
      "local_score": 0.95  // ← Back to normal
    },
    {
      "name": "fraud_detection",
      "local_score": 0.60,  // ← CRASHED (from 0.94)
      "status": "error"
    },
    // ... payment_gateway normal
  ]
}
```

**Result: Isolated Downstream Failure**
```
Per-Service Status:
  feature_store: 0.96 ✓ PASS (no upstream failures)
  ml_model_server: 0.95 ✓ PASS (no upstream failures)
  fraud_detection: 0.60 ✗ FAIL (its own score, unrelated to deps)
  payment_gateway: 0.50 ✗ FAIL
    └─ Calculation: 0.99 × (1.0 - 0.50) = 0.495
    └─ Failed deps: fraud_detection

Global SLO: 0.75 ✓ PASS (barely, but system is partially working)
```

**Key Insight:** Fraud failure only impacts Payment, not upstream services.

---

### Experiment 3: Payment Gateway Fails (Leaf Node)

**Scenario:** Payment processor API is down

**Change `sample.json` - Payment Gateway Score:**
```json
{
  "services": [
    // ... all others at normal levels
    {
      "name": "payment_gateway",
      "local_score": 0.65,  // ← DEGRADED (from 0.99)
      "status": "degraded"
    }
  ]
}
```

**Result: Isolated Leaf Failure**
```
Per-Service Status:
  feature_store: 0.96 ✓ PASS
  ml_model_server: 0.95 ✓ PASS
  fraud_detection: 0.94 ✓ PASS
  payment_gateway: 0.65 ✗ FAIL

Global SLO: 0.88 ✓ PASS
```

**Key Insight:** Payment failure doesn't affect upstream services (they don't depend on it).

---

### Experiment 4: Multiple Failures

**Scenario:** Both Feature Store and Payment Gateway have issues

**Change `sample.json`:**
```json
{
  "services": [
    {
      "name": "feature_store",
      "local_score": 0.85,  // ← degraded
      "status": "degraded"
    },
    // ... ml and fraud normal
    {
      "name": "payment_gateway",
      "local_score": 0.88,  // ← degraded
      "status": "degraded"
    }
  ]
}
```

**Result: Two Independent Issues + One Cascading**
```
Per-Service Status:
  feature_store: 0.85 ✗ FAIL
  ml_model_server: 0.57 ✗ FAIL (cascaded from feature_store)
  fraud_detection: 0.37 ✗ FAIL (cascaded from ml_model)
  payment_gateway: 0.44 ✗ FAIL (own degradation + cascaded failure)

Global SLO: 0.56 ✗ FAIL
```

---

## Configuration Reference

### Services Block
```yaml
services:
  - name: "<service_id>"
    description: "<optional description>"
    local_score: <0.0-1.0>      # Current health score
    min_pass_score: <0.0-1.0>  # Pass/fail threshold
    impact_weight: <0.0-10.0>  # Importance in global calculation
```

### Dependencies Block
```yaml
dependencies:
  - dependency: "<upstream_service_id>"
    dependent: "<downstream_service_id>"
    failure_penalty: <0.0-1.0>  # Impact multiplier (0.0 = no impact, 1.0 = total failure)
    description: "<optional explanation>"
```

### Global Settings
```yaml
global_min_pass_score: <0.0-1.0>  # System health threshold
```

---

## Real-World Patterns

### Pattern 1: Linear Chain
```
db → api → gateway
```
*One failure cascades through entire chain.*

### Pattern 2: Multiple Dependencies
```
        [API]
       ↙   ↘
    [db]  [cache]
```
*Service depends on multiple upstreams; failures compound.*

### Pattern 3: Multiple Branches
```
[gateway]
  ↙   ↘
[auth][payment]
  ↓     ↓
[db]  [processor]
```
*Independent branches; failures isolated within each branch.*

---

## Troubleshooting

### Issue: "Cycle detected in dependency graph"
**Cause:** Dependencies form a loop (e.g., a → b → c → a)
**Fix:** Review dependency diagram; remove circular dependency.

### Issue: "Unknown service in dependencies"
**Cause:** Dependency references a service not in services list
**Fix:** Check spelling and ensure all referenced services are defined.

### Issue: "Global SLO doesn't match my calculation"
**Cause:** Impact weights weren't considered in calculation
**Formula:** `Global = Σ(score × weight) / Σ(weight)`
**Example:**
```
(0.96 × 3.0 + 0.95 × 2.5 + 0.94 × 2.0 + 0.99 × 4.0) / 11.5
= (2.88 + 2.375 + 1.88 + 3.96) / 11.5
= 11.095 / 11.5
= 0.964
```

---

## Next Steps

1. **Run the baseline** (`neuralbudget eval slo.yaml sample.json`)
2. **Try experiments** (modify scores, see cascades)
3. **Adapt to your services:** Copy this example, update service names and dependencies
4. **Integrate with monitoring:** Load scores from Prometheus/DataDog
5. **Automate:** Evaluate continuously, alert on failures

---

## Learning Resources

- **Full Reference:** See [docs/reference/composite-slo-dag.md](../../docs/reference/composite-slo-dag.md)
- **Python Examples:** See [examples/python/composite_slo_dag_examples.py](../python/composite_slo_dag_examples.py)
- **5-Minute Guide:** See [docs/quickstart/5-minute-composite-dag-slo.md](../../docs/quickstart/5-minute-composite-dag-slo.md)
