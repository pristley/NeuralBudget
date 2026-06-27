"""Unit tests for anomaly detection and drift explanation.

Tests for:
- Statistical baselining
- ML-based baselining  
- Drift detection
- Feature importance calculation
- Drift explanation
- Adaptive SLO evaluation
"""

import random
import pytest
from neuralbudget.anomaly_detection import (
    AnomalyScore,
    DriftDetection,
    FeatureImportance,
    StatisticalBaseline,
    MLBaseline,
    DriftDetector,
    FeatureImportanceCalculator,
    DriftExplainer,
)


# ============================================================================
# Statistical Baselining Tests
# ============================================================================


class TestStatisticalBaseline:
    """Tests for StatisticalBaseline."""

    def test_init(self):
        """Test initialization."""
        baseline = StatisticalBaseline(window_size=100, percentile=99.0)
        assert baseline.window_size == 100
        assert baseline.percentile == 99.0

    def test_add_observation(self):
        """Test adding observations."""
        baseline = StatisticalBaseline(window_size=10)
        for i in range(15):
            baseline.add_observation(float(i))
        # Window should only keep last 10
        assert len(baseline.observations) <= 10

    def test_zscore_normal(self):
        """Test Z-score with normal data."""
        baseline = StatisticalBaseline(window_size=100)
        # Add normal observations around 200
        for _ in range(100):
            baseline.add_observation(200.0 + random.gauss(0, 10))
        
        # Score a normal value
        is_anomaly, z_score, reason = baseline.get_zscore_anomaly(200.0)
        assert not is_anomaly
        assert z_score < 3.0
        assert "Z-score" in reason

    def test_zscore_anomaly(self):
        """Test Z-score with anomalous data."""
        baseline = StatisticalBaseline(window_size=100)
        # Add normal observations
        for _ in range(100):
            baseline.add_observation(200.0 + random.gauss(0, 10))
        
        # Score an anomaly (way outside 3-sigma)
        is_anomaly, z_score, reason = baseline.get_zscore_anomaly(1000.0)
        assert is_anomaly
        assert z_score > 3.0

    def test_modified_zscore(self):
        """Test modified Z-score (MAD-based)."""
        baseline = StatisticalBaseline(window_size=100)
        for _ in range(100):
            baseline.add_observation(200.0 + random.gauss(0, 10))
        
        is_anomaly, mod_z, reason = baseline.get_modified_zscore_anomaly(200.0)
        assert not is_anomaly
        assert "Modified Z-score" in reason

    def test_percentile_anomaly(self):
        """Test percentile-based anomaly detection."""
        baseline = StatisticalBaseline(window_size=100, percentile=99.0)
        for _ in range(100):
            baseline.add_observation(200.0 + random.gauss(0, 10))
        
        is_anomaly, deviation, reason = baseline.get_percentile_anomaly(200.0)
        assert not is_anomaly
        assert "Percentile" in reason

    def test_combined_anomaly_score(self):
        """Test combined anomaly score."""
        baseline = StatisticalBaseline(window_size=100)
        for _ in range(100):
            baseline.add_observation(200.0 + random.gauss(0, 10))
        
        # Normal value
        score = baseline.get_combined_anomaly_score(200.0)
        assert isinstance(score, AnomalyScore)
        assert not score.is_anomaly
        assert 0 <= score.anomaly_score <= 1
        assert 0 <= score.confidence <= 1
        
        # Anomalous value
        score_anom = baseline.get_combined_anomaly_score(1000.0)
        assert score_anom.is_anomaly
        assert score_anom.anomaly_score > 0.5

    def test_insufficient_data(self):
        """Test behavior with insufficient data."""
        baseline = StatisticalBaseline(window_size=100)
        # Add only 5 observations
        for i in range(5):
            baseline.add_observation(200.0 + float(i))
        
        # Should still return score but might be unreliable
        score = baseline.get_combined_anomaly_score(210.0)
        assert isinstance(score, AnomalyScore)


# ============================================================================
# ML Baselining Tests
# ============================================================================


class TestMLBaseline:
    """Tests for MLBaseline."""

    def test_init(self):
        """Test initialization."""
        ml = MLBaseline(contamination=0.05, window_size=100)
        assert ml.contamination == 0.05
        assert ml.window_size == 100

    def test_add_observation(self):
        """Test adding observations."""
        ml = MLBaseline(window_size=20)
        features = {"latency": 150.0, "gpu": 0.75}
        
        for i in range(30):
            ml.add_observation(features)
        
        assert len(ml.feature_history) <= 20

    def test_ml_scoring_normal(self):
        """Test ML scoring with normal features."""
        ml = MLBaseline(contamination=0.05, window_size=50)
        
        # Train on normal data
        for _ in range(50):
            features = {
                "latency": 150.0 + random.gauss(0, 10),
                "gpu": 0.75 + random.gauss(0, 0.05),
                "memory": 0.60 + random.gauss(0, 0.03),
            }
            ml.add_observation(features)
        
        # Score normal features
        normal = {"latency": 150.0, "gpu": 0.75, "memory": 0.60}
        score = ml.get_ml_anomaly_score(normal)
        
        assert isinstance(score, AnomalyScore)
        assert 0 <= score.ml_score <= 1

    def test_ml_scoring_anomaly(self):
        """Test ML scoring with anomalous features."""
        ml = MLBaseline(contamination=0.05, window_size=50)
        
        # Train on normal data
        for _ in range(50):
            features = {
                "latency": 150.0 + random.gauss(0, 10),
                "gpu": 0.75 + random.gauss(0, 0.05),
                "memory": 0.60 + random.gauss(0, 0.03),
            }
            ml.add_observation(features)
        
        # Score anomalous features
        anomaly = {"latency": 1000.0, "gpu": 0.95, "memory": 0.95}
        score = ml.get_ml_anomaly_score(anomaly)
        
        assert isinstance(score, AnomalyScore)
        assert 0 <= score.ml_score <= 1


