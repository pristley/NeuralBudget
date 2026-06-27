# Advanced Alert Dispatch API Reference

Complete API reference for the advanced alert dispatch system.

## Module: `neuralbudget.alert_dispatch_advanced`

Enterprise-grade alert delivery with retry policies, deduplication, circuit breaker, and escalation.

### Classes

#### `RetryPolicy`

Configuration for automatic retry with exponential backoff and jitter.

**Attributes:**
- `max_retries: int = 3` — Maximum number of retry attempts
- `initial_delay_ms: int = 100` — Initial delay in milliseconds
- `max_delay_ms: int = 30_000` — Maximum delay cap
- `backoff_multiplier: float = 2.0` — Exponential backoff multiplier
- `jitter_percent: float = 10.0` — Jitter as percentage (0-100)
- `use_circuit_breaker: bool = True` — Enable circuit breaker
- `circuit_breaker_threshold: int = 5` — Failures to open circuit
- `circuit_breaker_open_seconds: int = 60` — How long circuit stays open

**Methods:**

##### `calculate_delay_ms(attempt: int) -> int`

Calculate delay for given attempt number (0-indexed).

```python
policy = RetryPolicy(initial_delay_ms=100, backoff_multiplier=2.0, jitter_percent=0.0)
delay_0 = policy.calculate_delay_ms(0)  # Returns 100
delay_1 = policy.calculate_delay_ms(1)  # Returns 200
delay_2 = policy.calculate_delay_ms(2)  # Returns 400
```

---

#### `DeduplicationPolicy`

Configuration for deduplicating alert dispatches.

**Attributes:**
- `enabled: bool = True` — Enable deduplication
- `window_seconds: int = 300` — Time window for dedup tracking
- `key_strategy: str = "content_hash"` — Strategy for generating dedup keys
  - `"content_hash"`: Hash alert content (mode, profile, result)
  - `"custom"`: Use explicit dedup_key from config

**Example:**

```python
# Standard deduplication
dedup = DeduplicationPolicy(
    enabled=True,
    window_seconds=300,  # 5 minutes
)

# Aggressive deduplication
strict_dedup = DeduplicationPolicy(
    enabled=True,
    window_seconds=10,  # Only 10 seconds
)

# Disabled (send all alerts)
no_dedup = DeduplicationPolicy(enabled=False)
```

---

#### `EscalationAction`

Enum for escalation actions.

**Values:**
- `ADD_CHANNELS` — Add notification channels (Slack, PagerDuty, etc.)
- `INCREASE_SEVERITY` — Escalate severity level
- `ADD_TAGS` — Add tags to alert
- `MODIFY_CONFIG` — Direct config modifications
- `FORCE_DISPATCH` — Force immediate dispatch

---

#### `EscalationStep`

Single escalation step: trigger after N seconds.

**Attributes:**
- `after_seconds: int` — Seconds after initial alert before escalating
- `action: str` — Escalation action (use `EscalationAction` enum)
- `config: Mapping[str, Any] = {}` — Action-specific configuration

**Example:**

```python
step = EscalationStep(
    after_seconds=600,  # After 10 minutes
    action=EscalationAction.ADD_CHANNELS,
    config={
        "channels": ["pagerduty"],
        "pagerduty_config": {"severity": "critical"},
    }
)
```

---

#### `EscalationPolicy`

Configuration for escalating unresolved alerts.

**Attributes:**
- `enabled: bool = True` — Enable escalation
- `steps: list[EscalationStep] = []` — Escalation steps
- `max_escalations: int = 10` — Maximum escalations per alert

**Example:**

```python
escalation = EscalationPolicy(
    enabled=True,
    steps=[
        EscalationStep(
            after_seconds=300,
            action=EscalationAction.ADD_CHANNELS,
            config={"channels": ["pagerduty"]},
        ),
        EscalationStep(
            after_seconds=900,
            action=EscalationAction.INCREASE_SEVERITY,
            config={},
        ),
    ],
)
```

---

#### `AlertDeduplicationEntry`

Internal class tracking an alert that has been recently sent.

**Attributes:**
- `dedup_key: str` — Unique dedup key
- `sent_at: datetime` — When alert was sent
- `dedup_count: int = 0` — Times deduplicated
- `escalation_level: int = 0` — Current escalation level
- `metadata: dict[str, Any] = {}` — Custom metadata

---

#### `CircuitBreakerState`

Internal class tracking circuit breaker state per provider.

**Attributes:**
- `provider: str` — Provider name (slack, pagerduty, opsgenie)
- `failure_count: int = 0` — Consecutive failures
- `last_failure_time: Optional[datetime] = None` — Last failure timestamp
- `is_open: bool = False` — Whether circuit is open
- `opened_at: Optional[datetime] = None` — When circuit opened

