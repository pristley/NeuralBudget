"""Examples for anomaly detection and drift explanation.

Shows how to use:
1. Statistical baselining for anomaly detection
2. ML-based baselining (Isolation Forest, One-Class SVM, LOF)
3. Drift detection with Kolmogorov-Smirnov test
4. Feature importance calculation
5. Adaptive SLO evaluation with anomaly detection
"""

import random
from neuralbudget.anomaly_detection import (
    StatisticalBaseline,
    MLBaseline,
    DriftDetector,
    FeatureImportanceCalculator,
    DriftExplainer,
)
from neuralbudget.adaptive_slo import AdaptiveMlSloEvaluator, AdaptiveGenAiSloEvaluator


# ============================================================================
# Example 1: Statistical Anomaly Detection
# ============================================================================
def example_1_statistical_anomaly():
    """Detect anomalies using statistical methods."""
    print("\n" + "="*70)
    print("Example 1: Statistical Anomaly Detection")
    print("="*70)
    
    code = """
from neuralbudget.anomaly_detection import StatisticalBaseline

# Create baseline with 100-observation window
baseline = StatisticalBaseline(window_size=100, percentile=99.0)

# Add normal observations
for i in range(100):
    value = 200.0 + random.gauss(0, 20)  # Normal: ~200ms ± 20ms
    baseline.add_observation(value)

# Anomaly: sudden spike
anomaly_value = 500.0  # Way above normal
result = baseline.get_combined_anomaly_score(anomaly_value)

print(f"Anomaly detected: {result.is_anomaly}")
print(f"Anomaly score: {result.anomaly_score:.3f}")
print(f"Reason: {result.reason}")
print(f"Confidence: {result.confidence:.1%}")

# More examples
print("\\nMultiple anomaly scores:")
for value in [200, 210, 190, 800]:
    result = baseline.get_combined_anomaly_score(value)
    print(f"  {value:3.0f}ms: anomaly={result.is_anomaly}, score={result.anomaly_score:.2f}")
"""
    print(code)


# ============================================================================
# Example 2: ML-Based Anomaly Detection
# ============================================================================
def example_2_ml_anomaly():
    """Detect anomalies using ML ensemble methods."""
    print("\n" + "="*70)
    print("Example 2: ML-Based Anomaly Detection (Ensemble)")
    print("="*70)
    
    code = """
from neuralbudget.anomaly_detection import MLBaseline

# Create ML baseline
ml_baseline = MLBaseline(contamination=0.05, window_size=100)

# Train on normal data
for i in range(100):
    features = {
        "latency": 200.0 + random.gauss(0, 20),
        "gpu_util": 0.75 + random.gauss(0, 0.05),
        "memory": 0.60 + random.gauss(0, 0.03),
    }
    ml_baseline.add_observation(features)

# Test with anomalous features
anomaly_features = {
    "latency": 1000.0,  # High latency
    "gpu_util": 0.95,   # High GPU
    "memory": 0.95,     # High memory
}

result = ml_baseline.get_ml_anomaly_score(anomaly_features)
print(f"Anomaly detected: {result.is_anomaly}")
print(f"ML anomaly score: {result.ml_score:.3f}")
print(f"Reason: {result.reason}")

# Test with normal features
normal_features = {
    "latency": 205.0,
    "gpu_util": 0.74,
    "memory": 0.61,
}

result = ml_baseline.get_ml_anomaly_score(normal_features)
print(f"\\nNormal features:")
print(f"  Anomaly detected: {result.is_anomaly}")
print(f"  ML score: {result.ml_score:.3f}")
"""
    print(code)


