"""Integration of anomaly detection with ML/GenAI SLO evaluation.

This module connects the dynamic baselining and drift detection with the
existing ML and GenAI SLO evaluation, moving beyond static thresholds to
adaptive, data-driven SLO evaluation.

Features:
- Dynamic threshold adjustment based on historical patterns
- Anomaly-aware SLO scoring
- Drift-aware model evaluation
- Integrated feature importance in evaluation results
"""

import logging
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

from .anomaly_detection import (
    DriftDetection,
    DriftExplainer,
    DriftExplanation,
    FeatureImportance,
    FeatureImportanceCalculator,
    MLBaseline,
    StatisticalBaseline,
    DriftDetector,
    AnomalyScore,
)

logger = logging.getLogger(__name__)


# ============================================================================
# Data Models
# ============================================================================


@dataclass(frozen=True)
class AdaptiveMlEvaluationResult:
    """ML evaluation with anomaly detection and adaptive thresholds."""
    timestamp: int
    inference_latency_score: float
    gpu_utilization_score: float
    system_score: float
    latency_score: float
    feature_drift_score: float
    prediction_confidence_score: float
    drift_score: float
    latency_weight: float
    drift_weight: float
    hybrid_score: float
    passed: bool
    
    # Anomaly detection
    anomaly_detected: bool
    anomaly_score: float
    anomaly_reason: str
    
    # Drift explanation
    is_drifting: bool
    drift_explanation: Optional[str] = None
    top_contributing_features: List[FeatureImportance] = None
    
    # Adaptive thresholds
    adaptive_latency_threshold: float = None
    adaptive_drift_threshold: float = None
    confidence_score: float = 1.0


@dataclass(frozen=True)
class AdaptiveGenAiEvaluationResult:
    """GenAI evaluation with anomaly detection and adaptive thresholds."""
    timestamp: int
    tokens_per_second: float
    time_to_first_token_ms: float
    semantic_similarity: float
    tokens_per_second_ok: bool
    time_to_first_token_ok: bool
    semantic_similarity_ok: bool
    passed: bool
    
    # Anomaly detection
    anomaly_detected: bool
    anomaly_score: float
    anomaly_reason: str
    
    # Drift explanation
    is_drifting: bool
    drift_explanation: Optional[str] = None
    top_contributing_features: List[FeatureImportance] = None
    
    # Adaptive thresholds
    adaptive_tps_threshold: float = None
    adaptive_ttft_threshold: float = None
    confidence_score: float = 1.0


# ============================================================================
# Adaptive ML SLO Evaluator
# ============================================================================


