# Advanced Alert Dispatch System

Enterprise-grade alert delivery with retry policies, deduplication, and escalation.

## Overview

The advanced alert dispatch system extends the basic webhook integration with production-ready reliability features:

- **Retry Policies**: Automatic retries with exponential backoff and jitter
- **Deduplication**: Prevent alert storms from duplicate notifications
- **Escalation**: Automatically escalate unresolved alerts over time
- **Circuit Breaker**: Fail fast when providers are unhealthy, recover gracefully

## Architecture

```
┌─────────────────────────────────────────┐
│  SLO Evaluation Result                  │
│  (violation detected)                   │
└──────────────┬──────────────────────────┘
               │
        ┌──────▼────────────────┐
        │ AlertDispatchManager  │
        └──────┬────────────────┘
               │
     ┌─────────┼──────────────┬──────────┐
     │         │              │          │
┌────▼──┐  ┌───▼───┐  ┌─────▼─┐  ┌────▼────┐
│Retry  │  │ Dedup │  │Circuit │  │Escalate │
│Engine │  │ Check │  │Breaker │  │Engine   │
└────┬──┘  └───┬───┘  └─────┬─┘  └────┬────┘
     │         │            │         │
     └─────────┼────────────┼─────────┘
               │            │
        ┌──────▼────────────▼──┐
        │ AlertDispatcher      │
        │ (stdlib urllib)      │
        └──────┬─────────┬──────┘
               │         │
        ┌──────▼┐   ┌────▼─────┐
        │Slack  │   │PagerDuty  │
        │Webhook│   │  Events   │
        └───────┘   └───────────┘
```

## Core Concepts

### Retry Policy

Handles transient failures with exponential backoff and jitter.

**When to use:**
- Network timeouts
- Provider rate limiting
- Temporary service unavailability

**Configuration:**
```python
from neuralbudget.alert_dispatch_advanced import RetryPolicy

# Standard: 3 retries with exponential backoff
retry_policy = RetryPolicy(
    max_retries=3,
    initial_delay_ms=100,
    max_delay_ms=30_000,
    backoff_multiplier=2.0,
    jitter_percent=10.0,
)

# Aggressive: For critical services
critical_retry = RetryPolicy(
    max_retries=5,
    initial_delay_ms=50,
    backoff_multiplier=3.0,
    use_circuit_breaker=False,  # Always retry
)

# Conservative: For best-effort services
lenient_retry = RetryPolicy(
    max_retries=1,
    initial_delay_ms=500,
    use_circuit_breaker=True,   # Fail fast
)
```

**Delay Calculation:**
```
delay = min(initial_delay * (multiplier ^ attempt), max_delay)
delay += (delay * jitter_percent/100 * random)
```

**Example sequence (standard policy):**
- Attempt 1: Fail → Wait 100ms (±10ms)
- Attempt 2: Fail → Wait 200ms (±20ms)
- Attempt 3: Fail → Wait 400ms (±40ms)
- Attempt 4: Success ✓

### Deduplication Policy

Prevents alert storms from repeatedly triggered identical alerts.

**When to use:**
- Preventing duplicate Slack messages
- Reducing noise in alert channels
- Tracking alert recurrence

**Configuration:**
```python
from neuralbudget.alert_dispatch_advanced import DeduplicationPolicy

# Standard: 5-minute window
dedup_policy = DeduplicationPolicy(
    enabled=True,
    window_seconds=300,
    key_strategy="content_hash",
)

# Strict: 10-second window (more aggressive dedup)
strict_dedup = DeduplicationPolicy(
    enabled=True,
    window_seconds=10,
)

# Lenient: 30-minute window (rarely deduplicate)
lenient_dedup = DeduplicationPolicy(
    enabled=True,
    window_seconds=1800,
)

# Disabled: For critical alerts that should never be deduplicated
no_dedup = DeduplicationPolicy(enabled=False)
```

