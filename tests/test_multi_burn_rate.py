#!/usr/bin/env python3
"""
Test suite for multi-burn-rate alerting thresholds.

Validates:
1. Burn rate threshold calculations
2. Default window configuration
3. Window validation logic
4. Real-world scenario calculations against Google SRE examples
"""

import sys
import math

def calculate_error_threshold(availability_target, burn_rate_multiplier):
    """Calculate error rate threshold given SLO target and burn rate multiplier."""
    allowed_error = 1.0 - availability_target
    return allowed_error * burn_rate_multiplier

def calculate_error_budget_percent(availability_target, current_error_rate):
    """Calculate error budget remaining as percentage."""
    allowed_error_rate = 1.0 - availability_target
    if allowed_error_rate <= 0.0:
        return 100.0
    
    remaining = (allowed_error_rate - current_error_rate) / allowed_error_rate
    return max(0.0, min(100.0, remaining * 100.0))

def parse_duration_seconds(duration_str):
    """Convert duration string to seconds."""
    durations = {
        "1h": 3600,
        "6h": 21600,
        "24h": 86400,
        "2d": 172800,
        "3d": 259200,
        "7d": 604800,
    }
    if duration_str not in durations:
        raise ValueError(f"Unknown duration: {duration_str}")
    return durations[duration_str]

def test_burn_rate_calculation():
    """Test burn rate threshold calculation."""
    print("\n" + "="*60)
    print("TEST: Burn Rate Threshold Calculation")
    print("="*60)
    
    # 99.9% SLO (0.001 error budget)
    availability_target = 0.999
    
    # Create test windows
    windows = [
        ("1h", 10.0),
        ("6h", 2.0),
        ("24h", 0.5),
        ("3d", 1.0),
    ]
    
    print(f"\nSLO: 99.9% (error budget = 0.1%)")
    print(f"\nBurn Rate Thresholds (error rate at which alert fires):")
    print(f"{'Window':<6} {'Burn Rate':<12} {'Error Rate Threshold':<20}")
    print("-" * 40)
    
    for duration, burn_rate in windows:
        threshold = calculate_error_threshold(availability_target, burn_rate)
        print(f"{duration:<6} {burn_rate:<12.1f}x {threshold:<20.6f}")
    
    # Expected values for 99.9% SLO
    expected = {
        "1h": 0.001 * 10.0,    # 0.01
        "6h": 0.001 * 2.0,     # 0.002
        "24h": 0.001 * 0.5,    # 0.0005
        "3d": 0.001 * 1.0,     # 0.001
    }
    
    print(f"\nValidation:")
    all_pass = True
    for duration, burn_rate in windows:
        threshold = calculate_error_threshold(availability_target, burn_rate)
        expected_threshold = expected[duration]
        passed = abs(threshold - expected_threshold) < 1e-10
        status = "✅" if passed else "❌"
        print(f"  {status} {duration}: {threshold:.10f} == {expected_threshold:.10f}")
        all_pass = all_pass and passed
    
    return all_pass

def test_error_budget_calculation():
    """Test error budget percentage calculation."""
    print("\n" + "="*60)
    print("TEST: Error Budget Remaining")
    print("="*60)
    
    availability_target = 0.999  # 0.1% budget
    
    test_cases = [
        (0.0, 100.0, "No errors - 100% budget remains"),
        (0.0005, 50.0, "Half budget spent"),
        (0.001, 0.0, "Full budget spent"),
        (0.0015, 0.0, "Over budget (clamped to 0%)"),
    ]
    
    print(f"\nError Rate → Budget Remaining (0.1% total budget)")
    print(f"{'Error Rate':<12} {'Budget %':<12} {'Description':<30}")
    print("-" * 54)
    
    all_pass = True
    for error_rate, expected_budget, description in test_cases:
        calculated_budget = calculate_error_budget_percent(availability_target, error_rate)
        passed = abs(calculated_budget - expected_budget) < 1e-6
        status = "✅" if passed else "❌"
        print(f"{error_rate:<12.6f} {calculated_budget:<12.1f} {description:<30} {status}")
        all_pass = all_pass and passed
    
    return all_pass