class AdaptiveMlSloEvaluator:
    """ML SLO evaluator with dynamic baselining and drift detection."""

    def __init__(
        self,
        baseline_window: int = 100,
        contamination: float = 0.05,
        enable_drift_detection: bool = True,
    ):
        """Initialize adaptive ML SLO evaluator.

        Args:
            baseline_window: Window size for baseline
            contamination: Expected anomaly rate
            enable_drift_detection: Enable drift detection
        """
        self.stat_baseline = StatisticalBaseline(window_size=baseline_window)
        self.ml_baseline = MLBaseline(
            contamination=contamination,
            window_size=baseline_window,
        )
        self.drift_detector = DriftDetector(
            reference_window=baseline_window,
            test_window=baseline_window // 2,
        )
        self.enable_drift_detection = enable_drift_detection
        self.history: List[Dict[str, Any]] = []

    def evaluate_with_anomaly_detection(
        self,
        sample: Dict[str, Any],
        baseline_result: Dict[str, Any],
    ) -> AdaptiveMlEvaluationResult:
        """Evaluate ML sample with anomaly detection.

        Args:
            sample: ML sample with metrics
            baseline_result: Standard SLO evaluation result

        Returns:
            AdaptiveMlEvaluationResult with anomaly detection
        """
        # Add to history
        self.history.append(sample)

        # Extract feature dict for anomaly detection
        features = {
            "inference_latency": sample.get("inference_latency_ms", 0.0),
            "gpu_utilization": sample.get("gpu_utilization", 0.0),
            "feature_drift": sample.get("feature_drift", 0.0),
            "prediction_confidence": sample.get("prediction_confidence", 0.0),
        }

        # Detect anomalies
        stat_anomaly = self.stat_baseline.get_combined_anomaly_score(
            sample.get("inference_latency_ms", 0.0)
        )
        self.stat_baseline.add_observation(sample.get("inference_latency_ms", 0.0))

        ml_anomaly = self.ml_baseline.get_ml_anomaly_score(features)
        self.ml_baseline.add_observation(features)

        # Combine scores
        anomaly_score = (
            stat_anomaly.anomaly_score * 0.4 +
            ml_anomaly.anomaly_score * 0.6
        )
        is_anomaly = stat_anomaly.is_anomaly or ml_anomaly.is_anomaly

        # Detect drift
        is_drifting = False
        drift_explanation = None
        top_features = []

        if self.enable_drift_detection:
            self.drift_detector.add_observation(
                sample.get("inference_latency_ms", 0.0)
            )
            drift = self.drift_detector.detect_drift()
            is_drifting = drift.is_drifting

            # Generate explanation
            if is_drifting:
                reference_features = {
                    k: v for k, v in features.items()
                    if "baseline" in str(v).lower()
                }
                if not reference_features:
                    reference_features = {k: 0.0 for k in features.keys()}

                explanation = DriftExplainer.explain_drift(
                    drift,
                    features,
                    reference_features,
                )
                drift_explanation = explanation.explanation
                top_features = explanation.top_contributing_features

        # Calculate adaptive thresholds
        adaptive_latency_threshold = self._calculate_adaptive_threshold(
            "inference_latency_ms",
            baseline_value=200.0,  # Default
        )
        adaptive_drift_threshold = self._calculate_adaptive_threshold(
            "feature_drift",
            baseline_value=0.2,  # Default
        )

        # Adjust pass status based on anomaly
        passed = baseline_result.get("pass", True)
        confidence = 1.0
        if is_anomaly:
            passed = False
            confidence = 0.7
        if is_drifting:
            confidence *= 0.8

        return AdaptiveMlEvaluationResult(
            timestamp=baseline_result.get("timestamp", 0),
            inference_latency_score=baseline_result.get("inference_latency_score", 0.0),
            gpu_utilization_score=baseline_result.get("gpu_utilization_score", 0.0),
            system_score=baseline_result.get("system_score", 0.0),
            latency_score=baseline_result.get("latency_score", 0.0),
            feature_drift_score=baseline_result.get("feature_drift_score", 0.0),
            prediction_confidence_score=baseline_result.get("prediction_confidence_score", 0.0),
            drift_score=baseline_result.get("drift_score", 0.0),
            latency_weight=baseline_result.get("latency_weight", 0.0),
            drift_weight=baseline_result.get("drift_weight", 0.0),
            hybrid_score=baseline_result.get("hybrid_score", 0.0),
            passed=passed,
            anomaly_detected=is_anomaly,
            anomaly_score=anomaly_score,
            anomaly_reason=stat_anomaly.reason,
            is_drifting=is_drifting,
            drift_explanation=drift_explanation,
            top_contributing_features=top_features,
            adaptive_latency_threshold=adaptive_latency_threshold,
            adaptive_drift_threshold=adaptive_drift_threshold,
            confidence_score=confidence,
        )

    def _calculate_adaptive_threshold(
        self,
        metric_name: str,
        baseline_value: float,
    ) -> float:
        """Calculate adaptive threshold based on history.

        Args:
            metric_name: Metric name to calculate threshold for
            baseline_value: Default/baseline threshold

        Returns:
            Adaptive threshold
        """
        if len(self.history) < 10:
            return baseline_value

        try:
            import numpy as np

            values = [
                s.get(metric_name, baseline_value)
                for s in self.history[-100:]
            ]
            values = [v for v in values if v is not None]

            if not values:
                return baseline_value

            p95 = float(np.percentile(values, 95))
            p99 = float(np.percentile(values, 99))

            # Adaptive threshold is 1.1 * p99 (10% buffer above 99th percentile)
            adaptive_threshold = p99 * 1.1
            return adaptive_threshold
        except ImportError:
            return baseline_value


# ============================================================================
# Adaptive GenAI SLO Evaluator
# ============================================================================


