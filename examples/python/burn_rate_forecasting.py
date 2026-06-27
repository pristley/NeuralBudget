#!/usr/bin/env python3
"""
Burn-Rate Forecasting Example

Demonstrates multi-window burn rate calculation and SRE workbook-style alerting.
"""

import time
import json
import neuralbudget


def example_1_basic_burn_rate():
    """Example 1: Calculate basic burn rates"""
    print("\n=== Example 1: Basic Burn Rate Calculation ===\n")
    
    # Create a metric stream with errors over time
    # In practice, this would come from your observability stack
    metric_stream = [
        neuralbudget.MetricPoint(1, 1.0),      # Error
        neuralbudget.MetricPoint(2, 0.0),      # OK
        neuralbudget.MetricPoint(3, 1.0),      # Error
        neuralbudget.MetricPoint(4, 1.0),      # Error
        neuralbudget.MetricPoint(5, 0.0),      # OK
    ]
    
    # Calculate for specific windows
    burn_5m = neuralbudget.calculate_burn_rate_for_window(metric_stream, 300)
    burn_1h = neuralbudget.calculate_burn_rate_for_window(metric_stream, 3600)
    
    print(f"5-minute burn rate: {burn_5m:.4f}")
    print(f"1-hour burn rate: {burn_1h:.4f}")
    print("\nInterpretation:")
    print("- Burn rate 0.0 = no errors (100% availability)")
    print("- Burn rate 1.0 = all requests failed (0% availability)")
    print("- Burn rate 0.002 = 0.2% error rate")


def example_2_multi_window():
    """Example 2: Multi-window burn rates"""
    print("\n=== Example 2: Multi-Window Burn Rates ===\n")
    
    # Simulate 1 hour of traffic with some errors
    metric_stream = []
    now = int(time.time())
    
    # Simulate: 95% success rate (roughly 0.05 burn rate)
    for i in range(3600):
        timestamp = now - 3600 + i
        # 5% error rate
        is_error = (i % 20) == 0
        metric_stream.append(
            neuralbudget.MetricPoint(timestamp, 1.0 if is_error else 0.0)
        )
    
    # Calculate multi-window burn rates
    multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
    
    print(f"Timestamp: {multi.timestamp}")
    print(f"5-minute burn rate:  {multi.burn_rate_5m:.4f}")
    print(f"30-minute burn rate: {multi.burn_rate_30m:.4f}")
    print(f"1-hour burn rate:    {multi.burn_rate_1h:.4f}")
    print(f"6-hour burn rate:    {multi.burn_rate_6h:.4f}")
    
    # Export to JSON/YAML
    print("\nJSON representation:")
    print(json.dumps(json.loads(multi.to_json()), indent=2))


def example_3_alert_rules():
    """Example 3: SRE alert rules"""
    print("\n=== Example 3: SRE Alert Rules ===\n")
    
    # Get standard SRE workbook rules
    rules = neuralbudget.BurnRateAlertRule.standard_rules()
    
    print(f"Loaded {len(rules)} standard rules:\n")
    for rule in rules:
        print(f"Rule: {rule.name}")
        print(f"  Severity: {rule.severity}")
        print(f"  Short window: {rule.short_window_seconds}s > {rule.short_window_threshold}x")
        print(f"  Long window:  {rule.long_window_seconds}s > {rule.long_window_threshold}x")
        print()


def example_4_alert_evaluation():
    """Example 4: Evaluate alerts"""
    print("\n=== Example 4: Alert Evaluation ===\n")
    
    # Create burn rates that would trigger alerts
    print("Testing different burn rate scenarios:\n")
    
    # Scenario 1: Low burn rate (everything OK)
    print("Scenario 1: Low burn rate")
    rule = neuralbudget.BurnRateAlertRule.fast_burn()
    result = neuralbudget.evaluate_burn_rate_alert(rule, 2.0, 1.0)
    print(f"  Short burn: 2.0, Long burn: 1.0")
    print(f"  Alert triggered: {result.triggered}")
    print(f"  Message: {result.message}\n")
    
    # Scenario 2: High short-term burn (fast burn)
    print("Scenario 2: Fast burn rate")
    result = neuralbudget.evaluate_burn_rate_alert(rule, 15.0, 8.0)
    print(f"  Short burn: 15.0, Long burn: 8.0")
    print(f"  Alert triggered: {result.triggered}")
    print(f"  Message: {result.message}\n")


def example_5_multi_evaluation():
    """Example 5: Evaluate all rules at once"""
    print("\n=== Example 5: Multi-Rule Evaluation ===\n")
    
    # Create metric stream with elevated error rate
    metric_stream = []
    now = int(time.time())
    
    # Simulate: 10% error rate for last hour
    for i in range(3600):
        timestamp = now - 3600 + i
        is_error = (i % 10) == 0  # 10% errors
        metric_stream.append(
            neuralbudget.MetricPoint(timestamp, 1.0 if is_error else 0.0)
        )
    
    # Calculate multi-window and evaluate
    multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
    rules = neuralbudget.BurnRateAlertRule.standard_rules()
    alerts = neuralbudget.evaluate_multi_window_alerts(multi, rules)
    
    print("Burn rates:")
    print(f"  5m:  {multi.burn_rate_5m:.4f}")
    print(f"  30m: {multi.burn_rate_30m:.4f}")
    print(f"  1h:  {multi.burn_rate_1h:.4f}")
    print(f"  6h:  {multi.burn_rate_6h:.4f}")
    
    print("\nAlert evaluation:")
    for alert in alerts:
        status = "✓ TRIGGERED" if alert.triggered else "✗ OK"
        print(f"  {status} - {alert.message}")
    
    # Determine overall severity
    severity = neuralbudget.determine_overall_severity(alerts)
    print(f"\nOverall severity: {severity}")


