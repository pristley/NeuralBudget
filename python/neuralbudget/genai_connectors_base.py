"""GenAI telemetry connectors for NeuralBudget.

Connectors for popular GenAI platforms and inference servers:
- OpenAI API usage and costs
- Anthropic API usage and metrics
- vLLM inference server metrics
- Triton Inference Server metrics

Each connector provides real-time telemetry that feeds into GenAI SLO evaluation.

Usage:
    from neuralbudget.genai_connectors import OpenAIConnector, AnthropicConnector
    
    openai = OpenAIConnector(api_key="sk-...")
    metrics = openai.get_usage_metrics()
    
    anthropic = AnthropicConnector(api_key="ant-...")
    metrics = anthropic.get_usage_metrics()
"""

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Dict, List, Optional
from enum import Enum

logger = logging.getLogger(__name__)


# ============================================================================
# Data Models
# ============================================================================


class ModelType(Enum):
    """GenAI model types."""
    GPT4 = "gpt-4"
    GPT4_TURBO = "gpt-4-turbo"
    GPT35_TURBO = "gpt-3.5-turbo"
    CLAUDE3_OPUS = "claude-3-opus"
    CLAUDE3_SONNET = "claude-3-sonnet"
    CLAUDE3_HAIKU = "claude-3-haiku"
    CLAUDE_INSTANT = "claude-instant"
    LLAMA2 = "llama-2"
    MISTRAL = "mistral"
    OTHER = "other"


@dataclass
class TokenUsage:
    """Token usage metrics."""
    prompt_tokens: int
    completion_tokens: int
    total_tokens: int
    timestamp: str = field(default_factory=lambda: datetime.utcnow().isoformat())


@dataclass
class CostMetrics:
    """Cost metrics."""
    prompt_cost_usd: float  # Cost per 1M tokens
    completion_cost_usd: float  # Cost per 1M tokens
    total_cost_usd: float  # Actual cost for this period
    currency: str = "USD"
    timestamp: str = field(default_factory=lambda: datetime.utcnow().isoformat())


@dataclass
class LatencyMetrics:
    """Latency metrics."""
    mean_latency_ms: float
    p50_latency_ms: float
    p95_latency_ms: float
    p99_latency_ms: float
    max_latency_ms: float
    timestamp: str = field(default_factory=lambda: datetime.utcnow().isoformat())


@dataclass
class ErrorMetrics:
    """Error metrics."""
    error_count: int
    rate_limit_errors: int
    timeout_errors: int
    authentication_errors: int
    server_errors: int
    error_rate_percent: float
    timestamp: str = field(default_factory=lambda: datetime.utcnow().isoformat())


@dataclass
class GenAIMetrics:
    """Complete GenAI metrics snapshot."""
    timestamp: str
    model: str
    provider: str
    tokens: TokenUsage
    costs: CostMetrics
    latency: Optional[LatencyMetrics] = None
    errors: Optional[ErrorMetrics] = None
    request_count: int = 0
    success_count: int = 0
    throughput_rps: float = 0.0
    availability_percent: float = 100.0
    metadata: Dict[str, Any] = field(default_factory=dict)


# ============================================================================
# Base Connector Class
# ============================================================================


