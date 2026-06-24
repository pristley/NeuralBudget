#!/usr/bin/env python3

"""Example usage for the pure-Python convenience layer."""

from neuralbudget.convenience import (
    availability_snapshot,
    burn_rate_from_values,
    evaluate_http_histogram_once,
    evaluate_stateful_once,
)


def main() -> None:
    snapshot = availability_snapshot(success=9_995, total=10_000, slo_target=0.999, window_secs=3_600)
    print("availability_snapshot", snapshot)

    burn = burn_rate_from_values([0.0, 1.0, 1.0, 0.0, 1.0], 5)
    print("burn_rate_from_values", burn)

    http_eval = evaluate_http_histogram_once(
        {
            "timestamp": 1,
            "success": 9_995,
            "total": 10_000,
            "buckets": [
                {"upper_bound_ms": 100.0, "count": 9_700},
                {"upper_bound_ms": 150.0, "count": 200},
                {"upper_bound_ms": 220.0, "count": 100},
            ],
            "format": "open_telemetry_delta",
        }
    )
    print("evaluate_http_histogram_once", http_eval)

    stateful_eval = evaluate_stateful_once(
        {
            "timestamp": 10,
            "replication_lag_ms": 180.0,
            "queue_depth": 700,
            "connection_pool_saturation": 0.7,
            "connection_wait_time_ms": 8.0,
        }
    )
    print("evaluate_stateful_once", stateful_eval)


if __name__ == "__main__":
    main()
