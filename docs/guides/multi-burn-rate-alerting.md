# Multi-Burn-Rate Alerting Based on Google SRE

This guide explains NeuralBudget's implementation of baked-in alerting thresholds using the multi-burn-rate strategy from Google's Site Reliability Engineering handbook.

## The Problem

Traditional alerting on SLO metrics like availability is either too noisy (fires constantly) or too late (by the time you're alerted, error budget is already exhausted).

**Google SRE Solution:** Multi-burn-rate alerting checks for elevated error rates across multiple time windows with different sensitivities, allowing operators to respond proportionally to the severity of degradation.

## How It Works

### SLO Error Budget Concept

For any SLO, you have a fixed error budget over a time period:

```
SLO: 99.9% availability (0.999)
Error Budget: 1 - 0.999 = 0.001 (0.1%)
Monthly Window: 30 days × 24 hours = 720 hours
Monthly Budget: 0.001 × 720 = 0.72 hours ≈ 43 minutes/month
```

### Multi-Window Burn Rates

Instead of alerting on absolute error rate, alert on **burn rate** (how fast error budget is consumed):

| Window | Rate | Threshold | Duration | Severity | Interpretation |
|--------|------|-----------|----------|----------|-----------------|
| **1h** | 10x | 0.01 (1%) | 1 min | 🔴 Critical | Budget exhausted in ~10 hours if continues |
| **6h** | 2x | 0.002 (0.2%) | 15 min | 🟠 Warning | Budget exhausted in ~3 days if continues |
| **24h** | 0.5x | 0.0005 (0.05%) | 1 hour | 🟡 Info | Budget exhausted in ~30 days if continues |
| **3d** | 1x | 0.001 (0.1%) | 3 hours | 🟠 Warning | Budget fully exhausted soon |

### Why Multiple Windows?

1. **Fast window (1h @ 10x):** Catches acute outages immediately → immediate page
2. **Medium window (6h @ 2x):** Detects sustained degradation → urgent investigation
3. **Slow window (24h @ 0.5x):** Tracks trends and patterns → plan remediation
4. **Longest window (3d @ 1x):** Predicts budget exhaustion → post-incident review

## Configuration

### Default Configuration (4-Window Pattern)

```yaml
# slo.yaml
service: "payment-api"
availability_threshold: 0.999    # 99.9% SLO
latency_threshold_ms: 200

# Default multi-window alerting is ALWAYS enabled
# These can be overridden per SLO:
alerts:
  multi_burn_rate:
    windows:
      - duration: 1h
        burn_rate: 10.0
        for: 1m
        severity: critical
      
      - duration: 6h
        burn_rate: 2.0
        for: 15m
        severity: warning
      
      - duration: 24h
        burn_rate: 0.5
        for: 1h
        severity: info
      
      - duration: 3d
        burn_rate: 1.0
        for: 3h
        severity: warning
```

### Custom Configuration

Adjust burn rates for your service characteristics:

```yaml
# Aggressive SLO (e.g., payment processing)
alerts:
  multi_burn_rate:
    windows:
      - duration: 30m
        burn_rate: 50.0      # Very aggressive
        for: 30s
        severity: critical
      
      - duration: 2h
        burn_rate: 5.0
        for: 5m
        severity: warning

# Lenient SLO (e.g., batch processing)
alerts:
  multi_burn_rate:
    windows:
      - duration: 6h
        burn_rate: 5.0
        for: 1h
        severity: warning
      
      - duration: 7d
        burn_rate: 1.0
        for: 6h
        severity: info
```

## Burn Rate Threshold Calculation

For each window and burn rate, NeuralBudget automatically calculates the error rate threshold:

```
Allowed Error Rate = 1 - availability_target
Error Rate Threshold = Allowed Error Rate × Burn Rate Multiplier

Example (99.9% SLO):
- Allowed Error Rate = 0.001
- 1h window @ 10x: threshold = 0.001 × 10 = 0.01 (1% error rate)
- 6h window @ 2x: threshold = 0.001 × 2 = 0.002 (0.2% error rate)
```

## Prometheus Integration

Generated Prometheus recording rules for burn rate tracking:

```promql
# Recording rules (evaluated every 30s)
neuralbudget:slo:burn_rate_1h = error_rate / allowed_error_rate
neuralbudget:slo:burn_rate_6h = error_rate / allowed_error_rate
neuralbudget:slo:burn_rate_24h = error_rate / allowed_error_rate
neuralbudget:slo:burn_rate_3d = error_rate / allowed_error_rate

# Alerting rules (checked every 1 minute)
SloErrorBudgetBurnRate1h fires if error_rate > threshold for 1m
SloErrorBudgetBurnRate6h fires if error_rate > threshold for 15m
SloErrorBudgetBurnRate24h fires if error_rate > threshold for 1h
SloErrorBudgetBurnRate3d fires if error_rate > threshold for 3h
```

## Real-World Example

### Scenario: Payment API (99.95% SLO)

**Config:**
```yaml
service: "payment-api"
availability_threshold: 0.9995    # 99.95%
```

**Calculated Thresholds:**
```
Monthly Budget: 0.0005 × 720h = 0.36 hours ≈ 21.6 minutes

1h window @ 10x:
  Error threshold = 0.0005 × 10 = 0.005 (0.5%)
  Fires if: error_rate > 0.5% for 1 minute
  Interpretation: ~2 hours until budget exhausted

6h window @ 2x:
  Error threshold = 0.0005 × 2 = 0.001 (0.1%)
  Fires if: error_rate > 0.1% for 15 minutes
  Interpretation: ~3 days until budget exhausted

24h window @ 0.5x:
  Error threshold = 0.0005 × 0.5 = 0.00025 (0.025%)
  Fires if: error_rate > 0.025% for 1 hour
  Interpretation: Full month on current trajectory

3d window @ 1x:
  Error threshold = 0.0005 × 1 = 0.0005 (0.05%)
  Fires if: error_rate > 0.05% for 3 hours
  Interpretation: Budget exhausted soon
```

### Incident Timeline

```
12:00 PM: Error rate spikes to 1.5% (disk full issue)
12:01 PM: 1h @ 10x alert FIRES → Critical
         (error rate 1.5% > threshold 0.5%)
         On-call page: "Payment API burning budget at 30x rate"

12:15 PM: On-call starts investigation
         "Payment API disk full, clearing cache..."

12:30 PM: Incident escalates, database failover initiated
         Error rate now 0.3%
         1h alert still firing (0.3% > 0.5%? No, resolves)
         6h alert now FIRES → Warning
         (0.3% > 0.1% for 15min)

12:45 PM: Failover complete, error rate drops to 0.05%
         All alerts resolve (below thresholds)
         Post-incident: "We had 45 min of 30x burn, used ~2.25% of monthly budget"

Analysis: Without multi-burn-rate alerts:
- Traditional availability alert (99.9%): Fires 6 min late, after 10% of budget spent
- Our 1h window: Fired in 1 min, early warning enabled quick response
- Savings: ~7% of budget from faster response
```

## Tuning Guidelines

### For Critical Services (99.99%+ SLO)

```yaml
alerts:
  multi_burn_rate:
    windows:
      - duration: 30m
        burn_rate: 100        # Very aggressive
        for: 30s
        severity: critical
      
      - duration: 1h
        burn_rate: 10
        for: 5m
        severity: warning
```

### For Standard Services (99.5-99.9% SLO)

Use the default 4-window configuration (recommended).

### For Best-Effort Services (95-99% SLO)

```yaml
alerts:
  multi_burn_rate:
    windows:
      - duration: 2h
        burn_rate: 5
        for: 30m
        severity: warning
      
      - duration: 1d
        burn_rate: 1
        for: 6h
        severity: info
```

## Alertmanager Routing

Route alerts based on burn rate severity:

```yaml
# alertmanager.yml
route:
  routes:
    # Critical burns (1h @ 10x) → immediate page
    - match:
        alertname: SloErrorBudgetBurnRate1h
      receiver: pagerduty
      group_wait: 0s
      repeat_interval: 5m
    
    # Medium burns (6h @ 2x) → urgent ticket
    - match:
        alertname: SloErrorBudgetBurnRate6h
      receiver: slack-oncall
      group_wait: 5m
      repeat_interval: 2h
    
    # Slow burns (24h/3d) → analytics only
    - match:
        alertname: 'SloErrorBudgetBurnRate(24h|3d)'
      receiver: datadog
      group_wait: 15m
      repeat_interval: 1d
```

## Validation

NeuralBudget validates multi-window configurations to prevent alert storms:

```python
# Python API
from neuralbudget import MultiWindowAlertConfig

config = MultiWindowAlertConfig.default_four_window()
config.validate()  # Raises error if windows conflict

# Custom configuration with validation
config = (MultiWindowAlertConfig()
    .with_window(BurnRateWindow.new("1h", 10, "1m", AlertSeverity.Critical))
    .with_window(BurnRateWindow.new("6h", 2, "15m", AlertSeverity.Warning)))
config.validate()
```

## Limitations & Trade-offs

### False Positives

Multi-window alerting reduces false positives compared to single-metric alerts, but:
- Requires 1-15+ minutes for first alert (by design)
- May miss extremely brief spikes
- Needs realistic thresholds for your service

### Configuration Complexity

- More windows = more alerts to route and act on
- Requires understanding of burn rates and error budgets
- Threshold tuning is service-specific

### Performance

NeuralBudget's burn rate calculations are O(1) per window and negligible overhead.

## See Also

- [Google SRE Handbook: Error Budgets](https://sre.google/books/)
- [Google SRE Workbook: Alerting on Burn Rate](https://sre.google/workbook/)
- [Sloth SLO Generator](https://sloth.dev/)
- [Prometheus Rule Generation Guide](prometheus-rule-generation.md)

## CLI Usage

Default multi-window alerting is automatically applied by `gen-rules`:

```bash
# Generates alerting rules with default 4-window configuration
neuralbudget gen-rules slo.yaml --kubernetes

# Shows which windows are being used
neuralbudget check slo.yaml --verbose
```
