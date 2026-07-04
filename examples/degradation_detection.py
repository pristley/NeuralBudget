"""Simple example showing the DegradationDetector convenience wrapper."""
from neuralbudget.degradation import DegradationDetector


def main():
    d = DegradationDetector(window_size=60, drift_threshold=0.10, spike_threshold=2.0)

    # warm-up: stable values
    for _ in range(40):
        d.add_metric(50.0)

    # introduce a gradual drift
    for i in range(20):
        d.add_metric(50.0 + i * 1.5)

    ev = d.detect_change()
    print("Degradation event:", ev)


if __name__ == "__main__":
    main()
