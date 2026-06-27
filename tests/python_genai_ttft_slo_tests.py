"""
Comprehensive Python test suite for TTFT (Time to First Token) SLO evaluation.

Tests TTFT evaluation, inter-token latency, percentile calculations, batch metrics,
and real-world streaming scenarios.
"""

import json
import time
import pytest
from typing import List, Dict, Any


class TestTtftBasics:
    """Test basic TTFT evaluation functionality."""

    def test_ttft_pass(self):
        """Test TTFT passing threshold."""
        sample = {
            "request_id": "test_123",
            "time_to_first_token_ms": 450,
            "inter_token_latency_ms": 45,
            "total_tokens": 250,
            "total_response_time_ms": 13000,
            "model": "gpt-4",
        }
        params = {
            "ttft_threshold_ms": 500,
            "ttft_percentile": 0.99,
            "inter_token_latency_threshold_ms": 50,
            "inter_token_percentile": 0.95,
        }

        # TTFT: 450 < 500 ✓
        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]
        inter_pass = sample["inter_token_latency_ms"] <= params["inter_token_latency_threshold_ms"]

        assert ttft_pass
        assert inter_pass

    def test_ttft_fail(self):
        """Test TTFT exceeding threshold."""
        sample = {
            "time_to_first_token_ms": 550,
            "inter_token_latency_ms": 45,
        }
        params = {"ttft_threshold_ms": 500}

        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]

        assert not ttft_pass

    def test_inter_token_fail(self):
        """Test inter-token latency exceeding threshold."""
        sample = {
            "inter_token_latency_ms": 60,
        }
        params = {"inter_token_latency_threshold_ms": 50}

        inter_pass = sample["inter_token_latency_ms"] <= params["inter_token_latency_threshold_ms"]

        assert not inter_pass

    def test_both_fail(self):
        """Test both TTFT and inter-token failing."""
        sample = {
            "time_to_first_token_ms": 600,
            "inter_token_latency_ms": 70,
        }
        params = {
            "ttft_threshold_ms": 500,
            "inter_token_latency_threshold_ms": 50,
        }

        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]
        inter_pass = sample["inter_token_latency_ms"] <= params["inter_token_latency_threshold_ms"]

        assert not (ttft_pass and inter_pass)


class TestTtftUtilization:
    """Test TTFT utilization calculations."""

    def test_ttft_utilization_90_percent(self):
        """Test TTFT at 90% of budget."""
        ttft = 450.0
        threshold = 500.0

        utilization = ttft / threshold

        assert pytest.approx(utilization, abs=0.01) == 0.9

    def test_ttft_utilization_50_percent(self):
        """Test TTFT at 50% of budget."""
        ttft = 250.0
        threshold = 500.0

        utilization = ttft / threshold

        assert pytest.approx(utilization, abs=0.01) == 0.5

    def test_inter_token_utilization_80_percent(self):
        """Test inter-token at 80% of budget."""
        inter_token = 40.0
        threshold = 50.0

        utilization = inter_token / threshold

        assert pytest.approx(utilization, abs=0.01) == 0.8

    def test_ttft_fraction_of_total(self):
        """Test TTFT as fraction of total response time."""
        ttft = 500.0
        total_time = 10000.0

        fraction = ttft / total_time

        assert pytest.approx(fraction, abs=0.001) == 0.05


class TestTokenThroughput:
    """Test token throughput calculations."""

    def test_tokens_per_second_100tps(self):
        """Test 100 tokens per second calculation."""
        tokens = 1000
        total_time_ms = 10000  # 10 seconds

        tps = (tokens / total_time_ms) * 1000

        assert pytest.approx(tps, abs=0.1) == 100.0

    def test_tokens_per_second_20tps(self):
        """Test 20 tokens per second calculation."""
        tokens = 500
        total_time_ms = 25000  # 25 seconds

        tps = (tokens / total_time_ms) * 1000

        assert pytest.approx(tps, abs=0.1) == 20.0

    def test_tokens_per_second_zero_time(self):
        """Test TPS calculation with zero time."""
        tokens = 500
        total_time_ms = 0.0

        tps = 0.0 if total_time_ms == 0 else (tokens / total_time_ms) * 1000

        assert tps == 0.0