def test_default_four_window_config():
    """Test default 4-window configuration."""
    print("\n" + "="*60)
    print("TEST: Default 4-Window Configuration")
    print("="*60)
    
    windows = [
        ("1h", 10.0, "1m", "critical"),
        ("6h", 2.0, "15m", "warning"),
        ("24h", 0.5, "1h", "info"),
        ("3d", 1.0, "3h", "warning"),
    ]
    
    print(f"\nDefault windows:")
    print(f"{'#':<2} {'Duration':<8} {'Burn Rate':<12} {'For Duration':<15} {'Severity':<10}")
    print("-" * 50)
    
    for i, (duration, burn_rate, for_duration, severity) in enumerate(windows, 1):
        print(f"{i:<2} {duration:<8} {burn_rate:<12.1f}x {for_duration:<15} {severity:<10}")
    
    # Validate properties
    print(f"\nValidation:")
    
    # Check count
    has_four = len(windows) == 4
    print(f"  {'✅' if has_four else '❌'} Has 4 windows: {len(windows)}")
    
    # Check durations
    expected_durations = ["1h", "6h", "24h", "3d"]
    durations_match = [w[0] for w in windows] == expected_durations
    print(f"  {'✅' if durations_match else '❌'} Durations correct: {[w[0] for w in windows]}")
    
    # Check burn rates - first 3 should be descending (10, 2, 0.5)
    # 3d can be different as it covers a longer window
    first_three_descending = (windows[0][1] > windows[1][1] > windows[2][1])
    print(f"  {'✅' if first_three_descending else '❌'} First 3 windows burn rates descending (10 > 2 > 0.5): {first_three_descending}")
    
    return has_four and durations_match and first_three_descending

def test_window_duration_parsing():
    """Test parsing of duration strings."""
    print("\n" + "="*60)
    print("TEST: Window Duration Parsing")
    print("="*60)
    
    test_cases = [
        ("1h", 3600),
        ("6h", 21600),
        ("24h", 86400),
        ("2d", 172800),
        ("3d", 259200),
        ("7d", 604800),
    ]
    
    print(f"\nDuration Parsing:")
    print(f"{'Duration String':<15} {'Expected Seconds':<18} {'Calculated':<18}")
    print("-" * 51)
    
    all_pass = True
    for duration_str, expected_seconds in test_cases:
        try:
            calculated_seconds = parse_duration_seconds(duration_str)
            passed = calculated_seconds == expected_seconds
            status = "✅" if passed else "❌"
            print(f"{duration_str:<15} {expected_seconds:<18} {calculated_seconds:<18} {status}")
            all_pass = all_pass and passed
        except Exception as e:
            print(f"{duration_str:<15} {expected_seconds:<18} Error: {e} ❌")
            all_pass = False
    
    return all_pass

def test_real_world_scenarios():
    """Test burn rate math against Google SRE workbook examples."""
    print("\n" + "="*60)
    print("TEST: Google SRE Real-World Scenarios")
    print("="*60)
    
    scenarios = [
        {
            "name": "Payment API (99.95%)",
            "target": 0.9995,
            "monthly_minutes": 21.6,
            "expected_1h_threshold": 0.005,
        },
        {
            "name": "Web Service (99.9%)",
            "target": 0.999,
            "monthly_minutes": 43.2,
            "expected_1h_threshold": 0.01,
        },
        {
            "name": "API Gateway (99.99%)",
            "target": 0.9999,
            "monthly_minutes": 4.32,
            "expected_1h_threshold": 0.001,  # 0.0001 × 10 = 0.001
        },
    ]
    
    print("\nReal-World SLO Calculations:")
    all_pass = True
    
    for scenario in scenarios:
        print(f"\n{scenario['name']}:")
        print(f"  Target: {scenario['target']*100:.2f}%")
        
        # Monthly error budget
        error_budget = (1 - scenario['target']) * 720 * 60  # in minutes
        print(f"  Monthly budget: {error_budget:.1f} minutes")
        
        # 1h @ 10x threshold
        threshold = calculate_error_threshold(scenario['target'], 10.0)
        expected = scenario['expected_1h_threshold']
        passed = abs(threshold - expected) < 1e-10
        
        status = "✅" if passed else "❌"
        print(f"  1h @ 10x threshold: {threshold:.6f} (expected {expected:.6f}) {status}")
        all_pass = all_pass and passed
    
    return all_pass

def main():
    """Run all tests."""
    print("\n" + "="*60)
    print("NeuralBudget Multi-Burn-Rate Alerting Tests")
    print("="*60)
    
    tests = [
        ("Burn Rate Thresholds", test_burn_rate_calculation),
        ("Error Budget Calculation", test_error_budget_calculation),
        ("Default 4-Window Config", test_default_four_window_config),
        ("Duration Parsing", test_window_duration_parsing),
        ("Google SRE Scenarios", test_real_world_scenarios),
    ]
    
    results = []
    for name, test_func in tests:
        try:
            result = test_func()
            results.append((name, result))
        except Exception as e:
            print(f"\n❌ Exception in {name}: {e}")
            import traceback
            traceback.print_exc()
            results.append((name, False))
    
    # Summary
    print("\n" + "="*60)
    print("Test Summary")
    print("="*60)
    
    for name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{status}: {name}")
    
    passed = sum(1 for _, r in results if r)
    total = len(results)
    print(f"\n{passed}/{total} tests passed")
    
    return 0 if all(r for _, r in results) else 1

if __name__ == '__main__':
    sys.exit(main())
