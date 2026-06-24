#!/usr/bin/env python3

"""Python example for stateless HTTP/gRPC histogram SLO evaluation."""

import neuralbudget


def main() -> None:
    slo = neuralbudget.HttpSlo(
        latency_threshold_ms=200.0,
        latency_percentile=0.99,
        availability_threshold=0.999,
    )

    samples = [
        neuralbudget.HistogramSample(
            timestamp=1,
            success=9_995,
            total=10_000,
            buckets=[
                neuralbudget.HistogramBucket(upper_bound_ms=100.0, count=9_700),
                neuralbudget.HistogramBucket(upper_bound_ms=150.0, count=200),
                neuralbudget.HistogramBucket(upper_bound_ms=220.0, count=100),
            ],
            format="open_telemetry_delta",
        ),
        neuralbudget.HistogramSample(
            timestamp=2,
            success=9_980,
            total=10_000,
            buckets=[
                neuralbudget.HistogramBucket(upper_bound_ms=100.0, count=9_500),
                neuralbudget.HistogramBucket(upper_bound_ms=200.0, count=9_600),
                neuralbudget.HistogramBucket(upper_bound_ms=500.0, count=10_000),
            ],
            format="prometheus_cumulative",
        ),
    ]

    evaluations = slo.evaluate_stream(samples)
    for evaluation in evaluations:
        passed = getattr(evaluation, "pass")
        print(
            f"timestamp={evaluation.timestamp} "
            f"pass={passed} "
            f"availability={evaluation.availability:.6f} "
            f"p{int(evaluation.evaluated_percentile * 100)}={evaluation.percentile_latency_ms:.2f}ms"
        )


if __name__ == "__main__":
    main()
