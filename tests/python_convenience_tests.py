#!/usr/bin/env python3

"""Unit tests for convenience layer dataclass and preset behavior.

These tests stub the native extension module so they can run without building
or importing the compiled PyO3 binary.
"""

from __future__ import annotations

import importlib.util
import sys
import types
import unittest
from pathlib import Path


class _PassAttrMixin:
    @property
    def pass_(self) -> bool:
        return self._pass

    def __getattr__(self, name: str):
        if name == "pass":
            return self._pass
        raise AttributeError(name)


class FakeMetricPoint:
    def __init__(self, timestamp: int, value: float):
        self.timestamp = timestamp
        self.value = value


class FakeHistogramBucket:
    def __init__(self, upper_bound_ms: float, count: int):
        self.upper_bound_ms = upper_bound_ms
        self.count = count


class FakeHistogramSample:
    def __init__(self, timestamp: int, success: int, total: int, buckets, format: str):
        self.timestamp = timestamp
        self.success = success
        self.total = total
        self.buckets = buckets
        self.format = format


class FakeHttpEvaluation(_PassAttrMixin):
    def __init__(self, *, timestamp: int, availability: float, percentile_latency_ms: float, passed: bool):
        self.timestamp = timestamp
        self.availability = availability
        self.percentile_latency_ms = percentile_latency_ms
        self.evaluated_percentile = 0.99
        self.latency_ok = percentile_latency_ms < 200.0
        self.availability_ok = availability > 0.999
        self._pass = passed


class FakeHttpSlo:
    def __init__(self, latency_threshold_ms: float, latency_percentile: float, availability_threshold: float):
        self.latency_threshold_ms = latency_threshold_ms
        self.latency_percentile = latency_percentile
        self.availability_threshold = availability_threshold

    def evaluate_stream(self, samples):
        sample = samples[0]
        availability = sample.success / max(sample.total, 1)
        percentile_latency = 180.0
        passed = (
            percentile_latency < self.latency_threshold_ms
            and availability > self.availability_threshold
        )
        return [
            FakeHttpEvaluation(
                timestamp=sample.timestamp,
                availability=availability,
                percentile_latency_ms=percentile_latency,
                passed=passed,
            )
        ]


class FakeStatefulSample:
    def __init__(
        self,
        timestamp: int,
        replication_lag_ms: float,
        queue_depth: int,
        connection_pool_saturation: float,
        connection_wait_time_ms: float,
    ):
        self.timestamp = timestamp
        self.replication_lag_ms = replication_lag_ms
        self.queue_depth = queue_depth
        self.connection_pool_saturation = connection_pool_saturation
        self.connection_wait_time_ms = connection_wait_time_ms


class FakeStatefulEvaluation(_PassAttrMixin):
    def __init__(self, sample: FakeStatefulSample, passed: bool):
        self.timestamp = sample.timestamp
        self.score = 1.0 if passed else 0.5
        self.replication_lag_ok = sample.replication_lag_ms <= 250.0
        self.queue_depth_ok = sample.queue_depth <= 1_000
        self.connection_pool_ok = sample.connection_pool_saturation <= 0.8
        self.connection_wait_penalized = sample.connection_wait_time_ms > 20.0
        self._pass = passed


class FakeStatefulSlo:
    def __init__(
        self,
        replication_lag_threshold_ms: float,
        queue_depth_threshold: int,
        connection_pool_saturation_threshold: float,
        connection_wait_time_threshold_ms: float,
        connection_wait_penalty_weight: float,
        min_pass_score: float,
    ):
        self.replication_lag_threshold_ms = replication_lag_threshold_ms
        self.queue_depth_threshold = queue_depth_threshold
        self.connection_pool_saturation_threshold = connection_pool_saturation_threshold
        self.connection_wait_time_threshold_ms = connection_wait_time_threshold_ms
        self.connection_wait_penalty_weight = connection_wait_penalty_weight
        self.min_pass_score = min_pass_score

    def evaluate_sample(self, sample: FakeStatefulSample):
        passed = (
            sample.replication_lag_ms <= self.replication_lag_threshold_ms
            and sample.queue_depth <= self.queue_depth_threshold
            and sample.connection_pool_saturation <= self.connection_pool_saturation_threshold
        )
        return FakeStatefulEvaluation(sample, passed=passed)


class FakeMlSample:
    def __init__(
        self,
        timestamp: int,
        inference_latency_ms: float,
        gpu_utilization: float,
        feature_drift: float,
        prediction_confidence: float,
    ):
        self.timestamp = timestamp
        self.inference_latency_ms = inference_latency_ms
        self.gpu_utilization = gpu_utilization
        self.feature_drift = feature_drift
        self.prediction_confidence = prediction_confidence


class FakeMlEvaluation(_PassAttrMixin):
    def __init__(self, sample: FakeMlSample, *, latency_weight: float, drift_weight: float, hybrid_score: float, passed: bool):
        self.timestamp = sample.timestamp
        self.inference_latency_score = 0.95
        self.gpu_utilization_score = 0.9
        self.system_score = 0.925
        self.latency_score = self.system_score
        self.feature_drift_score = 0.8
        self.prediction_confidence_score = 0.9
        self.drift_score = 0.85
        self.latency_weight = latency_weight
        self.drift_weight = drift_weight
        self.hybrid_score = hybrid_score
        self._pass = passed


