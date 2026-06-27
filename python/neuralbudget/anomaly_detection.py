"""Anomaly/drift detection with dynamic baselining and feature importance.

This module provides statistical and ML-based dynamic baselining for detecting
anomalies and concept drift in ML/GenAI SLO evaluation data. It goes beyond
static thresholds to adapt baselines based on historical data patterns.

Features:
- Statistical baselining: Z-score, modified Z-score, percentile-based
- ML-based baselining: Isolation Forest, One-Class SVM, Local Outlier Factor
- Drift detection: Kolmogorov-Smirnov test, drift indices
- Feature importance: SHAP, permutation importance, correlation analysis
- Anomaly scoring: Combined statistical and ML scores
"""

import logging
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)


# ============================================================================
# Data Models
# ============================================================================


@dataclass(frozen=True)
class AnomalyScore:
    """Anomaly detection score and details."""
    timestamp: str
    raw_value: float
    is_anomaly: bool
    anomaly_score: float  # 0-1, higher = more anomalous
    statistical_score: float  # Z-score or percentile deviation
    ml_score: float  # ML-based anomaly score
    confidence: float  # 0-1, higher = more confident
    reason: str  # Human-readable explanation


@dataclass(frozen=True)
class DriftDetection:
    """Drift detection result."""
    timestamp: str
    is_drifting: bool
    drift_score: float  # 0-1, higher = more drift
    ks_statistic: float  # Kolmogorov-Smirnov statistic
    ks_p_value: float  # Statistical significance
    drift_indices: Dict[str, float]  # Per-feature drift indicators
    reference_mean: float  # Reference distribution mean
    current_mean: float  # Current window mean
    mean_shift: float  # Mean shift magnitude


@dataclass(frozen=True)
class FeatureImportance:
    """Feature importance for drift/anomaly."""
    feature_name: str
    importance_score: float  # 0-1, higher = more important
    baseline_value: float  # Expected value
    current_value: float  # Observed value
    deviation: float  # Current - baseline
    contribution_percent: float  # % contribution to anomaly


@dataclass(frozen=True)
class DriftExplanation:
    """Complete drift explanation with feature importance."""
    timestamp: str
    is_drifting: bool
    drift_score: float
    explanation: str
    top_contributing_features: List[FeatureImportance]  # Top 3-5 features
    affected_metrics: List[str]  # Which metrics are affected
    severity: str  # "low", "medium", "high", "critical"
    recommended_action: str  # Recommended response


# ============================================================================
# Statistical Baselining
# ============================================================================