# ============================================================================
# Example 3: Drift Detection with Statistical Tests
# ============================================================================
def example_3_drift_detection():
    """Detect concept drift using KS test."""
    print("\n" + "="*70)
    print("Example 3: Drift Detection (Kolmogorov-Smirnov Test)")
    print("="*70)
    
    code = """
from neuralbudget.anomaly_detection import DriftDetector
import random

# Create drift detector
detector = DriftDetector(reference_window=100, test_window=50)

# Phase 1: Add reference data (mean=200ms)
print("Phase 1: Building reference (100 samples, mean~200ms)")
for i in range(100):
    value = 200.0 + random.gauss(0, 20)
    detector.add_observation(value)

# Phase 2: Add test data (still mean~200ms)
print("Phase 2: Adding test data (50 samples, mean~200ms)")
for i in range(50):
    value = 200.0 + random.gauss(0, 20)
    detector.add_observation(value)

drift = detector.detect_drift()
print(f"\\nNo drift expected:")
print(f"  Is drifting: {drift.is_drifting}")
print(f"  Drift score: {drift.drift_score:.3f}")
print(f"  KS p-value: {drift.ks_p_value:.4f}")

# Phase 3: Significant mean shift (mean=300ms)
print("\\nPhase 3: Adding drifted data (50 samples, mean~300ms)")
for i in range(50):
    value = 300.0 + random.gauss(0, 20)
    detector.add_observation(value)

drift = detector.detect_drift()
print(f"\\nDrift expected:")
print(f"  Is drifting: {drift.is_drifting}")
print(f"  Drift score: {drift.drift_score:.3f}")
print(f"  KS p-value: {drift.ks_p_value:.4f}")
print(f"  Mean shift: {drift.mean_shift:.2f}")
print(f"  Drift indices: {drift.drift_indices}")
"""
    print(code)


# ============================================================================
# Example 4: Feature Importance for Drift
# ============================================================================
def example_4_feature_importance():
    """Calculate feature importance for drift/anomalies."""
    print("\n" + "="*70)
    print("Example 4: Feature Importance Analysis")
    print("="*70)
    
    code = """
from neuralbudget.anomaly_detection import FeatureImportanceCalculator

# Normal baseline features
baseline = {
    "inference_latency": 150.0,
    "gpu_utilization": 0.75,
    "model_size": 2048.0,
    "cache_hits": 0.85,
}

# Current observation with anomalies
current = {
    "inference_latency": 800.0,   # 5.3x baseline (BIG change)
    "gpu_utilization": 0.92,      # 1.2x baseline
    "model_size": 2048.0,         # No change
    "cache_hits": 0.30,           # 0.35x baseline (BIG change)
}

# Calculate importance
importances = FeatureImportanceCalculator.get_baseline_feature_importance(
    current, baseline
)

print("Feature importance (sorted by impact):")
for imp in importances:
    print(f"\\n  {imp.feature_name}")
    print(f"    Baseline: {imp.baseline_value:.2f}")
    print(f"    Current:  {imp.current_value:.2f}")
    print(f"    Deviation: {imp.deviation:.2f}")
    print(f"    Importance: {imp.importance_score:.3f}")
    print(f"    Contribution: {imp.contribution_percent:.1f}%")

# Top 3 contributing features
print(f"\\nTop 3 features driving the anomaly:")
for i, imp in enumerate(importances[:3], 1):
    print(f"  {i}. {imp.feature_name}: {imp.contribution_percent:.1f}%")
"""
    print(code)


# ============================================================================
# Example 5: Drift Explanation
# ============================================================================
def example_5_drift_explanation():
    """Generate human-readable drift explanations."""
    print("\n" + "="*70)
    print("Example 5: Drift Explanation with Feature Importance")
    print("="*70)
    
    code = """
from neuralbudget.anomaly_detection import (
    DriftDetection, DriftExplainer, FeatureImportanceCalculator
)

# Simulate drift detection result
drift = DriftDetection(
    timestamp="2024-01-15T10:30:00Z",
    is_drifting=True,
    drift_score=0.72,
    ks_statistic=0.45,
    ks_p_value=0.001,
    drift_indices={"mean_shift": 50.0},
    reference_mean=200.0,
    current_mean=250.0,
    mean_shift=50.0,
)

# Feature baselines and current values
baseline = {
    "model_accuracy": 0.92,
    "latency": 150.0,
    "throughput": 100.0,
}
current = {
    "model_accuracy": 0.85,  # 7% drop
    "latency": 210.0,        # 40% increase
    "throughput": 75.0,      # 25% decrease
}

# Generate explanation
explanation = DriftExplainer.explain_drift(drift, current, baseline)

print(f"Drift Summary")
print(f"  Is Drifting: {explanation.is_drifting}")
print(f"  Severity: {explanation.severity.upper()}")
print(f"  Score: {explanation.drift_score:.3f}")

print(f"\\nExplanation:")
print(f"  {explanation.explanation}")

print(f"\\nTop Contributing Features:")
for i, feat in enumerate(explanation.top_contributing_features[:3], 1):
    print(f"  {i}. {feat.feature_name}")
    print(f"     - Baseline: {feat.baseline_value:.2f}")
    print(f"     - Current: {feat.current_value:.2f}")
    print(f"     - Contribution: {feat.contribution_percent:.1f}%")

print(f"\\nAffected Metrics: {', '.join(explanation.affected_metrics)}")
print(f"\\nRecommended Action:")
print(f"  {explanation.recommended_action}")
"""
    print(code)