def example_6_budget_forecast():
    """Example 6: Budget exhaustion forecasting"""
    print("\n=== Example 6: Budget Exhaustion Forecast ===\n")
    
    now = int(time.time())
    current_burn_rate = 0.01  # 1% burn rate
    remaining_budget = 3600.0  # 1 hour of budget
    
    forecast = neuralbudget.forecast_budget_exhaustion(
        current_burn_rate=current_burn_rate,
        remaining_budget_seconds=remaining_budget,
        now=now
    )
    
    print(f"Current state:")
    print(f"  Burn rate: {forecast.current_burn_rate:.4f}")
    print(f"  Remaining budget: {forecast.remaining_budget_seconds:.0f}s")
    
    print(f"\nForecast:")
    print(f"  Will exhaust: {forecast.will_exhaust}")
    print(f"  Time to exhaustion: {forecast.time_to_exhaustion_seconds / 60:.1f} minutes")
    print(f"  Projected exhaustion: {time.ctime(forecast.projected_exhaustion_timestamp)}")
    
    print(f"\nExport to JSON:")
    print(json.dumps(json.loads(forecast.to_json()), indent=2))


def example_7_custom_rules():
    """Example 7: Custom alert rules"""
    print("\n=== Example 7: Custom Alert Rules ===\n")
    
    # Define a custom "ultra-critical" rule for extreme burn rates
    custom_rule = neuralbudget.BurnRateAlertRule(
        name="ultra-critical-5m",
        short_window_threshold=50.0,  # 50x burn rate in 5 minutes
        long_window_threshold=30.0,   # 30x burn rate in 5 minutes
        short_window_seconds=300,
        long_window_seconds=300,
        severity="critical_burn"
    )
    
    print("Custom rule created:")
    print(f"  Name: {custom_rule.name}")
    print(f"  Severity: {custom_rule.severity}")
    print(f"  Short threshold: {custom_rule.short_window_threshold}x")
    print(f"  Long threshold: {custom_rule.long_window_threshold}x")
    
    # Test the custom rule
    print("\nTesting custom rule:")
    result = neuralbudget.evaluate_burn_rate_alert(custom_rule, 55.0, 35.0)
    print(f"  Trigger: {result.triggered}")
    print(f"  Message: {result.message}")


def example_8_integration():
    """Example 8: Full integration example"""
    print("\n=== Example 8: Full Integration ===\n")
    
    # Simulate a service degradation event
    print("Simulating service degradation over 2 hours...\n")
    
    metric_stream = []
    now = int(time.time())
    
    # First hour: normal (1% errors)
    for i in range(3600):
        timestamp = now - 7200 + i
        is_error = (i % 100) == 0  # 1% errors
        metric_stream.append(
            neuralbudget.MetricPoint(timestamp, 1.0 if is_error else 0.0)
        )
    
    # Second hour: degraded (25% errors)
    for i in range(3600):
        timestamp = now - 3600 + i
        is_error = (i % 4) == 0  # 25% errors
        metric_stream.append(
            neuralbudget.MetricPoint(timestamp, 1.0 if is_error else 0.0)
        )
    
    # Analyze current state
    multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
    print("Current burn rates:")
    print(f"  5m:  {multi.burn_rate_5m:.4f}")
    print(f"  30m: {multi.burn_rate_30m:.4f}")
    print(f"  1h:  {multi.burn_rate_1h:.4f}")
    print(f"  6h:  {multi.burn_rate_6h:.4f}")
    
    # Evaluate rules
    rules = neuralbudget.BurnRateAlertRule.standard_rules()
    alerts = neuralbudget.evaluate_multi_window_alerts(multi, rules)
    severity = neuralbudget.determine_overall_severity(alerts)
    
    print(f"\nAlert status: {severity}")
    triggered_count = sum(1 for a in alerts if a.triggered)
    print(f"Triggered alerts: {triggered_count}/{len(rules)}")
    
    # Forecast exhaustion
    forecast = neuralbudget.forecast_budget_exhaustion(
        current_burn_rate=multi.burn_rate_1h,
        remaining_budget_seconds=7200.0,  # 2 hours
        now=now
    )
    
    print(f"\nBudget forecast:")
    print(f"  Current rate: {multi.burn_rate_1h:.4f}")
    print(f"  Time to exhaustion: {forecast.time_to_exhaustion_seconds / 3600:.2f} hours")
    print(f"  Budget will exhaust: {forecast.will_exhaust}")


if __name__ == "__main__":
    print("╔════════════════════════════════════════╗")
    print("║  Burn-Rate Forecasting Examples        ║")
    print("║  NeuralBudget Multi-Window SLO Alerts  ║")
    print("╚════════════════════════════════════════╝")
    
    example_1_basic_burn_rate()
    example_2_multi_window()
    example_3_alert_rules()
    example_4_alert_evaluation()
    example_5_multi_evaluation()
    example_6_budget_forecast()
    example_7_custom_rules()
    example_8_integration()
    
    print("\n✓ All examples completed successfully!\n")