class StatisticalBaseline:
    """Statistical methods for baseline estimation and anomaly detection."""

    def __init__(self, window_size: int = 100, percentile: float = 99.0):
        """Initialize statistical baseline.

        Args:
            window_size: Number of recent observations for baseline
            percentile: Percentile for anomaly threshold (default: 99th)
        """
        self.window_size = window_size
        self.percentile = percentile
        self.history: List[float] = []

    def add_observation(self, value: float) -> None:
        """Add observation to history."""
        self.history.append(value)
        if len(self.history) > self.window_size * 2:
            self.history = self.history[-self.window_size:]

    def get_zscore_anomaly(self, value: float) -> Tuple[bool, float, str]:
        """Detect anomaly using Z-score method.

        Returns:
            (is_anomaly, z_score, explanation)
        """
        if len(self.history) < 10:
            return False, 0.0, "Insufficient history"

        try:
            import numpy as np
            data = np.array(self.history[-self.window_size:])
            mean = float(np.mean(data))
            std = float(np.std(data))

            if std == 0:
                return False, 0.0, "Zero variance"

            z_score = (value - mean) / std
            is_anomaly = abs(z_score) > 3.0  # 3-sigma rule

            return is_anomaly, abs(z_score), f"Z-score: {z_score:.2f}"
        except ImportError:
            logger.warning("NumPy not available for Z-score calculation")
            return False, 0.0, "NumPy unavailable"

    def get_modified_zscore_anomaly(self, value: float) -> Tuple[bool, float, str]:
        """Detect anomaly using modified Z-score (MAD-based).

        More robust to outliers than standard Z-score.

        Returns:
            (is_anomaly, modified_z_score, explanation)
        """
        if len(self.history) < 10:
            return False, 0.0, "Insufficient history"

        try:
            import numpy as np
            data = np.array(self.history[-self.window_size:])
            median = float(np.median(data))
            mad = float(np.median(np.abs(data - median)))

            if mad == 0:
                return False, 0.0, "Zero MAD"

            modified_z = 0.6745 * (value - median) / mad
            is_anomaly = abs(modified_z) > 3.5

            return is_anomaly, abs(modified_z), f"Modified Z: {modified_z:.2f}"
        except ImportError:
            logger.warning("NumPy not available for modified Z-score")
            return False, 0.0, "NumPy unavailable"

    def get_percentile_anomaly(self, value: float) -> Tuple[bool, float, str]:
        """Detect anomaly using percentile-based method.

        Returns:
            (is_anomaly, percentile_deviation, explanation)
        """
        if len(self.history) < 10:
            return False, 0.0, "Insufficient history"

        try:
            import numpy as np
            data = np.array(self.history[-self.window_size:])
            lower_bound = float(np.percentile(data, 1))
            upper_bound = float(np.percentile(data, self.percentile))
            iqr = upper_bound - lower_bound

            if iqr == 0:
                return False, 0.0, "Zero IQR"

            if value > upper_bound:
                deviation = (value - upper_bound) / iqr
                is_anomaly = deviation > 1.5  # 1.5 * IQR
                return is_anomaly, deviation, f"Above {self.percentile}th: {deviation:.2f}*IQR"
            elif value < lower_bound:
                deviation = (lower_bound - value) / iqr
                is_anomaly = deviation > 1.5
                return is_anomaly, deviation, f"Below 1st: {deviation:.2f}*IQR"
            else:
                return False, 0.0, "Within bounds"
        except ImportError:
            logger.warning("NumPy not available for percentile analysis")
            return False, 0.0, "NumPy unavailable"

    def get_combined_anomaly_score(self, value: float) -> AnomalyScore:
        """Get combined anomaly score using multiple statistical methods.

        Returns:
            AnomalyScore with combined scoring
        """
        z_anomaly, z_score, z_reason = self.get_zscore_anomaly(value)
        mod_z_anomaly, mod_z_score, mod_z_reason = self.get_modified_zscore_anomaly(value)
        perc_anomaly, perc_score, perc_reason = self.get_percentile_anomaly(value)

        # Combine scores (normalized to 0-1)
        def normalize_score(score: float) -> float:
            return min(1.0, score / 5.0)  # 5-sigma = 1.0

        combined_score = (
            normalize_score(z_score) * 0.3 +
            normalize_score(mod_z_score) * 0.4 +
            normalize_score(perc_score) * 0.3
        )

        is_anomaly = combined_score > 0.5 or z_anomaly or mod_z_anomaly or perc_anomaly

        reasons = [r for r in [z_reason, mod_z_reason, perc_reason] if "anomaly" in r.lower()]
        reason = "; ".join(reasons) if reasons else "Normal"

        return AnomalyScore(
            timestamp=datetime.utcnow().isoformat(),
            raw_value=value,
            is_anomaly=is_anomaly,
            anomaly_score=combined_score,
            statistical_score=z_score,
            ml_score=0.0,  # Set by ML method
            confidence=0.7,
            reason=reason,
        )


# ============================================================================
# ML-Based Baselining
# ============================================================================


