"""Ergonomic helpers around the Rust-backed neuralbudget API.

This module intentionally keeps logic thin and delegates calculations to the
native extension to preserve deterministic behavior across Rust and Python call
sites.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from typing import Any

from . import neuralbudget as _native


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
) -> dict[str, Any]:
    """Return an availability and error-budget summary dictionary."""
    availability = float(_native.calculate_availability(int(success), int(total)))
    error_budget_seconds = float(_native.calculate_error_budget(float(slo_target), int(window_secs)))

    return {
        "success": int(success),
        "total": int(total),
        "availability": availability,
        "slo_target": float(slo_target),
        "window_secs": int(window_secs),
        "error_budget_seconds": error_budget_seconds,
        "target_met": availability >= float(slo_target),
    }


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
    latency_threshold_ms: float = 200.0,
    latency_percentile: float = 0.99,
    availability_threshold: float = 0.999,
) -> dict[str, Any]:
    """Evaluate one histogram sample and return a plain dictionary."""
    slo = _native.HttpSlo(
        latency_threshold_ms=float(latency_threshold_ms),
        latency_percentile=float(latency_percentile),
        availability_threshold=float(availability_threshold),
    )
    evaluations = slo.evaluate_stream([_build_histogram_sample(sample)])
    evaluation = evaluations[0]

    return {
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


def evaluate_stateful_once(
    sample: Mapping[str, Any],
    replication_lag_threshold_ms: float = 250.0,
    queue_depth_threshold: int = 1_000,
    connection_pool_saturation_threshold: float = 0.8,
    connection_wait_time_threshold_ms: float = 20.0,
    connection_wait_penalty_weight: float = 0.25,
    min_pass_score: float = 0.85,
) -> dict[str, Any]:
    """Evaluate one stateful sample and return a plain dictionary."""
    slo = _native.StatefulSlo(
        replication_lag_threshold_ms=float(replication_lag_threshold_ms),
        queue_depth_threshold=int(queue_depth_threshold),
        connection_pool_saturation_threshold=float(connection_pool_saturation_threshold),
        connection_wait_time_threshold_ms=float(connection_wait_time_threshold_ms),
        connection_wait_penalty_weight=float(connection_wait_penalty_weight),
        min_pass_score=float(min_pass_score),
    )

    stateful_sample = _native.StatefulSample(
        timestamp=int(sample["timestamp"]),
        replication_lag_ms=float(sample["replication_lag_ms"]),
        queue_depth=int(sample["queue_depth"]),
        connection_pool_saturation=float(sample["connection_pool_saturation"]),
        connection_wait_time_ms=float(sample["connection_wait_time_ms"]),
    )

    evaluation = slo.evaluate_sample(stateful_sample)

    return {
        "timestamp": int(evaluation.timestamp),
        "score": float(evaluation.score),
        "replication_lag_ok": bool(evaluation.replication_lag_ok),
        "queue_depth_ok": bool(evaluation.queue_depth_ok),
        "connection_pool_ok": bool(evaluation.connection_pool_ok),
        "connection_wait_penalized": bool(evaluation.connection_wait_penalized),
        "pass": bool(getattr(evaluation, "pass")),
    }
