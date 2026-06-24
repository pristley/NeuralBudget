"""Ergonomic helpers around the Rust-backed neuralbudget API.

This module intentionally keeps logic thin and delegates calculations to the
native extension to preserve deterministic behavior across Rust and Python call
sites.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Any, TypeVar

from . import neuralbudget as _native


@dataclass(frozen=True)
class AvailabilitySnapshotResult:
    success: int
    total: int
    availability: float
    slo_target: float
    window_secs: int
    error_budget_seconds: float
    target_met: bool


@dataclass(frozen=True)
class HttpHistogramEvaluationResult:
    timestamp: int
    availability: float
    percentile_latency_ms: float
    evaluated_percentile: float
    latency_pass: bool
    availability_pass: bool
    passed: bool


@dataclass(frozen=True)
class StatefulEvaluationResult:
    timestamp: int
    score: float
    replication_lag_ok: bool
    queue_depth_ok: bool
    connection_pool_ok: bool
    connection_wait_penalized: bool
    passed: bool


@dataclass(frozen=True)
class MlEvaluationResult:
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


@dataclass(frozen=True)
class HttpSloProfile:
    latency_threshold_ms: float
    latency_percentile: float
    availability_threshold: float


@dataclass(frozen=True)
class StatefulSloProfile:
    replication_lag_threshold_ms: float
    queue_depth_threshold: int
    connection_pool_saturation_threshold: float
    connection_wait_time_threshold_ms: float
    connection_wait_penalty_weight: float
    min_pass_score: float


@dataclass(frozen=True)
class MlSloProfile:
    max_inference_latency_ms: float
    max_gpu_utilization: float
    max_feature_drift: float
    min_prediction_confidence: float
    latency_weight: float
    drift_weight: float
    min_pass_score: float


HTTP_PROFILE_PRESETS: dict[str, HttpSloProfile] = {
    "default": HttpSloProfile(200.0, 0.99, 0.999),
    "strict_latency": HttpSloProfile(150.0, 0.99, 0.999),
    "availability_first": HttpSloProfile(250.0, 0.99, 0.9995),
}

STATEFUL_PROFILE_PRESETS: dict[str, StatefulSloProfile] = {
    "default": StatefulSloProfile(250.0, 1_000, 0.8, 20.0, 0.25, 0.85),
    "database_primary": StatefulSloProfile(200.0, 800, 0.75, 15.0, 0.3, 0.9),
    "queue_hot_path": StatefulSloProfile(300.0, 500, 0.85, 25.0, 0.2, 0.85),
}

ML_PROFILE_PRESETS: dict[str, MlSloProfile] = {
    "default": MlSloProfile(200.0, 0.85, 0.2, 0.8, 0.6, 0.4, 0.9),
    "latency_critical": MlSloProfile(180.0, 0.8, 0.25, 0.75, 0.75, 0.25, 0.9),
    "drift_sensitive": MlSloProfile(220.0, 0.9, 0.15, 0.85, 0.4, 0.6, 0.9),
}


TProfile = TypeVar("TProfile")


def _profile_from_preset(
    preset: str | TProfile | None,
    *,
    profiles: Mapping[str, TProfile],
    profile_name: str,
) -> TProfile:
    if preset is None:
        return profiles["default"]

    if isinstance(preset, str):
        if preset not in profiles:
            valid = ", ".join(sorted(profiles))
            raise ValueError(f"unknown {profile_name} preset '{preset}'. valid presets: {valid}")
        return profiles[preset]

    return preset


def _resolved_float(value: float | None, fallback: float) -> float:
    if value is None:
        return float(fallback)
    return float(value)


def _resolved_int(value: int | None, fallback: int) -> int:
    if value is None:
        return int(fallback)
    return int(value)


def metric_stream(points: Iterable[float]) -> list[Any]:
    """Build a MetricPoint stream from raw values with synthetic timestamps."""
    return [_native.MetricPoint(i + 1, float(value)) for i, value in enumerate(points)]


def burn_rate_from_values(values: Iterable[float], window_secs: int) -> float:
    """Compute burn rate from a plain numeric sequence."""
    return float(_native.calculate_burn_rate(metric_stream(values), int(window_secs)))


def availability_snapshot(
    success: int,
    total: int,
    slo_target: float = 0.999,
    window_secs: int = 3_600,
    return_dataclass: bool = False,
) -> dict[str, Any] | AvailabilitySnapshotResult:
    """Return an availability and error-budget summary.

    Set return_dataclass=True to receive AvailabilitySnapshotResult.
    """
    availability = float(_native.calculate_availability(int(success), int(total)))
    error_budget_seconds = float(_native.calculate_error_budget(float(slo_target), int(window_secs)))

    result = {
        "success": int(success),
        "total": int(total),
        "availability": availability,
        "slo_target": float(slo_target),
        "window_secs": int(window_secs),
        "error_budget_seconds": error_budget_seconds,
        "target_met": availability >= float(slo_target),
    }
    if return_dataclass:
        return AvailabilitySnapshotResult(**result)
    return result


def _build_histogram_sample(sample: Mapping[str, Any]) -> Any:
    buckets = []
    for bucket in sample.get("buckets", []):
        if isinstance(bucket, Mapping):
            upper_bound_ms = bucket["upper_bound_ms"]
            count = bucket["count"]
        else:
            upper_bound_ms, count = bucket
        buckets.append(_native.HistogramBucket(float(upper_bound_ms), int(count)))

    return _native.HistogramSample(
        timestamp=int(sample["timestamp"]),
        success=int(sample["success"]),
        total=int(sample["total"]),
        buckets=buckets,
        format=str(sample.get("format", "prometheus_cumulative")),
    )


def evaluate_http_histogram_once(
    sample: Mapping[str, Any],
    latency_threshold_ms: float | None = None,
    latency_percentile: float | None = None,
    availability_threshold: float | None = None,
    profile: str | HttpSloProfile | None = None,
    return_dataclass: bool = False,
) -> dict[str, Any] | HttpHistogramEvaluationResult:
    """Evaluate one histogram sample.

    Use profile to apply preset thresholds and return_dataclass=True
    to receive HttpHistogramEvaluationResult.
    """
    selected_profile = _profile_from_preset(
        profile,
        profiles=HTTP_PROFILE_PRESETS,
        profile_name="http",
    )

    slo = _native.HttpSlo(
        latency_threshold_ms=_resolved_float(
            latency_threshold_ms,
            selected_profile.latency_threshold_ms,
        ),
        latency_percentile=_resolved_float(
            latency_percentile,
            selected_profile.latency_percentile,
        ),
        availability_threshold=_resolved_float(
            availability_threshold,
            selected_profile.availability_threshold,
        ),
    )
    evaluations = slo.evaluate_stream([_build_histogram_sample(sample)])
    evaluation = evaluations[0]

    result = {
        "timestamp": int(evaluation.timestamp),
        "availability": float(evaluation.availability),
        "percentile_latency_ms": float(evaluation.percentile_latency_ms),
        "evaluated_percentile": float(evaluation.evaluated_percentile),
        "latency_pass": bool(getattr(evaluation, "latency_pass", evaluation.latency_ok)),
        "availability_pass": bool(
            getattr(evaluation, "availability_pass", evaluation.availability_ok)
        ),
        "pass": bool(getattr(evaluation, "pass")),
    }
    if return_dataclass:
        return HttpHistogramEvaluationResult(
            timestamp=result["timestamp"],
            availability=result["availability"],
            percentile_latency_ms=result["percentile_latency_ms"],
            evaluated_percentile=result["evaluated_percentile"],
            latency_pass=result["latency_pass"],
            availability_pass=result["availability_pass"],
            passed=result["pass"],
        )
    return result


def evaluate_stateful_once(
    sample: Mapping[str, Any],
    replication_lag_threshold_ms: float | None = None,
    queue_depth_threshold: int | None = None,
    connection_pool_saturation_threshold: float | None = None,
    connection_wait_time_threshold_ms: float | None = None,
    connection_wait_penalty_weight: float | None = None,
    min_pass_score: float | None = None,
    profile: str | StatefulSloProfile | None = None,
    return_dataclass: bool = False,
) -> dict[str, Any] | StatefulEvaluationResult:
    """Evaluate one stateful sample.

    Use profile to apply preset thresholds and return_dataclass=True
    to receive StatefulEvaluationResult.
    """
    selected_profile = _profile_from_preset(
        profile,
        profiles=STATEFUL_PROFILE_PRESETS,
        profile_name="stateful",
    )

    slo = _native.StatefulSlo(
        replication_lag_threshold_ms=_resolved_float(
            replication_lag_threshold_ms,
            selected_profile.replication_lag_threshold_ms,
        ),
        queue_depth_threshold=_resolved_int(
            queue_depth_threshold,
            selected_profile.queue_depth_threshold,
        ),
        connection_pool_saturation_threshold=_resolved_float(
            connection_pool_saturation_threshold,
            selected_profile.connection_pool_saturation_threshold,
        ),
        connection_wait_time_threshold_ms=_resolved_float(
            connection_wait_time_threshold_ms,
            selected_profile.connection_wait_time_threshold_ms,
        ),
        connection_wait_penalty_weight=_resolved_float(
            connection_wait_penalty_weight,
            selected_profile.connection_wait_penalty_weight,
        ),
        min_pass_score=_resolved_float(min_pass_score, selected_profile.min_pass_score),
    )

    stateful_sample = _native.StatefulSample(
        timestamp=int(sample["timestamp"]),
        replication_lag_ms=float(sample["replication_lag_ms"]),
        queue_depth=int(sample["queue_depth"]),
        connection_pool_saturation=float(sample["connection_pool_saturation"]),
        connection_wait_time_ms=float(sample["connection_wait_time_ms"]),
    )

    evaluation = slo.evaluate_sample(stateful_sample)

    result = {
        "timestamp": int(evaluation.timestamp),
        "score": float(evaluation.score),
        "replication_lag_ok": bool(evaluation.replication_lag_ok),
        "queue_depth_ok": bool(evaluation.queue_depth_ok),
        "connection_pool_ok": bool(evaluation.connection_pool_ok),
        "connection_wait_penalized": bool(evaluation.connection_wait_penalized),
        "pass": bool(getattr(evaluation, "pass")),
    }
    if return_dataclass:
        return StatefulEvaluationResult(
            timestamp=result["timestamp"],
            score=result["score"],
            replication_lag_ok=result["replication_lag_ok"],
            queue_depth_ok=result["queue_depth_ok"],
            connection_pool_ok=result["connection_pool_ok"],
            connection_wait_penalized=result["connection_wait_penalized"],
            passed=result["pass"],
        )
    return result


def evaluate_ml_once(
    sample: Mapping[str, Any],
    max_inference_latency_ms: float | None = None,
    max_gpu_utilization: float | None = None,
    max_feature_drift: float | None = None,
    min_prediction_confidence: float | None = None,
    latency_weight: float | None = None,
    drift_weight: float | None = None,
    min_pass_score: float | None = None,
    profile: str | MlSloProfile | None = None,
    return_dataclass: bool = False,
) -> dict[str, Any] | MlEvaluationResult:
    """Evaluate one ML-serving sample with hybrid latency and drift weighting.

    Use profile to apply preset thresholds/weights and return_dataclass=True
    to receive MlEvaluationResult.
    """
    selected_profile = _profile_from_preset(
        profile,
        profiles=ML_PROFILE_PRESETS,
        profile_name="ml",
    )

    slo = _native.MlSlo(
        max_inference_latency_ms=_resolved_float(
            max_inference_latency_ms,
            selected_profile.max_inference_latency_ms,
        ),
        max_gpu_utilization=_resolved_float(
            max_gpu_utilization,
            selected_profile.max_gpu_utilization,
        ),
        max_feature_drift=_resolved_float(
            max_feature_drift,
            selected_profile.max_feature_drift,
        ),
        min_prediction_confidence=_resolved_float(
            min_prediction_confidence,
            selected_profile.min_prediction_confidence,
        ),
        latency_weight=_resolved_float(latency_weight, selected_profile.latency_weight),
        drift_weight=_resolved_float(drift_weight, selected_profile.drift_weight),
        min_pass_score=_resolved_float(min_pass_score, selected_profile.min_pass_score),
    )

    ml_sample = _native.MlSample(
        timestamp=int(sample["timestamp"]),
        inference_latency_ms=float(sample["inference_latency_ms"]),
        gpu_utilization=float(sample["gpu_utilization"]),
        feature_drift=float(sample["feature_drift"]),
        prediction_confidence=float(sample["prediction_confidence"]),
    )

    evaluation = slo.evaluate_sample(ml_sample)

    result = {
        "timestamp": int(evaluation.timestamp),
        "inference_latency_score": float(evaluation.inference_latency_score),
        "gpu_utilization_score": float(evaluation.gpu_utilization_score),
        "system_score": float(evaluation.system_score),
        "latency_score": float(evaluation.latency_score),
        "feature_drift_score": float(evaluation.feature_drift_score),
        "prediction_confidence_score": float(evaluation.prediction_confidence_score),
        "drift_score": float(evaluation.drift_score),
        "latency_weight": float(evaluation.latency_weight),
        "drift_weight": float(evaluation.drift_weight),
        "hybrid_score": float(evaluation.hybrid_score),
        "pass": bool(getattr(evaluation, "pass")),
    }
    if return_dataclass:
        return MlEvaluationResult(
            timestamp=result["timestamp"],
            inference_latency_score=result["inference_latency_score"],
            gpu_utilization_score=result["gpu_utilization_score"],
            system_score=result["system_score"],
            latency_score=result["latency_score"],
            feature_drift_score=result["feature_drift_score"],
            prediction_confidence_score=result["prediction_confidence_score"],
            drift_score=result["drift_score"],
            latency_weight=result["latency_weight"],
            drift_weight=result["drift_weight"],
            hybrid_score=result["hybrid_score"],
            passed=result["pass"],
        )
    return result


def get_http_profile_preset(name: str) -> HttpSloProfile:
    """Return a named HTTP profile preset."""
    return _profile_from_preset(name, profiles=HTTP_PROFILE_PRESETS, profile_name="http")


def get_stateful_profile_preset(name: str) -> StatefulSloProfile:
    """Return a named stateful profile preset."""
    return _profile_from_preset(name, profiles=STATEFUL_PROFILE_PRESETS, profile_name="stateful")


def get_ml_profile_preset(name: str) -> MlSloProfile:
    """Return a named ML profile preset."""
    return _profile_from_preset(name, profiles=ML_PROFILE_PRESETS, profile_name="ml")
