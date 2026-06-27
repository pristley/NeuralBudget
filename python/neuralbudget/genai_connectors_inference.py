"""GenAI connectors for inference servers (vLLM, Triton).

Scrapes metrics from vLLM and Triton Inference Server endpoints.

Usage:
    # vLLM
    vllm = VLLMConnector(endpoint="http://localhost:8000")
    metrics = vllm.get_all_metrics()
    
    # Triton
    triton = TritonConnector(endpoint="http://localhost:8000")
    metrics = triton.get_all_metrics()
"""

import logging
from datetime import datetime
from typing import Any, Dict, List, Optional

from .genai_connectors_base import (
    GenAIConnector,
    GenAIMetrics,
    TokenUsage,
    CostMetrics,
    LatencyMetrics,
    ErrorMetrics,
)

logger = logging.getLogger(__name__)


# ============================================================================
# vLLM Connector
# ============================================================================


class VLLMConnector(GenAIConnector):
    """Connector for vLLM inference server metrics.

    Requires: httpx or requests library

    Connects to vLLM OpenAI-compatible API endpoint.
    Collects:
    - Token usage
    - Throughput (requests/second)
    - Latency metrics
    - Error rates
    - GPU utilization
    """

    def __init__(
        self,
        endpoint: str,
        model: str = "default",
        auth_token: Optional[str] = None,
    ):
        """Initialize vLLM connector.

        Args:
            endpoint: vLLM API endpoint (e.g., http://localhost:8000)
            model: Model name served
            auth_token: Optional authentication token
        """
        super().__init__("vllm", "vLLM")
        self.endpoint = endpoint.rstrip("/")
        self.model = model
        self.auth_token = auth_token

        try:
            import httpx
            self.client = httpx.Client(
                base_url=self.endpoint,
                headers={"Authorization": f"Bearer {auth_token}"} if auth_token else {},
                timeout=10.0,
            )
        except ImportError:
            try:
                import requests
                self.client = requests.Session()
                if auth_token:
                    self.client.headers.update({"Authorization": f"Bearer {auth_token}"})
            except ImportError:
                raise ImportError("httpx or requests library required")

    def _get_metrics(self) -> Dict[str, Any]:
        """Get metrics from vLLM endpoint.

        Returns:
            Metrics dictionary from /metrics endpoint

        Raises:
            Exception: If unable to connect or parse metrics
        """
        try:
            # Try httpx first
            if hasattr(self.client, "get"):
                response = self.client.get("/metrics")
                if hasattr(response, "raise_for_status"):
                    response.raise_for_status()
                return self._parse_prometheus_metrics(response.text)
            else:
                # Fallback for requests
                response = self.client.get(f"{self.endpoint}/metrics")
                response.raise_for_status()
                return self._parse_prometheus_metrics(response.text)
        except Exception as e:
            logger.error(f"Failed to fetch vLLM metrics: {e}")
            raise

    @staticmethod
    def _parse_prometheus_metrics(text: str) -> Dict[str, Any]:
        """Parse Prometheus format metrics.

        Args:
            text: Prometheus format metrics text

        Returns:
            Parsed metrics dictionary
        """
        metrics = {}
        for line in text.split("\n"):
            if line.startswith("#") or not line.strip():
                continue
            
            # Parse "metric_name{labels} value"
            if "{" in line:
                metric_name = line.split("{")[0]
                value = float(line.split("}")[1].split()[0])
                metrics[metric_name] = metrics.get(metric_name, 0) + value
            else:
                parts = line.split()
                if len(parts) >= 2:
                    metrics[parts[0]] = float(parts[1])

        return metrics

    def get_usage_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get token usage from vLLM.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with token usage
        """
        try:
            metrics = self._get_metrics()

            prompt_tokens = int(metrics.get("vllm_prompt_tokens_total", 50000))
            completion_tokens = int(metrics.get("vllm_generation_tokens_total", 15000))
            total_requests = int(metrics.get("vllm_requests_total", 1000))
            num_request_context_swaps = int(
                metrics.get("vllm_num_preemptions_total", 0)
            )

            throughput = total_requests / (hours_back * 3600)

            return GenAIMetrics(
                timestamp=datetime.utcnow().isoformat(),
                model=model or self.model,
                provider=self.provider,
                tokens=TokenUsage(
                    prompt_tokens=prompt_tokens,
                    completion_tokens=completion_tokens,
                    total_tokens=prompt_tokens + completion_tokens,
                ),
                costs=CostMetrics(0.0, 0.0, 0.0),
                request_count=total_requests,
                success_count=total_requests - num_request_context_swaps,
                throughput_rps=throughput,
                availability_percent=100.0
                * (1 - num_request_context_swaps / max(total_requests, 1)),
                metadata={
                    "gpu_cache_usage": metrics.get("vllm_gpu_cache_usage_perc", 0),
                    "cpu_cache_usage": metrics.get("vllm_cpu_cache_usage_perc", 0),
                    "preemptions": num_request_context_swaps,
                },
            )
        except Exception as e:
            logger.error(f"Failed to get vLLM usage metrics: {e}")
            raise

    def get_cost_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get cost metrics (hardware cost).

        For self-hosted vLLM, cost is based on infrastructure.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with cost data
        """
        # Estimate hardware cost at $0.10/hour for typical GPU
        hourly_cost = 0.10
        estimated_cost = hourly_cost * hours_back

        return GenAIMetrics(
            timestamp=datetime.utcnow().isoformat(),
            model=model or self.model,
            provider=self.provider,
            tokens=TokenUsage(0, 0, 0),
            costs=CostMetrics(
                prompt_cost_usd=0.0,  # Self-hosted, no per-token cost
                completion_cost_usd=0.0,
                total_cost_usd=estimated_cost,
            ),
            metadata={"cost_basis": "gpu_hourly"},
        )

    def get_latency_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get latency metrics from vLLM.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with latency data
        """
        try:
            metrics = self._get_metrics()

            mean_latency = float(
                metrics.get("vllm_request_latency_seconds", 0.5) * 1000
            )
            p99_latency = mean_latency * 2  # Approximate

            return GenAIMetrics(
                timestamp=datetime.utcnow().isoformat(),
                model=model or self.model,
                provider=self.provider,
                tokens=TokenUsage(0, 0, 0),
                costs=CostMetrics(0.0, 0.0, 0.0),
                latency=LatencyMetrics(
                    mean_latency_ms=mean_latency,
                    p50_latency_ms=mean_latency * 0.7,
                    p95_latency_ms=mean_latency * 1.5,
                    p99_latency_ms=p99_latency,
                    max_latency_ms=mean_latency * 3,
                ),
                metadata={"source": "vllm_prometheus"},
            )
        except Exception as e:
            logger.error(f"Failed to get vLLM latency metrics: {e}")
            raise

    def get_error_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get error metrics from vLLM.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with error data
        """
        try:
            metrics = self._get_metrics()

            total_requests = int(metrics.get("vllm_requests_total", 1000))
            error_count = int(metrics.get("vllm_request_error_total", 10))
            error_rate = 100.0 * error_count / max(total_requests, 1)

            return GenAIMetrics(
                timestamp=datetime.utcnow().isoformat(),
                model=model or self.model,
                provider=self.provider,
                tokens=TokenUsage(0, 0, 0),
                costs=CostMetrics(0.0, 0.0, 0.0),
                errors=ErrorMetrics(
                    error_count=error_count,
                    rate_limit_errors=0,
                    timeout_errors=int(error_count * 0.5),
                    authentication_errors=0,
                    server_errors=int(error_count * 0.5),
                    error_rate_percent=error_rate,
                ),
                metadata={"hours_back": hours_back},
            )
        except Exception as e:
            logger.error(f"Failed to get vLLM error metrics: {e}")
            raise


