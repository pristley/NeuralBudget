# Burn-Rate Forecasting & Multi-Window Alerts

Forecast error budget exhaustion and generate SRE workbook-style multi-window burn-rate alerts.

## Features

- **Multi-Window Burn Rates**: Calculate burn rates at 5m, 30m, 1h, and 6h windows
- **SRE Alert Rules**: Standard fast-burn, medium-burn, and slow-burn alert patterns  
- **Budget Exhaustion Forecasting**: Project when error budget will be exhausted
- **Severity Levels**: Automatically determine alert severity based on combined burn rates

## Quick Start

```python
import neuralbudget

# Create metric stream (errors over time, 1 second granularity)
metric_stream = [
    neuralbudget.MetricPoint(1, 1.0),      # Error at t=1
    neuralbudget.MetricPoint(2, 0.0),      # OK at t=2
    neuralbudget.MetricPoint(3, 1.0),      # Error at t=3
    # ... more points ...
    neuralbudget.MetricPoint(3600, 0.5),   # Continuing pattern
]

# Calculate multi-window burn rates
multi_window = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
print(f"5m burn rate: {multi_window.burn_rate_5m}")
print(f"1h burn rate: {multi_window.burn_rate_1h}")

# Evaluate using standard SRE rules
rules = neuralbudget.BurnRateAlertRule.standard_rules()
alerts = neuralbudget.evaluate_multi_window_alerts(multi_window, rules)

for alert in alerts:
    if alert.triggered:
        print(f"ALERT: {alert.message}")

# Forecast budget exhaustion
forecast = neuralbudget.forecast_budget_exhaustion(
    current_burn_rate=multi_window.burn_rate_1h,
    remaining_budget_seconds=3600.0,  # 1 hour remaining
    now=int(time.time())
)
print(f"Budget exhaustion in {forecast.time_to_exhaustion_seconds / 3600:.1f} hours")
```

## Architecture

The burn-rate forecasting system implements the Google SRE workbook approach:

### Multi-Window Burn Rates

Burn rates are calculated at different time scales to detect different failure modes:

- **5-minute window**: Detects rapid failures; used for fast-burn alerts
- **30-minute window**: Detects sustained issues; used for medium-burn alerts  
- **1-hour window**: Overall health indicator; used for forecasting
- **6-hour window**: Long-term trend; used for slow-burn alerts

### Alert Rules

Three alert rule types based on combined short/long window burn rates:

```
Fast-Burn Alert:
  - Short window (5m) > 10x × SLO target
  - AND Long window (1h) > 6x × SLO target
  - Time to exhaustion: < 1 hour

Medium-Burn Alert:
  - Short window (30m) > 3x × SLO target
  - AND Long window (6h) > 1x × SLO target
  - Time to exhaustion: 1-6 days

Slow-Burn Alert:
  - Short window (1h) > 1x × SLO target
  - AND Long window (30d) > 0.1x × SLO target
  - Time to exhaustion: 6+ days
```

### Time-to-Error-Exhaustion (TTEE)

Given current burn rate and remaining budget, forecast when the budget is exhausted:

```
time_to_exhaustion = remaining_budget / current_burn_rate
exhaustion_timestamp = now + time_to_exhaustion
```

## API Reference

### Classes

#### `MultiWindowBurnRate`

Burn rates calculated at standard windows.

**Properties:**
- `timestamp: i64` - Timestamp (Unix seconds)
- `burn_rate_5m: f64` - 5-minute burn rate
- `burn_rate_30m: f64` - 30-minute burn rate  
- `burn_rate_1h: f64` - 1-hour burn rate
- `burn_rate_6h: f64` - 6-hour burn rate

**Methods:**
- `to_dict() -> dict` - Convert to dictionary
- `to_json() -> str` - Serialize to JSON
- `to_yaml() -> str` - Serialize to YAML

#### `BurnRateAlertRule`

Alert rule definition.