---

#### `AlertDispatchManager`

Main class orchestrating alert dispatch with all policies.

**Constructor:**

```python
def __init__(
    self,
    dispatcher: Optional[AlertDispatcher] = None,
    retry_policy: Optional[RetryPolicy] = None,
    dedup_policy: Optional[DeduplicationPolicy] = None,
    escalation_policy: Optional[EscalationPolicy] = None,
)
```

**Parameters:**
- `dispatcher` — Underlying AlertDispatcher (creates new if None)
- `retry_policy` — Retry configuration
- `dedup_policy` — Deduplication configuration
- `escalation_policy` — Escalation configuration

**Attributes:**
- `dispatcher: AlertDispatcher` — Underlying dispatcher
- `retry_policy: RetryPolicy` — Retry policy
- `dedup_policy: DeduplicationPolicy` — Dedup policy
- `escalation_policy: EscalationPolicy` — Escalation policy

**Methods:**

##### `dispatch_with_policies(...) -> AlertDispatchSummary`

Main entry point for alert dispatch with all policies applied.

```python
def dispatch_with_policies(
    self,
    *,
    mode: str,
    profile: str | None,
    metric_data: Mapping[str, Any],
    evaluation_result: Mapping[str, Any],
    alerts_config: Mapping[str, Any],
) -> AlertDispatchSummary
```

**Parameters:**
- `mode` — SLO evaluation mode (e.g., "http", "stateful")
- `profile` — Optional SLO profile name
- `metric_data` — Raw metric data that triggered alert
- `evaluation_result` — SLO evaluation result
- `alerts_config` — Alert provider configuration

**Returns:** `AlertDispatchSummary` with dispatch results

**Example:**

```python
summary = manager.dispatch_with_policies(
    mode="http",
    profile="strict_latency",
    metric_data={"requests": 1000, "errors": 50},
    evaluation_result={"violation": True, "burn_rate": 2.5},
    alerts_config={
        "slack": {"webhook_url": "https://hooks.slack.com/..."},
        "pagerduty": {"routing_key": "..."},
    },
)

print(f"Sent: {summary.succeeded}/{summary.attempted}")
```

---

##### `get_dedup_stats() -> dict[str, Any]`

Get deduplication statistics.

**Returns:**

```python
{
    "tracked_alerts": int,        # Number of tracked alerts
    "total_dedup_preventions": int,  # Total prevented sends
    "entries": [
        {
            "key": str,           # Dedup key
            "sent_at": str,       # ISO timestamp
            "dedup_count": int,   # Prevention count
            "escalation_level": int,  # Current escalation
        }
    ]
}
```

**Example:**

```python
stats = manager.get_dedup_stats()
print(f"Prevented: {stats['total_dedup_preventions']} duplicate sends")
```

---

##### `get_circuit_breaker_stats() -> dict[str, Any]`

Get circuit breaker statistics.

**Returns:**

```python
{
    "providers": [
        {
            "provider": str,      # Provider name
            "is_open": bool,      # Circuit state
            "failure_count": int, # Consecutive failures
            "last_failure": str | None,  # ISO timestamp
        }
    ]
}
```

**Example:**

```python
stats = manager.get_circuit_breaker_stats()
for p in stats["providers"]:
    print(f"{p['provider']}: {'OPEN' if p['is_open'] else 'CLOSED'}")
```

---

##### `get_escalation_history(dedup_key: str) -> list[dict[str, Any]]`

Get escalation history for a specific alert.

**Parameters:**
- `dedup_key` — The dedup key to query

**Returns:**

```python
[
    {
        "timestamp": str,  # ISO timestamp
        "step": int,       # Escalation step number
        "action": str,     # Escalation action
        "config": dict,    # Action configuration
    }
]
```

**Example:**

```python
history = manager.get_escalation_history("http:strict:abc123")
for event in history:
    print(f"{event['timestamp']}: {event['action']}")
```

---

##### `cleanup_expired_dedup_entries() -> int`

Remove expired deduplication entries.

**Returns:** Number of entries removed

**Example:**

```python
# Run periodically (e.g., hourly)
removed = manager.cleanup_expired_dedup_entries()
logger.info(f"Cleaned up {removed} expired entries")
```

---

##### `reset_circuit_breaker(provider: str) -> bool`

Manually reset circuit breaker for a provider.

**Parameters:**
- `provider` — Provider name ("slack", "pagerduty", "opsgenie")

**Returns:** True if breaker was open, False otherwise

**Example:**

```python
# Manually reset stuck circuit breaker
if manager.reset_circuit_breaker("pagerduty"):
    print("Circuit breaker was open, now reset")
```

---

### Data Classes (Internal)