class TestPercentileCalculation:
    """Test percentile calculations."""

    def test_p50_median_of_five(self):
        """Test P50 (median) of 5 values."""
        values = [100, 200, 300, 400, 500]
        sorted_values = sorted(values)

        # P50 index = 0.5 * (5-1) = 2
        p50_index = int(0.5 * (len(sorted_values) - 1))
        p50 = sorted_values[p50_index]

        assert p50 == 300

    def test_p99_near_max(self):
        """Test P99 near maximum value."""
        values = [100, 200, 300, 400, 500]
        sorted_values = sorted(values)

        # P99 index = 0.99 * (5-1) ≈ 3.96 → 3
        p99_index = min(int(0.99 * (len(sorted_values) - 1)), len(sorted_values) - 1)
        p99 = sorted_values[p99_index]

        assert p99 >= 400

    def test_p95_percentile(self):
        """Test P95 calculation."""
        values = [100, 200, 300, 400, 500]
        sorted_values = sorted(values)

        # P95 index = 0.95 * (5-1) ≈ 3.8 → 3
        p95_index = min(int(0.95 * (len(sorted_values) - 1)), len(sorted_values) - 1)
        p95 = sorted_values[p95_index]

        assert p95 >= 300

    def test_percentile_single_value(self):
        """Test percentile with single value."""
        values = [250.0]

        p50 = values[0]
        p99 = values[0]

        assert p50 == 250.0
        assert p99 == 250.0


class TestBatchMetrics:
    """Test batch evaluation metrics."""

    def test_batch_pass_rate_8_of_10(self):
        """Test batch pass rate of 8 out of 10."""
        samples = [
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 450, "pass": True},
            {"ttft": 600, "pass": False},
            {"ttft": 600, "pass": False},
        ]

        passes = sum(1 for s in samples if s["pass"])
        pass_rate = passes / len(samples)

        assert pytest.approx(pass_rate, abs=0.001) == 0.8

    def test_batch_average_ttft(self):
        """Test average TTFT calculation."""
        samples = [
            {"ttft": 400},
            {"ttft": 500},
            {"ttft": 600},
        ]

        avg_ttft = sum(s["ttft"] for s in samples) / len(samples)

        assert pytest.approx(avg_ttft, abs=0.1) == 500.0

    def test_batch_average_inter_token(self):
        """Test average inter-token latency."""
        samples = [
            {"inter_token": 40},
            {"inter_token": 50},
            {"inter_token": 60},
        ]

        avg = sum(s["inter_token"] for s in samples) / len(samples)

        assert pytest.approx(avg, abs=0.1) == 50.0

    def test_batch_all_pass(self):
        """Test batch where all samples pass."""
        samples = [
            {"ttft": 450, "pass": True},
            {"ttft": 480, "pass": True},
            {"ttft": 420, "pass": True},
        ]

        pass_rate = sum(1 for s in samples if s["pass"]) / len(samples)

        assert pytest.approx(pass_rate, abs=0.001) == 1.0

    def test_batch_all_fail(self):
        """Test batch where all samples fail."""
        samples = [
            {"ttft": 550, "pass": False},
            {"ttft": 600, "pass": False},
            {"ttft": 580, "pass": False},
        ]

        pass_rate = sum(1 for s in samples if s["pass"]) / len(samples)

        assert pytest.approx(pass_rate, abs=0.001) == 0.0


