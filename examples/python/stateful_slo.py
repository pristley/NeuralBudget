#!/usr/bin/env python3

"""Python example for stateful database/queue SLO evaluation."""

import neuralbudget


def main() -> None:
    slo = neuralbudget.StatefulSlo(
        replication_lag_threshold_ms=250.0,
        queue_depth_threshold=1_000,
        connection_pool_saturation_threshold=0.8,
        connection_wait_time_threshold_ms=20.0,
        connection_wait_penalty_weight=0.25,
        min_pass_score=0.85,
    )

    samples = [
        neuralbudget.StatefulSample(
            timestamp=1,
            replication_lag_ms=180.0,
            queue_depth=700,
            connection_pool_saturation=0.7,
            connection_wait_time_ms=8.0,
        ),
        neuralbudget.StatefulSample(
            timestamp=2,
            replication_lag_ms=200.0,
            queue_depth=800,
            connection_pool_saturation=0.75,
            connection_wait_time_ms=60.0,
        ),
    ]

    evaluations = slo.evaluate_stream(samples)
    for evaluation in evaluations:
        passed = getattr(evaluation, "pass")
        print(
            f"timestamp={evaluation.timestamp} "
            f"pass={passed} "
            f"score={evaluation.score:.3f} "
            f"wait_penalized={evaluation.connection_wait_penalized}"
        )


if __name__ == "__main__":
    main()
