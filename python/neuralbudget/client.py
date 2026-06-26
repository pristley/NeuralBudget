"""Typed high-level facade for notebook and pipeline-first workflows.

`NeuralBudgetClient` provides a stable entrypoint:

- `load_config("slo.yaml")`
- `evaluate(metric_data)`

The client delegates to the convenience and native APIs while keeping the
runtime behavior explicit and autocomplete-friendly.
"""

from __future__ import annotations

import json
import warnings
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal, cast

from .alerting import AlertDispatcher
from . import convenience
from . import neuralbudget as _native

EvaluationMode = Literal["http", "stateful", "ml", "genai", "composite"]


@dataclass(frozen=True)
class CompositeServiceInput:
    """Service node payload for composite DAG evaluation."""

    service: str
    local_score: float
    min_pass_score: float
    impact_weight: float


@dataclass(frozen=True)
class CompositeDependencyInput:
    """Edge payload describing dependency impact in a composite DAG."""

    dependency: str
    dependent: str
    failure_penalty: float


@dataclass(frozen=True)
class CompositeMetricData:
    """Input payload for composite mode evaluate calls."""

    services: list[CompositeServiceInput]
    dependencies: list[CompositeDependencyInput]
    global_min_pass_score: float


@dataclass(frozen=True)
class ClientConfigFile:
    """Serialized config schema accepted by NeuralBudgetClient.load_config."""

    mode: EvaluationMode
    schema_version: int = 1
    profile: str | None = None
    params: dict[str, Any] = field(default_factory=dict)
    alerts: dict[str, Any] = field(default_factory=dict)


MetricData = dict[str, Any] | CompositeMetricData
EvaluationResult = (
    dict[str, Any]
    | convenience.HttpHistogramEvaluationResult
    | convenience.StatefulEvaluationResult
    | convenience.MlEvaluationResult
    | convenience.GenAiEvaluationResult
    | Any
)


@dataclass(frozen=True)
class NeuralBudgetClientConfig:
    mode: EvaluationMode
    schema_version: int = 1
    profile: str | None = None
    params: dict[str, Any] | None = None
    alerts: dict[str, Any] | None = None


