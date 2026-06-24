"""Python package shim for the Rust-backed neuralbudget extension.

This package re-exports symbols from the native extension module and exposes
small convenience helpers for dictionary-oriented workflows.
"""

from . import neuralbudget as _native
from .convenience import (
    availability_snapshot,
    burn_rate_from_values,
    evaluate_http_histogram_once,
    evaluate_ml_once,
    evaluate_stateful_once,
    metric_stream,
)

for _name in dir(_native):
    if _name.startswith("_"):
        continue
    globals()[_name] = getattr(_native, _name)

__all__ = [
    name for name in globals() if not name.startswith("_")
]
