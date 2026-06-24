#!/usr/bin/env python3

"""Example: hybrid ML serving + data drift SLO evaluation."""

from neuralbudget.convenience import evaluate_ml_once


def main() -> None:
    healthy = evaluate_ml_once(
        {
            "timestamp": 1,
            "inference_latency_ms": 178.0,
            "gpu_utilization": 0.70,
            "feature_drift": 0.06,
            "prediction_confidence": 0.94,
        },
        latency_weight=0.6,
        drift_weight=0.4,
        min_pass_score=0.9,
    )

    drifting = evaluate_ml_once(
        {
            "timestamp": 2,
            "inference_latency_ms": 252.0,
            "gpu_utilization": 0.93,
            "feature_drift": 0.23,
            "prediction_confidence": 0.73,
        },
        latency_weight=0.6,
        drift_weight=0.4,
        min_pass_score=0.9,
    )

    print("healthy", healthy)
    print("drifting", drifting)

    # Example of weight tuning to emphasize data quality more strongly.
    data_weighted = evaluate_ml_once(
        {
            "timestamp": 3,
            "inference_latency_ms": 205.0,
            "gpu_utilization": 0.82,
            "feature_drift": 0.19,
            "prediction_confidence": 0.78,
        },
        latency_weight=0.35,
        drift_weight=0.65,
        min_pass_score=0.85,
    )
    print("data_weighted", data_weighted)


if __name__ == "__main__":
    main()
