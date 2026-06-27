"""
Tests for burn-rate forecasting module.

Verifies multi-window burn rate calculations, alert evaluation, and
budget exhaustion forecasting.
"""

import pytest
import time
import json
import neuralbudget


class TestBurnRateCalculation:
    """Test burn rate calculations."""
    
    def test_calculate_burn_rate_no_errors(self):
        """Burn rate should be 0 with no errors."""
        metric_stream = [
            neuralbudget.MetricPoint(1, 0.0),
            neuralbudget.MetricPoint(2, 0.0),
            neuralbudget.MetricPoint(3, 0.0),
        ]
        burn_rate = neuralbudget.calculate_burn_rate_for_window(metric_stream, 300)
        assert burn_rate == pytest.approx(0.0, abs=0.001)
    
    def test_calculate_burn_rate_all_errors(self):
        """Burn rate should be high with all errors."""
        metric_stream = [
            neuralbudget.MetricPoint(1, 1.0),
            neuralbudget.MetricPoint(2, 1.0),
            neuralbudget.MetricPoint(3, 1.0),
        ]
        burn_rate = neuralbudget.calculate_burn_rate_for_window(metric_stream, 300)
        assert burn_rate > 0.5  # High error rate
    
    def test_calculate_burn_rate_50_percent(self):
        """Burn rate should be ~0.5 with 50% errors."""
        metric_stream = [
            neuralbudget.MetricPoint(1, 1.0),
            neuralbudget.MetricPoint(2, 0.0),
            neuralbudget.MetricPoint(3, 1.0),
            neuralbudget.MetricPoint(4, 0.0),
        ]
        burn_rate = neuralbudget.calculate_burn_rate_for_window(metric_stream, 300)
        assert burn_rate == pytest.approx(0.5, abs=0.1)


class TestMultiWindowBurnRate:
    """Test multi-window burn rate calculations."""
    
    def test_multi_window_structure(self):
        """MultiWindowBurnRate should have all fields."""
        metric_stream = [neuralbudget.MetricPoint(i, 0.0) for i in range(10)]
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        
        assert multi.timestamp > 0
        assert multi.burn_rate_5m >= 0.0
        assert multi.burn_rate_30m >= 0.0
        assert multi.burn_rate_1h >= 0.0
        assert multi.burn_rate_6h >= 0.0
    
    def test_multi_window_to_dict(self):
        """MultiWindowBurnRate should convert to dict."""
        metric_stream = [neuralbudget.MetricPoint(i, 0.0) for i in range(10)]
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        
        d = multi.to_dict()
        assert "timestamp" in d
        assert "burn_rate_5m" in d
        assert "burn_rate_30m" in d
        assert "burn_rate_1h" in d
        assert "burn_rate_6h" in d
    
    def test_multi_window_json_serialization(self):
        """MultiWindowBurnRate should serialize to JSON."""
        metric_stream = [neuralbudget.MetricPoint(i, 0.0) for i in range(10)]
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        
        json_str = multi.to_json()
        parsed = json.loads(json_str)
        
        assert "timestamp" in parsed
        assert "burn_rate_5m" in parsed
    
    def test_multi_window_yaml_serialization(self):
        """MultiWindowBurnRate should serialize to YAML."""
        metric_stream = [neuralbudget.MetricPoint(i, 0.0) for i in range(10)]
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        
        yaml_str = multi.to_yaml()
        assert "timestamp:" in yaml_str
        assert "burn_rate_5m:" in yaml_str


