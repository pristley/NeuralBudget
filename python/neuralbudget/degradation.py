"""
Convenience wrapper for the Degradation detector exposed by the Rust extension.
This module provides a small, Pythonic adapter around the PyO3 class so callers
get plain dicts and simple methods.
"""
from typing import Optional, Dict, Any

import neuralbudget


class DegradationDetector:
    """Python convenience wrapper around the Rust `DegradationDetector`.

    Parameters
    - window_size: rolling window size (int)
    - drift_threshold: threshold used for gradual drift detection (float)
    - spike_threshold: z-score multiplier for spike detection (float)
    """

    def __init__(self, window_size: int = 100, drift_threshold: float = 0.15, spike_threshold: float = 2.5):
        self._inner = neuralbudget.DegradationDetector(window_size, drift_threshold, spike_threshold)

    def add_metric(self, value: float) -> None:
        """Add a new metric sample to the detector."""
        return self._inner.add_metric(value)

    def detect_change(self) -> Optional[Dict[str, Any]]:
        """Run detection and return either None or a dict describing the event.

        The returned dict uses plain Python primitives for easy consumption.
        """
        ev = self._inner.detect_change()
        if ev is None:
            return None
        # Convert the Rust/PyO3 object to a simple dict (best-effort attribute access)
        return {
            "timestamp": getattr(ev, "timestamp", None),
            "degradation_type": str(getattr(ev, "degradation_type", getattr(ev, "kind", None))),
            "magnitude": getattr(ev, "magnitude", None),
            "confidence": getattr(ev, "confidence", None),
            "duration_samples": getattr(ev, "duration_samples", None),
            "affected_metric": getattr(ev, "affected_metric", None),
        }

    def reset(self) -> None:
        """Reset internal detector state and history."""
        return self._inner.reset()

    def history_len(self) -> int:
        """Return the length of the internal history buffer."""
        return int(self._inner.history_len())
