"""Webhook alert dispatchers for SLO violations.

Supported providers:
- Slack incoming webhooks
- PagerDuty Events API v2
- Opsgenie Alerts API v2
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Mapping
from urllib import error, request


@dataclass(frozen=True)
class AlertDispatchResult:
    provider: str
    ok: bool
    status_code: int | None = None
    error: str | None = None


@dataclass(frozen=True)
class AlertDispatchSummary:
    attempted: int
    succeeded: int
    failed: int
    results: list[AlertDispatchResult]


class AlertDispatcher:
    """Dispatch violation notifications to configured webhook providers."""

    def send_violation(
        self,
        *,
        mode: str,
        profile: str | None,
        metric_data: Mapping[str, Any],
        result: Mapping[str, Any],
        alerts_config: Mapping[str, Any],
    ) -> AlertDispatchSummary:
        results: list[AlertDispatchResult] = []

        slack_config = alerts_config.get("slack")
        if isinstance(slack_config, Mapping):
            results.append(
                self._send_slack(
                    config=slack_config,
                    mode=mode,
                    profile=profile,
                    result=result,
                )
            )

        pagerduty_config = alerts_config.get("pagerduty")
        if isinstance(pagerduty_config, Mapping):
            results.append(
                self._send_pagerduty(
                    config=pagerduty_config,
                    mode=mode,
                    profile=profile,
                    metric_data=metric_data,
                    result=result,
                )
            )

        opsgenie_config = alerts_config.get("opsgenie")
        if isinstance(opsgenie_config, Mapping):
            results.append(
                self._send_opsgenie(
                    config=opsgenie_config,
                    mode=mode,
                    profile=profile,
                    metric_data=metric_data,
                    result=result,
                )
            )

        attempted = len(results)
        succeeded = sum(1 for entry in results if entry.ok)
        failed = attempted - succeeded
        return AlertDispatchSummary(
            attempted=attempted,
            succeeded=succeeded,
            failed=failed,
            results=results,
        )

    def _send_slack(
        self,
        *,
        config: Mapping[str, Any],
        mode: str,
        profile: str | None,
        result: Mapping[str, Any],
    ) -> AlertDispatchResult:
        webhook_url = str(config.get("webhook_url", "")).strip()
        if not webhook_url:
            return AlertDispatchResult(
                provider="slack",
                ok=False,
                error="missing slack.webhook_url",
            )

        profile_label = profile if profile else "default"
        text = (
            f"NeuralBudget SLO violation detected: mode={mode}, "
            f"profile={profile_label}, result={json.dumps(result, sort_keys=True)}"
        )
        payload = {"text": text}
        return self._post_json(
            provider="slack",
            url=webhook_url,
            payload=payload,
            timeout_seconds=self._timeout_seconds(config),
            headers={},
        )

    def _send_pagerduty(
        self,
        *,
        config: Mapping[str, Any],
        mode: str,
        profile: str | None,
        metric_data: Mapping[str, Any],
        result: Mapping[str, Any],
    ) -> AlertDispatchResult:
        routing_key = str(config.get("routing_key", "")).strip()
        if not routing_key:
            return AlertDispatchResult(
                provider="pagerduty",
                ok=False,
                error="missing pagerduty.routing_key",
            )

        event_action = str(config.get("event_action", "trigger"))
        dedup_key = str(config.get("dedup_key", "")).strip() or None
        source = str(config.get("source", "neuralbudget"))
        severity = str(config.get("severity", "error"))
        component = str(config.get("component", mode))
        group = str(config.get("group", "slo"))
        class_name = str(config.get("class", "slo_violation"))

        profile_label = profile if profile else "default"
        payload = {
            "routing_key": routing_key,
            "event_action": event_action,
            "payload": {
                "summary": (
                    "NeuralBudget SLO violation "
                    f"(mode={mode}, profile={profile_label})"
                ),
                "source": source,
                "severity": severity,
                "component": component,
                "group": group,
                "class": class_name,
                "custom_details": {
                    "mode": mode,
                    "profile": profile,
                    "metric_data": dict(metric_data),
                    "result": dict(result),
                },
            },
        }
        if dedup_key is not None:
            payload["dedup_key"] = dedup_key

        return self._post_json(
            provider="pagerduty",
            url=str(
                config.get(
                    "events_url",
                    "https://events.pagerduty.com/v2/enqueue",
                )
            ),
            payload=payload,
            timeout_seconds=self._timeout_seconds(config),
            headers={},
        )

    def _send_opsgenie(
        self,
        *,
        config: Mapping[str, Any],
        mode: str,
        profile: str | None,
        metric_data: Mapping[str, Any],
        result: Mapping[str, Any],
    ) -> AlertDispatchResult:
        api_key = str(config.get("api_key", "")).strip()
        if not api_key:
            return AlertDispatchResult(
                provider="opsgenie",
                ok=False,
                error="missing opsgenie.api_key",
            )

        api_url = str(config.get("api_url", "https://api.opsgenie.com/v2/alerts"))
        priority = str(config.get("priority", "P3"))
        source = str(config.get("source", "neuralbudget"))
        tags = config.get("tags", ["neuralbudget", mode, "slo-violation"])
        if not isinstance(tags, list):
            tags = ["neuralbudget", mode, "slo-violation"]

        profile_label = profile if profile else "default"
        payload = {
            "message": f"NeuralBudget SLO violation ({mode}/{profile_label})",
            "description": json.dumps(
                {
                    "mode": mode,
                    "profile": profile,
                    "metric_data": dict(metric_data),
                    "result": dict(result),
                },
                sort_keys=True,
            ),
            "source": source,
            "priority": priority,
            "tags": tags,
            "details": {
                "mode": mode,
                "profile": profile_label,
            },
        }

        return self._post_json(
            provider="opsgenie",
            url=api_url,
            payload=payload,
            timeout_seconds=self._timeout_seconds(config),
            headers={"Authorization": f"GenieKey {api_key}"},
        )

    @staticmethod
    def _timeout_seconds(config: Mapping[str, Any]) -> float:
        raw = config.get("timeout_seconds", 5.0)
        try:
            timeout = float(raw)
        except (TypeError, ValueError):
            return 5.0
        if timeout <= 0:
            return 5.0
        return timeout

    @staticmethod
    def _post_json(
        *,
        provider: str,
        url: str,
        payload: Mapping[str, Any],
        timeout_seconds: float,
        headers: Mapping[str, str],
    ) -> AlertDispatchResult:
        body = json.dumps(payload).encode("utf-8")
        req_headers = {
            "Content-Type": "application/json",
            "User-Agent": "neuralbudget-alerting/1",
        }
        req_headers.update(dict(headers))
        req = request.Request(url=url, data=body, headers=req_headers, method="POST")

        try:
            with request.urlopen(req, timeout=timeout_seconds) as response:
                status = int(getattr(response, "status", 200))
                if 200 <= status < 300:
                    return AlertDispatchResult(provider=provider, ok=True, status_code=status)
                return AlertDispatchResult(
                    provider=provider,
                    ok=False,
                    status_code=status,
                    error=f"unexpected status code: {status}",
                )
        except error.HTTPError as exc:
            return AlertDispatchResult(
                provider=provider,
                ok=False,
                status_code=int(exc.code),
                error=str(exc),
            )
        except Exception as exc:  # pragma: no cover - defensive catch-all
            return AlertDispatchResult(provider=provider, ok=False, error=str(exc))