# ============================================================================
# Example 6: Adaptive ML SLO Evaluation
# ============================================================================
def example_6_adaptive_ml():
    """Adaptive ML SLO evaluation with anomaly detection."""
    print("\n" + "="*70)
    print("Example 6: Adaptive ML SLO with Anomaly Detection")
    print("="*70)
    
    code = """
from neuralbudget.adaptive_slo import AdaptiveMlSloEvaluator
from neuralbudget import evaluate_ml_once

# Create adaptive evaluator
adaptive = AdaptiveMlSloEvaluator(
    baseline_window=100,
    contamination=0.05,
    enable_drift_detection=True,
)

# Sample data
samples = [
    {"timestamp": i, "inference_latency_ms": 150 + random.gauss(0, 10),
     "gpu_utilization": 0.75 + random.gauss(0, 0.02),
     "feature_drift": 0.1 + random.gauss(0, 0.02),
     "prediction_confidence": 0.9 + random.gauss(0, 0.01)}
    for i in range(50)
]

# Add anomaly
samples.append({
    "timestamp": 50,
    "inference_latency_ms": 800.0,  # Anomaly!
    "gpu_utilization": 0.95,
    "feature_drift": 0.8,
    "prediction_confidence": 0.6,
})

# Evaluate samples
for i, sample in enumerate(samples[-3:]):
    baseline_result = evaluate_ml_once(sample)
    adaptive_result = adaptive.evaluate_with_anomaly_detection(
        sample, baseline_result
    )
    
    print(f"\\nSample {i+1}:")
    print(f"  Latency: {sample['inference_latency_ms']:.0f}ms")
    print(f"  Hybrid score: {adaptive_result.hybrid_score:.3f}")
    print(f"  Anomaly detected: {adaptive_result.anomaly_detected}")
    print(f"  Anomaly score: {adaptive_result.anomaly_score:.3f}")
    print(f"  Drift detected: {adaptive_result.is_drifting}")
    print(f"  Confidence: {adaptive_result.confidence_score:.1%}")
    print(f"  Pass: {adaptive_result.passed}")
"""
    print(code)


# ============================================================================
# Example 7: Adaptive GenAI SLO Evaluation
# ============================================================================
def example_7_adaptive_genai():
    """Adaptive GenAI SLO evaluation with anomaly detection."""
    print("\n" + "="*70)
    print("Example 7: Adaptive GenAI SLO with Anomaly Detection")
    print("="*70)
    
    code = """
from neuralbudget.adaptive_slo import AdaptiveGenAiSloEvaluator
from neuralbudget import evaluate_genai_once

# Create adaptive evaluator
adaptive = AdaptiveGenAiSloEvaluator(
    baseline_window=100,
    contamination=0.05,
    enable_drift_detection=True,
)

# Normal GenAI samples
samples = [
    {"timestamp": i,
     "tokens_per_second": 25.0 + random.gauss(0, 2),
     "time_to_first_token_ms": 100.0 + random.gauss(0, 15),
     "semantic_similarity": 0.85 + random.gauss(0, 0.03)}
    for i in range(50)
]

# Add anomaly: degraded performance
samples.append({
    "timestamp": 50,
    "tokens_per_second": 5.0,     # Way below normal!
    "time_to_first_token_ms": 800.0,  # High latency!
    "semantic_similarity": 0.45,       # Quality drop!
})

# Evaluate
for i, sample in enumerate(samples[-3:]):
    baseline_result = evaluate_genai_once(sample)
    adaptive_result = adaptive.evaluate_with_anomaly_detection(
        sample, baseline_result
    )
    
    print(f"\\nSample {i+1}:")
    print(f"  TPS: {sample['tokens_per_second']:.1f}")
    print(f"  TTFT: {sample['time_to_first_token_ms']:.0f}ms")
    print(f"  Similarity: {sample['semantic_similarity']:.2f}")
    print(f"  Anomaly detected: {adaptive_result.anomaly_detected}")
    print(f"  Anomaly score: {adaptive_result.anomaly_score:.3f}")
    print(f"  Drift detected: {adaptive_result.is_drifting}")
    if adaptive_result.drift_explanation:
        print(f"  Drift reason: {adaptive_result.drift_explanation[:60]}...")
    print(f"  Pass: {adaptive_result.passed}")
"""
    print(code)