**Flow:**
1. Alert triggered at T=0s → Generate dedup key (hash of content)
2. Identical alert at T=60s → Check if in dedup window → SKIP
3. Identical alert at T=400s → Window expired → SEND

**Dedup key strategies:**
- `content_hash`: Hash (mode, profile, result) → stable key
- `custom`: Use explicit dedup_key from config

### Circuit Breaker

Protects against cascading failures by failing fast when providers are unhealthy.

**When to use:**
- Provider is consistently failing
- Want to avoid wasting retries
- Need to trigger fallback channels

**Configuration:**
```python
from neuralbudget.alert_dispatch_advanced import RetryPolicy

retry_policy = RetryPolicy(
    use_circuit_breaker=True,
    circuit_breaker_threshold=5,        # Open after 5 failures
    circuit_breaker_open_seconds=60,    # Stay open for 60s
)
```

**State transitions:**
```
CLOSED (normal)
  ↓
  └─ Failure #5 → OPEN (fail fast)
                    ↓
                 [60s passes]
                    ↓
                  HALF-OPEN (try one request)
                    ↓
                    Success → CLOSED
                    or
                    Failure → OPEN (retry timer)
```

**Benefits:**
- Reduce wasted retries
- Faster failure detection
- Automatic recovery when provider heals

### Escalation Policy

Automatically escalate unresolved alerts to ensure critical issues get attention.

**When to use:**
- Alert not acknowledged after N minutes
- Need to page on-call
- Route to different teams for different durations

**Configuration:**
```python
from neuralbudget.alert_dispatch_advanced import (
    EscalationPolicy,
    EscalationStep,
    EscalationAction,
)

escalation_policy = EscalationPolicy(
    enabled=True,
    steps=[
        # After 5 minutes: Add PagerDuty
        EscalationStep(
            after_seconds=300,
            action=EscalationAction.ADD_CHANNELS,
            config={
                "channels": ["pagerduty"],
                "pagerduty_config": {"severity": "error"},
            }
        ),
        # After 15 minutes: Increase severity
        EscalationStep(
            after_seconds=900,
            action=EscalationAction.INCREASE_SEVERITY,
            config={}
        ),
        # After 30 minutes: Add OpsGenie
        EscalationStep(
            after_seconds=1800,
            action=EscalationAction.ADD_CHANNELS,
            config={
                "channels": ["opsgenie"],
                "opsgenie_config": {"priority": "P1"},
            }
        ),
    ],
    max_escalations=10,
)
```

**Available actions:**
- `ADD_CHANNELS`: Add notification channels (Slack, PagerDuty, OpsGenie)
- `INCREASE_SEVERITY`: Escalate severity level
- `ADD_TAGS`: Add tags to escalated alert
- `MODIFY_CONFIG`: Direct config modifications
- `FORCE_DISPATCH`: Force immediate dispatch to channel

## Usage

### Basic Usage

```python
from neuralbudget.alert_dispatch_advanced import (
    AlertDispatchManager,
    RetryPolicy,
    DeduplicationPolicy,
    EscalationPolicy,
    EscalationStep,
)

# Configure policies
retry_policy = RetryPolicy(
    max_retries=3,
    initial_delay_ms=100,
)

dedup_policy = DeduplicationPolicy(
    enabled=True,
    window_seconds=300,
)

escalation_policy = EscalationPolicy(enabled=False)  # Disabled for now

# Create manager
dispatch_mgr = AlertDispatchManager(
    retry_policy=retry_policy,
    dedup_policy=dedup_policy,
    escalation_policy=escalation_policy,
)

# Dispatch alert with all policies
summary = dispatch_mgr.dispatch_with_policies(
    mode="http",
    profile="strict_latency",
    metric_data=metric_data,
    evaluation_result=evaluation_result,
    alerts_config=alerts_config,
)

print(f"Attempted: {summary.attempted}")
print(f"Succeeded: {summary.succeeded}")
print(f"Failed: {summary.failed}")
```

