#!/usr/bin/env python3

"""Python example for evaluating stateful SLO by profile-specific policies.

The current Python API does not yet expose StatefulPolicyProfileSet directly,
so this script demonstrates tier behavior by applying different StatefulSlo
instances that reflect distinct database and queue thresholds.
"""

import neuralbudget


def main() -> None:
    database_slo = neuralbudget.StatefulSlo(
        replication_lag_threshold_ms=250.0,
        queue_depth_threshold=1_200,
        connection_pool_saturation_threshold=0.85,
        connection_wait_time_threshold_ms=30.0,
        connection_wait_penalty_weight=0.2,
        min_pass_score=0.88,
    )

    queue_slo = neuralbudget.StatefulSlo(
        replication_lag_threshold_ms=350.0,
        queue_depth_threshold=700,
        connection_pool_saturation_threshold=0.75,
        connection_wait_time_threshold_ms=20.0,
        connection_wait_penalty_weight=0.3,
        min_pass_score=0.90,
    )

    sample = neuralbudget.StatefulSample(
        timestamp=10,
        replication_lag_ms=220.0,
        queue_depth=800,
        connection_pool_saturation=0.74,
        connection_wait_time_ms=25.0,
    )

    database_eval = database_slo.evaluate_sample(sample)
    queue_eval = queue_slo.evaluate_sample(sample)
    database_pass = getattr(database_eval, "pass")
    queue_pass = getattr(queue_eval, "pass")

    print("database_profile", database_pass, f"score={database_eval.score:.3f}")
    print("queue_profile", queue_pass, f"score={queue_eval.score:.3f}")


if __name__ == "__main__":
    main()
