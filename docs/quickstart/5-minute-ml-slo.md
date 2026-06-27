# ML Model Drift & Confidence - Quick Start (5 Minutes)

Monitor ML model performance, accuracy, and data drift. Track model degradation and confidence metrics.

## ⏱️ Time: ~2 Minutes

## What You'll Do

1. ✅ Copy ML `slo.yaml` configuration
2. ✅ Copy `sample.json` with drift/confidence metrics
3. ✅ Run `neuralbudget eval`
4. ✅ See model health status

## 📋 Prerequisites

- NeuralBudget installed (see [HTTP guide](5-minute-http-slo.md))

## Step 1: Copy ML SLO Configuration

Create `ml-slo.yaml`:

```yaml
# ML Model SLO Configuration
service: "ml-model-quickstart"
description: "Quick start example for ML SLO evaluation"

# SLO Target
target: 99.5

# Measurement window (typically shorter for ML)
window: "7d"

# ML-specific thresholds
accuracy_threshold: 0.92          # Model accuracy must be >= 92%
latency_threshold_ms: 150         # Inference latency must be < 150ms
min_confidence: 0.80              # Predictions must have >= 80% confidence
max_drift: 0.15                   # Feature drift must be <= 15%

# Quality gates (multiple metrics must pass)
quality_gates:
  - metric: "accuracy"
    threshold: 0.92
  - metric: "precision"
    threshold: 0.90
  - metric: "recall"
    threshold: 0.90

# GPU/compute utilization
gpu_utilization_threshold: 0.90

# Model-specific settings
model:
  name: "recommendation-v1"
  version: "1.0.0"
  retraining_schedule: "weekly"
  canary_deployment: true

# Burn rate alerts
alerts:
  - window: "1h"
    threshold: 0.10
  - window: "12h"
    threshold: 0.05
  - window: "7d"
    threshold: 0.02

tags:
  mode: "ml"
  tier: "prod"
```

## Step 2: Copy ML Sample Metrics

Create `ml-sample.json`:

```json
{
  "timestamp": 1704067200,
  "service": "ml-model-quickstart",
  "measurement_window": "5m",
  "model_metrics": {
    "accuracy": 0.945,
    "precision": 0.952,
    "recall": 0.938,
    "f1_score": 0.945,
    "auc_roc": 0.978,
    "log_loss": 0.125
  },
  "confidence": {
    "mean_confidence": 0.88,
    "min_confidence": 0.72,
    "predictions_below_threshold": 45,
    "total_predictions": 50000
  },
  "drift": {
    "feature_drift_score": 0.082,
    "target_drift_score": 0.045,
    "concept_drift_detected": false
  },
  "latency": {
    "mean_ms": 85.3,
    "p99_ms": 142.5,
    "max_ms": 298.1
  },
  "inference": {
    "total": 50000,
    "successful": 49950,
    "errors": 50
  },
  "model_info": {
    "version": "1.0.0",
    "deployment_timestamp": 1704000000,
    "gpu_utilization": 0.72,
    "batch_size": 32
  }
}
```

## Step 3: Evaluate

```bash
neuralbudget eval ml-slo.yaml ml-sample.json
```

## Expected Output: PASS

```
✓ SLO PASS - Model performing well
  Accuracy: 94.50% ✓ (target: ≥92%)
  Precision: 95.20% ✓
  Recall: 93.80% ✓
  Drift Score: 0.082 ✓ (threshold: ≤0.15)
  Latency P99: 142.5ms ✓ (threshold: <150ms)
  Confidence: 88% ✓ (target: ≥80%)
  GPU Utilization: 72% ✓ (threshold: <90%)
```

## Experiment 1: Trigger Drift Detection

Edit `ml-sample.json` to simulate feature drift:

```json
"drift": {
  "feature_drift_score": 0.25,  // Changed from 0.082
  "target_drift_score": 0.18
}
```

Re-run:
```bash
neuralbudget eval ml-slo.yaml ml-sample.json
```

**Expected:**
```
✗ SLO FAIL
  Drift Score: 0.25 ✗ (threshold: ≤0.15)
  ⚠️ Action: Check for data distribution changes
  ⚠️ Consider: Trigger model retraining
```

## Experiment 2: Trigger Accuracy Drop

Edit metrics:
```json
"accuracy": 0.88,        // Drop from 0.945
"f1_score": 0.87
```

Re-run:
```bash
neuralbudget eval ml-slo.yaml ml-sample.json
```

**Expected:**
```
✗ SLO FAIL
  Accuracy: 88.00% ✗ (target: ≥92%)
  ⚠️ Action: Model performance degraded
  ⚠️ Consider: Retraining or rollback
```