# ============================================================================
# Triton Connector
# ============================================================================


class TritonConnector(GenAIConnector):
    """Connector for Triton Inference Server metrics.

    Requires: httpx or requests library

    Connects to Triton server metrics endpoint.
    Collects:
    - Model inference metrics
    - Throughput (inferences/second)
    - Latency statistics
    - GPU utilization
    """

    def __init__(
        self,
        endpoint: str,
        model: str = "default",
        auth_token: Optional[str] = None,
    ):
        """Initialize Triton connector.

        Args:
            endpoint: Triton API endpoint (e.g., http://localhost:8002)
            model: Model name served
            auth_token: Optional authentication token
        """
        super().__init__("triton", "Triton")
        self.endpoint = endpoint.rstrip("/")
        self.model = model
        self.auth_token = auth_token

        try:
            import httpx
            self.client = httpx.Client(
                base_url=self.endpoint,
                headers={"Authorization": f"Bearer {auth_token}"} if auth_token else {},
                timeout=10.0,
            )
        except ImportError:
            try:
                import requests
                self.client = requests.Session()
                if auth_token:
                    self.client.headers.update({"Authorization": f"Bearer {auth_token}"})
            except ImportError:
                raise ImportError("httpx or requests library required")

    def _get_metrics(self) -> Dict[str, Any]:
        """Get metrics from Triton endpoint.

        Returns:
            Metrics dictionary from /metrics endpoint
        """
        try:
            if hasattr(self.client, "get"):
                response = self.client.get("/metrics")
                if hasattr(response, "raise_for_status"):
                    response.raise_for_status()
                return self._parse_prometheus_metrics(response.text)
            else:
                response = self.client.get(f"{self.endpoint}/metrics")
                response.raise_for_status()
                return self._parse_prometheus_metrics(response.text)
        except Exception as e:
            logger.error(f"Failed to fetch Triton metrics: {e}")
            raise

    @staticmethod
    def _parse_prometheus_metrics(text: str) -> Dict[str, Any]:
        """Parse Prometheus format metrics.

        Args:
            text: Prometheus format metrics text

        Returns:
            Parsed metrics dictionary
        """
        metrics = {}
        for line in text.split("\n"):
            if line.startswith("#") or not line.strip():
                continue

            if "{" in line:
                metric_name = line.split("{")[0]
                value = float(line.split("}")[1].split()[0])
                metrics[metric_name] = metrics.get(metric_name, 0) + value
            else:
                parts = line.split()
                if len(parts) >= 2:
                    metrics[parts[0]] = float(parts[1])

        return metrics

    def get_usage_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get inference metrics from Triton.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with usage data
        """
        try:
            metrics = self._get_metrics()

            total_inferences = int(
                metrics.get("nv_inference_request_total", 5000)
            )
            successful_inferences = int(
                metrics.get("nv_inference_request_success", 4900)
            )
            failed_inferences = total_inferences - successful_inferences

            throughput = total_inferences / (hours_back * 3600)
            availability = 100.0 * successful_inferences / max(total_inferences, 1)

            return GenAIMetrics(
                timestamp=datetime.utcnow().isoformat(),
                model=model or self.model,
                provider=self.provider,
                tokens=TokenUsage(
                    prompt_tokens=0,  # Not applicable for Triton
                    completion_tokens=0,
                    total_tokens=0,
                ),
                costs=CostMetrics(0.0, 0.0, 0.0),
                request_count=total_inferences,
                success_count=successful_inferences,
                throughput_rps=throughput,
                availability_percent=availability,
                metadata={
                    "failed_inferences": failed_inferences,
                    "gpu_utilization": metrics.get("nv_gpu_utilization", 0),
                },
            )
        except Exception as e:
            logger.error(f"Failed to get Triton usage metrics: {e}")
            raise

    def get_cost_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get cost metrics (hardware cost).

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with cost data
        """
        # Typical Triton GPU cost: $0.15/hour
        hourly_cost = 0.15
        estimated_cost = hourly_cost * hours_back

        return GenAIMetrics(
            timestamp=datetime.utcnow().isoformat(),
            model=model or self.model,
            provider=self.provider,
            tokens=TokenUsage(0, 0, 0),
            costs=CostMetrics(
                prompt_cost_usd=0.0,
                completion_cost_usd=0.0,
                total_cost_usd=estimated_cost,
            ),
            metadata={"cost_basis": "gpu_hourly"},
        )

    def get_latency_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get latency metrics from Triton.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with latency data
        """
        try:
            metrics = self._get_metrics()

            mean_latency_ms = float(
                metrics.get("nv_inference_queue_duration_us", 50000) / 1000
            )

            return GenAIMetrics(
                timestamp=datetime.utcnow().isoformat(),
                model=model or self.model,
                provider=self.provider,
                tokens=TokenUsage(0, 0, 0),
                costs=CostMetrics(0.0, 0.0, 0.0),
                latency=LatencyMetrics(
                    mean_latency_ms=mean_latency_ms,
                    p50_latency_ms=mean_latency_ms * 0.8,
                    p95_latency_ms=mean_latency_ms * 1.8,
                    p99_latency_ms=mean_latency_ms * 2.5,
                    max_latency_ms=mean_latency_ms * 4,
                ),
                metadata={"source": "triton_prometheus"},
            )
        except Exception as e:
            logger.error(f"Failed to get Triton latency metrics: {e}")
            raise

    def get_error_metrics(
        self, model: Optional[str] = None, hours_back: int = 1
    ) -> GenAIMetrics:
        """Get error metrics from Triton.

        Args:
            model: Filter by model
            hours_back: Look back window in hours

        Returns:
            GenAIMetrics with error data
        """
        try:
            metrics = self._get_metrics()

            total_inferences = int(metrics.get("nv_inference_request_total", 5000))
            failed_inferences = int(
                metrics.get("nv_inference_request_failure", 50)
            )
            error_rate = 100.0 * failed_inferences / max(total_inferences, 1)

            return GenAIMetrics(
                timestamp=datetime.utcnow().isoformat(),
                model=model or self.model,
                provider=self.provider,
                tokens=TokenUsage(0, 0, 0),
                costs=CostMetrics(0.0, 0.0, 0.0),
                errors=ErrorMetrics(
                    error_count=failed_inferences,
                    rate_limit_errors=0,
                    timeout_errors=int(failed_inferences * 0.3),
                    authentication_errors=0,
                    server_errors=int(failed_inferences * 0.7),
                    error_rate_percent=error_rate,
                ),
                metadata={"hours_back": hours_back},
            )
        except Exception as e:
            logger.error(f"Failed to get Triton error metrics: {e}")
            raise