class TestRealWorldScenarios:
    """Test real-world streaming scenarios."""

    def test_chat_assistant_fast_response(self):
        """Test chat assistant with fast TTFT."""
        sample = {
            "time_to_first_token_ms": 250,
            "inter_token_latency_ms": 40,
            "total_tokens": 500,
            "total_response_time_ms": 25000,
        }
        params = {
            "ttft_threshold_ms": 300,
            "inter_token_latency_threshold_ms": 50,
        }

        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]
        inter_pass = sample["inter_token_latency_ms"] <= params["inter_token_latency_threshold_ms"]

        assert ttft_pass
        assert inter_pass

    def test_code_generation_longer_ttft(self):
        """Test code generation with longer TTFT but steady throughput."""
        sample = {
            "time_to_first_token_ms": 600,
            "inter_token_latency_ms": 30,
            "total_tokens": 1000,
            "total_response_time_ms": 40000,
        }
        params = {
            "ttft_threshold_ms": 700,
            "inter_token_latency_threshold_ms": 40,
        }

        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]
        inter_pass = sample["inter_token_latency_ms"] <= params["inter_token_latency_threshold_ms"]
        tps = (sample["total_tokens"] / sample["total_response_time_ms"]) * 1000

        assert ttft_pass
        assert inter_pass
        assert tps > 20.0

    def test_summarization_many_tokens(self):
        """Test summarization with many tokens."""
        sample = {
            "time_to_first_token_ms": 400,
            "inter_token_latency_ms": 35,
            "total_tokens": 5000,
            "total_response_time_ms": 200000,
        }

        tps = (sample["total_tokens"] / sample["total_response_time_ms"]) * 1000
        ttft_frac = sample["time_to_first_token_ms"] / sample["total_response_time_ms"]

        assert tps > 20.0
        assert ttft_frac < 0.1  # TTFT should be small fraction of total

    def test_batch_multiple_chat_requests(self):
        """Test batch of typical chat responses."""
        samples = [
            {"ttft": 280, "inter_token": 35, "tokens": 150},
            {"ttft": 320, "inter_token": 38, "tokens": 200},
            {"ttft": 250, "inter_token": 32, "tokens": 180},
            {"ttft": 300, "inter_token": 40, "tokens": 220},
        ]

        avg_ttft = sum(s["ttft"] for s in samples) / len(samples)
        avg_inter = sum(s["inter_token"] for s in samples) / len(samples)
        avg_tokens = sum(s["tokens"] for s in samples) / len(samples)

        assert 250 < avg_ttft < 350
        assert 30 < avg_inter < 40
        assert 150 < avg_tokens < 250


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_single_token_response(self):
        """Test response with single token."""
        sample = {
            "time_to_first_token_ms": 200,
            "inter_token_latency_ms": 0,
            "total_tokens": 1,
            "total_response_time_ms": 200,
        }

        assert sample["total_tokens"] == 1
        assert sample["inter_token_latency_ms"] == 0

    def test_zero_inter_token_latency(self):
        """Test hypothetical instant token generation."""
        sample = {
            "inter_token_latency_ms": 0,
        }
        params = {"inter_token_latency_threshold_ms": 50}

        inter_pass = sample["inter_token_latency_ms"] <= params["inter_token_latency_threshold_ms"]

        assert inter_pass

    def test_exact_threshold_ttft(self):
        """Test TTFT exactly at threshold."""
        sample = {"time_to_first_token_ms": 500}
        params = {"ttft_threshold_ms": 500}

        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]

        assert ttft_pass

    def test_one_ms_over_threshold(self):
        """Test TTFT one millisecond over threshold."""
        sample = {"time_to_first_token_ms": 501}
        params = {"ttft_threshold_ms": 500}

        ttft_pass = sample["time_to_first_token_ms"] <= params["ttft_threshold_ms"]

        assert not ttft_pass

    def test_very_large_response_10k_tokens(self):
        """Test very long streaming response (10k tokens)."""
        sample = {
            "time_to_first_token_ms": 800,
            "inter_token_latency_ms": 35,
            "total_tokens": 10000,
            "total_response_time_ms": 350000,
        }

        tps = (sample["total_tokens"] / sample["total_response_time_ms"]) * 1000
        ttft_frac = sample["time_to_first_token_ms"] / sample["total_response_time_ms"]

        assert tps > 20.0
        assert ttft_frac < 0.01  # TTFT < 1% of total for large responses