### With NeuralBudgetClient

```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.alert_dispatch_advanced import AlertDispatchManager, RetryPolicy

# Initialize client
client = NeuralBudgetClient().load_config("config.json")

# Create advanced dispatch manager
dispatch_mgr = AlertDispatchManager(
    retry_policy=RetryPolicy(max_retries=3),
)

# Evaluate SLO
result = client.evaluate(metric_data)

# If violation, use advanced dispatch
if result.get("violation"):
    summary = dispatch_mgr.dispatch_with_policies(
        mode="http",
        profile="strict",
        metric_data=metric_data,
        evaluation_result=result,
        alerts_config=client.config.get("alerts", {}),
    )
```

### Monitoring Dispatch Health

```python
# Get deduplication statistics
dedup_stats = dispatch_mgr.get_dedup_stats()
print(f"Prevented duplicates: {dedup_stats['total_dedup_preventions']}")

# Get circuit breaker status
cb_stats = dispatch_mgr.get_circuit_breaker_stats()
for provider in cb_stats["providers"]:
    print(f"{provider['provider']}: {provider['failure_count']} failures")

# Get escalation history for specific alert
history = dispatch_mgr.get_escalation_history("http:strict:abc123")
for event in history:
    print(f"{event['timestamp']}: {event['action']}")

# Manual circuit breaker reset
dispatch_mgr.reset_circuit_breaker("pagerduty")

# Cleanup expired dedup entries
removed = dispatch_mgr.cleanup_expired_dedup_entries()
print(f"Cleaned up {removed} expired entries")
```

## Real-World Scenarios

### Scenario 1: Payment Service (Aggressive)

For revenue-impacting services:
```python
dispatch_mgr = AlertDispatchManager(
    retry_policy=RetryPolicy(
        max_retries=5,
        initial_delay_ms=50,
        use_circuit_breaker=False,  # Always try
    ),
    dedup_policy=DeduplicationPolicy(
        enabled=True,
        window_seconds=60,  # Very short window
    ),
    escalation_policy=EscalationPolicy(
        enabled=True,
        steps=[
            EscalationStep(after_seconds=300, action="add_channels", ...),
            EscalationStep(after_seconds=600, action="increase_severity", ...),
        ]
    )
)
```

### Scenario 2: Batch Processing (Moderate)

For non-critical but important services:
```python
dispatch_mgr = AlertDispatchManager(
    retry_policy=RetryPolicy(
        max_retries=3,
        initial_delay_ms=500,
        use_circuit_breaker=True,
    ),
    dedup_policy=DeduplicationPolicy(
        enabled=True,
        window_seconds=600,  # 10 minutes
    ),
    escalation_policy=EscalationPolicy(
        enabled=True,
        steps=[
            EscalationStep(after_seconds=1800, action="add_channels", ...),
        ]
    )
)
```

### Scenario 3: Monitoring Infrastructure (Conservative)

For observability systems:
```python
dispatch_mgr = AlertDispatchManager(
    retry_policy=RetryPolicy(
        max_retries=2,
        initial_delay_ms=1000,
        use_circuit_breaker=True,
    ),
    dedup_policy=DeduplicationPolicy(
        enabled=True,
        window_seconds=3600,  # 1 hour
    ),
    escalation_policy=EscalationPolicy(enabled=False)
)
```

## Troubleshooting

### Issue: Alerts Not Being Sent

**Possible causes:**
- Circuit breaker is open
- All alerts are being deduplicated
- Provider configuration is incorrect

**Diagnosis:**
```python
# Check circuit breaker state
cb_stats = dispatch_mgr.get_circuit_breaker_stats()
print(cb_stats)

# Check dedup tracking
dedup_stats = dispatch_mgr.get_dedup_stats()
print(f"Tracked: {dedup_stats['tracked_alerts']}")
print(f"Prevented: {dedup_stats['total_dedup_preventions']}")

# Reset circuit breaker if stuck
dispatch_mgr.reset_circuit_breaker("pagerduty")
```

