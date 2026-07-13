"""High-level Python convenience wrapper for Rust drift explanation.

This module provides `DriftExplainer.explain` which accepts baseline and current
feature dictionaries and returns a human-readable explanation including SHAP
approximations.
"""
from typing import Dict, List
from neuralbudget import FeatureDriftAnalyzer, ShapExplainer


class DriftExplainer:
    @staticmethod
    def explain(baseline_data: Dict[str, List[float]], current_data: Dict[str, List[float]]):
        analyzer = FeatureDriftAnalyzer()
        for feat, values in baseline_data.items():
            for v in values:
                analyzer.add_baseline(feat, float(v))
        for feat, values in current_data.items():
            for v in values:
                analyzer.add_current(feat, float(v))

        result = analyzer.analyze()
        # attach SHAP values if available
        shap = None
        if isinstance(result, dict) and "shap_values" in result:
            shap = result.get("shap_values")

        return {
            "top_features": result.get("top_features"),
            "ks_statistic": result.get("ks_statistic"),
            "p_value": result.get("p_value"),
            "summary": result.get("summary"),
            "shap": shap,
        }
