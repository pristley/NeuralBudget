"""
Convenience helpers for hallucination/groundedness analysis.
This adapter exposes a tiny, stable surface that converts Rust results to plain
Python dicts for easier consumption in examples and notebooks.
"""
from typing import Any, Dict, List
import neuralbudget


def analyze_hallucination(response: str, documents: List[str]) -> Dict[str, Any]:
    """Run the higher-level hallucination analysis and return a dict.

    This is a convenience wrapper that uses the library's default detector.
    """
    # There is a Rust-side `HallucinationDetector` that returns a struct of
    # metrics; call that and convert to a dict.
    detector = neuralbudget.HallucinationDetector()
    metrics = detector.analyze(response, documents)

    return {
        "factual_consistency": getattr(metrics, "factual_consistency", None),
        "hallucination_rate": getattr(metrics, "hallucination_rate", None),
        "grounded_count": getattr(metrics, "grounded_count", None),
        "hallucinated_count": getattr(metrics, "hallucinated_count", None),
        "summary": getattr(metrics, "summary", None),
    }