class MLBaseline:
    """ML-based anomaly detection using ensemble methods."""

    def __init__(self, contamination: float = 0.05, window_size: int = 200):
        """Initialize ML baseline.

        Args:
            contamination: Expected proportion of anomalies (default: 5%)
            window_size: Number of observations for training
        """
        self.contamination = contamination
        self.window_size = window_size
        self.history: List[List[float]] = []  # Multivariate history
        self.models = {}
        self._trained = False

    def add_observation(self, features: Dict[str, float]) -> None:
        """Add observation with multiple features.

        Args:
            features: Dict mapping feature names to values
        """
        self.history.append(list(features.values()))
        if len(self.history) > self.window_size * 2:
            self.history = self.history[-self.window_size:]
        self._trained = False  # Invalidate training

    def _train_models(self) -> bool:
        """Train anomaly detection models on history."""
        if len(self.history) < 20:
            logger.warning("Insufficient data for ML training")
            return False

        try:
            import numpy as np
            from sklearn.ensemble import IsolationForest
            from sklearn.svm import OneClassSVM
            from sklearn.neighbors import LocalOutlierFactor

            X = np.array(self.history[-self.window_size:])

            # Isolation Forest
            self.models["isolation_forest"] = IsolationForest(
                contamination=self.contamination,
                random_state=42
            ).fit(X)

            # One-Class SVM
            self.models["one_class_svm"] = OneClassSVM(
                nu=self.contamination,
                gamma="auto"
            ).fit(X)

            # Local Outlier Factor
            self.models["lof"] = LocalOutlierFactor(
                n_neighbors=min(20, len(X) - 1),
                contamination=self.contamination
            ).fit(X)

            self._trained = True
            return True
        except ImportError:
            logger.warning("scikit-learn not available for ML-based detection")
            return False

    def get_ml_anomaly_score(self, features: Dict[str, float]) -> AnomalyScore:
        """Get anomaly score using ML methods.

        Returns:
            AnomalyScore with ML-based scoring
        """
        if not self._trained:
            self._train_models()

        if not self._trained:
            return AnomalyScore(
                timestamp=datetime.utcnow().isoformat(),
                raw_value=sum(features.values()),
                is_anomaly=False,
                anomaly_score=0.0,
                statistical_score=0.0,
                ml_score=0.0,
                confidence=0.0,
                reason="ML models not trained",
            )

        try:
            import numpy as np
            X = np.array([list(features.values())])

            scores = []
            predictions = []

            # Isolation Forest
            iso_pred = self.models["isolation_forest"].predict(X)[0]
            iso_score = -self.models["isolation_forest"].score_samples(X)[0]
            predictions.append(iso_pred)
            scores.append(min(1.0, iso_score / 2))  # Normalize

            # One-Class SVM
            svm_pred = self.models["one_class_svm"].predict(X)[0]
            svm_score = -self.models["one_class_svm"].decision_function(X)[0]
            predictions.append(svm_pred)
            scores.append(min(1.0, svm_score / 5))  # Normalize

            # LOF
            lof_score = self.models["lof"].negative_outlier_factor_[0]
            lof_pred = self.models["lof"].predict(X)[0]
            predictions.append(lof_pred)
            scores.append(min(1.0, -lof_score))  # Normalize

            # Combine
            ml_score = sum(scores) / len(scores)
            anomaly_votes = sum(1 for p in predictions if p == -1)
            is_anomaly = anomaly_votes >= 2

            reasons = []
            if anomaly_votes >= 2:
                reasons.append(f"Flagged by {anomaly_votes}/3 models")

            return AnomalyScore(
                timestamp=datetime.utcnow().isoformat(),
                raw_value=sum(features.values()),
                is_anomaly=is_anomaly,
                anomaly_score=ml_score,
                statistical_score=0.0,
                ml_score=ml_score,
                confidence=0.8,
                reason="; ".join(reasons) if reasons else "Normal",
            )
        except Exception as e:
            logger.error(f"Error in ML anomaly scoring: {e}")
            return AnomalyScore(
                timestamp=datetime.utcnow().isoformat(),
                raw_value=sum(features.values()),
                is_anomaly=False,
                anomaly_score=0.0,
                statistical_score=0.0,
                ml_score=0.0,
                confidence=0.0,
                reason=f"Error: {str(e)[:50]}",
            )


# ============================================================================
# Drift Detection
# ============================================================================