#### `AlertDeduplicationEntry`

Tracks an alert sent within dedup window.

```python
@dataclass
class AlertDeduplicationEntry:
    dedup_key: str
    sent_at: datetime
    dedup_count: int = 0
    escalation_level: int = 0
    metadata: dict[str, Any] = field(default_factory=dict)
```

#### `CircuitBreakerState`

Tracks circuit breaker state per provider.

```python
@dataclass
class CircuitBreakerState:
    provider: str
    failure_count: int = 0
    last_failure_time: Optional[datetime] = None
    is_open: bool = False
    opened_at: Optional[datetime] = None
```

## Usage Patterns

### Pattern 1: Default Configuration

```python
from neuralbudget.alert_dispatch_advanced import AlertDispatchManager

manager = AlertDispatchManager()  # Uses sensible defaults

summary = manager.dispatch_with_policies(
    mode="http",
    profile="strict",
    metric_data=...,
    evaluation_result=...,
    alerts_config=...,
)
```

### Pattern 2: Custom Retry Policy

```python
from neuralbudget.alert_dispatch_advanced import (
    AlertDispatchManager,
    RetryPolicy,
)

retry = RetryPolicy(
    max_retries=5,
    initial_delay_ms=50,
    backoff_multiplier=3.0,
)

manager = AlertDispatchManager(retry_policy=retry)
```

### Pattern 3: Multiple Policies

```python
from neuralbudget.alert_dispatch_advanced import (
    AlertDispatchManager,
    RetryPolicy,
    DeduplicationPolicy,
    EscalationPolicy,
    EscalationStep,
    EscalationAction,
)

manager = AlertDispatchManager(
    retry_policy=RetryPolicy(max_retries=3),
    dedup_policy=DeduplicationPolicy(window_seconds=600),
    escalation_policy=EscalationPolicy(
        steps=[
            EscalationStep(
                after_seconds=900,
                action=EscalationAction.ADD_CHANNELS,
                config={"channels": ["pagerduty"]},
            ),
        ]
    ),
)
```

### Pattern 4: Integration with NeuralBudgetClient

```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.alert_dispatch_advanced import AlertDispatchManager

client = NeuralBudgetClient().load_config("config.json")
manager = AlertDispatchManager()

result = client.evaluate(metric_data)

if result.get("violation"):
    summary = manager.dispatch_with_policies(
        mode="http",
        profile="strict",
        metric_data=metric_data,
        evaluation_result=result,
        alerts_config=client.config.get("alerts", {}),
    )
```

### Pattern 5: Health Monitoring

```python
import logging

manager = AlertDispatchManager()
logger = logging.getLogger(__name__)

# Periodic health check
while True:
    dedup_stats = manager.get_dedup_stats()
    cb_stats = manager.get_circuit_breaker_stats()
    
    logger.info(f"Dedup: {dedup_stats['total_dedup_preventions']} prevented")
    
    for p in cb_stats["providers"]:
        if p["is_open"]:
            logger.warning(f"Circuit breaker OPEN: {p['provider']}")
    
    # Cleanup
    removed = manager.cleanup_expired_dedup_entries()
    if removed > 0:
        logger.info(f"Cleaned {removed} expired entries")
    
    time.sleep(3600)  # Every hour
```

## Error Handling

All methods are type-safe and raise appropriate exceptions:

```python
try:
    summary = manager.dispatch_with_policies(
        mode="http",
        profile="strict",
        metric_data=...,
        evaluation_result=...,
        alerts_config=...,
    )
    
    if summary.failed > 0:
        logger.error(f"Failed to send to {summary.failed} providers")
    
except Exception as e:
    logger.error(f"Dispatch error: {e}")
    # Fallback notification to backup channel
```

## Logging

The module uses Python's standard logging:

```python
import logging

# Get module logger
logger = logging.getLogger("neuralbudget.alert_dispatch_advanced")
logger.setLevel(logging.INFO)

# Now see info about retries, circuit breaker, escalation
```

Log messages include:
- Retry attempts and backoff delays
- Circuit breaker state changes
- Escalation triggers
- Dedup preventions

## Performance

- Dedup key generation: O(1) - hash computation
- Retry delay calculation: O(1) - simple math
- Circuit breaker logic: O(1) - map lookup
- Escalation evaluation: O(n) where n = escalation steps (typically 3-5)
- Memory per tracked alert: ~500 bytes
- Typical memory usage: 50-250 KB (100-500 tracked alerts)

## See Also

- [Advanced Alert Dispatch Guide](../guides/advanced_alert_dispatch.md)
- [Basic Alert Dispatcher](../../python/neuralbudget/alerting.py)
- [Examples](../../../examples/python/advanced_alert_dispatch.py)
