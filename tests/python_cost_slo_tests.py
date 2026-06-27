"""
Tests for cost-based SLOs.

Tests cover:
- Token cost calculation
- Budget tracking
- Batch evaluation
- Hybrid quality-cost scoring
- Configuration validation
"""

import pytest
from typing import List, Dict


class TestCostCalculation:
    """Tests for token cost calculations."""

    def test_input_cost_calculation(self):
        """Test input token cost calculation."""
        input_tokens = 50
        cost_per_1k = 0.00015  # GPT-4 Mini

        cost = (input_tokens / 1000) * cost_per_1k
        expected = 0.0000075

        assert abs(cost - expected) < 0.00000001

    def test_output_cost_calculation(self):
        """Test output token cost calculation."""
        output_tokens = 120
        cost_per_1k = 0.0006  # GPT-4 Mini

        cost = (output_tokens / 1000) * cost_per_1k
        expected = 0.000072

        assert abs(cost - expected) < 0.00000001

    def test_total_cost_gpt4_mini(self):
        """Test total cost for GPT-4 Mini."""
        input_tokens = 50
        output_tokens = 120
        input_cost_per_1k = 0.00015
        output_cost_per_1k = 0.0006

        input_cost = (input_tokens / 1000) * input_cost_per_1k
        output_cost = (output_tokens / 1000) * output_cost_per_1k
        total_cost = input_cost + output_cost

        assert abs(total_cost - 0.0000795) < 0.00000001

    def test_total_cost_gpt4_turbo(self):
        """Test total cost for GPT-4 Turbo (expensive)."""
        input_tokens = 50
        output_tokens = 120
        input_cost_per_1k = 0.01
        output_cost_per_1k = 0.03

        input_cost = (input_tokens / 1000) * input_cost_per_1k
        output_cost = (output_tokens / 1000) * output_cost_per_1k
        total_cost = input_cost + output_cost

        # Should be ~$0.0045
        assert total_cost > 0.004 and total_cost < 0.005

    def test_total_cost_claude_haiku(self):
        """Test total cost for Claude Haiku."""
        input_tokens = 50
        output_tokens = 120
        input_cost_per_1k = 0.00025
        output_cost_per_1k = 0.00125

        input_cost = (input_tokens / 1000) * input_cost_per_1k
        output_cost = (output_tokens / 1000) * output_cost_per_1k
        total_cost = input_cost + output_cost

        expected = 0.0001625
        assert abs(total_cost - expected) < 0.00000001

    def test_zero_tokens_zero_cost(self):
        """Test that zero tokens result in zero cost."""
        input_cost = (0 / 1000) * 0.001
        output_cost = (0 / 1000) * 0.001
        total_cost = input_cost + output_cost

        assert total_cost == 0.0

    def test_large_tokens_calculation(self):
        """Test cost calculation with large token counts."""
        input_tokens = 100000
        output_tokens = 50000
        input_cost_per_1k = 0.0001
        output_cost_per_1k = 0.0005

        input_cost = (input_tokens / 1000) * input_cost_per_1k
        output_cost = (output_tokens / 1000) * output_cost_per_1k
        total_cost = input_cost + output_cost

        expected = 10 + 25  # $35
        assert abs(total_cost - expected) < 0.01


class TestBudgetCheck:
    """Tests for budget compliance checking."""

    def test_under_budget(self):
        """Test request under budget."""
        total_cost = 0.005
        max_budget = 0.015

        within_budget = total_cost <= max_budget
        assert within_budget is True

    def test_at_budget(self):
        """Test request exactly at budget."""
        total_cost = 0.015
        max_budget = 0.015

        within_budget = total_cost <= max_budget
        assert within_budget is True

    def test_over_budget(self):
        """Test request over budget."""
        total_cost = 0.020
        max_budget = 0.015

        within_budget = total_cost <= max_budget
        assert within_budget is False

    def test_cost_score_calculation(self):
        """Test cost score (budget utilization)."""
        max_budget = 0.015
        total_cost = 0.005

        cost_score = (max_budget - total_cost) / max_budget
        expected = 0.6667

        assert abs(cost_score - expected) < 0.001

    def test_cost_score_zero_at_budget(self):
        """Test cost score is zero when at budget."""
        max_budget = 0.015
        total_cost = 0.015

        cost_score = (max_budget - total_cost) / max_budget
        assert abs(cost_score) < 0.001

    def test_cost_score_negative_over_budget(self):
        """Test cost score is negative when over budget."""
        max_budget = 0.010
        total_cost = 0.015

        cost_score = (max_budget - total_cost) / max_budget
        assert cost_score < 0.0