class AdaptiveGenAiSloEvaluator:
    """GenAI SLO evaluator with dynamic baselining and drift detection."""

    def __init__(
        self,
        baseline_window: int = 100,
        contamination: float = 0.05,
        enable_drift_detection: bool = True,
    ):
        """Initialize adaptive GenAI SLO evaluator.

        Args:
            baseline_window: Window size for baseline
            contamination: Expected anomaly rate
            enable_drift_detection: Enable drift detection
        """
        self.stat_baseline = StatisticalBaseline(window_size=baseline_window)
        self.ml_baseline = MLBaseline(
            contamination=contamination,
            window_size=baseline_window,
        )
        self.drift_detector = DriftDetector(
            reference_window=baseline_window,
            test_window=baseline_window // 2,
        )
        self.enable_drift_detection = enable_drift_detection
        self.history: List[Dict[str, Any]] = []

    def evaluate_with_anomaly_detection(
        self,
        sample: Dict[str, Any],
        baseline_result: Dict[str, Any],
    ) -> AdaptiveGenAiEvaluationResult:
        """Evaluate GenAI sample with anomaly detection.

        Args:
            sample: GenAI sample with metrics
            baseline_result: Standard SLO evaluation result

        Returns:
            AdaptiveGenAiEvaluationResult with anomaly detection
        """
        # Add to history
        self.history.append(sample)

        # Extract feature dict for anomaly detection
        features = {
            "tokens_per_second": sample.get("tokens_per_second", 0.0),
            "time_to_first_token_ms": sample.get("time_to_first_token_ms", 0.0),
            "semantic_similarity": sample.get("semantic_similarity", 0.0),
        }

        # Detect anomalies
        stat_anomaly = self.stat_baseline.get_combined_anomaly_score(
            sample.get("tokens_per_second", 0.0)
        )
        self.stat_baseline.add_observation(sample.get("tokens_per_second", 0.0))

        ml_anomaly = self.ml_baseline.get_ml_anomaly_score(features)
        self.ml_baseline.add_observation(features)

        # Combine scores
        anomaly_score = (
            stat_anomaly.anomaly_score * 0.4 +
            ml_anomaly.anomaly_score * 0.6
        )
        is_anomaly = stat_anomaly.is_anomaly or ml_anomaly.is_anomaly

        # Detect drift
        is_drifting = False
        drift_explanation = None
        top_features = []

        if self.enable_drift_detection:
            self.drift_detector.add_observation(
                sample.get("tokens_per_second", 0.0)
            )
            drift = self.drift_detector.detect_drift()
            is_drifting = drift.is_drifting

            # Generate explanation
            if is_drifting:
                reference_features = {k: 0.0 for k in features.keys()}

                explanation = DriftExplainer.explain_drift(
                    drift,
                    features,
                    reference_features,
                )
                drift_explanation = explanation.explanation
                top_features = explanation.top_contributing_features

        # Calculate adaptive thresholds
        adaptive_tps_threshold = self._calculate_adaptive_threshold(
            "tokens_per_second",
            baseline_value=20.0,  # Default
        )
        adaptive_ttft_threshold = self._calculate_adaptive_threshold(
            "time_to_first_token_ms",
            baseline_value=1200.0,  # Default
        )

        # Adjust pass status based on anomaly
        passed = baseline_result.get("pass", True)
        confidence = 1.0
        if is_anomaly:
            passed = False
            confidence = 0.7
        if is_drifting:
            confidence *= 0.8

        return AdaptiveGenAiEvaluationResult(
            timestamp=baseline_result.get("timestamp", 0),
            tokens_per_second=baseline_result.get("tokens_per_second", 0.0),
            time_to_first_token_ms=baseline_result.get("time_to_first_token_ms", 0.0),
            semantic_similarity=baseline_result.get("semantic_similarity", 0.0),
            tokens_per_second_ok=baseline_result.get("tokens_per_second_ok", True),
            time_to_first_token_ok=baseline_result.get("time_to_first_token_ok", True),
            semantic_similarity_ok=baseline_result.get("semantic_similarity_ok", True),
            passed=passed,
            anomaly_detected=is_anomaly,
            anomaly_score=anomaly_score,
            anomaly_reason=stat_anomaly.reason,
            is_drifting=is_drifting,
            drift_explanation=drift_explanation,
            top_contributing_features=top_features,
            adaptive_tps_threshold=adaptive_tps_threshold,
            adaptive_ttft_threshold=adaptive_ttft_threshold,
            confidence_score=confidence,
        )

    def _calculate_adaptive_threshold(
        self,
        metric_name: str,
        baseline_value: float,
    ) -> float:
        """Calculate adaptive threshold based on history.

        Args:
            metric_name: Metric name to calculate threshold for
            baseline_value: Default/baseline threshold

        Returns:
            Adaptive threshold
        """
        if len(self.history) < 10:
            return baseline_value

        try:
            import numpy as np

            values = [
                s.get(metric_name, baseline_value)
                for s in self.history[-100:]
            ]
            values = [v for v in values if v is not None]

            if not values:
                return baseline_value

            p95 = float(np.percentile(values, 95))
            p99 = float(np.percentile(values, 99))

            # Adaptive threshold
            if "latency" in metric_name.lower():
                # For latency: 1.1 * p99 (10% buffer above 99th percentile)
                adaptive_threshold = p99 * 1.1
            elif "similarity" in metric_name.lower():
                # For similarity: 0.9 * p5 (10% buffer below 5th percentile)
                p5 = float(np.percentile(values, 5))
                adaptive_threshold = p5 * 0.9
            else:
                # Default: 1.1 * p99
                adaptive_threshold = p99 * 1.1

            return adaptive_threshold
        except ImportError:
            return baseline_value
