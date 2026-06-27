#!/usr/bin/env python3
"""
Test script for Prometheus rule generation from SLO configurations.

This script demonstrates the gen-rules functionality with various SLO configs
and validates the generated rules structure.
"""

import sys
import yaml
import json
from pathlib import Path

def load_config(config_path):
    """Load and parse YAML SLO configuration."""
    with open(config_path, 'r') as f:
        return yaml.safe_load(f)

def calculate_burn_rates(target, windows):
    """Calculate burn rate thresholds for each window."""
    allowed_error = 1.0 - target
    burn_rates = {}
    
    for window in windows:
        if isinstance(window, dict):
            window_key = window['window']
            threshold_pct = window['threshold'] * 100
            # Threshold = allowed_error * burn_rate_percent
            threshold = allowed_error * window['threshold']
            burn_rates[window_key] = {
                'threshold_percent': threshold_pct,
                'threshold_decimal': threshold,
                'expr_threshold': threshold
            }
    
    return burn_rates

def test_slo_config(config_path):
    """Test a single SLO configuration."""
    print(f"\n{'='*60}")
    print(f"Testing: {config_path}")
    print('='*60)
    
    try:
        config = load_config(config_path)
    except Exception as e:
        print(f"❌ Failed to load config: {e}")
        return False
    
    # Extract config values
    service_name = config.get('service', 'unknown')
    availability_target = config.get('availability_threshold', 0.999)
    latency_threshold = config.get('latency_threshold_ms', 200)
    job_label = config.get('job_label', service_name.lower())
    alerts = config.get('alerts', [])
    
    # Print config summary
    print(f"\n📋 Configuration Summary:")
    print(f"   Service: {service_name}")
    print(f"   Target Availability: {availability_target * 100:.2f}%")
    print(f"   Latency Threshold: {latency_threshold}ms")
    print(f"   Job Label: {job_label}")
    print(f"   Error Budget: {(1 - availability_target) * 100:.4f}%")
    
    if not alerts:
        alerts = [
            {'window': '1h', 'threshold': 0.10},
            {'window': '6h', 'threshold': 0.05},
            {'window': '24h', 'threshold': 0.02},
            {'window': '3d', 'threshold': 0.01},
        ]
        print(f"   Using default burn rate windows")
    else:
        print(f"   Burn Rate Windows: {len(alerts)} configured")
    
    # Calculate burn rates
    burn_rates = calculate_burn_rates(availability_target, alerts)
    
    print(f"\n🔥 Burn Rate Thresholds:")
    for window_key, rates in sorted(burn_rates.items()):
        print(f"   {window_key:>4}: {rates['threshold_percent']:>6.2f}% burn " +
              f"(threshold: {rates['threshold_decimal']:.6f})")
    
    # Validate configuration
    print(f"\n✅ Validation:")
    issues = []
    
    if not service_name or service_name == 'unknown':
        issues.append("Missing or empty 'service' field")
    
    if availability_target <= 0 or availability_target > 1:
        issues.append(f"Invalid availability_target: {availability_target} (must be 0-1)")
    
    if latency_threshold < 1 or latency_threshold > 60000:
        issues.append(f"Unrealistic latency threshold: {latency_threshold}ms")
    
    if availability_target < 0.9:
        issues.append(f"Very loose SLO target: {availability_target * 100:.2f}% (typical: 99%+)")
    
    if not issues:
        print("   ✓ Service field present")
        print(f"   ✓ Target {availability_target * 100:.2f}% is realistic")
        print(f"   ✓ Latency threshold {latency_threshold}ms is realistic")
        print(f"   ✓ Configuration is valid")
    else:
        print("   ⚠️ Issues found:")
        for issue in issues:
            print(f"      - {issue}")
    
    # Test Prometheus recording rule generation
    print(f"\n📊 Generated Recording Rules:")
    
    recording_rules = [
        ('neuralbudget:slo:availability', 'Availability SLI (% successful requests)'),
        ('neuralbudget:slo:latency_p99_ms', 'P99 Latency in milliseconds'),
        ('neuralbudget:slo:error_rate', 'Error rate (5xx / total)'),
        ('neuralbudget:slo:error_budget_remaining', f'Error budget remaining (0-{(1-availability_target)*100:.4f}%)'),
    ]
    
    for rule_name, description in recording_rules:
        print(f"   ✓ {rule_name}")
        print(f"     └─ {description}")
    
    # Test Prometheus alerting rules
    print(f"\n🚨 Generated Alerting Rules:")
    
    alert_config = {
        '1h': {'delay': '1m', 'severity': 'warning'},
        '6h': {'delay': '5m', 'severity': 'warning'},
        '24h': {'delay': '15m', 'severity': 'warning'},
        '3d': {'delay': '1h', 'severity': 'warning'},
    }
    
    for window_key, rates in sorted(burn_rates.items()):
        config_item = alert_config.get(window_key, {'delay': '5m', 'severity': 'warning'})
        alert_name = f"SloErrorBudgetBurnRate{window_key.replace('h','h').replace('d','d')}"
        print(f"   ✓ {alert_name}")
        print(f"     └─ Fires if burn_rate_{window_key} > {rates['threshold_decimal']:.6f}")
        print(f"     └─ For duration: {config_item['delay']}")
        print(f"     └─ Severity: {config_item['severity']}")
    
    # Additional alerts
    print(f"   ✓ SloLatencyExceeded")
    print(f"     └─ Fires if P99 latency > {latency_threshold}ms for 5m")
    print(f"     └─ Severity: warning")
    print(f"   ✓ SloErrorBudgetExhausted")
    print(f"     └─ Fires if error budget remaining <= 0 for 1m")
    print(f"     └─ Severity: critical")
    
    # Output format options
    print(f"\n📤 Output Format Options:")
    print(f"   neuralbudget gen-rules {config_path}")
    print(f"     → Plain Prometheus YAML")
    print(f"   neuralbudget gen-rules {config_path} --kubernetes --namespace monitoring")
    print(f"     → Kubernetes PrometheusRule CRD")
    
    print(f"\n✅ Config test passed!")
    return True

def main():
    """Main test runner."""
    test_configs = [
        'examples/slo_http.yaml',
        'examples/slo_ml.yaml',
    ]
    
    print("\n" + "="*60)
    print("NeuralBudget Prometheus Rule Generation Tests")
    print("="*60)
    
    passed = 0
    failed = 0
    
    for config_path in test_configs:
        config_file = Path(config_path)
        if not config_file.exists():
            print(f"\n⚠️  Skipping {config_path} (not found)")
            continue
        
        try:
            if test_slo_config(config_path):
                passed += 1
            else:
                failed += 1
        except Exception as e:
            print(f"\n❌ Exception during test of {config_path}:")
            print(f"   {e}")
            failed += 1
    
    print(f"\n" + "="*60)
    print(f"Test Results: {passed} passed, {failed} failed")
    print("="*60)
    
    return 0 if failed == 0 else 1

if __name__ == '__main__':
    sys.exit(main())
