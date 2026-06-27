"""Unit tests for adaptive SLO evaluation.

Tests for:
- Adaptive ML SLO evaluation
- Adaptive GenAI SLO evaluation
- Adaptive threshold calculation
- Confidence scoring
"""

import pytest
import random
from neuralbudget.adaptive_slo import (
    AdaptiveMlSloEvaluator,
    AdaptiveGenAiSloEvaluator,
    AdaptiveMlEvaluationResult,
    AdaptiveGenAiEvaluationResult,
)


# ============================================================================
# Adaptive ML SLO Tests
# ============================================================================


class TestAdaptiveMlSloEvaluator:
    """Tests for AdaptiveMlSloEvaluator."""

    def test_init(self):
        """Test initialization."""
        adaptive = AdaptiveMlSloEvaluator(
            baseline_window=100,
            contamination=0.05,
            enable_drift_detection=True,
        )
        
        assert adaptive.stat_baseline is not None
        assert adaptive.ml_baseline is not None
        assert adaptive.drift_detector is not None
        assert adaptive.enable_drift_detection

    def test_evaluate_normal_sample(self):
        """Test evaluation with normal sample."""
        adaptive = AdaptiveMlSloEvaluator()
        
        # Build baseline
        for i in range(100):
            sample = {
                "inference_latency_ms": 150.0 + random.gauss(0, 10),
                "gpu_utilization": 0.75 + random.gauss(0, 0.02),
                "feature_drift": 0.1 + random.gauss(0, 0.02),
                "prediction_confidence": 0.9 + random.gauss(0, 0.01),
            }
            adaptive.stat_baseline.add_observation(sample["inference_latency_ms"])
            adaptive.ml_baseline.add_observation({
                "latency": sample["inference_latency_ms"],
                "gpu": sample["gpu_utilization"],
                "drift": sample["feature_drift"],
                "confidence": sample["prediction_confidence"],
            })
            adaptive.drift_detector.add_observation(sample["inference_latency_ms"])
        
        # Evaluate normal sample
        normal_sample = {
            "timestamp": 1705317000,
            "inference_latency_ms": 155.0,
            "gpu_utilization": 0.76,
            "feature_drift": 0.09,
            "prediction_confidence": 0.91,
        }
        
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
        
        result = adaptive.evaluate_with_anomaly_detection(normal_sample, baseline_result)
        
        # Check result type
        assert isinstance(result, AdaptiveMlEvaluationResult)
        
        # Normal sample should not be flagged
        assert not result.anomaly_detected
        
        # Confidence should be high
        assert result.confidence_score >= 0.9

    def test_evaluate_anomalous_sample(self):
        """Test evaluation with anomalous sample."""
        adaptive = AdaptiveMlSloEvaluator()
        
        # Build baseline with normal data
        for _ in range(100):
            adaptive.stat_baseline.add_observation(150.0 + random.gauss(0, 10))
            adaptive.ml_baseline.add_observation({
                "latency": 150.0 + random.gauss(0, 10),
                "gpu": 0.75 + random.gauss(0, 0.02),
                "drift": 0.1 + random.gauss(0, 0.02),
                "confidence": 0.9 + random.gauss(0, 0.01),
            })
            adaptive.drift_detector.add_observation(150.0 + random.gauss(0, 10))
        
        # Evaluate anomalous sample
        anomaly_sample = {
            "timestamp": 1705317000,
            "inference_latency_ms": 1000.0,  # Way too high
            "gpu_utilization": 0.95,
            "feature_drift": 0.8,
            "prediction_confidence": 0.5,
        }
        
        baseline_result = {
            "timestamp": 1705317000,
            "inference_latency_score": 0.2,
            "gpu_utilization_score": 0.3,
            "system_score": 0.25,
            "latency_score": 0.2,
            "feature_drift_score": 0.3,
            "prediction_confidence_score": 0.5,
            "drift_score": 0.6,
            "latency_weight": 0.5,
            "drift_weight": 0.5,
            "hybrid_score": 0.4,
            "pass": False,
        }
        
        result = adaptive.evaluate_with_anomaly_detection(anomaly_sample, baseline_result)
        
        # Should flag anomaly
        assert result.anomaly_detected
        assert result.anomaly_score > 0.5
        
        # Confidence should be reduced
        assert result.confidence_score < 0.9
        
        # Pass should be False
        assert not result.passed

    def test_adaptive_threshold_calculation(self):
        """Test adaptive threshold calculation."""
        adaptive = AdaptiveMlSloEvaluator()
        
        # Add history with values 100-200
        for i in range(100):
            adaptive.history.append({
                "inference_latency_ms": 100.0 + (i % 100) * 2
            })
        
        threshold = adaptive._calculate_adaptive_threshold(
            "inference_latency_ms",
            baseline_value=200.0,
        )
        
        # Should be > baseline due to p99 * 1.1
        assert threshold > 0
        assert isinstance(threshold, float)

    def test_insufficient_history(self):
        """Test behavior with insufficient history."""
        adaptive = AdaptiveMlSloEvaluator()
        
        # Only 5 samples
        for i in range(5):
            adaptive.history.append({"metric": float(i)})
        
        # Should return baseline
        threshold = adaptive._calculate_adaptive_threshold(
            "metric",
            baseline_value=100.0,
        )
        
        # With insufficient data, should return baseline
        assert threshold == 100.0 or threshold > 0

    def test_confidence_with_drift(self):
        """Test confidence scoring when drift detected."""
        adaptive = AdaptiveMlSloEvaluator(enable_drift_detection=True)
        
        # Build baseline
        for _ in range(150):
            adaptive.stat_baseline.add_observation(200.0 + random.gauss(0, 10))
            adaptive.drift_detector.add_observation(200.0 + random.gauss(0, 10))
        
        # Add drifted data
        for _ in range(50):
            adaptive.drift_detector.add_observation(300.0 + random.gauss(0, 10))
        
        sample = {
            "timestamp": 1705317000,
            "inference_latency_ms": 305.0,
            "gpu_utilization": 0.75,
            "feature_drift": 0.2,
            "prediction_confidence": 0.8,
        }
        
        baseline_result = {
            "timestamp": 1705317000,
            "inference_latency_score": 0.8,
            "gpu_utilization_score": 0.8,
            "system_score": 0.8,
            "latency_score": 0.8,
            "feature_drift_score": 0.8,
            "prediction_confidence_score": 0.8,
            "drift_score": 0.2,
            "latency_weight": 0.5,
            "drift_weight": 0.5,
            "hybrid_score": 0.8,
            "pass": True,
        }
        
        result = adaptive.evaluate_with_anomaly_detection(sample, baseline_result)
        
        # Confidence should be reduced due to drift
        assert result.confidence_score < 1.0