# ============================================================================
# Example 8: Practical Integration
# ============================================================================
def example_8_integration():
    """Practical integration with production ML pipeline."""
    print("\n" + "="*70)
    print("Example 8: Production Integration Pattern")
    print("="*70)
    
    code = """
from neuralbudget.adaptive_slo import AdaptiveMlSloEvaluator
from neuralbudget import evaluate_ml_once
import logging

logger = logging.getLogger("ml_slo")

# Initialize evaluator
adaptive = AdaptiveMlSloEvaluator()

def evaluate_model_with_drift_detection(model_metrics):
    \"\"\"Evaluate model with adaptive SLO and drift detection.\"\"\"
    
    # Standard SLO evaluation
    baseline_result = evaluate_ml_once(model_metrics)
    
    # Enhanced with anomaly detection
    adaptive_result = adaptive.evaluate_with_anomaly_detection(
        model_metrics, baseline_result
    )
    
    # Log results
    if adaptive_result.anomaly_detected:
        logger.warning(
            f"Anomaly detected: {adaptive_result.anomaly_reason} "
            f"(score: {adaptive_result.anomaly_score:.3f})"
        )
    
    if adaptive_result.is_drifting:
        logger.error(
            f"Drift detected: {adaptive_result.drift_explanation} "
            f"Top features: {[f.feature_name for f in adaptive_result.top_contributing_features[:2]]}"
        )
    
    # Alert if confidence is low
    if adaptive_result.confidence_score < 0.8:
        logger.warning(
            f"Low confidence SLO evaluation: {adaptive_result.confidence_score:.1%}"
        )
    
    # Determine action
    if adaptive_result.passed:
        logger.info("SLO: PASS")
        return "PASS"
    else:
        logger.error("SLO: FAIL")
        if adaptive_result.anomaly_detected:
            return "FAIL_ANOMALY"
        elif adaptive_result.is_drifting:
            return "FAIL_DRIFT"
        else:
            return "FAIL_THRESHOLD"

# Usage in production
model_metrics = {
    "timestamp": 1705317000,
    "inference_latency_ms": 165.0,
    "gpu_utilization": 0.73,
    "feature_drift": 0.08,
    "prediction_confidence": 0.92,
}

result = evaluate_model_with_drift_detection(model_metrics)
print(f"Evaluation result: {result}")
"""
    print(code)


# ============================================================================
# Main
# ============================================================================
def main():
    """Run all examples."""
    import sys

    examples = [
        ("1", "Statistical Anomaly Detection", example_1_statistical_anomaly),
        ("2", "ML-Based Anomaly Detection", example_2_ml_anomaly),
        ("3", "Drift Detection", example_3_drift_detection),
        ("4", "Feature Importance", example_4_feature_importance),
        ("5", "Drift Explanation", example_5_drift_explanation),
        ("6", "Adaptive ML SLO", example_6_adaptive_ml),
        ("7", "Adaptive GenAI SLO", example_7_adaptive_genai),
        ("8", "Production Integration", example_8_integration),
    ]

    if len(sys.argv) > 1:
        example_num = sys.argv[1]
        for num, title, func in examples:
            if num == example_num:
                func()
                return
        print(f"Example {example_num} not found")
        sys.exit(1)

    # Show menu
    print("\n" + "="*70)
    print("Anomaly Detection & Drift Explanation - Examples")
    print("="*70)
    print("\nAvailable examples:\n")
    for num, title, _ in examples:
        print(f"  {num}. {title}")

    print("\n" + "="*70)
    print("Usage: python examples/python/anomaly_drift_examples.py <number>")
    print("Example: python examples/python/anomaly_drift_examples.py 1")
    print("="*70 + "\n")


if __name__ == "__main__":
    main()