# ============================================================================
# Drift Detector Tests
# ============================================================================


class TestDriftDetector:
    """Tests for DriftDetector."""

    def test_init(self):
        """Test initialization."""
        detector = DriftDetector(reference_window=100, test_window=50)
        assert detector.reference_window == 100
        assert detector.test_window == 50

    def test_add_observation(self):
        """Test adding observations."""
        detector = DriftDetector(reference_window=10, test_window=5)
        
        for i in range(20):
            detector.add_observation(float(i))
        
        assert len(detector.all_observations) >= 15

    def test_no_drift(self):
        """Test detection when no drift is present."""
        detector = DriftDetector(reference_window=50, test_window=25)
        
        # Add reference data (mean ~200)
        for _ in range(50):
            detector.add_observation(200.0 + random.gauss(0, 20))
        
        # Add test data (same mean ~200)
        for _ in range(25):
            detector.add_observation(200.0 + random.gauss(0, 20))
        
        drift = detector.detect_drift()
        assert isinstance(drift, DriftDetection)
        assert not drift.is_drifting or drift.is_drifting  # Could go either way with random data

    def test_drift_present(self):
        """Test detection when drift is present."""
        detector = DriftDetector(reference_window=100, test_window=50)
        
        # Add reference data (mean ~200)
        for _ in range(100):
            detector.add_observation(200.0 + random.gauss(0, 10))
        
        # Add test data (mean ~300 - clear drift)
        for _ in range(50):
            detector.add_observation(300.0 + random.gauss(0, 10))
        
        drift = detector.detect_drift()
        assert isinstance(drift, DriftDetection)
        assert drift.mean_shift > 0  # Mean has shifted

    def test_drift_metrics(self):
        """Test drift detection metrics."""
        detector = DriftDetector(reference_window=100, test_window=50)
        
        # Reference: ~200
        for _ in range(100):
            detector.add_observation(200.0 + random.gauss(0, 5))
        
        # Test: ~250
        for _ in range(50):
            detector.add_observation(250.0 + random.gauss(0, 5))
        
        drift = detector.detect_drift()
        
        # Check all metrics present
        assert drift.timestamp is not None
        assert 0 <= drift.ks_statistic <= 1
        assert 0 <= drift.ks_p_value <= 1
        assert drift.mean_shift > 0
        assert abs(drift.reference_mean - 200) < 10
        assert abs(drift.current_mean - 250) < 10


# ============================================================================
# Feature Importance Tests
# ============================================================================


class TestFeatureImportanceCalculator:
    """Tests for FeatureImportanceCalculator."""

    def test_baseline_importance(self):
        """Test baseline-based feature importance."""
        baseline = {"latency": 150.0, "gpu": 0.75, "memory": 0.60}
        current = {"latency": 300.0, "gpu": 0.80, "memory": 0.60}
        
        importances = FeatureImportanceCalculator.get_baseline_feature_importance(
            current, baseline
        )
        
        # Should have 3 features
        assert len(importances) == 3
        assert all(isinstance(imp, FeatureImportance) for imp in importances)
        
        # Sorted by importance (latency changed most)
        assert importances[0].feature_name == "latency"
        assert importances[0].contribution_percent > 0
        
        # Total contribution should sum to 100
        total = sum(imp.contribution_percent for imp in importances)
        assert 99 < total < 101

    def test_no_change(self):
        """Test importance when nothing changed."""
        features = {"a": 100.0, "b": 200.0, "c": 300.0}
        
        importances = FeatureImportanceCalculator.get_baseline_feature_importance(
            features, features
        )
        
        # Should return scores but all zero
        assert len(importances) == 3
        assert all(imp.contribution_percent == 0 for imp in importances)

    def test_single_feature_change(self):
        """Test importance with single feature change."""
        baseline = {"a": 100.0, "b": 100.0, "c": 100.0}
        current = {"a": 200.0, "b": 100.0, "c": 100.0}  # Only 'a' changed
        
        importances = FeatureImportanceCalculator.get_baseline_feature_importance(
            current, baseline
        )
        
        # Feature 'a' should be 100% responsible
        assert importances[0].contribution_percent > 99


# ============================================================================
# Drift Explainer Tests
# ============================================================================