class TestBurnRateAlertRule:
    """Test burn rate alert rules."""
    
    def test_standard_rules_count(self):
        """Standard rules should include fast, medium, and slow burn."""
        rules = neuralbudget.BurnRateAlertRule.standard_rules()
        assert len(rules) >= 3
    
    def test_standard_rules_properties(self):
        """Standard rules should have valid properties."""
        rules = neuralbudget.BurnRateAlertRule.standard_rules()
        
        for rule in rules:
            assert rule.name
            assert rule.severity in ["ok", "slow_burn", "medium_burn", "fast_burn", "critical_burn"]
            assert rule.short_window_threshold > 0
            assert rule.long_window_threshold > 0
            assert rule.short_window_seconds > 0
            assert rule.long_window_seconds > 0
    
    def test_fast_burn_rule(self):
        """Fast burn rule should have correct properties."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        assert "fast" in rule.name.lower()
        assert rule.severity == "fast_burn"
        assert rule.short_window_seconds == 300  # 5m
        assert rule.long_window_seconds == 3600  # 1h
    
    def test_medium_burn_rule(self):
        """Medium burn rule should have correct properties."""
        rule = neuralbudget.BurnRateAlertRule.medium_burn()
        assert "medium" in rule.name.lower()
        assert rule.severity == "medium_burn"
    
    def test_slow_burn_rule(self):
        """Slow burn rule should have correct properties."""
        rule = neuralbudget.BurnRateAlertRule.slow_burn()
        assert "slow" in rule.name.lower()
        assert rule.severity == "slow_burn"
    
    def test_custom_rule_creation(self):
        """Should be able to create custom rules."""
        rule = neuralbudget.BurnRateAlertRule(
            name="custom-rule",
            short_window_threshold=5.0,
            long_window_threshold=2.0,
            short_window_seconds=300,
            long_window_seconds=3600,
            severity="fast_burn"
        )
        
        assert rule.name == "custom-rule"
        assert rule.short_window_threshold == 5.0
        assert rule.long_window_threshold == 2.0
    
    def test_rule_to_dict(self):
        """Rule should convert to dict."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        d = rule.to_dict()
        
        assert "name" in d
        assert "severity" in d
        assert "short_window_threshold" in d


class TestAlertEvaluation:
    """Test alert evaluation."""
    
    def test_alert_not_triggered_low_burn(self):
        """Alert should not trigger with low burn rates."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        result = neuralbudget.evaluate_burn_rate_alert(rule, 2.0, 1.0)
        
        assert not result.triggered
        assert result.short_burn_rate == 2.0
        assert result.long_burn_rate == 1.0
    
    def test_alert_triggered_high_burn(self):
        """Alert should trigger with high burn rates."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        result = neuralbudget.evaluate_burn_rate_alert(rule, 15.0, 8.0)
        
        assert result.triggered
        assert result.message
    
    def test_alert_result_to_dict(self):
        """Alert result should convert to dict."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        result = neuralbudget.evaluate_burn_rate_alert(rule, 2.0, 1.0)
        
        d = result.to_dict()
        assert "triggered" in d
        assert "short_burn_rate" in d
        assert "long_burn_rate" in d
        assert "message" in d


class TestMultiWindowAlerts:
    """Test evaluating multiple alert rules."""
    
    def test_multi_window_alerts_count(self):
        """Should evaluate all rules."""
        metric_stream = [neuralbudget.MetricPoint(i, 0.0) for i in range(100)]
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        rules = neuralbudget.BurnRateAlertRule.standard_rules()
        
        alerts = neuralbudget.evaluate_multi_window_alerts(multi, rules)
        assert len(alerts) == len(rules)
    
    def test_multi_window_alerts_with_errors(self):
        """Should detect errors in multi-window evaluation."""
        # Create a stream with elevated errors
        metric_stream = []
        for i in range(1000):
            is_error = i % 5 == 0  # 20% errors
            metric_stream.append(
                neuralbudget.MetricPoint(i, 1.0 if is_error else 0.0)
            )
        
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        rules = neuralbudget.BurnRateAlertRule.standard_rules()
        
        alerts = neuralbudget.evaluate_multi_window_alerts(multi, rules)
        
        # At least one alert should trigger
        assert any(a.triggered for a in alerts)


class TestSeverityDetermination:
    """Test overall severity determination."""
    
    def test_severity_no_alerts(self):
        """Severity should be 'ok' with no triggered alerts."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        result = neuralbudget.evaluate_burn_rate_alert(rule, 1.0, 0.5)
        
        alerts = [result] if not result.triggered else []
        severity = neuralbudget.determine_overall_severity(alerts)
        
        # Should be 'ok' when no alerts are triggered
        assert severity in ["ok", "slow_burn", "medium_burn", "fast_burn", "critical_burn"]
    
    def test_severity_with_alerts(self):
        """Severity should escalate with triggered alerts."""
        rule = neuralbudget.BurnRateAlertRule.fast_burn()
        result = neuralbudget.evaluate_burn_rate_alert(rule, 15.0, 8.0)
        
        severity = neuralbudget.determine_overall_severity([result])
        assert severity in ["fast_burn", "critical_burn"]