### Issue: Too Many Alerts / Alert Storm

**Solutions:**
1. Increase dedup window
2. Adjust SLO thresholds
3. Add escalation to reduce noise

```python
# More aggressive deduplication
dedup_policy = DeduplicationPolicy(
    enabled=True,
    window_seconds=1800,  # 30 minutes
)
```

### Issue: Critical Alerts Not Escalating

**Possible causes:**
- Escalation policy disabled
- Steps not configured
- Alert resolved before escalation trigger

**Solution:**
```python
# Verify escalation is enabled
print(f"Escalation enabled: {dispatch_mgr.escalation_policy.enabled}")
print(f"Steps configured: {len(dispatch_mgr.escalation_policy.steps)}")

# Add escalation if missing
escalation_policy = EscalationPolicy(
    enabled=True,
    steps=[
        EscalationStep(
            after_seconds=600,
            action=EscalationAction.ADD_CHANNELS,
            config={"channels": ["pagerduty"]},
        ),
    ]
)
```

## Performance Considerations

### Memory Usage

- Each tracked alert: ~500 bytes (timestamp, counts, metadata)
- Typical scenario: 100-500 tracked alerts = 50-250 KB
- Dedup window cleanup: Run `cleanup_expired_dedup_entries()` periodically

### CPU Overhead

- Dedup key generation: O(1) - hash computation
- Retry delay calculation: O(1) - simple math
- Circuit breaker check: O(1) - map lookup
- Escalation logic: O(n) where n = escalation steps (typically 3-5)

### Network

- Retry logic: 1-5 HTTP requests per alert
- Escalation: +1-2 additional HTTP requests per escalation trigger
- Total: Minimal overhead compared to alert value

## Best Practices

1. **Start conservative**: Start with simple retry policy, add complexity as needed

2. **Monitor dispatch health**: Regularly check dedup stats and circuit breaker state

3. **Set appropriate dedup windows**:
   - Critical services: 1-5 minutes
   - Standard services: 5-15 minutes
   - Best-effort: 30+ minutes

4. **Test escalation paths**: Manually verify escalation works before enabling

5. **Clean up periodically**: Call `cleanup_expired_dedup_entries()` hourly

6. **Alert on dispatch failures**: Monitor `AlertDispatchSummary` for trends

7. **Use circuit breakers**: Enable for external services, disable for internal

8. **Document thresholds**: Keep escalation steps and retry counts in config

## Examples

See [examples/python/advanced_alert_dispatch.py](../../examples/python/advanced_alert_dispatch.py) for:
- Basic retry policy
- Deduplication strategies
- Circuit breaker usage
- Escalation policies
- Full dispatch workflow
- Health monitoring
- Real-world scenarios

## API Reference

See [neuralbudget.alert_dispatch_advanced](../../python/neuralbudget/alert_dispatch_advanced.py) module documentation.

### Key Classes

- `AlertDispatchManager`: Main class orchestrating all policies
- `RetryPolicy`: Configure retry behavior
- `DeduplicationPolicy`: Configure deduplication
- `EscalationPolicy`: Configure escalation steps
- `EscalationStep`: Individual escalation trigger
- `AlertDeduplicationEntry`: Track alert state
- `CircuitBreakerState`: Provider health state

### Key Methods

- `dispatch_with_policies()`: Main entry point for alert dispatch
- `get_dedup_stats()`: Get deduplication statistics
- `get_circuit_breaker_stats()`: Get provider health status
- `get_escalation_history()`: Get escalation events for alert
- `cleanup_expired_dedup_entries()`: Cleanup expired tracking
- `reset_circuit_breaker()`: Manual circuit breaker reset
