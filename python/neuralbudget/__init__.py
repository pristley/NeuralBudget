"""Python package shim for the Rust-backed neuralbudget extension.

This package re-exports symbols from the native extension module and exposes
small convenience helpers for dictionary-oriented workflows.
"""

from . import neuralbudget as _native
from .convenience import (
    AvailabilitySnapshotResult,
    HTTP_PROFILE_PRESETS,
    ML_PROFILE_PRESETS,
    STATEFUL_PROFILE_PRESETS,
    HttpHistogramEvaluationResult,
    HttpSloProfile,
    MlEvaluationResult,
    MlSloProfile,
    StatefulEvaluationResult,
    StatefulSloProfile,
    availability_snapshot,
    burn_rate_from_values,
    evaluate_http_histogram_once,
    evaluate_ml_once,
    evaluate_stateful_once,
    get_http_profile_preset,
    get_ml_profile_preset,
    get_stateful_profile_preset,
    metric_stream,
)

for _name in dir(_native):
    if _name.startswith("_"):
        continue
    globals()[_name] = getattr(_native, _name)

__all__ = [
    name for name in globals() if not name.startswith("_")
]