class DriftDetector:
    """Detect concept drift using statistical tests."""

    def __init__(self, reference_window: int = 100, test_window: int = 50):
        """Initialize drift detector.

        Args:
            reference_window: Size of reference distribution
            test_window: Size of test window
        """
        self.reference_window = reference_window
        self.test_window = test_window
        self.reference_data: List[float] = []
        self.test_data: List[float] = []

    def add_observation(self, value: float) -> None:
        """Add observation to test window."""
        self.test_data.append(value)
        if len(self.test_data) > self.test_window:
            # Move test data to reference and reset
            self.reference_data.extend(self.test_data[:-self.test_window])
            self.reference_data = self.reference_data[-self.reference_window:]
            self.test_data = []

    def detect_drift(self) -> DriftDetection:
        """Detect drift using Kolmogorov-Smirnov test.

        Returns:
            DriftDetection with statistical results
        """
        if len(self.reference_data) < 10 or len(self.test_data) < 10:
            return DriftDetection(
                timestamp=datetime.utcnow().isoformat(),
                is_drifting=False,
                drift_score=0.0,
                ks_statistic=0.0,
                ks_p_value=1.0,
                drift_indices={},
                reference_mean=0.0,
                current_mean=0.0,
                mean_shift=0.0,
            )

        try:
            import numpy as np
            from scipy.stats import ks_2samp

            ref = np.array(self.reference_data)
            test = np.array(self.test_data)

            # KS test
            ks_stat, ks_pval = ks_2samp(ref, test)

            # Mean shift
            ref_mean = float(np.mean(ref))
            test_mean = float(np.mean(test))
            mean_shift = abs(test_mean - ref_mean)

            # Drift score (combined)
            drift_score = (1 - ks_pval) * 0.5 + (mean_shift / (float(np.std(ref)) + 1e-6)) * 0.5
            drift_score = min(1.0, drift_score)

            is_drifting = drift_score > 0.3 and ks_pval < 0.05

            return DriftDetection(
                timestamp=datetime.utcnow().isoformat(),
                is_drifting=is_drifting,
                drift_score=drift_score,
                ks_statistic=float(ks_stat),
                ks_p_value=float(ks_pval),
                drift_indices={
                    "ks_statistic": float(ks_stat),
                    "mean_shift": float(mean_shift),
                    "std_ratio": float(np.std(test) / (np.std(ref) + 1e-6)),
                },
                reference_mean=ref_mean,
                current_mean=test_mean,
                mean_shift=mean_shift,
            )
        except ImportError:
            logger.warning("SciPy not available for drift detection")
            return DriftDetection(
                timestamp=datetime.utcnow().isoformat(),
                is_drifting=False,
                drift_score=0.0,
                ks_statistic=0.0,
                ks_p_value=1.0,
                drift_indices={},
                reference_mean=0.0,
                current_mean=0.0,
                mean_shift=0.0,
            )


# ============================================================================
# Feature Importance & Drift Explanation
# ============================================================================