**Properties:**
- `name: str` - Rule name (e.g., "fast-burn-1hr")
- `short_window_threshold: f64` - Short window burn rate threshold
- `long_window_threshold: f64` - Long window burn rate threshold
- `short_window_seconds: u64` - Short window duration
- `long_window_seconds: u64` - Long window duration
- `severity: str` - Severity level ("ok", "slow_burn", "medium_burn", "fast_burn", "critical_burn")

**Static Methods:**
- `fast_burn() -> BurnRateAlertRule` - Create fast-burn rule
- `medium_burn() -> BurnRateAlertRule` - Create medium-burn rule
- `slow_burn() -> BurnRateAlertRule` - Create slow-burn rule
- `standard_rules() -> List[BurnRateAlertRule]` - Get all standard rules

#### `BurnRateAlertResult`

Alert evaluation result.

**Properties:**
- `rule: BurnRateAlertRule` - The evaluated rule
- `triggered: bool` - Whether alert fired
- `short_burn_rate: f64` - Short window burn rate
- `long_burn_rate: f64` - Long window burn rate
- `message: str` - Alert message

#### `BudgetExhaustionForecast`

Budget exhaustion forecast.

**Properties:**
- `timestamp: i64` - Forecast timestamp
- `current_burn_rate: f64` - Current burn rate
- `remaining_budget_seconds: f64` - Remaining budget
- `time_to_exhaustion_seconds: f64` - Seconds until exhaustion
- `projected_exhaustion_timestamp: i64` - Projected exhaustion time
- `will_exhaust: bool` - Whether budget will exhaust

### Functions

#### `calculate_burn_rate_for_window(metric_stream, window_seconds) -> float`

Calculate burn rate for a specific time window.

**Parameters:**
- `metric_stream`: List of MetricPoint objects
- `window_seconds`: Window duration in seconds

**Returns:** Burn rate (0.0-1.0+)

#### `calculate_multi_window_burn_rate(metric_stream) -> MultiWindowBurnRate`

Calculate burn rates at standard windows.

**Parameters:**
- `metric_stream`: List of MetricPoint objects

**Returns:** MultiWindowBurnRate with 5m, 30m, 1h, 6h burn rates

#### `evaluate_burn_rate_alert(rule, short_burn, long_burn) -> BurnRateAlertResult`

Evaluate if an alert rule should fire.

**Parameters:**
- `rule`: BurnRateAlertRule
- `short_burn`: Short window burn rate
- `long_burn`: Long window burn rate

**Returns:** BurnRateAlertResult with trigger status

#### `evaluate_multi_window_alerts(multi_window, rules) -> List[BurnRateAlertResult]`

Evaluate all rules against multi-window burn rates.

**Parameters:**
- `multi_window`: MultiWindowBurnRate
- `rules`: List of BurnRateAlertRule

**Returns:** List of BurnRateAlertResult

#### `determine_overall_severity(alerts) -> str`

Determine highest severity from alert results.

**Parameters:**
- `alerts`: List of BurnRateAlertResult

**Returns:** Severity string ("ok", "slow_burn", "medium_burn", "fast_burn", "critical_burn")

#### `forecast_budget_exhaustion(current_burn_rate, remaining_budget_seconds, now) -> BudgetExhaustionForecast`

Forecast when error budget will be exhausted.

**Parameters:**
- `current_burn_rate`: Current burn rate (from 1h window)
- `remaining_budget_seconds`: Remaining budget in seconds
- `now`: Current timestamp (Unix seconds)

**Returns:** BudgetExhaustionForecast

## Examples

### Example 1: Basic Burn-Rate Monitoring

