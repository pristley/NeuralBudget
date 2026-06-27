"""Anomaly Detection & Drift Explanation - API Reference

Complete API documentation for anomaly detection and drift explanation.

## Table of Contents

1. Data Models
2. Statistical Baselining
3. ML-Based Baselining
4. Drift Detection
5. Feature Importance
6. Drift Explanation
7. Adaptive SLO Evaluation

---

## 1. Data Models

### AnomalyScore

Represents a single anomaly detection decision.

```python
@dataclass(frozen=True)
class AnomalyScore:
    timestamp: str                  # ISO format timestamp
    raw_value: float               # Observed value
    is_anomaly: bool               # True if anomalous
    anomaly_score: float           # 0-1 score
    statistical_score: float       # Z-score based
    ml_score: float                # ML-based score
    confidence: float              # 0-1 confidence
    reason: str                    # Human explanation
```

**Example:**
```python
from neuralbudget.anomaly_detection import StatisticalBaseline

baseline = StatisticalBaseline()
for i in range(100):
    baseline.add_observation(200.0 + random.gauss(0, 10))

score = baseline.get_combined_anomaly_score(500.0)
# AnomalyScore(timestamp='2024-...', is_anomaly=True, anomaly_score=0.95, ...)
```

### DriftDetection

Results from drift detection test.

```python
@dataclass(frozen=True)
class DriftDetection:
    timestamp: str                 # ISO format
    is_drifting: bool             # True if drift detected
    drift_score: float            # 0-1 score
    ks_statistic: float           # KS test statistic
    ks_p_value: float             # p-value
    drift_indices: Dict[str, float]  # Per-metric indices
    reference_mean: float         # Baseline mean
    current_mean: float           # Current mean
    mean_shift: float             # Absolute shift
```

**Interpretation:**
- `is_drifting`: True if KS p-value < 0.05 and drift_score > 0.3
- `drift_score`: Combined measure (0=no drift, 1=extreme drift)
- `ks_p_value`: Statistical significance (< 0.05 = significant)
- `mean_shift`: Magnitude of center movement

### FeatureImportance

Feature contribution to anomaly/drift.

```python
@dataclass(frozen=True)
class FeatureImportance:
    feature_name: str              # Feature identifier
    importance_score: float        # 0-1 importance
    baseline_value: float          # Expected value
    current_value: float           # Observed value
    deviation: float               # current - baseline
    contribution_percent: float    # % of total change
```

### DriftExplanation

Complete human-readable explanation.

```python
@dataclass(frozen=True)
class DriftExplanation:
    timestamp: str
    is_drifting: bool
    drift_score: float
    explanation: str               # Detailed text
    top_contributing_features: List[FeatureImportance]  # Top 3-5
    affected_metrics: List[str]    # Affected metric names
    severity: str                  # "low", "medium", "high", "critical"
    recommended_action: str        # Suggested response
```

---

## 2. Statistical Baselining

### StatisticalBaseline

Statistical anomaly detection using z-scores and percentiles.

```python
class StatisticalBaseline:
    def __init__(
        self,
        window_size: int = 100,
        percentile: float = 99.0
    )
```

**Parameters:**
- `window_size`: Lookback window for baseline (default: 100)
- `percentile`: Percentile threshold for bounds (default: 99.0)

#### add_observation(value)

Add observation to history.

```python
baseline.add_observation(150.0)  # Add latency sample
```

**Method signature:**
```python
def add_observation(self, value: float) -> None
```

#### get_zscore_anomaly(value)

Detect anomaly using Z-score method.

```python
is_anomaly, z_score, reason = baseline.get_zscore_anomaly(500.0)
# (True, 15.2, "Z-score: 15.20")
```

**Returns:**
- `is_anomaly`: bool (True if |z| > 3.0)
- `z_score`: float (absolute z-score)
- `reason`: str (explanation)

**Formula:**
```
z = (value - mean) / std
anomaly if |z| > 3.0  (99.7% of normal data)
```

#### get_modified_zscore_anomaly(value)

More robust Z-score using Median Absolute Deviation (MAD).

```python
is_anomaly, mod_z, reason = baseline.get_modified_zscore_anomaly(500.0)
```

**Returns:**
- `is_anomaly`: bool (True if |mod_z| > 3.5)
- `mod_z`: float (modified z-score)
- `reason`: str

**Advantages:**
- More resistant to outliers
- Better for long-tailed distributions

#### get_percentile_anomaly(value)

Percentile-based anomaly detection.

```python
is_anomaly, deviation, reason = baseline.get_percentile_anomaly(500.0)
```

**Returns:**
- `is_anomaly`: bool (True if deviation > 1.5 * IQR)
- `deviation`: float (in units of IQR)
- `reason`: str

#### get_combined_anomaly_score(value)

Combine all statistical methods.

```python
score = baseline.get_combined_anomaly_score(500.0)

print(f"Is anomaly: {score.is_anomaly}")
print(f"Score: {score.anomaly_score:.3f}")  # 0-1
print(f"Reason: {score.reason}")
print(f"Confidence: {score.confidence:.1%}")
```

**Weighting:**
- 30% Z-score
- 40% Modified Z-score
- 30% Percentile

**Performance:**
- Time: <1ms
- Memory: O(window_size)

---

## 3. ML-Based Baselining

### MLBaseline

Ensemble anomaly detection using 3 ML algorithms.

```python
class MLBaseline:
    def __init__(
        self,
        contamination: float = 0.05,
        window_size: int = 200
    )
```

**Parameters:**
- `contamination`: Expected anomaly rate (default: 5%)
- `window_size`: Training data window (default: 200)

#### add_observation(features)

Add multivariate observation.

```python
features = {
    "latency_ms": 150.0,
    "gpu_util": 0.75,
    "memory_gb": 8.5,
}
ml_baseline.add_observation(features)
```

**Requirements:**
- All features must be numeric
- Same feature set each time
- At least 20 observations before scoring

#### get_ml_anomaly_score(features)

Score with ML ensemble.

```python
score = ml_baseline.get_ml_anomaly_score(features)

print(f"Is anomaly: {score.is_anomaly}")
print(f"ML score: {score.ml_score:.3f}")
print(f"Algorithms flagging: {score.reason}")
```

**Algorithms:**
1. **Isolation Forest** (30%): Isolates anomalous points
2. **One-Class SVM** (30%): Learns normal boundary
3. **Local Outlier Factor** (40%): Density-based detection

**Ensemble Voting:**
- Flags anomaly if 2+ algorithms agree (-1 prediction)
- Combined score = average of algorithm scores

**Performance:**
- Time: 5-20ms
- Memory: O(window_size * n_features)
- Training: Automatic on first call after adding observations

**Dependencies:**
- scikit-learn (optional)

---

## 4. Drift Detection

### DriftDetector

Detect concept drift using statistical tests.

```python
class DriftDetector:
    def __init__(
        self,
        reference_window: int = 100,
        test_window: int = 50
    )
```

**Parameters:**
- `reference_window`: Size of reference distribution
- `test_window`: Size of test/current distribution

#### add_observation(value)

Add observation to detector.

```python
detector.add_observation(150.0)
```

#### detect_drift()

Perform drift detection test.

```python
drift = detector.detect_drift()

if drift.is_drifting:
    print(f"Drift detected!")
    print(f"KS statistic: {drift.ks_statistic:.3f}")
    print(f"p-value: {drift.ks_p_value:.4f}")
    print(f"Mean shift: {drift.mean_shift:.1f}")
```

**Returns:** `DriftDetection` object

**Drift Score Calculation:**
```python
drift_score = (1 - ks_p_value) * 0.5 + normalized_mean_shift * 0.5
is_drifting = (drift_score > 0.3) and (ks_p_value < 0.05)
```

**Metrics:**
- `ks_statistic`: KS test D statistic (0-1, higher = more different)
- `ks_p_value`: Statistical significance (< 0.05 = significant)
- `mean_shift`: |ref_mean - curr_mean|
- `drift_indices`: Dict with detailed metrics

**Dependencies:**
- scipy.stats.ks_2samp

---

## 5. Feature Importance

### FeatureImportanceCalculator

Calculate which features drove anomalies/drift.

```python
class FeatureImportanceCalculator:
    @staticmethod
    def get_baseline_feature_importance(
        current_features: Dict[str, float],
        reference_features: Dict[str, float],
    ) -> List[FeatureImportance]
```

**Method:**
Compare current vs. baseline feature-by-feature.

**Example:**
```python
baseline = {
    "latency": 150.0,
    "gpu": 0.75,
    "memory": 0.60,
}
current = {
    "latency": 800.0,
    "gpu": 0.92,
    "memory": 0.60,
}

importances = FeatureImportanceCalculator.get_baseline_feature_importance(
    current, baseline
)

# Sorted by importance_score descending
for imp in importances:
    print(f"{imp.feature_name}: {imp.contribution_percent:.1f}%")
    # latency: 81.3%
    # gpu: 18.7%
    # memory: 0.0%
```

**Calculation:**
```python
deviation[f] = abs((current[f] - baseline[f]) / baseline[f])
importance[f] = deviation[f] / sum(deviations)
```

#### get_shap_feature_importance(...)

SHAP-based feature importance (requires shap library).

```python
@staticmethod
def get_shap_feature_importance(
    features: Dict[str, float],
    model: Any = None,
    background_data: Optional[List[Dict[str, float]]] = None,
) -> List[FeatureImportance]
```

**Parameters:**
- `features`: Observation to explain
- `model`: Trained ML model with predict method
- `background_data`: Reference dataset for SHAP

**Returns:** List of FeatureImportance (SHAP-based)

**Dependencies:**
- shap library

---

## 6. Drift Explanation

### DriftExplainer

Generate human-readable drift explanations.

```python
class DriftExplainer:
    @staticmethod
    def explain_drift(
        drift_detection: DriftDetection,
        current_features: Dict[str, float],
        reference_features: Dict[str, float],
    ) -> DriftExplanation
```

**Example:**
```python
explanation = DriftExplainer.explain_drift(
    drift_result,
    current_features,
    baseline_features,
)

print(f"Severity: {explanation.severity}")
# Severity: high

print(f"Explanation: {explanation.explanation}")
# Explanation: Drift detected (score: 0.72). Mean shifted from 200.00 to 250.00 ...

print(f"Action: {explanation.recommended_action}")
# Action: Review feature distributions. Consider model retraining...

print(f"Top contributing features:")
for feat in explanation.top_contributing_features[:3]:
    print(f"  {feat.feature_name}: {feat.contribution_percent:.1f}%")
```

**Severity Mapping:**
- **low** (< 0.4): Continue monitoring
- **medium** (0.4-0.6): Update baselines if expected
- **high** (0.6-0.8): Review model and features
- **critical** (> 0.8): Investigate immediately

**Recommended Actions by Severity:**
- Low: "Continue monitoring. No immediate action required."
- Medium: "Monitor closely. Update baselines if change is expected."
- High: "Review feature distributions. Consider model retraining..."
- Critical: "Investigate immediately. May indicate model degradation..."

---

## 7. Adaptive SLO Evaluation

### AdaptiveMlSloEvaluator

ML SLO evaluation with anomaly detection and adaptive thresholds.

```python
class AdaptiveMlSloEvaluator:
    def __init__(
        self,
        baseline_window: int = 100,
        contamination: float = 0.05,
        enable_drift_detection: bool = True,
    )
```

#### evaluate_with_anomaly_detection(sample, baseline_result)

Evaluate ML sample with anomaly detection.

```python
from neuralbudget import evaluate_ml_once

# Standard evaluation
baseline = evaluate_ml_once(sample)

# Enhanced with anomaly detection
adaptive = AdaptiveMlSloEvaluator()
result = adaptive.evaluate_with_anomaly_detection(sample, baseline)

print(f"Pass: {result.passed}")
print(f"Anomaly detected: {result.anomaly_detected}")
print(f"Anomaly score: {result.anomaly_score:.3f}")
print(f"Drift detected: {result.is_drifting}")
print(f"Confidence: {result.confidence_score:.1%}")
print(f"Adaptive latency threshold: {result.adaptive_latency_threshold}ms")
```

**Returns:** `AdaptiveMlEvaluationResult`

**Fields:**
- Standard ML SLO fields (inherited from baseline_result)
- `anomaly_detected`: bool
- `anomaly_score`: float (0-1)
- `anomaly_reason`: str
- `is_drifting`: bool
- `drift_explanation`: Optional[str]
- `top_contributing_features`: List[FeatureImportance]
- `adaptive_latency_threshold`: float
- `adaptive_drift_threshold`: float
- `confidence_score`: float (0-1)

**Pass Logic:**
```python
passed = baseline_result.pass
if anomaly_detected:
    passed = False
    confidence *= 0.7
if is_drifting:
    confidence *= 0.8
```

### AdaptiveGenAiSloEvaluator

GenAI SLO evaluation with anomaly detection.

```python
class AdaptiveGenAiSloEvaluator:
    def __init__(
        self,
        baseline_window: int = 100,
        contamination: float = 0.05,
        enable_drift_detection: bool = True,
    )
```

#### evaluate_with_anomaly_detection(sample, baseline_result)

Evaluate GenAI sample with anomaly detection.

```python
from neuralbudget import evaluate_genai_once

baseline = evaluate_genai_once(sample)
adaptive = AdaptiveGenAiSloEvaluator()
result = adaptive.evaluate_with_anomaly_detection(sample, baseline)

print(f"Pass: {result.passed}")
print(f"Anomaly: {result.anomaly_detected}")
print(f"Drift: {result.is_drifting}")
if result.drift_explanation:
    print(f"Why: {result.drift_explanation}")
```

**Returns:** `AdaptiveGenAiEvaluationResult`

---

## Summary

**Key Classes:**
- `StatisticalBaseline`: Z-score, modified Z-score, percentile
- `MLBaseline`: Ensemble of Isolation Forest, One-Class SVM, LOF
- `DriftDetector`: Kolmogorov-Smirnov test
- `FeatureImportanceCalculator`: Baseline and SHAP methods
- `DriftExplainer`: Human-readable explanations
- `AdaptiveMlSloEvaluator`: ML SLO with anomalies
- `AdaptiveGenAiSloEvaluator`: GenAI SLO with anomalies

**Performance:**
- Statistical methods: <1ms
- ML methods: 5-20ms
- Drift detection: 10-50ms
- Full explanation: <10ms

**Dependencies:**
- Required: None
- Optional: numpy, scipy, scikit-learn, shap
"""