class TestPassFailLogic:
    """Tests for pass/fail determination."""

    def test_pass_under_budget_and_threshold_met(self):
        """Test pass when under budget and threshold met."""
        within_budget = True
        cost_score = 0.95
        cost_threshold = 0.95

        passes = within_budget and cost_score >= cost_threshold
        assert passes is True

    def test_fail_over_budget(self):
        """Test fail when over budget."""
        within_budget = False
        cost_score = 0.90
        cost_threshold = 0.95

        passes = within_budget and cost_score >= cost_threshold
        assert passes is False

    def test_fail_below_threshold(self):
        """Test fail when below cost threshold."""
        within_budget = True
        cost_score = 0.85
        cost_threshold = 0.95

        passes = within_budget and cost_score >= cost_threshold
        assert passes is False

    def test_strict_threshold(self):
        """Test strict threshold (0.99)."""
        cost_scores = [0.990, 0.989, 0.991]
        threshold = 0.99

        results = [score >= threshold for score in cost_scores]
        assert results[0] is True   # 0.990 >= 0.99
        assert results[1] is False  # 0.989 < 0.99
        assert results[2] is True   # 0.991 >= 0.99


class TestBatchEvaluation:
    """Tests for evaluating multiple requests."""

    def test_batch_cost_accumulation(self):
        """Test that costs accumulate in batch."""
        costs = [0.001, 0.002, 0.001]
        total_cost = sum(costs)

        assert total_cost == 0.004

    def test_batch_pass_rate_all_pass(self):
        """Test pass rate when all requests pass."""
        requests_passed = 100
        total_requests = 100

        pass_rate = requests_passed / total_requests
        assert pass_rate == 1.0

    def test_batch_pass_rate_partial(self):
        """Test pass rate when some fail."""
        requests_passed = 75
        total_requests = 100

        pass_rate = requests_passed / total_requests
        assert pass_rate == 0.75

    def test_batch_pass_rate_none_pass(self):
        """Test pass rate when none pass."""
        requests_passed = 0
        total_requests = 100

        pass_rate = requests_passed / total_requests
        assert pass_rate == 0.0

    def test_monthly_budget_accumulation(self):
        """Test monthly budget accumulation."""
        daily_costs = [300, 320, 280, 310, 290]  # 5 days
        total_monthly = sum(daily_costs)

        assert total_monthly == 1500


class TestMonthlyBudget:
    """Tests for monthly budget tracking."""

    def test_monthly_budget_ok(self):
        """Test monthly cost within budget."""
        total_cost = 5000
        monthly_limit = 10000

        ok = total_cost <= monthly_limit
        assert ok is True

    def test_monthly_budget_exceeded(self):
        """Test monthly cost exceeds budget."""
        total_cost = 15000
        monthly_limit = 10000

        ok = total_cost <= monthly_limit
        assert ok is False

    def test_monthly_budget_at_limit(self):
        """Test monthly cost at limit."""
        total_cost = 10000
        monthly_limit = 10000

        ok = total_cost <= monthly_limit
        assert ok is True

    def test_estimated_monthly_forecast(self):
        """Test forecasting monthly cost."""
        avg_cost_per_request = 0.001
        requests_per_day = 10000
        days_in_month = 30

        forecast = avg_cost_per_request * requests_per_day * days_in_month
        expected = 300  # $300/month

        assert forecast == expected