class FakeMlSlo:
    def __init__(
        self,
        max_inference_latency_ms: float,
        max_gpu_utilization: float,
        max_feature_drift: float,
        min_prediction_confidence: float,
        latency_weight: float,
        drift_weight: float,
        min_pass_score: float,
    ):
        self.max_inference_latency_ms = max_inference_latency_ms
        self.max_gpu_utilization = max_gpu_utilization
        self.max_feature_drift = max_feature_drift
        self.min_prediction_confidence = min_prediction_confidence
        self.latency_weight = latency_weight
        self.drift_weight = drift_weight
        self.min_pass_score = min_pass_score

    def evaluate_sample(self, sample: FakeMlSample):
        total = max(self.latency_weight + self.drift_weight, 1e-9)
        lw = self.latency_weight / total
        dw = self.drift_weight / total
        hybrid = 0.9 * lw + 0.8 * dw
        passed = hybrid >= self.min_pass_score
        return FakeMlEvaluation(
            sample,
            latency_weight=lw,
            drift_weight=dw,
            hybrid_score=hybrid,
            passed=passed,
        )


class FakeGenAiSample:
    def __init__(
        self,
        timestamp: int,
        tokens_generated: int,
        generation_duration_ms: float,
        time_to_first_token_ms: float,
        reference_text: str,
        generated_text: str,
    ):
        self.timestamp = timestamp
        self.tokens_generated = tokens_generated
        self.generation_duration_ms = generation_duration_ms
        self.time_to_first_token_ms = time_to_first_token_ms
        self.reference_text = reference_text
        self.generated_text = generated_text


class FakeGenAiEvaluation(_PassAttrMixin):
    def __init__(self, sample: FakeGenAiSample, *, semantic_similarity: float, passed: bool):
        self.timestamp = sample.timestamp
        self.tokens_per_second = (
            sample.tokens_generated / (sample.generation_duration_ms / 1000.0)
            if sample.generation_duration_ms > 0.0
            else 0.0
        )
        self.time_to_first_token_ms = sample.time_to_first_token_ms
        self.semantic_similarity = semantic_similarity
        self.tokens_per_second_ok = self.tokens_per_second >= 20.0
        self.time_to_first_token_ok = self.time_to_first_token_ms <= 1200.0
        self.semantic_similarity_ok = semantic_similarity >= 0.7
        self._pass = passed


class FakeGenAiSlo:
    def __init__(
        self,
        min_tokens_per_second: float,
        max_time_to_first_token_ms: float,
        min_semantic_similarity: float,
        semantic_model_name: str,
    ):
        self.min_tokens_per_second = min_tokens_per_second
        self.max_time_to_first_token_ms = max_time_to_first_token_ms
        self.min_semantic_similarity = min_semantic_similarity
        self.semantic_model_name = semantic_model_name

    def evaluate_sample(self, sample: FakeGenAiSample):
        semantic_similarity = 0.9 if sample.reference_text == sample.generated_text else 0.4
        tokens_per_second = (
            sample.tokens_generated / (sample.generation_duration_ms / 1000.0)
            if sample.generation_duration_ms > 0.0
            else 0.0
        )
        passed = (
            tokens_per_second >= self.min_tokens_per_second
            and sample.time_to_first_token_ms <= self.max_time_to_first_token_ms
            and semantic_similarity >= self.min_semantic_similarity
        )
        return FakeGenAiEvaluation(sample, semantic_similarity=semantic_similarity, passed=passed)


def _load_convenience_module():
    repo_root = Path(__file__).resolve().parents[1]
    convenience_path = repo_root / "python" / "neuralbudget" / "convenience.py"

    fake_pkg = types.ModuleType("neuralbudget")
    fake_pkg.__path__ = []

    fake_native = types.ModuleType("neuralbudget.neuralbudget")
    fake_native.MetricPoint = FakeMetricPoint
    fake_native.HistogramBucket = FakeHistogramBucket
    fake_native.HistogramSample = FakeHistogramSample
    fake_native.HttpSlo = FakeHttpSlo
    fake_native.StatefulSample = FakeStatefulSample
    fake_native.StatefulSlo = FakeStatefulSlo
    fake_native.MlSample = FakeMlSample
    fake_native.MlSlo = FakeMlSlo
    fake_native.GenAiSample = FakeGenAiSample
    fake_native.GenAiSlo = FakeGenAiSlo
    fake_native.calculate_availability = lambda success, total: success / max(total, 1)
    fake_native.calculate_error_budget = lambda target, window: (1.0 - target) * window
    fake_native.calculate_burn_rate = lambda stream, window: 0.5 if window else 0.0

    sys.modules["neuralbudget"] = fake_pkg
    sys.modules["neuralbudget.neuralbudget"] = fake_native

    spec = importlib.util.spec_from_file_location("neuralbudget.convenience", convenience_path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["neuralbudget.convenience"] = module
    spec.loader.exec_module(module)
    return module


class ConvenienceLayerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.convenience = _load_convenience_module()

    def test_http_profile_preset_and_override(self):
        preset = self.convenience.get_http_profile_preset("strict_latency")
        self.assertEqual(preset.latency_threshold_ms, 150.0)

        result = self.convenience.evaluate_http_histogram_once(
            {
                "timestamp": 1,
                "success": 1000,
                "total": 1000,
                "buckets": [
                    {"upper_bound_ms": 100.0, "count": 900},
                    {"upper_bound_ms": 200.0, "count": 1000},
                ],
                "format": "prometheus_cumulative",
            },
            profile="strict_latency",
            latency_threshold_ms=220.0,
        )

        self.assertIsInstance(result, dict)
        self.assertTrue(result["pass"])

    def test_unknown_profile_raises(self):
        with self.assertRaises(ValueError):
            self.convenience.get_ml_profile_preset("not_a_real_profile")

    def test_genai_profile_preset(self):
        preset = self.convenience.get_genai_profile_preset("quality_first")
        self.assertGreater(preset.min_semantic_similarity, 0.7)


if __name__ == "__main__":
    unittest.main(verbosity=2)
