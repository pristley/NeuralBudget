"""Typed high-level facade for notebook and pipeline-first workflows.

`NeuralBudgetClient` provides a stable entrypoint:

- `load_config("slo.yaml")`
- `evaluate(metric_data)`

The client delegates to the convenience and native APIs while keeping the
runtime behavior explicit and autocomplete-friendly.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal, TypedDict, cast

from . import convenience
from . import neuralbudget as _native

EvaluationMode = Literal["http", "stateful", "ml", "genai", "composite"]


class ClientConfigFile(TypedDict, total=False):
    """Serialized config schema accepted by NeuralBudgetClient.load_config."""

    schema_version: int
    mode: EvaluationMode
    profile: str
    return_dataclass: bool
    params: dict[str, Any]


class CompositeServiceInput(TypedDict):
    """Service node payload for composite DAG evaluation."""

    service: str
    local_score: float
    min_pass_score: float
    impact_weight: float


class CompositeDependencyInput(TypedDict):
    """Edge payload describing dependency impact in a composite DAG."""

    dependency: str
    dependent: str
    failure_penalty: float


class CompositeMetricData(TypedDict):
    """Input payload for composite mode evaluate calls."""

    services: list[CompositeServiceInput]
    dependencies: list[CompositeDependencyInput]
    global_min_pass_score: float


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
    return_dataclass: bool = False
    params: dict[str, Any] | None = None


class NeuralBudgetClient:
    """Notebook and CI/CD friendly facade for SLO evaluation.

    Typical usage:

    1. `client.load_config("slo.yaml")`
    2. `result = client.evaluate(metric_data)`
    """

    CONFIG_SCHEMA_VERSION = 1
    SUPPORTED_CONFIG_SCHEMA_VERSIONS = {CONFIG_SCHEMA_VERSION}

    def __init__(self, config: NeuralBudgetClientConfig | None = None) -> None:
        self._config: NeuralBudgetClientConfig | None = config

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
        - `return_dataclass`: optional bool for convenience-layer modes
        - `params`: optional map of keyword overrides
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
        return_dataclass = bool(raw.get("return_dataclass", False))
        schema_version = int(raw["schema_version"])

        self._config = NeuralBudgetClientConfig(
            schema_version=schema_version,
            mode=mode,
            profile=str(profile) if profile is not None else None,
            return_dataclass=return_dataclass,
            params=params,
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
            return convenience.evaluate_http_histogram_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                return_dataclass=config.return_dataclass,
                **params,
            )

        if mode == "stateful":
            return convenience.evaluate_stateful_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                return_dataclass=config.return_dataclass,
                **params,
            )

        if mode == "ml":
            return convenience.evaluate_ml_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                return_dataclass=config.return_dataclass,
                **params,
            )

        if mode == "genai":
            return convenience.evaluate_genai_once(
                cast(dict[str, Any], metric_data),
                profile=config.profile,
                return_dataclass=config.return_dataclass,
                **params,
            )

        return self._evaluate_composite(cast(dict[str, Any], metric_data), params)

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
    def _read_config_file(path: Path) -> ClientConfigFile:
        """Load JSON or YAML config and normalize to a ClientConfigFile dict."""
        suffix = path.suffix.lower()
        if suffix == ".json":
            with path.open("r", encoding="utf-8") as handle:
                data = json.load(handle)
                return cast(ClientConfigFile, data)

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
                return cast(ClientConfigFile, data or {})

        raise ValueError("Unsupported config extension. Use .json, .yaml, or .yml")

    @classmethod
    def _validate_config_schema(cls, raw: ClientConfigFile) -> ClientConfigFile:
        """Validate top-level client config schema and version compatibility."""
        if not isinstance(raw, dict):
            raise ValueError("Invalid config: expected a JSON/YAML object at the top level")

        allowed_keys = {
            "schema_version",
            "mode",
            "profile",
            "return_dataclass",
            "params",
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

        return_dataclass = raw.get("return_dataclass", False)
        if not isinstance(return_dataclass, bool):
            raise ValueError("Invalid config: return_dataclass must be a boolean")

        params = raw.get("params", {})
        if params is None:
            params = {}
        if not isinstance(params, dict):
            raise ValueError("Invalid config: params must be an object/map")

        normalized: ClientConfigFile = {
            "schema_version": schema_version,
            "mode": cast(EvaluationMode, mode),
            "return_dataclass": return_dataclass,
            "params": cast(dict[str, Any], params),
        }
        if profile is not None:
            normalized["profile"] = profile
        return normalized