## 🎯 Understanding ML Metrics

### Model Quality Metrics

| Metric | Meaning | Target |
|--------|---------|--------|
| `accuracy` | Overall correctness | ≥92% |
| `precision` | False positive rate (TP/(TP+FP)) | ≥90% |
| `recall` | False negative rate (TP/(TP+FN)) | ≥90% |
| `f1_score` | Balance of precision & recall | ≥0.90 |
| `auc_roc` | Discriminative power | ≥0.95 |

### Drift Detection

| Metric | Meaning | Risk |
|--------|---------|------|
| `feature_drift_score` | Input data changed | Model may fail |
| `target_drift_score` | Label distribution changed | Training data mismatch |
| `concept_drift` | Decision boundary shifted | Retraining needed |

### Confidence & Reliability

- `mean_confidence`: Average prediction score
- `predictions_below_threshold`: Count of low-confidence preds
- Risk: Low confidence = unreliable predictions

## 📚 Common ML Patterns

### Strict Quality (Production)

```yaml
accuracy_threshold: 0.95
min_confidence: 0.85
max_drift: 0.05        # Very strict
quality_gates:
  - metric: "precision"
    threshold: 0.94
  - metric: "recall"
    threshold: 0.94
```

### Balanced (Typical)

```yaml
accuracy_threshold: 0.92
min_confidence: 0.80
max_drift: 0.15
```

### Experimental (Testing)

```yaml
accuracy_threshold: 0.88
min_confidence: 0.70
max_drift: 0.25        # Relaxed for A/B testing
```

## 🔄 Integration with Training Pipeline

### Check Before Deployment

```bash
# Evaluate model before deployment
python scripts/evaluate_model.py \
  --model candidate_model.pkl \
  --test_data test_metrics.json \
  --slo ml-slo.yaml

# Only deploy if SLO passes
if [ $? -eq 0 ]; then
  kubectl apply -f model_deployment.yaml
fi
```

### Monitor Post-Deployment

```bash
# Continuous monitoring every hour
*/1 * * * * neuralbudget eval ml-slo.yaml prometheus_metrics.json
```

### Trigger Retraining

```bash
# If SLO fails, trigger retraining
if ! neuralbudget eval ml-slo.yaml live_metrics.json; then
  kubectl create job retrain-job --image=ml-trainer:latest
fi
```

## 🚨 Alert Scenarios

### Scenario 1: Sudden Accuracy Drop

- **Cause:** Bug in preprocessing or corrupted data
- **Action:** Rollback to previous model version
- **Timeline:** < 5 minutes

### Scenario 2: Gradual Drift

- **Cause:** Distribution shift (seasonal, new user cohort)
- **Action:** Schedule retraining, plan gradual rollout
- **Timeline:** Hours to days

### Scenario 3: High GPU Usage

- **Cause:** Model complexity or batch size too large
- **Action:** Optimize inference, increase replicas
- **Timeline:** 1-2 hours

### Scenario 4: Low Confidence

- **Cause:** Ambiguous examples, training-serving skew
- **Action:** Increase training data, add confidence thresholding
- **Timeline:** Days to weeks

## ❓ FAQs

**Q: How often should I retrain?**
A: Depends on data freshness. Daily for fast-changing data, weekly for stable. Monitor drift to decide.

**Q: What's acceptable drift?**
A: 5-10% for most models. Higher if you have robust retraining.

**Q: Should confidence be in 0-1 or 0-100?**
A: Code uses 0-1. Multiply by 100 for percentage display.

**Q: How do I measure drift?**
A: Use statistical tests (KL divergence, Kolmogorov-Smirnov) on feature distributions.

## 🔗 Next Steps

- [Advanced ML Monitoring](../../guides/anomaly_drift_detection.md)
- [ML Best Practices](../../guides/user-guide.md#ml-section)
- [Anomaly Detection](../../reference/ANOMALY_DETECTION_IMPLEMENTATION.md)
- [Try GenAI SLO](5-minute-genai-slo.md)
- [Go to Prometheus Integration](../examples/quickstart/prometheus/README.md)

## 🔗 Full Resources

- **🤖 Drift Detection:** [Anomaly Drift Detection](../../guides/anomaly_drift_detection.md)
- **📊 Model Monitoring:** [Advanced Monitoring](../../guides/advanced_alert_dispatch.md)
- **🚀 Production ML:** [Production Deployment](../../guides/production-deployment.md)
- **🔌 API Reference:** [Full API](../../reference/api.md)

---

**Questions?** [Ask on GitHub Discussions](https://github.com/pristley/NeuralBudget/discussions)