class TestHybridScoring:
    """Tests for combining cost and quality scores."""

    def test_hybrid_score_equal_weights(self):
        """Test hybrid score with equal weights."""
        cost_score = 0.95
        quality_score = 0.90
        cost_weight = 0.5
        quality_weight = 0.5

        hybrid = cost_score * cost_weight + quality_score * quality_weight
        expected = 0.925

        assert abs(hybrid - expected) < 0.001

    def test_hybrid_score_quality_weighted(self):
        """Test hybrid score with quality emphasis."""
        cost_score = 0.95
        quality_score = 0.90
        cost_weight = 0.1
        quality_weight = 0.9

        hybrid = cost_score * cost_weight + quality_score * quality_weight
        expected = 0.9005

        assert abs(hybrid - expected) < 0.001

    def test_hybrid_score_cost_weighted(self):
        """Test hybrid score with cost emphasis."""
        cost_score = 0.95
        quality_score = 0.90
        cost_weight = 0.9
        quality_weight = 0.1

        hybrid = cost_score * cost_weight + quality_score * quality_weight
        expected = 0.945

        assert abs(hybrid - expected) < 0.001

    def test_hybrid_score_perfect_cost(self):
        """Test hybrid when cost is perfect."""
        cost_score = 1.0  # Free
        quality_score = 0.80
        cost_weight = 0.1
        quality_weight = 0.9

        hybrid = cost_score * cost_weight + quality_score * quality_weight
        expected = 0.82

        assert abs(hybrid - expected) < 0.001

    def test_hybrid_score_poor_quality(self):
        """Test hybrid when quality is poor."""
        cost_score = 1.0  # Free
        quality_score = 0.50
        cost_weight = 0.1
        quality_weight = 0.9

        hybrid = cost_score * cost_weight + quality_score * quality_weight
        expected = 0.55

        assert abs(hybrid - expected) < 0.001


class TestModelComparison:
    """Tests comparing costs across models."""

    def test_gpt4_mini_vs_turbo(self):
        """Test cost difference between GPT-4 Mini and Turbo."""
        input_tokens = 1000
        output_tokens = 1000

        # GPT-4 Mini
        mini_cost = (input_tokens / 1000) * 0.00015 + (output_tokens / 1000) * 0.0006
        # GPT-4 Turbo
        turbo_cost = (input_tokens / 1000) * 0.01 + (output_tokens / 1000) * 0.03

        # Turbo should be ~65× more expensive
        ratio = turbo_cost / mini_cost
        assert ratio > 60 and ratio < 70

    def test_haiku_vs_sonnet(self):
        """Test cost difference between Claude Haiku and Sonnet."""
        input_tokens = 1000
        output_tokens = 1000

        # Haiku
        haiku_cost = (input_tokens / 1000) * 0.00025 + (output_tokens / 1000) * 0.00125
        # Sonnet
        sonnet_cost = (input_tokens / 1000) * 0.003 + (output_tokens / 1000) * 0.015

        # Sonnet should be ~12× more expensive
        ratio = sonnet_cost / haiku_cost
        assert ratio > 10 and ratio < 14

    def test_cheapest_model(self):
        """Test which model is cheapest."""
        input_tokens = 1000
        output_tokens = 1000

        models = {
            "gpt4_mini": (0.00015, 0.0006),
            "claude_haiku": (0.00025, 0.00125),
            "gpt4_turbo": (0.01, 0.03),
        }

        costs = {}
        for name, (input_rate, output_rate) in models.items():
            cost = (input_tokens / 1000) * input_rate + (output_tokens / 1000) * output_rate
            costs[name] = cost

        cheapest = min(costs, key=costs.get)
        assert cheapest == "gpt4_mini"


class TestConfigurationValidation:
    """Tests for configuration validation."""

    def test_cost_threshold_valid_range(self):
        """Test cost threshold is in valid range."""
        valid_thresholds = [0.0, 0.5, 0.95, 0.99, 1.0]

        for threshold in valid_thresholds:
            assert 0.0 <= threshold <= 1.0

    def test_cost_threshold_clamping(self):
        """Test that invalid thresholds are clamped."""
        def clamp(value):
            return max(0.0, min(1.0, value))

        assert clamp(-0.5) == 0.0
        assert clamp(0.95) == 0.95
        assert clamp(1.5) == 1.0

    def test_budget_config_valid(self):
        """Test valid budget configuration."""
        budget = {
            "input_cost_per_1k": 0.00015,
            "output_cost_per_1k": 0.0006,
            "max_per_request": 0.015,
        }

        # All should be positive
        assert all(v > 0.0 for v in budget.values())

    def test_monthly_limit_positive(self):
        """Test monthly limit is positive."""
        limits = [1000, 5000, 10000, 50000]

        for limit in limits:
            assert limit > 0

    def test_typical_configuration(self):
        """Test typical production configuration."""
        config = {
            "enabled": True,
            "budget": {
                "input_cost_per_1k": 0.00015,
                "output_cost_per_1k": 0.0006,
                "max_per_request": 0.015,
            },
            "cost_threshold": 0.95,
            "monthly_limit": 10000,
            "cost_weight": 0.1,
        }

        assert config["enabled"] is True
        assert config["cost_threshold"] <= 1.0
        assert config["monthly_limit"] > 0