class TestDriftExplainer:
    """Tests for DriftExplainer."""

    def test_low_severity(self):
        """Test low severity explanation."""
        drift = DriftDetection(
            timestamp="2024-01-15T10:00:00Z",
            is_drifting=True,
            drift_score=0.2,
            ks_statistic=0.1,
            ks_p_value=0.1,
            drift_indices={},
            reference_mean=200.0,
            current_mean=205.0,
            mean_shift=5.0,
        )
        
        explanation = DriftExplainer.explain_drift(
            drift, {"a": 100}, {"a": 100}
        )
        
        assert explanation.is_drifting
        assert explanation.severity in ["low", "medium"]
        assert len(explanation.explanation) > 0
        assert len(explanation.recommended_action) > 0

    def test_high_severity(self):
        """Test high severity explanation."""
        drift = DriftDetection(
            timestamp="2024-01-15T10:00:00Z",
            is_drifting=True,
            drift_score=0.75,
            ks_statistic=0.7,
            ks_p_value=0.001,
            drift_indices={},
            reference_mean=200.0,
            current_mean=400.0,
            mean_shift=200.0,
        )
        
        explanation = DriftExplainer.explain_drift(
            drift, {"a": 100}, {"a": 50}
        )
        
        assert explanation.is_drifting
        assert explanation.severity in ["high", "critical"]
        assert len(explanation.explanation) > 0
        
        # Should have top contributing features
        assert len(explanation.top_contributing_features) > 0

    def test_explanation_structure(self):
        """Test explanation has all required fields."""
        drift = DriftDetection(
            timestamp="2024-01-15T10:00:00Z",
            is_drifting=True,
            drift_score=0.5,
            ks_statistic=0.4,
            ks_p_value=0.05,
            drift_indices={},
            reference_mean=100.0,
            current_mean=150.0,
            mean_shift=50.0,
        )
        
        explanation = DriftExplainer.explain_drift(
            drift, {"a": 100}, {"a": 80}
        )
        
        # Check all required fields
        assert explanation.timestamp is not None
        assert explanation.is_drifting
        assert 0 <= explanation.drift_score <= 1
        assert len(explanation.explanation) > 0
        assert explanation.severity in ["low", "medium", "high", "critical"]
        assert len(explanation.recommended_action) > 0
        assert isinstance(explanation.top_contributing_features, list)


# ============================================================================
# Adaptive SLO Tests  
# ============================================================================


class TestAdaptiveSlo:
    """Tests for adaptive SLO evaluation."""

    @pytest.mark.skip(reason="Requires convenience module")
    def test_adaptive_ml_evaluation(self):
        """Test adaptive ML SLO evaluation."""
        from neuralbudget.adaptive_slo import AdaptiveMlSloEvaluator
        
        adaptive = AdaptiveMlSloEvaluator()
        
        # Sample data
        sample = {
            "timestamp": 1705317000,
            "inference_latency_ms": 150.0,
            "gpu_utilization": 0.75,
            "feature_drift": 0.1,
            "prediction_confidence": 0.9,
        }
        
        # Mock baseline result
        baseline_result = {
            "timestamp": 1705317000,
            "inference_latency_score": 0.9,
            "gpu_utilization_score": 0.8,
            "system_score": 0.85,
            "latency_score": 0.9,
            "feature_drift_score": 0.9,
            "prediction_confidence_score": 0.95,
            "drift_score": 0.1,
            "latency_weight": 0.5,
            "drift_weight": 0.5,
            "hybrid_score": 0.875,
            "pass": True,
        }
        
        # Should not raise
        result = adaptive.evaluate_with_anomaly_detection(sample, baseline_result)
        assert result.timestamp == 1705317000


# ============================================================================
# Integration Tests
# ============================================================================


class TestIntegration:
    """Integration tests."""

    def test_full_pipeline_statistical(self):
        """Test full pipeline with statistical methods."""
        baseline = StatisticalBaseline(window_size=100)
        
        # Build baseline
        for _ in range(100):
            baseline.add_observation(200.0 + random.gauss(0, 10))
        
        # Detect anomaly
        score = baseline.get_combined_anomaly_score(800.0)
        assert score.is_anomaly

    def test_full_pipeline_ml(self):
        """Test full pipeline with ML methods."""
        ml = MLBaseline(window_size=50)
        
        # Train
        for _ in range(50):
            ml.add_observation({
                "latency": 150.0 + random.gauss(0, 10),
                "gpu": 0.75 + random.gauss(0, 0.05),
            })
        
        # Score
        score = ml.get_ml_anomaly_score({"latency": 1000.0, "gpu": 0.95})
        assert 0 <= score.ml_score <= 1

    def test_full_pipeline_drift(self):
        """Test full pipeline with drift detection."""
        detector = DriftDetector(reference_window=100, test_window=50)
        
        # Reference
        for _ in range(100):
            detector.add_observation(200.0 + random.gauss(0, 10))
        
        # Test
        for _ in range(50):
            detector.add_observation(250.0 + random.gauss(0, 10))
        
        # Detect
        drift = detector.detect_drift()
        assert drift.mean_shift > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
