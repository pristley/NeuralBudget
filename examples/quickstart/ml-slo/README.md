# ML SLO Quick Start (5 Minutes)

Monitor ML model performance, accuracy, and data drift in 5 minutes.

## ⚡ What You'll Do

1. Copy `slo.yaml` and `sample.json`
2. Run one command
3. See model performance metrics ✓ PASS or ✗ FAIL

## 📋 Step-by-Step

### Step 1: Copy the Files

```bash
cp slo.yaml ./
cp sample.json ./
```

### Step 2: Install neuralbudget (if needed)

```bash
cargo install neuralbudget
# or
pip install neuralbudget
```

### Step 3: Evaluate Model Performance

```bash
neuralbudget eval slo.yaml sample.json
```

### Expected Output

```
✓ SLO PASS
  Accuracy: 94.50% ✓ (target: ≥92%)
  Precision: 95.20% ✓
  Recall: 93.80% ✓
  Drift Score: 0.082 ✓ (threshold: ≤0.15)
  Latency P99: 142.5ms ✓ (threshold: <150ms)
  Confidence: 88% ✓ (target: ≥80%)
  GPU Utilization: 72% ✓ (threshold: <90%)
```

### Step 4: Trigger a Failure - Model Drift

Edit `sample.json` to simulate feature drift:

```json
"drift": {
  "feature_drift_score": 0.25,  // Change from 0.082
  "target_drift_score": 0.18
}
```

Re-run:

```bash
neuralbudget eval slo.yaml sample.json
```

Expected:
```
✗ SLO FAIL
  Drift Score: 0.25 ✗ (threshold: ≤0.15)
  ⚠️ Action: Check for data distribution changes
  ⚠️ Consider triggering model retraining
```

### Step 5: Trigger a Failure - Accuracy Drop

Edit `sample.json` accuracy metrics:

```json
"accuracy": 0.88,        // Drop from 0.945
"f1_score": 0.87
```

Re-run:

```bash
neuralbudget eval slo.yaml sample.json
```

Expected:
```
✗ SLO FAIL
  Accuracy: 88.00% ✗ (target: ≥92%)
  ⚠️ Action: Model performance degraded
  ⚠️ Consider: Retraining or rollback
```

## 🎯 Key Metrics Explained

| Metric | Purpose |
|--------|---------|
| `accuracy` | Overall prediction correctness |
| `precision` | False positive rate |
| `recall` | False negative rate |
| `drift` | Feature/label distribution changes |
| `latency` | Model inference speed |
| `confidence` | Prediction confidence scores |

## 📚 Next Steps

- **Drift Detection**: [Anomaly Drift Detection](../../guides/anomaly_drift_detection.md)
- **Model Monitoring**: [ML Integration Guide](../../guides/genai_connectors.md)
- **Production Setup**: [Production Deployment](../../guides/production-deployment.md)
- **Advanced Monitoring**: [Advanced Alert Dispatch](../../guides/advanced_alert_dispatch.md)

## 🔗 Learn More

- [Full SLO Guide](../../guides/user-guide.md)
- [API Reference](../../reference/api.md)
- [Troubleshooting](../../guides/troubleshooting.md)