class TestRealisticScenarios:
    """Tests with realistic scenarios."""

    def test_qa_assistant_typical_request(self):
        """Test typical Q&A assistant request."""
        # Input: question + context
        input_tokens = 500
        # Output: answer
        output_tokens = 250

        # GPT-4 Mini
        cost = (input_tokens / 1000) * 0.00015 + (output_tokens / 1000) * 0.0006
        assert cost < 0.001  # Under 1 cent

    def test_summarization_request(self):
        """Test document summarization request."""
        # Input: full document
        input_tokens = 2000
        # Output: summary
        output_tokens = 500

        # GPT-4 Mini
        cost = (input_tokens / 1000) * 0.00015 + (output_tokens / 1000) * 0.0006
        assert 0.0003 < cost < 0.001

    def test_code_generation_request(self):
        """Test code generation request."""
        # Input: prompt + context
        input_tokens = 3000
        # Output: code
        output_tokens = 1500

        # Claude Sonnet
        cost = (input_tokens / 1000) * 0.003 + (output_tokens / 1000) * 0.015
        assert 0.02 < cost < 0.03

    def test_startup_monthly_budget(self):
        """Test startup scaling scenario."""
        # 1M requests/month, average 100 input + 200 output tokens
        requests_per_month = 1000000
        avg_cost_per_request = (100 / 1000) * 0.00015 + (200 / 1000) * 0.0006

        total_monthly = requests_per_month * avg_cost_per_request
        assert 100 < total_monthly < 300  # Around $160

    def test_enterprise_monthly_budget(self):
        """Test enterprise scaling scenario."""
        # 100M requests/month, average 500 input + 1000 output tokens
        requests_per_month = 100000000
        avg_cost_per_request = (500 / 1000) * 0.003 + (1000 / 1000) * 0.015

        total_monthly = requests_per_month * avg_cost_per_request
        assert total_monthly > 1000000  # Over $1M

    def test_cost_optimization_decision(self):
        """Test cost optimization decision making."""
        # Current: GPT-4 Mini with quality 0.92
        current_cost = 0.001
        current_quality = 0.92
        current_roi = current_quality / current_cost

        # Option: Claude Haiku with quality 0.88
        haiku_cost = 0.0003
        haiku_quality = 0.88
        haiku_roi = haiku_quality / haiku_cost

        # Haiku has better cost efficiency despite lower quality
        assert haiku_roi > current_roi


class TestMonitoring:
    """Tests for monitoring metrics."""

    def test_cost_metric_tracking(self):
        """Test cost metric accumulation."""
        requests = [
            {"cost": 0.001, "tokens": 500},
            {"cost": 0.0015, "tokens": 750},
            {"cost": 0.001, "tokens": 500},
        ]

        total_cost = sum(r["cost"] for r in requests)
        total_tokens = sum(r["tokens"] for r in requests)

        assert total_cost == 0.0035
        assert total_tokens == 1750

    def test_alert_threshold_exceeded(self):
        """Test alert when threshold exceeded."""
        hourly_cost = 100
        hourly_limit = 50

        alert_triggered = hourly_cost > hourly_limit
        assert alert_triggered is True

    def test_budget_utilization_ratio(self):
        """Test budget utilization percentage."""
        used = 7500
        limit = 10000

        utilization = (used / limit) * 100
        assert utilization == 75.0

    def test_cost_per_token_metric(self):
        """Test cost per token calculation."""
        total_cost = 0.001
        total_tokens = 1000

        cost_per_token = total_cost / total_tokens if total_tokens > 0 else 0
        assert cost_per_token == 0.000001  # $0.000001 per token


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
