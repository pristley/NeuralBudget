"""GenAI Connectors - API Reference

Complete API documentation for GenAI telemetry connector classes and data models.

## Table of Contents

1. Data Models
2. GenAIConnector Base Class
3. API Connectors
4. Inference Server Connectors
5. ConnectorRegistry
6. Error Handling
7. Complete Type Reference

---

## 1. Data Models

### TokenUsage

Represents token consumption metrics.

```python
@dataclass(frozen=True)
class TokenUsage:
    prompt_tokens: int           # Tokens in request
    completion_tokens: int       # Tokens in response
    total_tokens: int            # Sum of both
    timestamp: Optional[str] = None  # ISO format timestamp
```

**Example:**
```python
usage = TokenUsage(
    prompt_tokens=150,
    completion_tokens=50,
    total_tokens=200
)
```

### CostMetrics

Represents cost breakdown in USD.

```python
@dataclass(frozen=True)
class CostMetrics:
    prompt_cost_usd: float        # Cost for prompt tokens
    completion_cost_usd: float    # Cost for completion tokens
    total_cost_usd: float         # Total cost
    currency: str = "USD"         # Always "USD"
    timestamp: Optional[str] = None  # ISO format timestamp
```

**Example:**
```python
costs = CostMetrics(
    prompt_cost_usd=0.003,
    completion_cost_usd=0.015,
    total_cost_usd=0.018
)
```

### LatencyMetrics

Represents latency percentiles in milliseconds.

```python
@dataclass(frozen=True)
class LatencyMetrics:
    mean_latency_ms: float        # Average latency
    p50_latency_ms: float         # 50th percentile
    p95_latency_ms: float         # 95th percentile
    p99_latency_ms: float         # 99th percentile
    max_latency_ms: float         # Maximum observed
    timestamp: Optional[str] = None  # ISO format timestamp
```

**Example:**
```python
latency = LatencyMetrics(
    mean_latency_ms=245.0,
    p50_latency_ms=150.0,
    p95_latency_ms=500.0,
    p99_latency_ms=1200.0,
    max_latency_ms=2400.0
)
```

### ErrorMetrics

Represents error breakdown and rates.

```python
@dataclass(frozen=True)
class ErrorMetrics:
    error_count: int              # Total error count
    rate_limit_errors: int        # Rate limit errors
    timeout_errors: int           # Timeout errors
    authentication_errors: int    # Authentication failures
    server_errors: int            # 5xx server errors
    error_rate_percent: float     # Error rate as percentage
    timestamp: Optional[str] = None  # ISO format timestamp
```

**Example:**
```python
errors = ErrorMetrics(
    error_count=12,
    rate_limit_errors=5,
    timeout_errors=3,
    authentication_errors=0,
    server_errors=4,
    error_rate_percent=0.8
)
```

### GenAIMetrics

Aggregated metrics snapshot (main data structure).

```python
@dataclass
class GenAIMetrics:
    timestamp: str                       # ISO format timestamp
    model: str                          # Model name/identifier
    provider: str                       # Provider name (openai, anthropic, etc.)
    tokens: TokenUsage                  # Token metrics
    costs: CostMetrics                  # Cost metrics
    request_count: int                  # Total requests
    success_count: int                  # Successful requests
    throughput_rps: float               # Requests per second
    availability_percent: float         # Availability percentage (0-100)
    latency: Optional[LatencyMetrics] = None     # Latency stats (optional)
    errors: Optional[ErrorMetrics] = None       # Error stats (optional)
    metadata: Dict[str, Any] = field(default_factory=dict)  # Extra info
```

**Example:**
```python
metrics = GenAIMetrics(
    timestamp="2024-01-15T10:30:45.123Z",
    model="gpt-4",
    provider="openai",
    tokens=TokenUsage(150, 50, 200),
    costs=CostMetrics(0.003, 0.015, 0.018),
    request_count=1000,
    success_count=980,
    throughput_rps=16.7,
    availability_percent=98.0,
    latency=LatencyMetrics(...),
    errors=ErrorMetrics(...),
    metadata={"hours_back": 1}
)
```

---

## 2. GenAIConnector Base Class

Abstract base class for all connectors.

### Class Definition

```python
class GenAIConnector(ABC):
    """Base class for GenAI telemetry connectors."""
    
    def __init__(self, connector_id: str, provider: str)
    
    # Abstract methods (must implement)
    @abstractmethod
    def get_usage_metrics(
        self,
        model: Optional[str] = None,
        hours_back: int = 1
    ) -> GenAIMetrics
    
    @abstractmethod
    def get_cost_metrics(
        self,
        model: Optional[str] = None,
        hours_back: int = 1
    ) -> GenAIMetrics
    
    @abstractmethod
    def get_latency_metrics(
        self,
        model: Optional[str] = None,
        hours_back: int = 1
    ) -> GenAIMetrics
    
    @abstractmethod
    def get_error_metrics(
        self,
        model: Optional[str] = None,
        hours_back: int = 1
    ) -> GenAIMetrics
    
    # Concrete methods (inherited)
    def get_all_metrics(
        self,
        model: Optional[str] = None,
        hours_back: int = 1
    ) -> GenAIMetrics
    
    def get_last_metrics(self) -> Optional[GenAIMetrics]
    
    def get_metrics_history(self, limit: int = 10) -> List[GenAIMetrics]
    
    def health_check(self) -> bool
```

### Method Reference

#### `get_usage_metrics(model=None, hours_back=1) -> GenAIMetrics`

Get token usage and request count metrics.

**Parameters:**
- `model` (str, optional): Filter by specific model
- `hours_back` (int): Hours to look back (default: 1)

**Returns:** GenAIMetrics with tokens and request_count

**Raises:** Exception on API errors

**Example:**
```python
usage = connector.get_usage_metrics(model="gpt-4", hours_back=24)
print(f"Total tokens: {usage.tokens.total_tokens}")
print(f"Requests: {usage.request_count}")
```

#### `get_cost_metrics(model=None, hours_back=1) -> GenAIMetrics`

Get cost breakdown metrics.

**Parameters:**
- `model` (str, optional): Filter by specific model
- `hours_back` (int): Hours to look back (default: 1)

**Returns:** GenAIMetrics with costs breakdown

**Raises:** Exception on API errors

**Example:**
```python
costs = connector.get_cost_metrics(hours_back=24)
print(f"Total cost: ${costs.costs.total_cost_usd:.2f}")
```

#### `get_latency_metrics(model=None, hours_back=1) -> GenAIMetrics`

Get latency percentile metrics.

**Parameters:**
- `model` (str, optional): Filter by specific model
- `hours_back` (int): Hours to look back (default: 1)

**Returns:** GenAIMetrics with latency percentiles

**Raises:** Exception on API errors

**Example:**
```python
latency = connector.get_latency_metrics()
print(f"P99 latency: {latency.latency.p99_latency_ms:.0f}ms")
```

#### `get_error_metrics(model=None, hours_back=1) -> GenAIMetrics`

Get error breakdown and rates.

**Parameters:**
- `model` (str, optional): Filter by specific model
- `hours_back` (int): Hours to look back (default: 1)

**Returns:** GenAIMetrics with error metrics

**Raises:** Exception on API errors

**Example:**
```python
errors = connector.get_error_metrics()
print(f"Error rate: {errors.errors.error_rate_percent:.2f}%")
```

#### `get_all_metrics(model=None, hours_back=1) -> GenAIMetrics`

Get all metrics combined (inherited implementation).

**Combines:**
- Token usage
- Cost metrics
- Latency metrics
- Error metrics

**Example:**
```python
metrics = connector.get_all_metrics()
# metrics now has all data
```

#### `get_last_metrics() -> Optional[GenAIMetrics]`

Get cached metrics from last fetch (no API call).

**Returns:** Last GenAIMetrics or None if never fetched

**Performance:** O(1), instant

**Example:**
```python
# Fast check without API call
cached = connector.get_last_metrics()
if cached and cached.availability_percent > 99:
    print("Last check: healthy")
```

#### `get_metrics_history(limit=10) -> List[GenAIMetrics]`

Get historical metrics (up to 1,000 entries cached).

**Parameters:**
- `limit` (int): Max entries to return (default: 10)

**Returns:** List of GenAIMetrics in chronological order

**Performance:** O(n) where n = limit

**Example:**
```python
history = connector.get_metrics_history(limit=60)
for metrics in history:
    print(f"{metrics.timestamp}: {metrics.availability_percent:.1f}%")
```

#### `health_check() -> bool`

Validate connector health (connectivity, credentials, etc.).

**Returns:** True if healthy, False otherwise

**Performance:** Varies (typically <1 second)

**Example:**
```python
if connector.health_check():
    print("Connector is healthy")
else:
    print("Connector has issues")
```

---

## 3. API Connectors

### OpenAIConnector

Connects to OpenAI API.

```python
class OpenAIConnector(GenAIConnector):
    def __init__(
        self,
        api_key: str,
        org_id: Optional[str] = None
    )
```

**Parameters:**
- `api_key`: OpenAI API key (required)
- `org_id`: OpenAI organization ID (optional)

**Supported Models:**
- `gpt-4`, `gpt-4-turbo`, `gpt-3.5-turbo`, `gpt-4o`

**Environment Setup:**
```bash
pip install openai
export OPENAI_API_KEY=sk-...
```

**Example:**
```python
from neuralbudget.genai_connectors_api import OpenAIConnector

openai = OpenAIConnector(api_key="sk-...")
metrics = openai.get_all_metrics(model="gpt-4")

print(f"Tokens: {metrics.tokens.total_tokens}")
print(f"Cost: ${metrics.costs.total_cost_usd:.2f}")
print(f"Availability: {metrics.availability_percent:.1f}%")
```

**Pricing (as of Jan 2024):**
- GPT-4: $0.03/M prompt, $0.06/M completion
- GPT-4 Turbo: $0.01/M prompt, $0.03/M completion
- GPT-3.5 Turbo: $0.0005/M prompt, $0.0015/M completion
- GPT-4o: $0.005/M prompt, $0.015/M completion

### AnthropicConnector

Connects to Anthropic Claude API.

```python
class AnthropicConnector(GenAIConnector):
    def __init__(
        self,
        api_key: str
    )
```

**Parameters:**
- `api_key`: Anthropic API key (required)

**Supported Models:**
- `claude-3-opus`, `claude-3-sonnet`, `claude-3-haiku`, `claude-instant`

**Environment Setup:**
```bash
pip install anthropic
export ANTHROPIC_API_KEY=ant-...
```

**Example:**
```python
from neuralbudget.genai_connectors_api import AnthropicConnector

anthropic = AnthropicConnector(api_key="ant-...")
metrics = anthropic.get_all_metrics(model="claude-3-sonnet")

print(f"Requests: {metrics.request_count}")
print(f"Cost: ${metrics.costs.total_cost_usd:.2f}")
print(f"P99 latency: {metrics.latency.p99_latency_ms:.0f}ms")
```

**Pricing (as of Jan 2024):**
- Claude 3 Opus: $0.015/M prompt, $0.075/M completion
- Claude 3 Sonnet: $0.003/M prompt, $0.015/M completion
- Claude 3 Haiku: $0.00025/M prompt, $0.00125/M completion
- Claude Instant: $0.0008/M prompt, $0.0024/M completion

---

## 4. Inference Server Connectors

### VLLMConnector

Connects to vLLM inference server via Prometheus metrics.

```python
class VLLMConnector(GenAIConnector):
    def __init__(
        self,
        endpoint: str,
        model: str = "default",
        auth_token: Optional[str] = None
    )
```

**Parameters:**
- `endpoint`: vLLM API endpoint (e.g., "http://localhost:8000")
- `model`: Model name served (for labeling)
- `auth_token`: Optional authentication token

**Environment Setup:**
```bash
pip install httpx  # or requests
# Start vLLM server (see deployment guide)
```

**Example:**
```python
from neuralbudget.genai_connectors_inference import VLLMConnector

vllm = VLLMConnector(endpoint="http://localhost:8000")
metrics = vllm.get_all_metrics()

print(f"Throughput: {metrics.throughput_rps:.2f} req/s")
print(f"GPU cache: {metrics.metadata['gpu_cache_usage']:.1f}%")
print(f"Error rate: {metrics.errors.error_rate_percent:.2f}%")
```

**Metrics Available:**
- Token counts (prompt + completion)
- Throughput (requests/second)
- Latency percentiles
- GPU/CPU cache utilization
- Context preemption count
- Error tracking

### TritonConnector

Connects to Triton Inference Server via Prometheus metrics.

```python
class TritonConnector(GenAIConnector):
    def __init__(
        self,
        endpoint: str,
        model: str = "default",
        auth_token: Optional[str] = None
    )
```

**Parameters:**
- `endpoint`: Triton metrics endpoint (e.g., "http://localhost:8002")
- `model`: Model name served (for labeling)
- `auth_token`: Optional authentication token

**Environment Setup:**
```bash
pip install httpx  # or requests
# Start Triton server (see deployment guide)
```

**Example:**
```python
from neuralbudget.genai_connectors_inference import TritonConnector

triton = TritonConnector(endpoint="http://localhost:8002")
metrics = triton.get_all_metrics()

print(f"Total inferences: {metrics.request_count}")
print(f"Success rate: {metrics.availability_percent:.1f}%")
print(f"Mean latency: {metrics.latency.mean_latency_ms:.0f}ms")
```

**Metrics Available:**
- Inference counts (total, success, failure)
- Latency percentiles
- GPU utilization
- Per-model metrics
- Queue time

---

## 5. ConnectorRegistry

Central registry for managing multiple connectors.

```python
class ConnectorRegistry:
    @classmethod
    def register(cls, name: str, connector: GenAIConnector) -> None
    
    @classmethod
    def get(cls, name: str) -> GenAIConnector
    
    @classmethod
    def get_all(cls) -> Dict[str, GenAIConnector]
    
    @classmethod
    def list(cls) -> List[str]
    
    @classmethod
    def unregister(cls, name: str) -> None
```

### Methods

#### `register(name, connector) -> None`

Register a connector instance.

**Example:**
```python
from neuralbudget.genai_connectors_base import ConnectorRegistry
from neuralbudget.genai_connectors_api import OpenAIConnector

registry = ConnectorRegistry()
registry.register("gpt4_prod", 
    OpenAIConnector(api_key="sk-..."))
```

#### `get(name) -> GenAIConnector`

Retrieve registered connector by name.

**Raises:** KeyError if not found

**Example:**
```python
connector = ConnectorRegistry.get("gpt4_prod")
metrics = connector.get_all_metrics()
```

#### `get_all() -> Dict[str, GenAIConnector]`

Get all registered connectors as dictionary.

**Example:**
```python
all_connectors = ConnectorRegistry.get_all()
for name, connector in all_connectors.items():
    print(f"{name}: {connector.provider}")
```

#### `list() -> List[str]`

Get list of registered connector names.

**Example:**
```python
names = ConnectorRegistry.list()
print(f"Registered: {', '.join(names)}")
```

#### `unregister(name) -> None`

Unregister a connector.

**Raises:** KeyError if not found

**Example:**
```python
ConnectorRegistry.unregister("old_service")
```

---

## 6. Error Handling

### Common Exceptions

All methods may raise:

```python
ImportError      # Missing dependency (openai, anthropic, etc.)
ConnectionError  # Network connectivity issues
TimeoutError     # Request timeout
ValueError       # Invalid model name
Exception        # General API/parsing errors
```

### Graceful Error Handling

```python
from neuralbudget.genai_connectors_api import OpenAIConnector
import logging

logger = logging.getLogger(__name__)

try:
    openai = OpenAIConnector(api_key="sk-...")
    metrics = openai.get_all_metrics()
except ImportError as e:
    logger.error(f"Missing library: {e}")
    # Handle - install missing package
except ConnectionError as e:
    logger.error(f"Connection failed: {e}")
    # Handle - check network/endpoint
except TimeoutError as e:
    logger.error(f"Request timeout: {e}")
    # Handle - increase timeout or reduce frequency
except ValueError as e:
    logger.error(f"Invalid input: {e}")
    # Handle - check model name
except Exception as e:
    logger.error(f"Unexpected error: {e}")
    # Handle - check logs/API status
```

---

## 7. Complete Type Reference

### Imports

```python
# Base framework
from neuralbudget.genai_connectors_base import (
    GenAIConnector,           # Abstract base class
    GenAIMetrics,            # Aggregated metrics
    TokenUsage,              # Token metrics
    CostMetrics,             # Cost breakdown
    LatencyMetrics,          # Latency percentiles
    ErrorMetrics,            # Error breakdown
    ModelType,               # Supported models enum
    ConnectorRegistry,       # Connector management
    MockGenAIConnector,      # Testing implementation
)

# API connectors
from neuralbudget.genai_connectors_api import (
    OpenAIConnector,         # OpenAI API connector
    AnthropicConnector,      # Anthropic API connector
)

# Inference servers
from neuralbudget.genai_connectors_inference import (
    VLLMConnector,          # vLLM server connector
    TritonConnector,        # Triton server connector
)
```

### Type Hints

All classes use complete type hints:

```python
from typing import Optional, Dict, List, Any

# Metrics access
metrics: GenAIMetrics
metrics.tokens: TokenUsage
metrics.costs: CostMetrics
metrics.latency: Optional[LatencyMetrics]
metrics.errors: Optional[ErrorMetrics]
metrics.metadata: Dict[str, Any]

# Registry access
connector: GenAIConnector
all_connectors: Dict[str, GenAIConnector]
names: List[str]
```

---

## Performance Characteristics

| Operation | Time | Notes |
|-----------|------|-------|
| OpenAI get_all_metrics() | 200-500ms | API call required |
| Anthropic get_all_metrics() | 300-600ms | API call required |
| vLLM get_all_metrics() | 50-100ms | Local metrics scrape |
| Triton get_all_metrics() | 50-100ms | Local metrics scrape |
| get_last_metrics() | <1ms | Cached, no API call |
| get_metrics_history(60) | <5ms | In-memory, no API call |
| health_check() | 100-500ms | Depends on connector |

## Summary

- **Base Class**: GenAIConnector (ABC) with 4 abstract methods
- **Data Models**: TokenUsage, CostMetrics, LatencyMetrics, ErrorMetrics, GenAIMetrics
- **Connectors**: OpenAI, Anthropic, vLLM, Triton
- **Registry**: ConnectorRegistry for multi-connector management
- **Caching**: Up to 1,000 metrics cached per connector
- **Error Handling**: Graceful fallbacks for all failures
- **Type Safety**: Full type hints throughout

See implementation guide for integration patterns.
"""