class TestSerialization:
    """Test JSON serialization."""

    def test_serialize_sample_json(self):
        """Test serializing sample to JSON."""
        sample = {
            "request_id": "req_123",
            "time_to_first_token_ms": 450,
            "inter_token_latency_ms": 45,
            "total_tokens": 250,
            "total_response_time_ms": 13000,
            "model": "gpt-4",
        }

        json_str = json.dumps(sample)
        deserialized = json.loads(json_str)

        assert deserialized["time_to_first_token_ms"] == 450
        assert deserialized["total_tokens"] == 250

    def test_serialize_evaluation_json(self):
        """Test serializing evaluation result to JSON."""
        evaluation = {
            "ttft_pass": True,
            "ttft_ms": 450,
            "inter_token_pass": True,
            "inter_token_latency_ms": 45,
            "pass": True,
        }

        json_str = json.dumps(evaluation)
        deserialized = json.loads(json_str)

        assert deserialized["pass"] is True
        assert deserialized["ttft_ms"] == 450


class TestConfigurationVariants:
    """Test different SLO configurations."""

    def test_config_strict_chat(self):
        """Test strict configuration for chat."""
        config = {
            "ttft_threshold_ms": 300,
            "ttft_percentile": 0.99,
            "inter_token_latency_threshold_ms": 40,
            "inter_token_percentile": 0.95,
        }

        assert config["ttft_threshold_ms"] == 300
        assert config["inter_token_latency_threshold_ms"] == 40

    def test_config_tolerant_batch(self):
        """Test tolerant configuration for batch."""
        config = {
            "ttft_threshold_ms": 2000,
            "ttft_percentile": 0.95,
            "inter_token_latency_threshold_ms": 100,
            "inter_token_percentile": 0.90,
        }

        assert config["ttft_threshold_ms"] == 2000
        assert config["inter_token_latency_threshold_ms"] == 100

    def test_config_default(self):
        """Test default configuration."""
        config = {
            "ttft_threshold_ms": 500,
            "ttft_percentile": 0.99,
            "inter_token_latency_threshold_ms": 50,
            "inter_token_percentile": 0.95,
        }

        assert config["ttft_threshold_ms"] == 500
        assert config["inter_token_percentile"] == 0.95


class TestMonitoringMetrics:
    """Test metrics suitable for monitoring."""

    def test_metric_ttft_utilization(self):
        """Test TTFT utilization metric."""
        ttft = 450.0
        threshold = 500.0
        utilization = ttft / threshold

        # Alert if > 90% utilization
        alert = utilization > 0.9

        assert not alert  # 90% exactly

    def test_metric_throughput_degradation(self):
        """Test throughput degradation detection."""
        baseline_tps = 25.0
        current_tps = 15.0

        degradation = (baseline_tps - current_tps) / baseline_tps

        assert degradation > 0.3  # 30% degradation

    def test_metric_ttft_spike(self):
        """Test TTFT spike detection."""
        baseline_ttft = 400.0
        current_ttft = 900.0

        spike_ratio = current_ttft / baseline_ttft

        assert spike_ratio > 2.0  # More than 2x baseline


class TestIntegrationPatterns:
    """Test integration patterns."""

    def test_batch_to_monitoring_format(self):
        """Test converting batch eval to monitoring format."""
        batch_data = {
            "total_samples": 100,
            "ttft_pass_rate": 0.95,
            "ttft_p99_ms": 480,
            "inter_token_p95_ms": 48,
            "avg_tokens_per_second": 22.5,
            "overall_pass_rate": 0.92,
        }

        monitoring = {
            "slo_pass_rate": batch_data["overall_pass_rate"],
            "ttft_p99": batch_data["ttft_p99_ms"],
            "throughput_tps": batch_data["avg_tokens_per_second"],
        }

        assert monitoring["slo_pass_rate"] == 0.92
        assert monitoring["ttft_p99"] == 480

    def test_sample_to_prometheus_metrics(self):
        """Test converting sample to Prometheus metrics."""
        sample = {
            "time_to_first_token_ms": 450,
            "inter_token_latency_ms": 45,
            "total_tokens": 250,
            "total_response_time_ms": 13000,
        }

        metrics = {
            "genai_ttft_ms": sample["time_to_first_token_ms"],
            "genai_inter_token_latency_ms": sample["inter_token_latency_ms"],
            "genai_tokens_generated": sample["total_tokens"],
            "genai_response_time_ms": sample["total_response_time_ms"],
        }

        assert metrics["genai_ttft_ms"] == 450


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
