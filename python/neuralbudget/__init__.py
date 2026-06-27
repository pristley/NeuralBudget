"""Python package shim for the Rust-backed neuralbudget extension.

This package re-exports symbols from the native extension module and exposes
small convenience helpers for dictionary-oriented workflows.
"""

from . import neuralbudget as _native
from .client import (
    ClientConfigFile,
    CompositeDependencyInput,
    CompositeMetricData,
    CompositeServiceInput,
    EvaluationMode,
    MetricData,
    NeuralBudgetClient,
    NeuralBudgetClientConfig,
)
from .alerting import AlertDispatchResult, AlertDispatchSummary, AlertDispatcher
from .alert_dispatch_advanced import (
    AlertDispatchManager,
    AlertDeduplicationEntry,
    CircuitBreakerState,
    DeduplicationPolicy,
    EscalationAction,
    EscalationPolicy,
    EscalationStep,
    RetryPolicy,
)
from .convenience import (
    AvailabilitySnapshotResult,
    GENAI_PROFILE_PRESETS,
    HTTP_PROFILE_PRESETS,
    ML_PROFILE_PRESETS,
    STATEFUL_PROFILE_PRESETS,
    GenAiEvaluationResult,
    GenAiSloProfile,
    HttpHistogramEvaluationResult,
    HttpSloProfile,
    MlEvaluationResult,
    MlSloProfile,
    StatefulEvaluationResult,
    StatefulSloProfile,
    availability_snapshot,
    burn_rate_from_values,
    evaluate_genai_once,
    evaluate_http_histogram_once,
    evaluate_ml_once,
    evaluate_stateful_once,
    get_genai_profile_preset,
    get_http_profile_preset,
    get_ml_profile_preset,
    get_stateful_profile_preset,
    metric_stream,
)

# Optional dashboard and CLI TUI (requires fastapi/textual)
try:
    from .dashboard import Dashboard, SloSnapshot, AlertEvent
except ImportError:
    pass  # FastAPI not installed

try:
    from .cli_tui import CliTui
except ImportError:
    pass  # Textual/Rich not installed

for _name in dir(_native):
    if _name.startswith("_"):
        continue
    globals()[_name] = getattr(_native, _name)

__all__ = [
    name for name in globals() if not name.startswith("_")
]
