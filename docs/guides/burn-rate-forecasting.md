# Burn-Rate Forecasting Implementation Guide

This guide explains how to implement and integrate burn-rate forecasting using the Google SRE workbook approach in your service monitoring.

## Overview

Burn-rate forecasting predicts when your error budget will be exhausted based on current failure patterns. It uses multi-window analysis to detect different failure modes and provide actionable alerts.

**Key concepts:**
- **Multi-window analysis**: Calculate burn rates at 5m, 30m, 1h, and 6h to detect patterns
- **Alert rules**: Combine short and long window thresholds to avoid false positives
- **Severity levels**: Escalate response based on exhaustion timeline
- **Time-to-exhaustion (TTEE)**: Forecast when budget runs out

## Architecture

### System Components

```
┌─────────────────────────────────────────┐
│  Error/Request Stream                   │
│  (from observability platform)          │
└──────────────┬──────────────────────────┘
               │
        ┌──────▼───────┐
        │ Metric Points│
        │  (1s grains) │
        └──────┬───────┘
               │
        ┌──────▼─────────────────────────────┐
        │  Multi-Window Burn Rate Calculator │
        │  - 5m window                       │
        │  - 30m window                      │
        │  - 1h window                       │
        │  - 6h window                       │
        └──────┬─────────────────────────────┘
               │
     ┌─────────┼──────────────────┐
     │         │                  │
┌────▼──┐  ┌───▼────┐  ┌────────▼────┐
│Alert  │  │Severity│  │TTEE Forecast│
│Engine │  │Engine  │  │(Budget Info) │
└─┬──┬──┘  └───┬────┘  └────────┬────┘
  │  │        │                 │
  └──┼────────┼─────────────────┘
     │        │
  ┌──▼────────▼──────────────┐
  │ Alerting Platform        │
  │ (Prometheus, PagerDuty)  │
  └─────────────────────────┘
```

### Data Flow

1. **Metric Collection**: Observability platform collects error/success events
2. **Aggregation**: Group into 1-second granularity `MetricPoint`s
3. **Multi-Window Analysis**: Calculate burn rates at standard intervals
4. **Alert Evaluation**: Test against configurable alert rules
5. **Severity Determination**: Combine all alerts into overall status
6. **Time-to-Exhaustion**: Calculate remaining time at current burn rate

## Integration Patterns

### Pattern 1: Real-Time Monitoring

Monitor burn rates continuously and alert on threshold violations:

```python
import neuralbudget
import time

# In your monitoring loop
def monitor_loop():
    while True:
        # Get last hour of metrics
        metric_stream = get_recent_metrics(lookback_seconds=3600)
        
        # Calculate current burn rates
        multi_window = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        
        # Get SRE workbook rules
        rules = neuralbudget.BurnRateAlertRule.standard_rules()
        
        # Evaluate all rules
        alerts = neuralbudget.evaluate_multi_window_alerts(multi_window, rules)
        
        # Determine severity
        severity = neuralbudget.determine_overall_severity(alerts)
        
        # Send to monitoring platform
        send_metrics({
            "burn_rate_1h": multi_window.burn_rate_1h,
            "severity": severity,
            "alerts_triggered": sum(1 for a in alerts if a.triggered),
        })
        
        time.sleep(60)  # Check every minute
```

### Pattern 2: SLO-Based Alerting

Alert based on remaining budget and burn rate:

```python
def slo_alert_handler(metric_stream, slo_config):
    # Get current state
    multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
    
    # Calculate remaining budget for the month
    time_in_month = time.time() % (30 * 86_400)
    remaining_seconds = (30 * 86_400) - time_in_month
    remaining_budget = slo_config["error_budget"] * (remaining_seconds / (30 * 86_400))
    
    # Forecast exhaustion
    forecast = neuralbudget.forecast_budget_exhaustion(
        current_burn_rate=multi.burn_rate_6h,
        remaining_budget_seconds=remaining_budget,
        now=int(time.time())
    )
    
    # Alert if budget will exhaust before SLO window ends
    if forecast.will_exhaust:
        hours_to_exhaustion = forecast.time_to_exhaustion_seconds / 3600
        if hours_to_exhaustion < 24:
            severity = "critical"
        elif hours_to_exhaustion < 48:
            severity = "warning"
        else:
            severity = "info"
        
        alert(severity, f"Budget exhaustion in {hours_to_exhaustion:.1f}h")
```