class NeuralBudgetClient:
    """Notebook and CI/CD friendly facade for SLO evaluation.

    Typical usage:

    1. `client.load_config("slo.yaml")`
    2. `result = client.evaluate(metric_data)`
    """

    CONFIG_SCHEMA_VERSION = 1
    SUPPORTED_CONFIG_SCHEMA_VERSIONS = {CONFIG_SCHEMA_VERSION}

    def __init__(
        self,
        config: NeuralBudgetClientConfig | None = None,
        alert_dispatcher: AlertDispatcher | None = None,
    ) -> None:
        self._config: NeuralBudgetClientConfig | None = config
        self._alert_dispatcher = alert_dispatcher or AlertDispatcher()

    @property
    def config(self) -> NeuralBudgetClientConfig | None:
        """Return the currently loaded client configuration."""
        return self._config

    def load_config(self, path: str | Path) -> NeuralBudgetClient:
        """Load client config from `.json`, `.yaml`, or `.yml`.

        Supported keys:

        - `schema_version`: optional int, defaults to `1`
        - `mode`: `http | stateful | ml | genai | composite`
        - `profile`: optional named profile for non-composite modes
        - `params`: optional map of keyword overrides
        - `alerts`: optional map configuring Slack/PagerDuty/Opsgenie providers
        """
        config_path = Path(path)
        raw = self._read_config_file(config_path)
        raw = self._validate_config_schema(raw)

        mode = cast(EvaluationMode, raw["mode"])
        if mode not in {"http", "stateful", "ml", "genai", "composite"}:
            raise ValueError(
                "invalid mode in config. expected one of: "
                "http, stateful, ml, genai, composite"
            )

        params = dict(raw.get("params", {}))
        profile = raw.get("profile")
        alerts = raw.get("alerts")
        schema_version = int(raw["schema_version"])

        self._config = NeuralBudgetClientConfig(
            schema_version=schema_version,
            mode=mode,
            profile=str(profile) if profile is not None else None,
            params=params,
            alerts=dict(alerts or {}),
        )
        return self

    def evaluate(self, metric_data: MetricData) -> EvaluationResult:
        """Evaluate metric payload using the loaded client configuration.

        Input schema depends on configured mode:

        - http: histogram sample dict
        - stateful: stateful sample dict
        - ml: ml-serving sample dict
        - genai: genai sample dict
        - composite: services/dependencies payload
        """
        config = self._config
        if config is None:
            raise RuntimeError("No config loaded. Call load_config(path) first.")

        params = dict(config.params or {})
        mode = config.mode

        if mode == "http":
            result = convenience.evaluate_http_histogram_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                **params,
            )
            self._send_violation_alert_if_needed(
                mode=mode,
                profile=config.profile,
                metric_data=cast(dict[str, Any], metric_data),
                result=result,
                alerts=config.alerts,
            )
            return result

        if mode == "stateful":
            result = convenience.evaluate_stateful_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                **params,
            )
            self._send_violation_alert_if_needed(
                mode=mode,
                profile=config.profile,
                metric_data=cast(dict[str, Any], metric_data),
                result=result,
                alerts=config.alerts,
            )
            return result

        if mode == "ml":
            result = convenience.evaluate_ml_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                **params,
            )
            self._send_violation_alert_if_needed(
                mode=mode,
                profile=config.profile,
                metric_data=cast(dict[str, Any], metric_data),
                result=result,
                alerts=config.alerts,
            )
            return result

        if mode == "genai":
            result = convenience.evaluate_genai_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                **params,
            )
            self._send_violation_alert_if_needed(
                mode=mode,
                profile=config.profile,
                metric_data=cast(dict[str, Any], metric_data),
                result=result,
                alerts=config.alerts,
            )
            return result

        result = self._evaluate_composite(cast(dict[str, Any], metric_data), params)
        self._send_violation_alert_if_needed(
            mode=mode,
            profile=config.profile,
            metric_data=cast(dict[str, Any], metric_data),
            result=result,
            alerts=config.alerts,
        )
        return result

    def _send_violation_alert_if_needed(
        self,
        *,
        mode: str,
        profile: str | None,
        metric_data: dict[str, Any],
        result: EvaluationResult,
        alerts: dict[str, Any] | None,
    ) -> None:
        if not isinstance(alerts, dict) or not alerts:
            return

        if self._coerce_bool(alerts.get("enabled"), default=True) is False:
            return

        only_on_violation = self._coerce_bool(alerts.get("on_violation"), default=True)
        is_violation = self._is_violation_result(result)
        if only_on_violation and not is_violation:
            return

        summary = self._alert_dispatcher.send_violation(
            mode=mode,
            profile=profile,
            metric_data=metric_data,
            result=self._result_to_alert_payload(result),
            alerts_config=alerts,
        )

        fail_open = self._coerce_bool(alerts.get("fail_open"), default=True)
        if summary.failed > 0 and not fail_open:
            raise RuntimeError(
                "SLO violation alert dispatch failed for one or more providers"
            )
        if summary.failed > 0:
            warnings.warn(
                "SLO violation alert dispatch failed for one or more providers",
                RuntimeWarning,
                stacklevel=2,
            )

    @staticmethod
    def _is_violation_result(result: EvaluationResult) -> bool:
        if isinstance(result, dict):
            if "global_pass" in result:
                return not bool(result["global_pass"])
            if "pass" in result:
                return not bool(result["pass"])
            return False

        if hasattr(result, "passed"):
            return not bool(getattr(result, "passed"))
        if hasattr(result, "pass"):
            return not bool(getattr(result, "pass"))
        return False

    @staticmethod
    def _result_to_alert_payload(result: EvaluationResult) -> dict[str, Any]:
        if isinstance(result, dict):
            return dict(result)
        if hasattr(result, "to_dict"):
            value = result.to_dict()
            if isinstance(value, dict):
                return value
        if hasattr(result, "__dict__"):
            value = vars(result)
            if isinstance(value, dict):
                return dict(value)
        return {"value": str(result)}

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

    def _evaluate_composite(self, metric_data: dict[str, Any], params: dict[str, Any]) -> Any:
        """Map dict payload into native composite graph objects and evaluate."""
        services_raw = metric_data.get("services", [])
        dependencies_raw = metric_data.get("dependencies", [])
        global_min_pass_score = float(
            metric_data.get(
                "global_min_pass_score",
                params.get("global_min_pass_score", 0.9),
            )
        )

        services = [
            _native.CompositeServiceSlo(
                str(entry["service"]),
                float(entry["local_score"]),
                float(entry["min_pass_score"]),
                float(entry["impact_weight"]),
            )
            for entry in services_raw
        ]
        dependencies = [
            _native.CompositeDependencyEdge(
                str(entry["dependency"]),
                str(entry["dependent"]),
                float(entry["failure_penalty"]),
            )
            for entry in dependencies_raw
        ]

        graph = _native.CompositeSloGraph(
            services=services,
            dependencies=dependencies,
            global_min_pass_score=global_min_pass_score,
        )
        evaluation = _native.evaluate_composite_slo_graph(graph)
        if hasattr(evaluation, "to_dict"):
            return evaluation.to_dict()
        return evaluation

    @staticmethod
    def _read_config_file(path: Path) -> dict[str, Any]:
        """Load JSON or YAML config and return as dict."""
        suffix = path.suffix.lower()
        if suffix == ".json":
            with path.open("r", encoding="utf-8") as handle:
                data = json.load(handle)
                return data if isinstance(data, dict) else {}

        if suffix in {".yaml", ".yml"}:
            try:
                import yaml  # type: ignore[import-not-found]
            except ImportError as exc:
                raise RuntimeError(
                    "PyYAML is required to load YAML config files. "
                    "Install with: pip install pyyaml"
                ) from exc
            with path.open("r", encoding="utf-8") as handle:
                data = yaml.safe_load(handle)  # type: ignore[attr-defined]
                return data if isinstance(data, dict) else {}

        raise ValueError("Unsupported config extension. Use .json, .yaml, or .yml")

    @classmethod
    def _validate_config_schema(cls, raw: dict[str, Any]) -> dict[str, Any]:
        """Validate top-level client config schema and version compatibility."""
        if not isinstance(raw, dict):
            raise ValueError("Invalid config: expected a JSON/YAML object at the top level")

        allowed_keys = {
            "schema_version",
            "mode",
            "profile",
            "params",
            "alerts",
        }
        unknown_keys = sorted(set(raw.keys()) - allowed_keys)
        if unknown_keys:
            raise ValueError(
                "Invalid config: unknown keys: " + ", ".join(unknown_keys)
            )

        schema_version = raw.get("schema_version", cls.CONFIG_SCHEMA_VERSION)
        if not isinstance(schema_version, int):
            raise ValueError("Invalid config: schema_version must be an integer")
        if schema_version not in cls.SUPPORTED_CONFIG_SCHEMA_VERSIONS:
            supported = ", ".join(str(v) for v in sorted(cls.SUPPORTED_CONFIG_SCHEMA_VERSIONS))
            raise ValueError(
                f"Unsupported schema_version: {schema_version}. Supported: {supported}"
            )

        if "mode" not in raw:
            raise ValueError("Invalid config: missing required key 'mode'")

        mode = raw.get("mode")
        if not isinstance(mode, str):
            raise ValueError("Invalid config: mode must be a string")

        profile = raw.get("profile")
        if profile is not None and not isinstance(profile, str):
            raise ValueError("Invalid config: profile must be a string when provided")

        params = raw.get("params", {})
        if params is None:
            params = {}
        if not isinstance(params, dict):
            raise ValueError("Invalid config: params must be an object/map")

        alerts = raw.get("alerts", {})
        if alerts is None:
            alerts = {}
        if not isinstance(alerts, dict):
            raise ValueError("Invalid config: alerts must be an object/map")

        normalized: ClientConfigFile = {
            "schema_version": schema_version,
            "mode": cast(EvaluationMode, mode),
            "params": cast(dict[str, Any], params),
            "alerts": cast(dict[str, Any], alerts),
        }
        if profile is not None:
            normalized["profile"] = profile
        return normalized