class TestBudgetForecast:
    """Test budget exhaustion forecasting."""
    
    def test_forecast_structure(self):
        """Forecast should have all required fields."""
        now = int(time.time())
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=0.01,
            remaining_budget_seconds=3600.0,
            now=now
        )
        
        assert forecast.timestamp == now
        assert forecast.current_burn_rate == 0.01
        assert forecast.remaining_budget_seconds == 3600.0
        assert forecast.time_to_exhaustion_seconds > 0
        assert forecast.projected_exhaustion_timestamp > now
        assert isinstance(forecast.will_exhaust, bool)
    
    def test_forecast_time_calculation(self):
        """Forecast time should be budget / burn_rate."""
        now = int(time.time())
        burn_rate = 0.01
        budget = 3600.0
        
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=burn_rate,
            remaining_budget_seconds=budget,
            now=now
        )
        
        expected_time = budget / burn_rate
        assert forecast.time_to_exhaustion_seconds == pytest.approx(expected_time, rel=0.01)
    
    def test_forecast_zero_burn_rate(self):
        """Forecast with zero burn rate should have infinite time."""
        now = int(time.time())
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=0.0,
            remaining_budget_seconds=3600.0,
            now=now
        )
        
        # Zero burn rate means budget lasts forever
        assert not forecast.will_exhaust or forecast.time_to_exhaustion_seconds == float('inf')
    
    def test_forecast_high_burn_rate(self):
        """Forecast with high burn rate should show quick exhaustion."""
        now = int(time.time())
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=2.0,  # 200% burn rate
            remaining_budget_seconds=3600.0,
            now=now
        )
        
        # Should exhaust in 30 minutes
        expected_time = 3600.0 / 2.0
        assert forecast.time_to_exhaustion_seconds == pytest.approx(expected_time, rel=0.01)
        assert forecast.will_exhaust
    
    def test_forecast_to_dict(self):
        """Forecast should convert to dict."""
        now = int(time.time())
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=0.01,
            remaining_budget_seconds=3600.0,
            now=now
        )
        
        d = forecast.to_dict()
        assert "timestamp" in d
        assert "current_burn_rate" in d
        assert "time_to_exhaustion_seconds" in d
        assert "projected_exhaustion_timestamp" in d
        assert "will_exhaust" in d
    
    def test_forecast_json_serialization(self):
        """Forecast should serialize to JSON."""
        now = int(time.time())
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=0.01,
            remaining_budget_seconds=3600.0,
            now=now
        )
        
        json_str = forecast.to_json()
        parsed = json.loads(json_str)
        
        assert "timestamp" in parsed
        assert "current_burn_rate" in parsed
    
    def test_forecast_yaml_serialization(self):
        """Forecast should serialize to YAML."""
        now = int(time.time())
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=0.01,
            remaining_budget_seconds=3600.0,
            now=now
        )
        
        yaml_str = forecast.to_yaml()
        assert "timestamp:" in yaml_str
        assert "current_burn_rate:" in yaml_str


class TestIntegration:
    """Integration tests for complete workflows."""
    
    def test_full_alert_workflow(self):
        """Complete workflow: calculate → evaluate → determine severity."""
        # Create metric stream
        metric_stream = []
        for i in range(3600):
            is_error = i % 20 == 0  # 5% errors
            metric_stream.append(
                neuralbudget.MetricPoint(i, 1.0 if is_error else 0.0)
            )
        
        # Calculate multi-window
        multi = neuralbudget.calculate_multi_window_burn_rate(metric_stream)
        assert multi.burn_rate_1h >= 0.0
        
        # Evaluate rules
        rules = neuralbudget.BurnRateAlertRule.standard_rules()
        alerts = neuralbudget.evaluate_multi_window_alerts(multi, rules)
        assert len(alerts) > 0
        
        # Determine severity
        severity = neuralbudget.determine_overall_severity(alerts)
        assert severity in ["ok", "slow_burn", "medium_burn", "fast_burn", "critical_burn"]
    
    def test_full_forecast_workflow(self):
        """Complete workflow: calculate → forecast → analyze."""
        now = int(time.time())
        burn_rate = 0.002  # 0.2% burn rate
        slo_budget = 86_400  # 1 day budget
        
        # Calculate remaining budget
        remaining = slo_budget * 0.9  # 90% remaining
        
        # Forecast exhaustion
        forecast = neuralbudget.forecast_budget_exhaustion(
            current_burn_rate=burn_rate,
            remaining_budget_seconds=remaining,
            now=now
        )
        
        # Verify calculations
        hours_remaining = forecast.time_to_exhaustion_seconds / 3600
        assert hours_remaining > 100  # Should last many days


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