### Pattern 3: Custom Alert Rules

Define rules for your specific service requirements:

```python
# For a critical payment service - need stricter thresholds
payment_service_rules = [
    neuralbudget.BurnRateAlertRule(
        name="payment-critical-1m",
        short_window_threshold=100.0,  # 100x in 1 minute
        long_window_threshold=50.0,
        short_window_seconds=60,
        long_window_seconds=300,
        severity="critical_burn"
    ),
    neuralbudget.BurnRateAlertRule(
        name="payment-fast-5m",
        short_window_threshold=20.0,
        long_window_threshold=10.0,
        short_window_seconds=300,
        long_window_seconds=3600,
        severity="fast_burn"
    ),
]
```

### Pattern 4: Multi-Service Dashboard

Aggregate burn rates across services:

```python
def generate_dashboard_data(services):
    data = {}
    
    for service_name in services:
        metric_stream = get_metrics(service_name)
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        
        # Get remaining budget
        budget_info = get_service_budget(service_name)
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=multi.burn_rate_1h,
            remaining_budget_seconds=budget_info["remaining_seconds"],
            now=int(time.time())
        )
        
        data[service_name] = {
            "burn_rates": {
                "5m": multi.burn_rate_5m,
                "30m": multi.burn_rate_30m,
                "1h": multi.burn_rate_1h,
                "6h": multi.burn_rate_6h,
            },
            "ttee_hours": forecast.time_to_exhaustion_seconds / 3600,
            "will_exhaust": forecast.will_exhaust,
        }
    
    return data
```

## Configuration

### Standard Alert Rules (Google SRE Workbook)

The default rules follow Google's SRE patterns:

| Rule | Short Window | Long Window | Alert If | Implies |
|------|--------------|-------------|----------|---------|
| Fast-Burn | 5m > 10x | 1h > 6x | Budget exhausted in <1h | Page on-call immediately |
| Medium-Burn | 30m > 3x | 6h > 1x | Budget exhausted in 1-6d | Page on-call, prepare response |
| Slow-Burn | 1h > 1x | 30d > 0.1x | Budget exhausted in 6+ days | Create incident, schedule fixes |

### Customization

Adjust thresholds based on service characteristics:

```python
# For high-reliability services (99.99% SLO)
strict_rules = [
    neuralbudget.BurnRateAlertRule(
        name="ultra-fast-burn",
        short_window_threshold=200.0,
        long_window_threshold=100.0,
        short_window_seconds=60,
        long_window_seconds=300,
        severity="critical_burn"
    ),
]

# For best-effort services
loose_rules = [
    neuralbudget.BurnRateAlertRule(
        name="moderate-burn",
        short_window_threshold=5.0,
        long_window_threshold=2.0,
        short_window_seconds=600,
        long_window_seconds=7_200,
        severity="medium_burn"
    ),
]
```

## Performance Considerations

### Calculation Efficiency

Burn-rate calculations are O(n) where n = number of metric points:

- 5-minute window: ~300 points (1 Hz granularity)
- 1-hour window: ~3,600 points
- 6-hour window: ~21,600 points
- 30-day window: ~2.6M points

**Optimization strategies:**

1. **Streaming updates**: Maintain running window averages instead of recalculating
2. **Bucketing**: Pre-aggregate into larger time buckets
3. **Sampling**: For very long windows (30d), sample every 10 seconds

### Memory Usage

```python
# Estimate memory for metric stream
metric_count = 3600  # 1 hour at 1 Hz
bytes_per_metric = 16  # i64 timestamp + f64 value
total_mb = (metric_count * bytes_per_metric) / (1024 * 1024)
# ~55 KB per hour of data
```