```python
import neuralbudget
import time

def monitor_service_burn_rate(error_stream):
    # Calculate multi-window burn rates
    multi_window = neuralbudget.calculate_multi_window_burn_rate(error_stream)
    
    # Evaluate against standard SRE rules
    rules = neuralbudget.BurnRateAlertRule.standard_rules()
    alerts = neuralbudget.evaluate_multi_window_alerts(multi_window, rules)
    
    # Determine severity
    severity = neuralbudget.determine_overall_severity(alerts)
    
    return {
        "timestamp": multi_window.timestamp,
        "burn_rates": {
            "5m": multi_window.burn_rate_5m,
            "30m": multi_window.burn_rate_30m,
            "1h": multi_window.burn_rate_1h,
            "6h": multi_window.burn_rate_6h,
        },
        "alerts": [
            {
                "rule": a.rule.name,
                "triggered": a.triggered,
                "severity": a.rule.severity,
            }
            for a in alerts
        ],
        "overall_severity": severity,
    }
```

### Example 2: Custom Alert Rules

```python
# Create a custom alert rule for critical burn rates
critical_rule = neuralbudget.BurnRateAlertRule(
    name="critical-burn-30m",
    short_window_threshold=20.0,
    long_window_threshold=10.0,
    short_window_seconds=300,
    long_window_seconds=3_600,
    severity="critical_burn"
)

# Evaluate custom rule
result = neuralbudget.evaluate_burn_rate_alert(
    critical_rule,
    short_burn=25.0,
    long_burn=12.0
)

if result.triggered:
    print(f"CRITICAL: {result.message}")
```

### Example 3: Budget Exhaustion Forecasting

```python
# Current state
slo_target = 0.999  # 99.9% availability
time_window_seconds = 86_400  # 1 day

# Calculate remaining budget
error_budget_seconds = neuralbudget.calculate_error_budget(
    slo_target,
    time_window_seconds
)

# Get current burn rate
multi_window = neuralbudget.calculate_multi_window_burn_rate(error_stream)
current_burn_rate = multi_window.burn_rate_1h

# Forecast exhaustion
forecast = neuralbudget.forecast_budget_exhaustion(
    current_burn_rate=current_burn_rate,
    remaining_budget_seconds=error_budget_seconds,
    now=int(time.time())
)

if forecast.will_exhaust:
    exhaustion_time = datetime.fromtimestamp(
        forecast.projected_exhaustion_timestamp
    )
    hours_remaining = forecast.time_to_exhaustion_seconds / 3600
    print(f"Budget will exhaust in {hours_remaining:.1f} hours ({exhaustion_time})")
else:
    print("Budget is stable at current burn rate")
```

### Example 4: Prometheus Integration

```python
# Export burn rates to Prometheus
exporter = neuralbudget.PrometheusExporter(namespace="myapp_slo")

multi_window = neuralbudget.calculate_multi_window_burn_rate(error_stream)

# Add custom metrics
exporter.set_static_label("service", "api-gateway")
exporter.observe_error_budget("api-gateway", error_budget)

# Render Prometheus format
metrics_text = exporter.render()
print(metrics_text)
```

## Integration with Observability Platforms

### Grafana Dashboards

Use burn-rate metrics to create Grafana dashboards:

```json
{
  "panels": [
    {
      "title": "Multi-Window Burn Rate",
      "targets": [
        {
          "expr": "neuralbudget_burn_rate_5m{service='myapp'}"
        },
        {
          "expr": "neuralbudget_burn_rate_1h{service='myapp'}"
        }
      ]
    },
    {
      "title": "Time to Budget Exhaustion",
      "targets": [
        {
          "expr": "neuralbudget_time_to_exhaustion_seconds{service='myapp'} / 3600"
        }
      ]
    }
  ]
}
```

### Alert Manager Rules

Define AlertManager rules for burn-rate alerts:

```yaml
groups:
  - name: slo_burn_rate
    rules:
      - alert: FastBurnRate
        expr: |
          (neuralbudget_burn_rate_5m > 10) AND (neuralbudget_burn_rate_1h > 6)
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Fast burn rate detected"
```

## Reference

- [Google SRE Workbook: Alerting on SLOs](https://sre.google/books/site-reliability-engineering-workbook/)
- [Error Budget Calculator](https://cloud.google.com/blog/products/devops-sre/sre-fundamentals-error-budgets)
- [Monitoring Distributed Systems](https://sre.google/books/site-reliability-engineering/#monitoring_distributed_systems_8)
