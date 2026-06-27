"""Anomaly Detection & Drift Explanation - Implementation Guide

This guide covers NeuralBudget's anomaly detection and drift explanation system,
which enables dynamic baselining beyond static thresholds for ML/GenAI SLO modes.

## Overview

The anomaly detection system provides:
- **Statistical Baselining**: Z-score, modified Z-score, percentile-based detection
- **ML-Based Baselining**: Isolation Forest, One-Class SVM, Local Outlier Factor
- **Drift Detection**: Kolmogorov-Smirnov test for concept drift
- **Feature Importance**: Which features contributed to anomalies/drift
- **Drift Explanation**: Human-readable explanations with recommended actions

## Problem Statement

Previous ML/GenAI SLO evaluation relied on static thresholds:
```python
# Static threshold: always fail if latency > 200ms
passed = inference_latency_ms < 200.0
```

**Issues:**
- Doesn't adapt to normal system variability
- Can't distinguish anomalies from expected patterns
- No visibility into root causes (which feature changed)
- No warning before threshold breach

**Solution:**
Dynamic baselining learns normal patterns from history, detects anomalies
when they deviate significantly, and explains which features caused the drift.

## Architecture

### 1. Statistical Baselining

Uses statistical methods to establish normal ranges from historical data.

```python
from neuralbudget.anomaly_detection import StatisticalBaseline

baseline = StatisticalBaseline(window_size=100, percentile=99.0)

# Train on historical data
for value in historical_latencies:
    baseline.add_observation(value)

# Detect anomaly
is_anomaly, z_score, reason = baseline.get_zscore_anomaly(current_value)
```

**Methods:**
- **Z-Score**: Detects values >3σ from mean
- **Modified Z-Score (MAD)**: Robust to outliers, detects >3.5σ
- **Percentile**: Flags values outside 1st-99th percentile range

**Scoring:**
Combined score = 0.3*Z + 0.4*Modified_Z + 0.3*Percentile
- 0.0-0.3: Normal
- 0.3-0.7: Potential anomaly
- 0.7-1.0: Strong anomaly

### 2. ML-Based Baselining

Uses ensemble of unsupervised ML algorithms:

```python
from neuralbudget.anomaly_detection import MLBaseline

ml_baseline = MLBaseline(contamination=0.05, window_size=100)

# Train on normal data
features = {
    "latency": 150.0,
    "gpu_util": 0.75,
    "memory": 0.60,
}
ml_baseline.add_observation(features)

# Score multivariate anomaly
score = ml_baseline.get_ml_anomaly_score(features)
```

**Algorithms:**
- **Isolation Forest**: Identifies isolated points efficiently
- **One-Class SVM**: Learns boundary of normal data
- **Local Outlier Factor (LOF)**: Density-based detection

**Scoring:**
Ensemble vote: flags as anomaly if 2+ models agree

### 3. Drift Detection

Detects when the distribution of metrics has fundamentally shifted:

```python
from neuralbudget.anomaly_detection import DriftDetector

detector = DriftDetector(reference_window=100, test_window=50)

# Build reference distribution
for value in historical_data[:100]:
    detector.add_observation(value)

# Detect shift
drift = detector.detect_drift()
print(f"Is drifting: {drift.is_drifting}")
print(f"KS p-value: {drift.ks_p_value}")
```

**Test:**
- **Kolmogorov-Smirnov (KS)**: Compares distributions
- **Mean Shift**: Detects center of mass changes
- **Variance Ratio**: Detects volatility changes

**Interpretation:**
- KS p-value < 0.05: Statistically significant drift
- Mean shift > 1σ: Substantial center change
- Drift score combines all metrics (0-1 scale)

### 4. Feature Importance

Identifies which features contributed to anomalies/drift:

```python
from neuralbudget.anomaly_detection import FeatureImportanceCalculator

baseline_features = {
    "latency": 150.0,
    "gpu": 0.75,
    "memory": 0.60,
}
current_features = {
    "latency": 800.0,   # 5.3x change
    "gpu": 0.92,        # 1.2x change
    "memory": 0.60,     # No change
}

importances = FeatureImportanceCalculator.get_baseline_feature_importance(
    current_features, baseline_features
)

# Output sorted by contribution
for imp in importances:
    print(f"{imp.feature_name}: {imp.contribution_percent:.1f}%")
```

**Methods:**
- **Baseline Comparison**: Deviation from baseline
- **SHAP Values**: If model available (requires shap library)
- **Permutation Importance**: Feature impact on prediction

### 5. Drift Explanation

Generates human-readable explanations:

```python
from neuralbudget.anomaly_detection import DriftExplainer

explanation = DriftExplainer.explain_drift(
    drift_detection,
    current_features,
    reference_features,
)

print(f"Severity: {explanation.severity}")
print(f"Explanation: {explanation.explanation}")
print(f"Action: {explanation.recommended_action}")

# Top contributing features
for feat in explanation.top_contributing_features[:3]:
    print(f"  {feat.feature_name}: {feat.contribution_percent:.1f}%")
```

**Severity Levels:**
- **Low (0-0.4)**: Minor deviation, continue monitoring
- **Medium (0.4-0.6)**: Notable shift, update baselines if expected
- **High (0.6-0.8)**: Significant drift, review model
- **Critical (0.8-1.0)**: Severe drift, investigate immediately

## Integration with ML/GenAI SLO

### Adaptive ML SLO

```python
from neuralbudget.adaptive_slo import AdaptiveMlSloEvaluator
from neuralbudget import evaluate_ml_once

# Create adaptive evaluator
adaptive = AdaptiveMlSloEvaluator(
    baseline_window=100,
    contamination=0.05,  # Expect 5% anomalies
    enable_drift_detection=True,
)

# Evaluate with adaptive thresholds
sample = {
    "timestamp": 1705317000,
    "inference_latency_ms": 165.0,
    "gpu_utilization": 0.73,
    "feature_drift": 0.08,
    "prediction_confidence": 0.92,
}

# Standard evaluation
baseline_result = evaluate_ml_once(sample)

# Enhanced with anomaly detection
adaptive_result = adaptive.evaluate_with_anomaly_detection(
    sample, baseline_result
)

print(f"Pass: {adaptive_result.passed}")
print(f"Anomaly detected: {adaptive_result.anomaly_detected}")
print(f"Anomaly score: {adaptive_result.anomaly_score:.3f}")
print(f"Is drifting: {adaptive_result.is_drifting}")
print(f"Confidence: {adaptive_result.confidence_score:.1%}")

# Top contributing features to drift
if adaptive_result.top_contributing_features:
    print("Top contributors:")
    for feat in adaptive_result.top_contributing_features[:3]:
        print(f"  {feat.feature_name}: {feat.contribution_percent:.1f}%")
```

**Adaptive Thresholds:**
- Automatically calculated as 1.1 * 99th percentile
- Adapts as new data arrives
- Specific to each metric (latency, GPU, drift, confidence)

### Adaptive GenAI SLO

```python
from neuralbudget.adaptive_slo import AdaptiveGenAiSloEvaluator
from neuralbudget import evaluate_genai_once

adaptive = AdaptiveGenAiSloEvaluator(
    baseline_window=100,
    enable_drift_detection=True,
)

sample = {
    "tokens_per_second": 25.0,
    "time_to_first_token_ms": 100.0,
    "semantic_similarity": 0.85,
}

baseline_result = evaluate_genai_once(sample)
adaptive_result = adaptive.evaluate_with_anomaly_detection(
    sample, baseline_result
)

if adaptive_result.is_drifting:
    print(f"GenAI drift: {adaptive_result.drift_explanation}")
    print(f"Affected metrics: {adaptive_result.affected_metrics}")
```

## Use Cases

### 1. Model Degradation Detection

**Problem:** Model accuracy slowly degrades over time

**Solution:**
```python
# Track prediction confidence over time
adaptive = AdaptiveMlSloEvaluator()

for prediction in stream:
    result = adaptive.evaluate_with_anomaly_detection(
        prediction,
        baseline_result,
    )
    
    if result.is_drifting:
        # Model has degraded
        trigger_retraining()
    
    if result.anomaly_detected:
        # Single bad prediction or data issue
        log_for_analysis(result.anomaly_reason)
```

### 2. Concept Drift in Production

**Problem:** Data distribution changes (e.g., user behavior)

**Solution:**
```python
# Drift detector identifies when users' patterns have changed
drift = detector.detect_drift()

if drift.is_drifting:
    # Get explanation
    explanation = DriftExplainer.explain_drift(
        drift, current_features, reference_features
    )
    
    # Take action based on severity
    if explanation.severity in ["high", "critical"]:
        notify_data_team(explanation.explanation)
        update_feature_engineering()
```

### 3. Resource Anomaly Detection

**Problem:** Sudden spikes in GPU/memory usage

**Solution:**
```python
features = {
    "gpu_utilization": current_gpu,
    "memory_usage": current_memory,
    "queue_depth": current_queue,
}

result = ml_baseline.get_ml_anomaly_score(features)

if result.is_anomaly:
    # Identify which resource spiked
    for imp in feature_importances:
        if imp.importance_score > 0.3:
            alert_ops(f"Resource spike: {imp.feature_name}")
```

### 4. Performance Regression Detection

**Problem:** Latency suddenly increases

**Solution:**
```python
# Monitor latency with adaptive baseline
adaptive = AdaptiveMlSloEvaluator()

for request in stream:
    result = adaptive.evaluate_with_anomaly_detection(
        {"inference_latency_ms": request.latency, ...},
        evaluate_ml_once({...}),
    )
    
    if result.anomaly_detected and result.anomaly_score > 0.8:
        # High confidence anomaly
        trigger_investigation()
        # Adaptive threshold is too tight?
        print(f"New threshold: {result.adaptive_latency_threshold}ms")
```

## Configuration

### Statistical Baseline

```python
baseline = StatisticalBaseline(
    window_size=100,      # Lookback window
    percentile=99.0,      # Percentile threshold
)
```

### ML Baseline

```python
ml_baseline = MLBaseline(
    contamination=0.05,   # Expected anomaly rate (5%)
    window_size=100,      # Training window
)
```

### Drift Detector

```python
detector = DriftDetector(
    reference_window=100, # Reference distribution size
    test_window=50,       # Test window size
)
```

### Adaptive ML SLO

```python
adaptive = AdaptiveMlSloEvaluator(
    baseline_window=100,           # Baseline size
    contamination=0.05,            # Anomaly rate
    enable_drift_detection=True,   # Enable KS test
)
```

## Performance Characteristics

| Operation | Time | Memory | Notes |
|-----------|------|--------|-------|
| add_observation | <1ms | O(window_size) | Maintains rolling window |
| get_zscore_anomaly | <1ms | O(1) | Statistical, fast |
| get_modified_zscore | <1ms | O(1) | More robust |
| get_percentile_anomaly | <5ms | O(1) | Requires numpy |
| get_ml_anomaly_score | 5-20ms | O(window_size) | Ensemble voting |
| detect_drift | 10-50ms | O(window_size) | KS test + mean shift |
| get_baseline_importance | <5ms | O(n_features) | Feature comparison |
| explain_drift | <10ms | O(n_features) | Full explanation |

## Dependencies

### Required
- Python 3.9+
- NeuralBudget core

### Optional
- numpy: Statistical calculations
- scipy: KS test (drift detection)
- scikit-learn: ML-based detection (Isolation Forest, SVM, LOF)
- shap: SHAP-based feature importance

```bash
# Statistical only
pip install numpy

# Statistical + ML
pip install numpy scipy scikit-learn

# Full feature set
pip install numpy scipy scikit-learn shap
```

## Best Practices

1. **Warm-up Period**: Don't make decisions until baseline has ~100 observations
2. **Gradual Adaptation**: Increase contamination if too many false positives
3. **Feature Selection**: Include only features relevant to SLO
4. **Regular Review**: Monitor precision/recall of anomaly detection
5. **Action Thresholds**: Set severity-based action triggers
6. **Alert Tuning**: Tune to minimize false positives while catching real issues

## Troubleshooting

### Too Many Anomalies Detected
- Solution: Increase contamination parameter (0.05 → 0.10)
- Or: Increase statistical threshold multiplier (3.0 → 4.0 sigma)

### Missing Real Anomalies
- Solution: Decrease contamination parameter
- Or: Decrease threshold multiplier
- Or: Enable both statistical and ML detection

### Drift Detection Not Triggering
- Solution: Increase test window size
- Or: Lower p-value threshold from 0.05 to 0.10
- Or: Use smaller reference window

### Feature Importance Always Same
- Solution: Include more diverse features
- Or: Use SHAP instead of baseline comparison
- Or: Standardize feature scales first

## Summary

The anomaly detection system transforms ML/GenAI SLO evaluation from static
thresholds to adaptive, data-driven decisions that:

✓ Learn normal patterns automatically
✓ Detect true anomalies while tolerating normal variation
✓ Explain what caused problems (feature importance)
✓ Generate actionable recommendations
✓ Adapt thresholds as systems evolve
✓ Reduce false positives from static thresholds

Start with statistical baselining, add ML methods for multivariate analysis,
enable drift detection for long-term monitoring.
"""