## Testing

### Unit Tests

Basic functionality tests:

```python
def test_burn_rate_calculation():
    metrics = [MetricPoint(i, i % 2) for i in range(10)]
    rate = calculate_burn_rate_for_window(metrics, 300)
    assert 0.0 <= rate <= 1.0

def test_alert_evaluation():
    rule = BurnRateAlertRule.fast_burn()
    result = evaluate_burn_rate_alert(rule, 15.0, 8.0)
    assert result.triggered  # Should trigger
```

### Integration Tests

End-to-end workflow tests:

```python
def test_degradation_detection():
    # Simulate normal → degraded → recovery
    stream = simulate_degradation_event(duration=10_000)
    
    multi = calculate_multi_window_burn_rate(stream)
    rules = BurnRateAlertRule.standard_rules()
    alerts = evaluate_multi_window_alerts(multi, rules)
    
    # Should detect the degradation
    assert any(a.triggered for a in alerts)
```

### Load Tests

Verify performance under realistic load:

```python
def benchmark_burn_rate_calculation():
    # Generate 1 week of metrics
    metrics = [MetricPoint(i, random()) for i in range(604_800)]
    
    start = time.time()
    multi = calculate_multi_window_burn_rate(metrics)
    elapsed = time.time() - start
    
    assert elapsed < 1.0  # Should complete in < 1 second
```

## Troubleshooting

### Issue: Alerts not triggering

**Causes:**
- Thresholds too high
- Burn rate calculation window too small
- Metric stream missing data points

**Solution:**
```python
# Debug: Check actual burn rates
multi = calculate_multi_window_burn_rate(stream)
print(f"Burn rates: 5m={multi.burn_rate_5m}, 1h={multi.burn_rate_1h}")

# Verify rule thresholds
rule = BurnRateAlertRule.fast_burn()
print(f"Rule requires: short>{rule.short_window_threshold}, long>{rule.long_window_threshold}")
```

### Issue: Too many false positives

**Causes:**
- Thresholds too low
- Not enough data points in window
- Noisy metrics

**Solution:**
```python
# Increase thresholds
more_conservative_rules = [
    BurnRateAlertRule(
        name="...",
        short_window_threshold=25.0,  # Increased from 10.0
        long_window_threshold=12.0,   # Increased from 6.0
        ...
    )
]
```

### Issue: TTEE forecast inaccurate

**Causes:**
- Burn rate is varying over time
- Didn't account for recent degradation
- Remaining budget calculation incorrect

**Solution:**
```python
# Use shorter window for more responsive forecast
# Recalculate forecast frequently
forecast_interval = 60  # Update every minute

# Use latest burn rate only
latest_multi = calculate_multi_window_burn_rate(recent_stream)
forecast = forecast_budget_exhaustion(
    current_burn_rate=latest_multi.burn_rate_1h,  # Use latest
    remaining_budget_seconds=...,
    now=int(time.time())
)
```

## Best Practices

1. **Use standard rules as starting point**: Start with SRE workbook defaults, customize based on SLO
2. **Combine with context**: Don't alert on burn rates alone - include recent changes, deployment info
3. **Set response runbooks**: Define clear steps for each alert severity level
4. **Monitor the monitor**: Track alert accuracy, false positive rate, detection latency
5. **Review regularly**: Quarterly audit of alert rules and thresholds
6. **Test alert paths**: Regularly fire test alerts to verify notification delivery
7. **Document thresholds**: Record why you chose specific burn-rate thresholds

## References

- [Google SRE Workbook: Alerting on SLOs](https://sre.google/books/site-reliability-engineering-workbook/ch05-01-alerting-on-slos/)
- [Google SRE Book: Monitoring Distributed Systems](https://sre.google/books/site-reliability-engineering/#monitoring_distributed_systems_8)
- [Error Budget Calculator](https://www.cloudflare.com/learning/performance/error-budget/)
- [NeuralBudget Documentation](../guides/documentation-index.md)