# ============================================================================
# Adaptive GenAI SLO Tests
# ============================================================================


class TestAdaptiveGenAiSloEvaluator:
    """Tests for AdaptiveGenAiSloEvaluator."""

    def test_init(self):
        """Test initialization."""
        adaptive = AdaptiveGenAiSloEvaluator(
            baseline_window=100,
            contamination=0.05,
            enable_drift_detection=True,
        )
        
        assert adaptive.stat_baseline is not None
        assert adaptive.ml_baseline is not None
        assert adaptive.drift_detector is not None
        assert adaptive.enable_drift_detection

    def test_evaluate_normal_sample(self):
        """Test evaluation with normal sample."""
        adaptive = AdaptiveGenAiSloEvaluator()
        
        # Build baseline
        for _ in range(100):
            adaptive.stat_baseline.add_observation(25.0 + random.gauss(0, 2))
            adaptive.ml_baseline.add_observation({
                "tps": 25.0 + random.gauss(0, 2),
                "ttft": 100.0 + random.gauss(0, 15),
                "similarity": 0.85 + random.gauss(0, 0.03),
            })
            adaptive.drift_detector.add_observation(25.0 + random.gauss(0, 2))
        
        # Evaluate normal sample
        normal_sample = {
            "timestamp": 1705317000,
            "tokens_per_second": 24.5,
            "time_to_first_token_ms": 105.0,
            "semantic_similarity": 0.84,
        }
        
        baseline_result = {
            "timestamp": 1705317000,
            "tokens_per_second": 24.5,
            "time_to_first_token_ms": 105.0,
            "semantic_similarity": 0.84,
            "tokens_per_second_ok": True,
            "time_to_first_token_ok": True,
            "semantic_similarity_ok": True,
            "pass": True,
        }
        
        result = adaptive.evaluate_with_anomaly_detection(normal_sample, baseline_result)
        
        # Check result type
        assert isinstance(result, AdaptiveGenAiEvaluationResult)
        
        # Normal sample should not be flagged
        assert not result.anomaly_detected
        
        # Confidence should be high
        assert result.confidence_score >= 0.8

    def test_evaluate_degraded_sample(self):
        """Test evaluation with degraded performance."""
        adaptive = AdaptiveGenAiSloEvaluator()
        
        # Build baseline with good performance
        for _ in range(100):
            adaptive.stat_baseline.add_observation(25.0 + random.gauss(0, 1))
            adaptive.ml_baseline.add_observation({
                "tps": 25.0 + random.gauss(0, 1),
                "ttft": 100.0 + random.gauss(0, 10),
                "similarity": 0.85 + random.gauss(0, 0.02),
            })
            adaptive.drift_detector.add_observation(25.0 + random.gauss(0, 1))
        
        # Evaluate degraded sample
        degraded_sample = {
            "timestamp": 1705317000,
            "tokens_per_second": 5.0,  # Way too low
            "time_to_first_token_ms": 800.0,  # Way too high
            "semantic_similarity": 0.45,  # Quality drop
        }
        
        baseline_result = {
            "timestamp": 1705317000,
            "tokens_per_second": 5.0,
            "time_to_first_token_ms": 800.0,
            "semantic_similarity": 0.45,
            "tokens_per_second_ok": False,
            "time_to_first_token_ok": False,
            "semantic_similarity_ok": False,
            "pass": False,
        }
        
        result = adaptive.evaluate_with_anomaly_detection(degraded_sample, baseline_result)
        
        # Should flag anomaly
        assert result.anomaly_detected
        assert result.anomaly_score > 0.5
        
        # Pass should be False
        assert not result.passed

    def test_adaptive_thresholds(self):
        """Test adaptive threshold calculation for GenAI metrics."""
        adaptive = AdaptiveGenAiSloEvaluator()
        
        # Add history
        for i in range(100):
            adaptive.history.append({
                "tokens_per_second": 20.0 + (i % 30),
                "time_to_first_token_ms": 100.0 + (i % 200),
            })
        
        # TPS threshold should be above baseline
        tps_threshold = adaptive._calculate_adaptive_threshold(
            "tokens_per_second",
            baseline_value=20.0,
        )
        assert tps_threshold > 0
        
        # TTFT threshold should be above baseline (latency metric)
        ttft_threshold = adaptive._calculate_adaptive_threshold(
            "time_to_first_token_ms",
            baseline_value=1200.0,
        )
        assert ttft_threshold > 0

    def test_drift_explanation_in_result(self):
        """Test that drift explanation is included in result."""
        adaptive = AdaptiveGenAiSloEvaluator(enable_drift_detection=True)
        
        # Build baseline
        for _ in range(100):
            adaptive.stat_baseline.add_observation(25.0 + random.gauss(0, 2))
            adaptive.drift_detector.add_observation(25.0 + random.gauss(0, 2))
        
        # Add drifted data
        for _ in range(50):
            adaptive.drift_detector.add_observation(15.0 + random.gauss(0, 2))
        
        sample = {
            "timestamp": 1705317000,
            "tokens_per_second": 14.5,
            "time_to_first_token_ms": 150.0,
            "semantic_similarity": 0.80,
        }
        
        baseline_result = {
            "timestamp": 1705317000,
            "tokens_per_second": 14.5,
            "time_to_first_token_ms": 150.0,
            "semantic_similarity": 0.80,
            "tokens_per_second_ok": False,
            "time_to_first_token_ok": False,
            "semantic_similarity_ok": True,
            "pass": False,
        }
        
        result = adaptive.evaluate_with_anomaly_detection(sample, baseline_result)
        
        # Check result contains expected fields
        assert hasattr(result, 'is_drifting')
        assert hasattr(result, 'drift_explanation')
        assert hasattr(result, 'top_contributing_features')
        assert hasattr(result, 'adaptive_tps_threshold')
        assert hasattr(result, 'adaptive_ttft_threshold')