class GenAIConnector(ABC):
    """Abstract base class for GenAI telemetry connectors."""

    def __init__(self, name: str, provider: str):
        """Initialize connector.

        Args:
            name: Connector name
            provider: Provider name (OpenAI, Anthropic, etc.)
        """
        self.name = name
        self.provider = provider
        self._last_metrics: Optional[GenAIMetrics] = None
        self._metrics_history: List[GenAIMetrics] = []
        self._max_history = 1000

    @abstractmethod
    def get_usage_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get token usage metrics.

        Args:
            model: Filter by model name
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with token usage data
        """
        pass

    @abstractmethod
    def get_cost_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get cost metrics.

        Args:
            model: Filter by model name
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with cost data
        """
        pass

    @abstractmethod
    def get_latency_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get latency metrics.

        Args:
            model: Filter by model name
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with latency data
        """
        pass

    @abstractmethod
    def get_error_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get error metrics.

        Args:
            model: Filter by model name
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with error data
        """
        pass

    def get_all_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get all metrics combined.

        Args:
            model: Filter by model name
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with all data
        """
        metrics = self.get_usage_metrics(model, hours_back)
        
        # Merge in additional metrics
        cost_metrics = self.get_cost_metrics(model, hours_back)
        metrics.costs = cost_metrics.costs

        try:
            latency_metrics = self.get_latency_metrics(model, hours_back)
            metrics.latency = latency_metrics.latency
        except Exception as e:
            logger.warning(f"Failed to get latency metrics: {e}")

        try:
            error_metrics = self.get_error_metrics(model, hours_back)
            metrics.errors = error_metrics.errors
        except Exception as e:
            logger.warning(f"Failed to get error metrics: {e}")

        self._last_metrics = metrics
        self._metrics_history.append(metrics)
        
        # Keep bounded history
        if len(self._metrics_history) > self._max_history:
            self._metrics_history.pop(0)

        return metrics

    def get_metrics_history(self, limit: int = 100) -> List[GenAIMetrics]:
        """Get historical metrics.

        Args:
            limit: Maximum number of historical entries to return

        Returns:
            List of GenAIMetrics
        """
        return self._metrics_history[-limit:]

    def get_last_metrics(self) -> Optional[GenAIMetrics]:
        """Get last cached metrics.

        Returns:
            GenAIMetrics or None if not yet fetched
        """
        return self._last_metrics

    def health_check(self) -> bool:
        """Check connector health.

        Returns:
            True if connector is healthy, False otherwise
        """
        try:
            self.get_all_metrics()
            return True
        except Exception as e:
            logger.error(f"Connector health check failed: {e}")
            return False


# ============================================================================
# Mock Connector for Testing
# ============================================================================


class MockGenAIConnector(GenAIConnector):
    """Mock connector for testing."""

    def __init__(self, model: str = "mock-model"):
        super().__init__("mock", "mock")
        self.model = model

    def get_usage_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Return mock usage metrics."""
        return GenAIMetrics(
            timestamp=datetime.utcnow().isoformat(),
            model=model or self.model,
            provider=self.provider,
            tokens=TokenUsage(
                prompt_tokens=50000,
                completion_tokens=15000,
                total_tokens=65000,
            ),
            costs=CostMetrics(
                prompt_cost_usd=0.03,
                completion_cost_usd=0.06,
                total_cost_usd=1.95,
            ),
            request_count=1000,
            success_count=980,
            throughput_rps=0.28,
            availability_percent=98.0,
        )

    def get_cost_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Return mock cost metrics."""
        return GenAIMetrics(
            timestamp=datetime.utcnow().isoformat(),
            model=model or self.model,
            provider=self.provider,
            tokens=TokenUsage(0, 0, 0),
            costs=CostMetrics(
                prompt_cost_usd=0.03,
                completion_cost_usd=0.06,
                total_cost_usd=1.95,
            ),
        )

    def get_latency_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Return mock latency metrics."""
        return GenAIMetrics(
            timestamp=datetime.utcnow().isoformat(),
            model=model or self.model,
            provider=self.provider,
            tokens=TokenUsage(0, 0, 0),
            costs=CostMetrics(0.0, 0.0, 0.0),
            latency=LatencyMetrics(
                mean_latency_ms=125.5,
                p50_latency_ms=89.0,
                p95_latency_ms=245.0,
                p99_latency_ms=450.0,
                max_latency_ms=1200.0,
            ),
        )

    def get_error_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Return mock error metrics."""
        return GenAIMetrics(
            timestamp=datetime.utcnow().isoformat(),
            model=model or self.model,
            provider=self.provider,
            tokens=TokenUsage(0, 0, 0),
            costs=CostMetrics(0.0, 0.0, 0.0),
            errors=ErrorMetrics(
                error_count=20,
                rate_limit_errors=5,
                timeout_errors=10,
                authentication_errors=0,
                server_errors=5,
                error_rate_percent=2.0,
            ),
        )


# ============================================================================
# Connector Registry
# ============================================================================


class ConnectorRegistry:
    """Registry for GenAI connectors."""

    _connectors: Dict[str, GenAIConnector] = {}

    @classmethod
    def register(cls, name: str, connector: GenAIConnector) -> None:
        """Register a connector.

        Args:
            name: Unique connector name
            connector: GenAIConnector instance
        """
        cls._connectors[name] = connector
        logger.info(f"Registered connector: {name}")

    @classmethod
    def get(cls, name: str) -> Optional[GenAIConnector]:
        """Get a connector by name.

        Args:
            name: Connector name

        Returns:
            GenAIConnector or None if not found
        """
        return cls._connectors.get(name)

    @classmethod
    def get_all(cls) -> Dict[str, GenAIConnector]:
        """Get all registered connectors.

        Returns:
            Dictionary of connectors
        """
        return cls._connectors.copy()

    @classmethod
    def list(cls) -> List[str]:
        """List all registered connector names.

        Returns:
            List of connector names
        """
        return list(cls._connectors.keys())
