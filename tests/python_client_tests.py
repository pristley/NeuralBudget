#!/usr/bin/env python3

"""Unit tests for NeuralBudgetClient facade behavior.

These tests stub convenience/native modules so they run without the compiled
PyO3 extension.
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import types
import unittest
from pathlib import Path


class _EvalResult:
    def __init__(self, payload: dict):
        self._payload = payload

    def to_dict(self) -> dict:
        return dict(self._payload)


class _FakeCompositeService:
    def __init__(self, service: str, local_score: float, min_pass_score: float, impact_weight: float):
        self.service = service
        self.local_score = local_score
        self.min_pass_score = min_pass_score
        self.impact_weight = impact_weight


class _FakeCompositeEdge:
    def __init__(self, dependency: str, dependent: str, failure_penalty: float):
        self.dependency = dependency
        self.dependent = dependent
        self.failure_penalty = failure_penalty


class _FakeCompositeGraph:
    def __init__(self, services, dependencies, global_min_pass_score: float):
        self.services = services
        self.dependencies = dependencies
        self.global_min_pass_score = global_min_pass_score


def _load_client_module():
    repo_root = Path(__file__).resolve().parents[1]
    client_path = repo_root / "python" / "neuralbudget" / "client.py"

    fake_pkg = types.ModuleType("neuralbudget")
    fake_pkg.__path__ = []

    fake_convenience = types.ModuleType("neuralbudget.convenience")

    class HttpHistogramEvaluationResult:  # pragma: no cover - type holder
        pass

    class StatefulEvaluationResult:  # pragma: no cover - type holder
        pass

    class MlEvaluationResult:  # pragma: no cover - type holder
        pass

    class GenAiEvaluationResult:  # pragma: no cover - type holder
        pass

    fake_convenience.HttpHistogramEvaluationResult = HttpHistogramEvaluationResult
    fake_convenience.StatefulEvaluationResult = StatefulEvaluationResult
    fake_convenience.MlEvaluationResult = MlEvaluationResult
    fake_convenience.GenAiEvaluationResult = GenAiEvaluationResult

    def _http_eval(sample, **kwargs):
        return {"mode": "http", "sample": sample, "kwargs": kwargs}

    def _stateful_eval(sample, **kwargs):
        return {"mode": "stateful", "sample": sample, "kwargs": kwargs}

    def _ml_eval(sample, **kwargs):
        return {"mode": "ml", "sample": sample, "kwargs": kwargs}

    def _genai_eval(sample, **kwargs):
        return {"mode": "genai", "sample": sample, "kwargs": kwargs}

    fake_convenience.evaluate_http_histogram_once = _http_eval
    fake_convenience.evaluate_stateful_once = _stateful_eval
    fake_convenience.evaluate_ml_once = _ml_eval
    fake_convenience.evaluate_genai_once = _genai_eval

    fake_native = types.ModuleType("neuralbudget.neuralbudget")
    fake_native.CompositeServiceSlo = _FakeCompositeService
    fake_native.CompositeDependencyEdge = _FakeCompositeEdge
    fake_native.CompositeSloGraph = _FakeCompositeGraph

    def _evaluate_composite(graph):
        return _EvalResult(
            {
                "services": len(graph.services),
                "dependencies": len(graph.dependencies),
                "global_min_pass_score": graph.global_min_pass_score,
            }
        )

    fake_native.evaluate_composite_slo_graph = _evaluate_composite

    sys.modules["neuralbudget"] = fake_pkg
    sys.modules["neuralbudget.convenience"] = fake_convenience
    sys.modules["neuralbudget.neuralbudget"] = fake_native

    spec = importlib.util.spec_from_file_location("neuralbudget.client", client_path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["neuralbudget.client"] = module
    spec.loader.exec_module(module)
    return module


class NeuralBudgetClientTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.client_module = _load_client_module()

    def test_evaluate_requires_loaded_config(self):
        client = self.client_module.NeuralBudgetClient()
        with self.assertRaises(RuntimeError):
            client.evaluate({"timestamp": 1})

    def test_load_json_and_evaluate_http(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "mode": "http",
                        "profile": "strict_latency",
                        "return_dataclass": False,
                        "params": {"latency_threshold_ms": 240.0},
                    }
                ),
                encoding="utf-8",
            )

            client = self.client_module.NeuralBudgetClient().load_config(path)
            result = client.evaluate(
                {
                    "timestamp": 1,
                    "success": 990,
                    "total": 1000,
                    "buckets": [(100.0, 900), (200.0, 1000)],
                    "format": "prometheus_cumulative",
                }
            )

            self.assertEqual(result["mode"], "http")
            self.assertEqual(result["kwargs"]["profile"], "strict_latency")
            self.assertEqual(result["kwargs"]["latency_threshold_ms"], 240.0)

    def test_load_yaml_and_evaluate_composite(self):
        fake_yaml = types.ModuleType("yaml")
        fake_yaml.safe_load = lambda _: {
            "schema_version": 1,
            "mode": "composite",
            "params": {"global_min_pass_score": 0.88},
        }
        sys.modules["yaml"] = fake_yaml

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.yaml"
            path.write_text("mode: composite\n", encoding="utf-8")

            client = self.client_module.NeuralBudgetClient().load_config(path)
            result = client.evaluate(
                {
                    "services": [
                        {
                            "service": "a",
                            "local_score": 0.95,
                            "min_pass_score": 0.9,
                            "impact_weight": 1.0,
                        },
                        {
                            "service": "b",
                            "local_score": 0.96,
                            "min_pass_score": 0.9,
                            "impact_weight": 1.0,
                        },
                    ],
                    "dependencies": [
                        {"dependency": "a", "dependent": "b", "failure_penalty": 0.2}
                    ],
                }
            )

            self.assertEqual(result["services"], 2)
            self.assertEqual(result["dependencies"], 1)
            self.assertEqual(result["global_min_pass_score"], 0.88)

    def test_load_config_defaults_schema_version_to_one(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.json"
            path.write_text(
                json.dumps(
                    {
                        "mode": "http",
                        "params": {"latency_threshold_ms": 120.0},
                    }
                ),
                encoding="utf-8",
            )

            client = self.client_module.NeuralBudgetClient().load_config(path)
            self.assertIsNotNone(client.config)
            assert client.config is not None
            self.assertEqual(client.config.schema_version, 1)

    def test_load_config_rejects_unsupported_schema_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 99,
                        "mode": "http",
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "Unsupported schema_version"):
                self.client_module.NeuralBudgetClient().load_config(path)

    def test_load_config_rejects_missing_required_mode(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.json"
            path.write_text(json.dumps({"schema_version": 1}), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "missing required key 'mode'"):
                self.client_module.NeuralBudgetClient().load_config(path)

    def test_load_config_rejects_unknown_keys(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "mode": "http",
                        "unexpected": True,
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "unknown keys"):
                self.client_module.NeuralBudgetClient().load_config(path)

    def test_load_config_rejects_non_object_params(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "slo.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "mode": "http",
                        "params": ["not", "a", "dict"],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "params must be an object/map"):
                self.client_module.NeuralBudgetClient().load_config(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
