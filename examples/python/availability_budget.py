#!/usr/bin/env python3

"""Python example for foundational SLO math primitives."""

import neuralbudget


def main() -> None:
    availability = neuralbudget.calculate_availability(9_995, 10_000)
    error_budget_seconds = neuralbudget.calculate_error_budget(0.999, 3_600)

    stream = [
        neuralbudget.MetricPoint(1, 0.0),
        neuralbudget.MetricPoint(2, 1.0),
        neuralbudget.MetricPoint(3, 1.0),
        neuralbudget.MetricPoint(4, 0.0),
        neuralbudget.MetricPoint(5, 1.0),
    ]
    burn_rate = neuralbudget.calculate_burn_rate(stream, 5)

    print(f"availability={availability:.6f}")
    print(f"error_budget_seconds={error_budget_seconds:.3f}")
    print(f"burn_rate={burn_rate:.6f}")


if __name__ == "__main__":
    main()