class FeatureImportanceCalculator:
    """Calculate feature importance for drift/anomalies."""

    @staticmethod
    def get_baseline_feature_importance(
        current_features: Dict[str, float],
        reference_features: Dict[str, float],
    ) -> List[FeatureImportance]:
        """Calculate feature importance based on baseline comparison.

        Args:
            current_features: Current observation features
            reference_features: Reference/baseline features

        Returns:
            List of FeatureImportance sorted by importance
        """
        importances = []

        try:
            import numpy as np

            # Calculate deviations
            total_deviation = 0.0
            deviations = {}

            for feature_name, current_value in current_features.items():
                baseline_value = reference_features.get(feature_name, current_value)
                if baseline_value == 0:
                    baseline_value = 1.0  # Avoid division by zero

                deviation = abs((current_value - baseline_value) / baseline_value)
                deviations[feature_name] = deviation
                total_deviation += deviation

            # Calculate importance scores
            for feature_name, deviation in deviations.items():
                contribution = deviation / (total_deviation + 1e-6)

                importances.append(FeatureImportance(
                    feature_name=feature_name,
                    importance_score=min(1.0, deviation),
                    baseline_value=reference_features.get(feature_name, 0.0),
                    current_value=current_features[feature_name],
                    deviation=current_features[feature_name] - reference_features.get(feature_name, 0.0),
                    contribution_percent=contribution * 100,
                ))
        except Exception as e:
            logger.error(f"Error calculating feature importance: {e}")

        # Sort by importance
        return sorted(importances, key=lambda x: x.importance_score, reverse=True)

    @staticmethod
    def get_shap_feature_importance(
        features: Dict[str, float],
        model: Any = None,
        background_data: Optional[List[Dict[str, float]]] = None,
    ) -> List[FeatureImportance]:
        """Calculate SHAP-based feature importance.

        Args:
            features: Feature values to explain
            model: ML model to explain
            background_data: Reference/background data for SHAP

        Returns:
            List of FeatureImportance based on SHAP values
        """
        try:
            import shap
            import numpy as np

            if model is None or background_data is None:
                logger.warning("SHAP calculation requires model and background data")
                return []

            # Create explainer
            bg_array = np.array([list(f.values()) for f in background_data])
            explainer = shap.KernelExplainer(model.predict, bg_array)

            # Explain instance
            X_explain = np.array([list(features.values())])
            shap_values = explainer.shap_values(X_explain)

            importances = []
            feature_names = list(features.keys())

            for i, feature_name in enumerate(feature_names):
                importance_score = abs(float(shap_values[0][i]))

                importances.append(FeatureImportance(
                    feature_name=feature_name,
                    importance_score=min(1.0, importance_score),
                    baseline_value=0.0,
                    current_value=features[feature_name],
                    deviation=shap_values[0][i],
                    contribution_percent=0.0,
                ))

            return sorted(importances, key=lambda x: x.importance_score, reverse=True)
        except ImportError:
            logger.warning("SHAP library not available")
            return []


class DriftExplainer:
    """Generate human-readable drift explanations."""

    @staticmethod
    def explain_drift(
        drift_detection: DriftDetection,
        current_features: Dict[str, float],
        reference_features: Dict[str, float],
    ) -> DriftExplanation:
        """Generate comprehensive drift explanation.

        Args:
            drift_detection: DriftDetection result
            current_features: Current feature values
            reference_features: Reference feature values

        Returns:
            DriftExplanation with human-readable explanation
        """
        # Get feature importance
        top_features = FeatureImportanceCalculator.get_baseline_feature_importance(
            current_features,
            reference_features,
        )[:5]

        # Determine severity
        if drift_detection.drift_score > 0.8:
            severity = "critical"
        elif drift_detection.drift_score > 0.6:
            severity = "high"
        elif drift_detection.drift_score > 0.4:
            severity = "medium"
        else:
            severity = "low"

        # Build explanation
        explanation_parts = []
        if drift_detection.is_drifting:
            explanation_parts.append(
                f"Drift detected (score: {drift_detection.drift_score:.2f}). "
            )
        explanation_parts.append(
            f"Mean shifted from {drift_detection.reference_mean:.2f} to {drift_detection.current_mean:.2f} "
            f"(delta: {drift_detection.mean_shift:.2f}). "
        )
        explanation_parts.append(
            f"KS test: statistic={drift_detection.ks_statistic:.3f}, p-value={drift_detection.ks_p_value:.4f}"
        )

        explanation = "".join(explanation_parts)

        # Top contributing features
        affected_metrics = [f.feature_name for f in top_features[:3] if f.importance_score > 0.1]

        # Recommend action
        if severity == "critical":
            recommended_action = "Investigate immediately. May indicate model degradation or data pipeline issue."
        elif severity == "high":
            recommended_action = "Review feature distributions. Consider model retraining or data validation."
        elif severity == "medium":
            recommended_action = "Monitor closely. Update baselines if change is expected."
        else:
            recommended_action = "Continue monitoring. No immediate action required."

        return DriftExplanation(
            timestamp=drift_detection.timestamp,
            is_drifting=drift_detection.is_drifting,
            drift_score=drift_detection.drift_score,
            explanation=explanation,
            top_contributing_features=top_features,
            affected_metrics=affected_metrics,
            severity=severity,
            recommended_action=recommended_action,
        )
