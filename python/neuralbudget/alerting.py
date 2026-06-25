"""Webhook alert dispatchers for SLO violations.

Supported providers:
- Slack incoming webhooks
- PagerDuty Events API v2
- Opsgenie Alerts API v2
"""

from __future__ import annotations

import ipaddress
import json
import os
from dataclasses import dataclass
from typing import Any, Mapping
from urllib import error, request
from urllib.parse import urlparse


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

    MAX_PAYLOAD_BYTES = 64 * 1024

    _LOCAL_HOSTNAMES = {
        "localhost",
        "localhost.localdomain",
    }

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

        url_error = self._validate_url(
            provider="slack",
            url=webhook_url,
            allow_insecure_http=self._coerce_bool(config.get("allow_insecure_http"), False),
            allow_private_network=self._coerce_bool(
                config.get("allow_private_network"),
                False,
            ),
        )
        if url_error is not None:
            return AlertDispatchResult(provider="slack", ok=False, error=url_error)

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
        routing_key = self._resolve_secret(config.get("routing_key", ""))
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

        events_url = str(
            config.get(
                "events_url",
                "https://events.pagerduty.com/v2/enqueue",
            )
        )
        url_error = self._validate_url(
            provider="pagerduty",
            url=events_url,
            allow_insecure_http=self._coerce_bool(config.get("allow_insecure_http"), False),
            allow_private_network=self._coerce_bool(
                config.get("allow_private_network"),
                False,
            ),
        )
        if url_error is not None:
            return AlertDispatchResult(provider="pagerduty", ok=False, error=url_error)

        return self._post_json(
            provider="pagerduty",
            url=events_url,
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
        api_key = self._resolve_secret(config.get("api_key", ""))
        if not api_key:
            return AlertDispatchResult(
                provider="opsgenie",
                ok=False,
                error="missing opsgenie.api_key",
            )

        api_url = str(config.get("api_url", "https://api.opsgenie.com/v2/alerts"))
        url_error = self._validate_url(
            provider="opsgenie",
            url=api_url,
            allow_insecure_http=self._coerce_bool(config.get("allow_insecure_http"), False),
            allow_private_network=self._coerce_bool(
                config.get("allow_private_network"),
                False,
            ),
        )
        if url_error is not None:
            return AlertDispatchResult(provider="opsgenie", ok=False, error=url_error)

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
        # Keep requests bounded by default to reduce DoS blast radius.
        if timeout > 30.0:
            return 30.0
        return timeout

    @classmethod
    def _validate_url(
        cls,
        *,
        provider: str,
        url: str,
        allow_insecure_http: bool,
        allow_private_network: bool,
    ) -> str | None:
        parsed = urlparse(url)
        scheme = parsed.scheme.lower()
        host = parsed.hostname

        if scheme not in {"https", "http"}:
            return f"invalid {provider} url: unsupported scheme"
        if scheme != "https" and not allow_insecure_http:
            return f"invalid {provider} url: https is required"
        if host is None:
            return f"invalid {provider} url: missing host"

        lowered_host = host.lower()
        if not allow_private_network:
            if lowered_host in cls._LOCAL_HOSTNAMES or lowered_host.endswith(".local"):
                return f"invalid {provider} url: local/private hosts are blocked"

            try:
                ip = ipaddress.ip_address(host)
                if (
                    ip.is_private
                    or ip.is_loopback
                    or ip.is_link_local
                    or ip.is_reserved
                    or ip.is_multicast
                    or ip.is_unspecified
                ):
                    return f"invalid {provider} url: local/private hosts are blocked"
            except ValueError:
                # Non-IP hostnames are allowed.
                pass

        return None

    @staticmethod
    def _coerce_bool(value: Any, default: bool) -> bool:
        if value is None:
            return default
        if isinstance(value, bool):
            return value
        if isinstance(value, str):
            normalized = value.strip().lower()
            if normalized in {"true", "1", "yes", "y", "on"}:
                return True
            if normalized in {"false", "0", "no", "n", "off"}:
                return False
            return default
        if isinstance(value, (int, float)):
            if value == 1:
                return True
            if value == 0:
                return False
            return default
        return default

    @staticmethod
    def _resolve_secret(raw: Any) -> str:
        value = str(raw).strip()
        if not value:
            return ""
        if value.startswith("env:"):
            env_key = value[4:].strip()
            if not env_key:
                return ""
            return str(os.getenv(env_key, "")).strip()
        return value

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
        if len(body) > AlertDispatcher.MAX_PAYLOAD_BYTES:
            return AlertDispatchResult(
                provider=provider,
                ok=False,
                error="payload too large",
            )

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
                error=f"http error: {int(exc.code)}",
            )
        except Exception as exc:  # pragma: no cover - defensive catch-all
            return AlertDispatchResult(
                provider=provider,
                ok=False,
                error=f"transport error: {type(exc).__name__}",
            )
