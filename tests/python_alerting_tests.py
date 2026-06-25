#!/usr/bin/env python3

"""Unit tests for webhook payload formatting in alerting dispatchers."""

from __future__ import annotations

import importlib.util
import json
import os
import sys
from urllib import error
import unittest
from pathlib import Path
from unittest import mock


class _FakeResponse:
    def __init__(self, status: int = 202):
        self.status = status

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return False


def _load_alerting_module():
    repo_root = Path(__file__).resolve().parents[1]
    alerting_path = repo_root / "python" / "neuralbudget" / "alerting.py"

    spec = importlib.util.spec_from_file_location("neuralbudget.alerting", alerting_path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["neuralbudget.alerting"] = module
    spec.loader.exec_module(module)
    return module


class AlertingPayloadTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.alerting_module = _load_alerting_module()

    def test_slack_payload_shape(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        captured: dict[str, object] = {}

        def _fake_urlopen(req, timeout=0):
            captured["url"] = req.full_url
            captured["timeout"] = timeout
            captured["headers"] = dict(req.headers)
            captured["body"] = req.data.decode("utf-8")
            return _FakeResponse(status=200)

        with mock.patch.object(
            self.alerting_module.request,
            "urlopen",
            side_effect=_fake_urlopen,
        ):
            summary = dispatcher.send_violation(
                mode="http",
                profile="strict_latency",
                metric_data={"timestamp": 1},
                result={"pass": False, "availability": 0.98},
                alerts_config={
                    "slack": {
                        "webhook_url": "https://hooks.slack.com/services/T000/B000/XXX",
                        "timeout_seconds": 3,
                    }
                },
            )

        self.assertEqual(summary.attempted, 1)
        self.assertEqual(summary.succeeded, 1)
        self.assertEqual(summary.failed, 0)
        self.assertEqual(captured["url"], "https://hooks.slack.com/services/T000/B000/XXX")
        self.assertEqual(captured["timeout"], 3.0)

        payload = json.loads(str(captured["body"]))
        self.assertIn("text", payload)
        self.assertEqual(
            payload["text"],
            (
                "NeuralBudget SLO violation detected: mode=http, "
                "profile=strict_latency, result={\"availability\": 0.98, \"pass\": false}"
            ),
        )

    def test_pagerduty_payload_shape(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        captured: dict[str, object] = {}

        def _fake_urlopen(req, timeout=0):
            captured["url"] = req.full_url
            captured["timeout"] = timeout
            captured["headers"] = dict(req.headers)
            captured["body"] = req.data.decode("utf-8")
            return _FakeResponse(status=202)

        with mock.patch.object(
            self.alerting_module.request,
            "urlopen",
            side_effect=_fake_urlopen,
        ):
            summary = dispatcher.send_violation(
                mode="ml",
                profile="drift_sensitive",
                metric_data={"timestamp": 2, "feature_drift": 0.31},
                result={"pass": False, "hybrid_score": 0.81},
                alerts_config={
                    "pagerduty": {
                        "routing_key": "pd-routing-key",
                        "event_action": "trigger",
                        "dedup_key": "ml-prod",
                        "severity": "critical",
                        "source": "ml-serving",
                        "component": "inference-api",
                        "group": "reliability",
                        "class": "slo_violation",
                        "timeout_seconds": 4,
                    }
                },
            )

        self.assertEqual(summary.attempted, 1)
        self.assertEqual(summary.succeeded, 1)
        self.assertEqual(summary.failed, 0)
        self.assertEqual(captured["url"], "https://events.pagerduty.com/v2/enqueue")
        self.assertEqual(captured["timeout"], 4.0)

        payload = json.loads(str(captured["body"]))
        self.assertEqual(payload["routing_key"], "pd-routing-key")
        self.assertEqual(payload["event_action"], "trigger")
        self.assertEqual(payload["dedup_key"], "ml-prod")

        pd_payload = payload["payload"]
        self.assertEqual(
            pd_payload["summary"],
            "NeuralBudget SLO violation (mode=ml, profile=drift_sensitive)",
        )
        self.assertEqual(pd_payload["source"], "ml-serving")
        self.assertEqual(pd_payload["severity"], "critical")
        self.assertEqual(pd_payload["component"], "inference-api")
        self.assertEqual(pd_payload["group"], "reliability")
        self.assertEqual(pd_payload["class"], "slo_violation")
        self.assertEqual(
            pd_payload["custom_details"],
            {
                "mode": "ml",
                "profile": "drift_sensitive",
                "metric_data": {"timestamp": 2, "feature_drift": 0.31},
                "result": {"pass": False, "hybrid_score": 0.81},
            },
        )

    def test_opsgenie_payload_and_auth_header(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        captured: dict[str, object] = {}

        def _fake_urlopen(req, timeout=0):
            captured["url"] = req.full_url
            captured["timeout"] = timeout
            captured["headers"] = dict(req.headers)
            captured["auth"] = req.headers.get("Authorization")
            captured["body"] = req.data.decode("utf-8")
            return _FakeResponse(status=202)

        with mock.patch.object(
            self.alerting_module.request,
            "urlopen",
            side_effect=_fake_urlopen,
        ):
            summary = dispatcher.send_violation(
                mode="genai",
                profile="quality_first",
                metric_data={"timestamp": 3, "tokens_generated": 120},
                result={"pass": False, "semantic_similarity": 0.74},
                alerts_config={
                    "opsgenie": {
                        "api_key": "og-api-key",
                        "api_url": "https://api.opsgenie.com/v2/alerts",
                        "priority": "P1",
                        "source": "genai-gateway",
                        "tags": ["neuralbudget", "genai", "slo-violation"],
                        "timeout_seconds": 6,
                    }
                },
            )

        self.assertEqual(summary.attempted, 1)
        self.assertEqual(summary.succeeded, 1)
        self.assertEqual(summary.failed, 0)
        self.assertEqual(captured["url"], "https://api.opsgenie.com/v2/alerts")
        self.assertEqual(captured["timeout"], 6.0)
        self.assertEqual(captured["auth"], "GenieKey og-api-key")

        payload = json.loads(str(captured["body"]))
        self.assertEqual(payload["message"], "NeuralBudget SLO violation (genai/quality_first)")
        self.assertEqual(payload["source"], "genai-gateway")
        self.assertEqual(payload["priority"], "P1")
        self.assertEqual(payload["tags"], ["neuralbudget", "genai", "slo-violation"])
        self.assertEqual(payload["details"], {"mode": "genai", "profile": "quality_first"})

        description = json.loads(payload["description"])
        self.assertEqual(
            description,
            {
                "mode": "genai",
                "profile": "quality_first",
                "metric_data": {"timestamp": 3, "tokens_generated": 120},
                "result": {"pass": False, "semantic_similarity": 0.74},
            },
        )

    def test_non_2xx_response_marks_dispatch_failed(self):
        dispatcher = self.alerting_module.AlertDispatcher()

        def _fake_urlopen(req, timeout=0):
            return _FakeResponse(status=500)

        with mock.patch.object(
            self.alerting_module.request,
            "urlopen",
            side_effect=_fake_urlopen,
        ):
            summary = dispatcher.send_violation(
                mode="http",
                profile="strict_latency",
                metric_data={"timestamp": 1},
                result={"pass": False},
                alerts_config={
                    "slack": {
                        "webhook_url": "https://hooks.slack.com/services/T000/B000/XXX"
                    }
                },
            )

        self.assertEqual(summary.attempted, 1)
        self.assertEqual(summary.succeeded, 0)
        self.assertEqual(summary.failed, 1)
        result = summary.results[0]
        self.assertEqual(result.provider, "slack")
        self.assertFalse(result.ok)
        self.assertEqual(result.status_code, 500)
        self.assertIn("unexpected status code", str(result.error))

    def test_http_error_marks_dispatch_failed(self):
        dispatcher = self.alerting_module.AlertDispatcher()

        def _raise_http_error(req, timeout=0):
            raise error.HTTPError(
                req.full_url,
                429,
                "Too Many Requests",
                hdrs=None,
                fp=None,
            )

        with mock.patch.object(
            self.alerting_module.request,
            "urlopen",
            side_effect=_raise_http_error,
        ):
            summary = dispatcher.send_violation(
                mode="stateful",
                profile="database_primary",
                metric_data={"timestamp": 2},
                result={"pass": False},
                alerts_config={
                    "pagerduty": {
                        "routing_key": "pd-routing-key",
                    }
                },
            )

        self.assertEqual(summary.attempted, 1)
        self.assertEqual(summary.succeeded, 0)
        self.assertEqual(summary.failed, 1)
        result = summary.results[0]
        self.assertEqual(result.provider, "pagerduty")
        self.assertFalse(result.ok)
        self.assertEqual(result.status_code, 429)
        self.assertEqual(result.error, "http error: 429")

    def test_http_scheme_rejected_by_default(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        summary = dispatcher.send_violation(
            mode="http",
            profile="strict_latency",
            metric_data={"timestamp": 1},
            result={"pass": False},
            alerts_config={
                "slack": {
                    "webhook_url": "http://hooks.slack.com/services/T000/B000/XXX"
                }
            },
        )

        self.assertEqual(summary.failed, 1)
        self.assertEqual(summary.results[0].provider, "slack")
        self.assertIn("https is required", str(summary.results[0].error))

    def test_localhost_blocked_by_default(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        summary = dispatcher.send_violation(
            mode="http",
            profile="strict_latency",
            metric_data={"timestamp": 1},
            result={"pass": False},
            alerts_config={
                "slack": {
                    "webhook_url": "https://localhost/webhook"
                }
            },
        )

        self.assertEqual(summary.failed, 1)
        self.assertIn("local/private hosts are blocked", str(summary.results[0].error))

    def test_private_ip_blocked_by_default(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        summary = dispatcher.send_violation(
            mode="http",
            profile="strict_latency",
            metric_data={"timestamp": 1},
            result={"pass": False},
            alerts_config={
                "slack": {
                    "webhook_url": "https://127.0.0.1/webhook"
                }
            },
        )

        self.assertEqual(summary.failed, 1)
        self.assertIn("local/private hosts are blocked", str(summary.results[0].error))

    def test_env_secret_resolution_for_opsgenie_key(self):
        dispatcher = self.alerting_module.AlertDispatcher()
        captured: dict[str, object] = {}

        def _fake_urlopen(req, timeout=0):
            captured["auth"] = req.headers.get("Authorization")
            return _FakeResponse(status=202)

        with mock.patch.dict(os.environ, {"NB_OG_KEY": "secure-key"}, clear=False):
            with mock.patch.object(
                self.alerting_module.request,
                "urlopen",
                side_effect=_fake_urlopen,
            ):
                summary = dispatcher.send_violation(
                    mode="genai",
                    profile="quality_first",
                    metric_data={"timestamp": 1},
                    result={"pass": False},
                    alerts_config={
                        "opsgenie": {
                            "api_key": "env:NB_OG_KEY",
                            "api_url": "https://api.opsgenie.com/v2/alerts",
                        }
                    },
                )

        self.assertEqual(summary.succeeded, 1)
        self.assertEqual(captured["auth"], "GenieKey secure-key")


if __name__ == "__main__":
    unittest.main(verbosity=2)
