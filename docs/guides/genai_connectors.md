"""GenAI Telemetry Connectors - Implementation Guide

This guide covers the implementation of GenAI-specific telemetry connectors for NeuralBudget.

## Overview

GenAI connectors enable NeuralBudget to collect metrics from:
- **OpenAI & Anthropic**: Hosted API usage (tokens, costs, availability)
- **vLLM**: Open-source local inference server
- **Triton**: NVIDIA Triton Inference Server

All connectors implement a common interface (GenAIConnector) and follow the same patterns.

## Architecture

### Base Framework (genai_connectors_base.py)

Core components:

1. **Enums**
   - `ModelType`: Enum of supported models (GPT-4, Claude, Llama, etc.)

2. **Data Models** (immutable dataclasses)
   - `TokenUsage`: prompt_tokens, completion_tokens, total_tokens
   - `CostMetrics`: prompt_cost, completion_cost, total_cost in USD
   - `LatencyMetrics`: mean, p50, p95, p99, max latency in milliseconds
   - `ErrorMetrics`: error counts (rate_limit, timeout, auth, server), error rate %
   - `GenAIMetrics`: Aggregated metrics snapshot with timestamp, model, provider

3. **GenAIConnector (ABC)**
   Abstract base class with methods:
   - `get_usage_metrics()` → tokens and request counts
   - `get_cost_metrics()` → cost breakdown and totals
   - `get_latency_metrics()` → latency percentiles and max
   - `get_error_metrics()` → error counts and rates
   - `get_all_metrics()` → combined metrics (inherited)
   - `get_metrics_history(limit)` → last N cached metrics
   - `health_check()` → connector validation
   - `get_last_metrics()` → cached last fetch

4. **ConnectorRegistry**
   Central registry for managing multiple connector instances:
   - `register(name, connector)` - register connector with key
   - `get(name)` - retrieve connector by name
   - `get_all()` - get all registered connectors
   - `list()` - list all connector names

5. **MockGenAIConnector**
   Testing implementation returning sample data.

### API Connectors (genai_connectors_api.py)

#### OpenAIConnector

Connects to OpenAI API and usage endpoints.

**Features:**
- Token tracking via API calls
- Cost calculation using current pricing
- Latency estimation from API response times
- Rate limit and error tracking

**Constructor:**
```python
OpenAIConnector(api_key: str, org_id: Optional[str] = None)
```

**Pricing Model:**
- Maintains pricing dictionary: `gpt-4`, `gpt-4-turbo`, `gpt-3.5-turbo`, `gpt-4o`
- Per-token costs for prompt and completion
- Updates needed as OpenAI changes pricing

**Example:**
```python
openai = OpenAIConnector(api_key="sk-...")
metrics = openai.get_all_metrics(model="gpt-4", hours_back=24)
print(f"Cost: ${metrics.costs.total_cost_usd:.2f}")
print(f"Latency P99: {metrics.latency.p99_latency_ms:.0f}ms")
```

#### AnthropicConnector

Connects to Anthropic Claude API.

**Features:**
- Multi-model support (Opus, Sonnet, Haiku)
- Token estimation from request patterns
- Cost calculation based on Claude pricing
- Availability tracking

**Constructor:**
```python
AnthropicConnector(api_key: str)
```

**Pricing Model:**
- `claude-3-opus`: $0.015/M prompt, $0.075/M completion
- `claude-3-sonnet`: $0.003/M prompt, $0.015/M completion
- `claude-3-haiku`: $0.00025/M prompt, $0.00125/M completion
- `claude-instant`: $0.0008/M prompt, $0.0024/M completion

**Example:**
```python
anthropic = AnthropicConnector(api_key="ant-...")
metrics = anthropic.get_all_metrics(model="claude-3-sonnet")
print(f"Requests: {metrics.request_count}")
print(f"Success rate: {metrics.availability_percent:.1f}%")
```

### Inference Server Connectors (genai_connectors_inference.py)

#### VLLMConnector

Connects to vLLM OpenAI-compatible API server.

**Features:**
- Prometheus metrics scraping
- Token tracking (prompt + completion)
- Throughput measurement (requests/second)
- GPU cache utilization monitoring
- Context preemption detection

**Constructor:**
```python
VLLMConnector(
    endpoint: str,
    model: str = "default",
    auth_token: Optional[str] = None
)
```

**Metrics Collected:**
- `vllm_requests_total`: Total inference requests
- `vllm_request_latency_seconds`: Request latency (averaged)
- `vllm_prompt_tokens_total`: Total prompt tokens
- `vllm_generation_tokens_total`: Total completion tokens
- `vllm_num_preemptions_total`: Context swaps
- `vllm_gpu_cache_usage_perc`: GPU cache percentage
- `vllm_cpu_cache_usage_perc`: CPU cache percentage

**Example:**
```python
vllm = VLLMConnector(endpoint="http://localhost:8000")
metrics = vllm.get_all_metrics()
print(f"Throughput: {metrics.throughput_rps:.2f} req/s")
print(f"GPU cache: {metrics.metadata['gpu_cache_usage']:.1f}%")
print(f"Latency p99: {metrics.latency.p99_latency_ms:.0f}ms")
```

**Setup:**
```bash
# Start vLLM server
docker run --gpus all -p 8000:8000 \\
  -v /data/models:/models \\
  vllm/vllm-openai \\
  python -m vllm.entrypoints.openai.api_server \\
    --model meta-llama/Llama-2-7b-hf \\
    --port 8000
```

#### TritonConnector

Connects to NVIDIA Triton Inference Server.

**Features:**
- Prometheus metrics scraping
- Per-model inference tracking
- Latency statistics
- GPU utilization monitoring
- Error tracking

**Constructor:**
```python
TritonConnector(
    endpoint: str,
    model: str = "default",
    auth_token: Optional[str] = None
)
```

**Metrics Collected:**
- `nv_inference_request_total`: Total inferences
- `nv_inference_request_success`: Successful inferences
- `nv_inference_request_failure`: Failed inferences
- `nv_inference_queue_duration_us`: Queue time in microseconds
- `nv_gpu_utilization`: GPU utilization percentage

**Example:**
```python
triton = TritonConnector(endpoint="http://localhost:8002")
metrics = triton.get_all_metrics()
print(f"Total inferences: {metrics.request_count}")
print(f"Success rate: {metrics.availability_percent:.1f}%")
print(f"Mean latency: {metrics.latency.mean_latency_ms:.0f}ms")
```

**Setup:**
```bash
# Start Triton server
docker run --gpus all -p 8000:8000 -p 8001:8001 -p 8002:8002 \\
  -v /data/models:/models \\
  nvcr.io/nvidia/tritonserver:latest \\
  tritonserver --model-repository=/models
```

## Usage Patterns

### Pattern 1: Single Connector

```python
from neuralbudget.genai_connectors_api import OpenAIConnector

openai = OpenAIConnector(api_key="sk-...")
metrics = openai.get_all_metrics()

# Check health
if openai.health_check():
    print(f"Available: {metrics.availability_percent:.1f}%")
    print(f"Cost: ${metrics.costs.total_cost_usd:.2f}")
```

### Pattern 2: Multiple Services with Registry

```python
from neuralbudget.genai_connectors_base import ConnectorRegistry
from neuralbudget.genai_connectors_api import OpenAIConnector
from neuralbudget.genai_connectors_inference import VLLMConnector

# Register all services
ConnectorRegistry.register("gpt4_prod", 
    OpenAIConnector(api_key="sk-..."))
ConnectorRegistry.register("llama_internal",
    VLLMConnector(endpoint="http://vllm-prod:8000"))

# Get metrics for all
for name in ConnectorRegistry.list():
    connector = ConnectorRegistry.get(name)
    metrics = connector.get_all_metrics()
    print(f"{name}: {metrics.availability_percent:.1f}%")
```

### Pattern 3: Continuous Monitoring

```python
import time

connector = OpenAIConnector(api_key="sk-...")

while True:
    try:
        metrics = connector.get_all_metrics()
        print(f"[{metrics.timestamp}] Cost: ${metrics.costs.total_cost_usd:.2f}")
    except Exception as e:
        print(f"Error: {e}")
    
    time.sleep(60)  # Check every minute
```

### Pattern 4: SLO Integration

```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.convenience import GenAiSloProfile

client = NeuralBudgetClient()
connector = OpenAIConnector(api_key="sk-...")

# Define SLO targets
profile = GenAiSloProfile(
    availability_target=99.5,
    latency_p99_target_ms=500,
    cost_limit_usd_per_hour=100,
)

# Evaluate
metrics = connector.get_all_metrics()
result = client.evaluate({
    "genai_requests": metrics.request_count,
    "genai_errors": metrics.errors.error_count if metrics.errors else 0,
    "genai_latency_p99_ms": metrics.latency.p99_latency_ms if metrics.latency else 0,
    "genai_cost_usd": metrics.costs.total_cost_usd,
}, profile=profile)

print(f"SLO Status: {result['status']}")
```

## Integration Points

### 1. Dashboard Integration

```python
from neuralbudget.dashboard import Dashboard

dashboard = Dashboard()

# Add GenAI metrics
connector = OpenAIConnector(api_key="sk-...")
metrics = connector.get_all_metrics()

dashboard.update_slo_snapshot({
    "service_name": "gpt4-prod",
    "metric_name": "genai_requests",
    "total_requests": metrics.request_count,
    "total_errors": metrics.errors.error_count if metrics.errors else 0,
    # ... other fields
})

dashboard.run()
```

### 2. Alert Integration

```python
from neuralbudget.alerting import AlertDispatcher
from neuralbudget.alert_dispatch_advanced import AlertDispatchManager

connector = OpenAIConnector(api_key="sk-...")
dispatcher = AlertDispatcher()
manager = AlertDispatchManager(dispatcher)

# Monitor for cost overrun
metrics = connector.get_all_metrics()
if metrics.costs.total_cost_usd > 100:
    alert = {
        "service": "gpt4-prod",
        "alert_type": "cost_overrun",
        "message": f"Cost exceeded: ${metrics.costs.total_cost_usd:.2f}",
    }
    manager.dispatch_with_policies(alert)
```

### 3. CLI TUI Integration

```python
from neuralbudget.cli_tui import CliTui

cli = CliTui()

# Populate with GenAI metrics
connector = OpenAIConnector(api_key="sk-...")
metrics = connector.get_all_metrics()

# Display in CLI
cli.print_dashboard()  # Includes GenAI service metrics
```

## Error Handling

All connectors implement graceful error handling:

```python
from neuralbudget.genai_connectors_api import OpenAIConnector
import logging

logger = logging.getLogger(__name__)

try:
    openai = OpenAIConnector(api_key="sk-...")
    metrics = openai.get_all_metrics()
except ImportError as e:
    logger.error(f"Missing dependency: {e}")
    # Handle missing 'openai' library
except Exception as e:
    logger.error(f"Failed to collect metrics: {e}")
    # Handle API errors, network issues, etc.
```

## Performance Considerations

### Caching

Base class automatically caches metrics:
- Last fetch cached in `_last_metrics`
- History of up to 1,000 entries in `_metrics_history`
- `get_last_metrics()` returns cached value (no API call)

### Rate Limiting

When integrating with real APIs:
- OpenAI: Requests limited by API quota
- Anthropic: Varies by pricing tier
- vLLM/Triton: Local, no rate limits

Consider:
```python
import time

def fetch_with_backoff(connector, max_retries=3):
    for attempt in range(max_retries):
        try:
            return connector.get_all_metrics()
        except Exception as e:
            if attempt < max_retries - 1:
                time.sleep(2 ** attempt)  # Exponential backoff
            else:
                raise
```

### Network Efficiency

For inference servers (vLLM, Triton):
- Prometheus endpoints are lightweight
- Scraping typically <50ms
- Can scrape frequently without impact
- Consider caching results for rapid polling

## Testing

All connectors support testing via MockGenAIConnector:

```python
from neuralbudget.genai_connectors_base import MockGenAIConnector

# In tests
connector = MockGenAIConnector("gpt-4")
metrics = connector.get_all_metrics()

# Returns sample data:
# - 50,000 prompt tokens
# - 15,000 completion tokens
# - 98% availability
# - p99 latency: 500ms
# - $1.25 cost estimate
```

## Future Extensions

To add a new GenAI provider:

1. Create new file: `python/neuralbudget/genai_connectors_X.py`
2. Inherit from `GenAIConnector`
3. Implement 4 abstract methods:
   - `get_usage_metrics()`
   - `get_cost_metrics()`
   - `get_latency_metrics()`
   - `get_error_metrics()`
4. Register in ConnectorRegistry
5. Add examples and documentation

Example template:

```python
from neuralbudget.genai_connectors_base import GenAIConnector, GenAIMetrics

class MyProviderConnector(GenAIConnector):
    def __init__(self, config: dict):
        super().__init__("my_provider", "My Provider")
        self.config = config
    
    def get_usage_metrics(self, model=None, hours_back=1):
        # Fetch from provider API
        return GenAIMetrics(...)
    
    def get_cost_metrics(self, model=None, hours_back=1):
        # Calculate costs
        return GenAIMetrics(...)
    
    def get_latency_metrics(self, model=None, hours_back=1):
        # Scrape latency metrics
        return GenAIMetrics(...)
    
    def get_error_metrics(self, model=None, hours_back=1):
        # Track errors
        return GenAIMetrics(...)
```

## Summary

GenAI connectors provide:
- **Unified interface** for multiple providers
- **Type-safe** metrics models
- **Extensible** architecture for new providers
- **Production-ready** error handling
- **Integration** with NeuralBudget ecosystem
- **Testing** via MockGenAIConnector
- **Performance** via caching and efficient scraping

Start with MockGenAIConnector for development, integrate with real providers in production.
"""
