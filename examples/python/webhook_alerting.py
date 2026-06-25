#!/usr/bin/env python3

"""Example: trigger webhook alerting on SLO violation."""

from neuralbudget import NeuralBudgetClient


def main() -> None:
    client = NeuralBudgetClient().load_config("examples/python/webhook_alerting_config.json")

    # This sample is intentionally unhealthy to trigger a violation alert.
    result = client.evaluate(
        {
            "timestamp": 1,
            "success": 980,
            "total": 1000,
            "buckets": [
                {"upper_bound_ms": 100.0, "count": 850},
                {"upper_bound_ms": 250.0, "count": 940},
                {"upper_bound_ms": 500.0, "count": 1000},
            ],
            "format": "prometheus_cumulative",
        }
    )

    print(result)


if __name__ == "__main__":
    main()