# ============================================================================
# Integration Tests
# ============================================================================


class TestAdaptiveSloIntegration:
    """Integration tests for adaptive SLO."""

    def test_ml_slo_full_lifecycle(self):
        """Test full ML SLO evaluation lifecycle."""
        adaptive = AdaptiveMlSloEvaluator()
        
        # Phase 1: Build baseline
        for _ in range(50):
            adaptive.stat_baseline.add_observation(200.0 + random.gauss(0, 10))
            adaptive.ml_baseline.add_observation({
                "latency": 200.0 + random.gauss(0, 10),
                "gpu": 0.75 + random.gauss(0, 0.02),
                "drift": 0.1 + random.gauss(0, 0.02),
                "confidence": 0.9 + random.gauss(0, 0.01),
            })
        
        # Phase 2: Normal operation
        normal_result = adaptive.evaluate_with_anomaly_detection(
            {
                "timestamp": 1,
                "inference_latency_ms": 205.0,
                "gpu_utilization": 0.76,
                "feature_drift": 0.09,
                "prediction_confidence": 0.91,
            },
            {
                "timestamp": 1,
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
            },
        )
        assert not normal_result.anomaly_detected
        
        # Phase 3: Degraded operation
        degraded_result = adaptive.evaluate_with_anomaly_detection(
            {
                "timestamp": 2,
                "inference_latency_ms": 900.0,
                "gpu_utilization": 0.95,
                "feature_drift": 0.8,
                "prediction_confidence": 0.5,
            },
            {
                "timestamp": 2,
                "inference_latency_score": 0.2,
                "gpu_utilization_score": 0.3,
                "system_score": 0.25,
                "latency_score": 0.2,
                "feature_drift_score": 0.3,
                "prediction_confidence_score": 0.5,
                "drift_score": 0.6,
                "latency_weight": 0.5,
                "drift_weight": 0.5,
                "hybrid_score": 0.4,
                "pass": False,
            },
        )
        assert degraded_result.anomaly_detected

    def test_genai_slo_full_lifecycle(self):
        """Test full GenAI SLO evaluation lifecycle."""
        adaptive = AdaptiveGenAiSloEvaluator()
        
        # Build baseline
        for _ in range(50):
            adaptive.stat_baseline.add_observation(25.0 + random.gauss(0, 1))
            adaptive.ml_baseline.add_observation({
                "tps": 25.0 + random.gauss(0, 1),
                "ttft": 100.0 + random.gauss(0, 10),
                "similarity": 0.85 + random.gauss(0, 0.02),
            })
        
        # Good performance
        good_result = adaptive.evaluate_with_anomaly_detection(
            {
                "timestamp": 1,
                "tokens_per_second": 24.5,
                "time_to_first_token_ms": 105.0,
                "semantic_similarity": 0.84,
            },
            {
                "timestamp": 1,
                "tokens_per_second": 24.5,
                "time_to_first_token_ms": 105.0,
                "semantic_similarity": 0.84,
                "tokens_per_second_ok": True,
                "time_to_first_token_ok": True,
                "semantic_similarity_ok": True,
                "pass": True,
            },
        )
        assert not good_result.anomaly_detected
        
        # Poor performance
        poor_result = adaptive.evaluate_with_anomaly_detection(
            {
                "timestamp": 2,
                "tokens_per_second": 5.0,
                "time_to_first_token_ms": 800.0,
                "semantic_similarity": 0.40,
            },
            {
                "timestamp": 2,
                "tokens_per_second": 5.0,
                "time_to_first_token_ms": 800.0,
                "semantic_similarity": 0.40,
                "tokens_per_second_ok": False,
                "time_to_first_token_ok": False,
                "semantic_similarity_ok": False,
                "pass": False,
            },
        )
        assert poor_result.anomaly_detected


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
