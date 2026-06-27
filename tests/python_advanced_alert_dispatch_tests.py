"""Tests for advanced alert dispatch system.

Covers retry policies, deduplication, circuit breaker, and escalation.
"""

import pytest
import json
import time
from datetime import datetime, timedelta
from unittest.mock import Mock, patch, MagicMock

from neuralbudget.alert_dispatch_advanced import (
    AlertDispatchManager,
    RetryPolicy,
    DeduplicationPolicy,
    EscalationPolicy,
    EscalationStep,
    EscalationAction,
    AlertDeduplicationEntry,
    CircuitBreakerState,
)
from neuralbudget.alerting import AlertDispatchResult, AlertDispatchSummary


class TestRetryPolicy:
    """Test retry policy calculations."""
    
    def test_exponential_backoff(self):
        """Backoff should double each attempt."""
        policy = RetryPolicy(
            initial_delay_ms=100,
            backoff_multiplier=2.0,
            jitter_percent=0.0,
        )
        
        assert policy.calculate_delay_ms(0) == 100
        assert policy.calculate_delay_ms(1) == 200
        assert policy.calculate_delay_ms(2) == 400
        assert policy.calculate_delay_ms(3) == 800
    
    def test_max_delay_cap(self):
        """Backoff should not exceed max_delay_ms."""
        policy = RetryPolicy(
            initial_delay_ms=100,
            max_delay_ms=1000,
            backoff_multiplier=2.0,
            jitter_percent=0.0,
        )
        
        assert policy.calculate_delay_ms(0) == 100
        assert policy.calculate_delay_ms(10) == 1000  # Capped
    
    def test_jitter_range(self):
        """Jitter should add randomness within bounds."""
        policy = RetryPolicy(
            initial_delay_ms=100,
            max_delay_ms=10_000,
            jitter_percent=10.0,
        )
        
        # Run multiple times to test randomness
        delays = [policy.calculate_delay_ms(0) for _ in range(100)]
        
        # All should be near 100ms (within ±10%)
        for delay in delays:
            assert 90 <= delay <= 110
    
    def test_custom_backoff_multiplier(self):
        """Support custom backoff multiplier."""
        policy = RetryPolicy(
            initial_delay_ms=100,
            backoff_multiplier=3.0,
            jitter_percent=0.0,
        )
        
        assert policy.calculate_delay_ms(0) == 100
        assert policy.calculate_delay_ms(1) == 300
        assert policy.calculate_delay_ms(2) == 900


class TestDeduplicationPolicy:
    """Test deduplication logic."""
    
    def test_dedup_key_generation(self):
        """Dedup keys should be stable and deterministic."""
        mgr = AlertDispatchManager()
        
        result = {"violation": True, "severity": "error"}
        
        key1 = mgr._generate_dedup_key("http", "strict", result)
        key2 = mgr._generate_dedup_key("http", "strict", result)
        
        # Same input should generate same key
        assert key1 == key2
    
    def test_dedup_different_inputs(self):
        """Different inputs should generate different keys."""
        mgr = AlertDispatchManager()
        
        result1 = {"violation": True, "severity": "error"}
        result2 = {"violation": True, "severity": "warning"}
        
        key1 = mgr._generate_dedup_key("http", "strict", result1)
        key2 = mgr._generate_dedup_key("http", "strict", result2)
        
        # Different results should generate different keys
        assert key1 != key2
    
    def test_should_skip_first_alert(self):
        """First occurrence should not be skipped."""
        mgr = AlertDispatchManager(
            dedup_policy=DeduplicationPolicy(enabled=True, window_seconds=300)
        )
        
        dedup_key = "test:alert:123"
        
        # Should not skip on first occurrence
        assert not mgr._should_skip_due_to_dedup(dedup_key)
    
    def test_should_skip_within_window(self):
        """Alert should be skipped within dedup window."""
        mgr = AlertDispatchManager(
            dedup_policy=DeduplicationPolicy(enabled=True, window_seconds=300)
        )
        
        dedup_key = "test:alert:123"
        
        # First occurrence
        mgr._should_skip_due_to_dedup(dedup_key)
        
        # Add to tracking
        mgr._dedup_entries[dedup_key] = AlertDeduplicationEntry(
            dedup_key=dedup_key,
            sent_at=datetime.now(),
        )
        
        # Second occurrence within window should be skipped
        assert mgr._should_skip_due_to_dedup(dedup_key)
    
    def test_dedup_window_expiration(self):
        """Alert should be sent after dedup window expires."""
        mgr = AlertDispatchManager(
            dedup_policy=DeduplicationPolicy(enabled=True, window_seconds=1)
        )
        
        dedup_key = "test:alert:123"
        
        # Add to tracking with old timestamp
        mgr._dedup_entries[dedup_key] = AlertDeduplicationEntry(
            dedup_key=dedup_key,
            sent_at=datetime.now() - timedelta(seconds=2),
        )
        
        # Should not skip after window expires
        assert not mgr._should_skip_due_to_dedup(dedup_key)


class TestCircuitBreaker:
    """Test circuit breaker logic."""
    
    def test_circuit_breaker_closed_initially(self):
        """Circuit breaker should start closed."""
        mgr = AlertDispatchManager(
            retry_policy=RetryPolicy(use_circuit_breaker=True)
        )
        
        stats = mgr.get_circuit_breaker_stats()
        
        # Should have no tracked providers initially
        assert len(stats["providers"]) == 0
    
    def test_circuit_breaker_opens_on_threshold(self):
        """Circuit breaker should open after threshold failures."""
        mgr = AlertDispatchManager(
            retry_policy=RetryPolicy(
                use_circuit_breaker=True,
                circuit_breaker_threshold=3,
            )
        )
        
        # Simulate failures
        for i in range(3):
            result = AlertDispatchResult(provider="slack", ok=False)
            summary = AlertDispatchSummary(
                attempted=1,
                succeeded=0,
                failed=1,
                results=[result],
            )
            mgr._update_circuit_breakers(summary)
        
        stats = mgr.get_circuit_breaker_stats()
        slack_breaker = next(
            (p for p in stats["providers"] if p["provider"] == "slack"),
            None
        )
        
        assert slack_breaker is not None
        assert slack_breaker["is_open"]
    
    def test_circuit_breaker_closes_on_success(self):
        """Circuit breaker should close on success."""
        mgr = AlertDispatchManager(
            retry_policy=RetryPolicy(
                use_circuit_breaker=True,
                circuit_breaker_threshold=5,
            )
        )
        
        # Simulate failures
        for i in range(5):
            result = AlertDispatchResult(provider="slack", ok=False)
            summary = AlertDispatchSummary(
                attempted=1,
                succeeded=0,
                failed=1,
                results=[result],
            )
            mgr._update_circuit_breakers(summary)
        
        # Circuit should be open
        stats = mgr.get_circuit_breaker_stats()
        slack_breaker = next(p for p in stats["providers"] if p["provider"] == "slack")
        assert slack_breaker["is_open"]
        
        # Simulate success
        result = AlertDispatchResult(provider="slack", ok=True)
        summary = AlertDispatchSummary(
            attempted=1,
            succeeded=1,
            failed=0,
            results=[result],
        )
        mgr._update_circuit_breakers(summary)
        
        # Circuit should be closed again
        stats = mgr.get_circuit_breaker_stats()
        slack_breaker = next(p for p in stats["providers"] if p["provider"] == "slack")
        assert not slack_breaker["is_open"]
    
    def test_circuit_breaker_filters_config(self):
        """Circuit breaker should filter disabled providers."""
        mgr = AlertDispatchManager(
            retry_policy=RetryPolicy(use_circuit_breaker=True)
        )
        
        # Open circuit for pagerduty
        breaker = CircuitBreakerState(provider="pagerduty")
        breaker.is_open = True
        breaker.opened_at = datetime.now()
        mgr._circuit_breakers["pagerduty"] = breaker
        
        config = {
            "slack": {"webhook_url": "http://example.com"},
            "pagerduty": {"routing_key": "test"},
            "opsgenie": {"api_key": "test"},
        }
        
        filtered = mgr._apply_circuit_breaker_filters(config)
        
        # pagerduty should be filtered out
        assert "slack" in filtered
        assert "pagerduty" not in filtered
        assert "opsgenie" in filtered


class TestEscalation:
    """Test escalation logic."""
    
    def test_escalation_add_channels(self):
        """Escalation should add channels."""
        step = EscalationStep(
            after_seconds=300,
            action=EscalationAction.ADD_CHANNELS,
            config={
                "channels": ["pagerduty"],
                "pagerduty_config": {"severity": "critical"},
            }
        )
        
        mgr = AlertDispatchManager()
        config = {"slack": {"webhook_url": "http://example.com"}}
        
        result = mgr._apply_escalation_action(step, config)
        
        assert "slack" in result
        assert "pagerduty" in result
        assert result["pagerduty"]["severity"] == "critical"
    
    def test_escalation_increase_severity(self):
        """Escalation should increase severity."""
        step = EscalationStep(
            after_seconds=300,
            action=EscalationAction.INCREASE_SEVERITY,
            config={},
        )
        
        mgr = AlertDispatchManager()
        config = {
            "pagerduty": {
                "severity": "warning",
                "routing_key": "test",
            }
        }
        
        result = mgr._apply_escalation_action(step, config)
        
        assert result["pagerduty"]["severity"] == "error"
    
    def test_escalation_add_tags(self):
        """Escalation should add tags."""
        step = EscalationStep(
            after_seconds=300,
            action=EscalationAction.ADD_TAGS,
            config={"tags": ["escalated", "critical"]},
        )
        
        mgr = AlertDispatchManager()
        config = {
            "opsgenie": {
                "api_key": "test",
                "tags": ["original"],
            }
        }
        
        result = mgr._apply_escalation_action(step, config)
        
        tags = set(result["opsgenie"]["tags"])
        assert "original" in tags
        assert "escalated" in tags
        assert "critical" in tags


class TestAlertDispatchManager:
    """Integration tests for AlertDispatchManager."""
    
    def test_init_creates_default_policies(self):
        """Should create default policies if not provided."""
        mgr = AlertDispatchManager()
        
        assert mgr.retry_policy is not None
        assert mgr.dedup_policy is not None
        assert mgr.escalation_policy is not None
    
    def test_dedup_stats(self):
        """Should provide dedup statistics."""
        mgr = AlertDispatchManager(
            dedup_policy=DeduplicationPolicy(enabled=True)
        )
        
        # Add some dedup entries
        mgr._dedup_entries["key1"] = AlertDeduplicationEntry(
            dedup_key="key1",
            sent_at=datetime.now(),
            dedup_count=5,
        )
        
        stats = mgr.get_dedup_stats()
        
        assert stats["tracked_alerts"] == 1
        assert stats["total_dedup_preventions"] == 5
        assert len(stats["entries"]) == 1
    
    def test_circuit_breaker_stats(self):
        """Should provide circuit breaker statistics."""
        mgr = AlertDispatchManager(
            retry_policy=RetryPolicy(use_circuit_breaker=True)
        )
        
        breaker = CircuitBreakerState(provider="slack")
        breaker.failure_count = 3
        mgr._circuit_breakers["slack"] = breaker
        
        stats = mgr.get_circuit_breaker_stats()
        
        assert len(stats["providers"]) == 1
        assert stats["providers"][0]["provider"] == "slack"
        assert stats["providers"][0]["failure_count"] == 3
    
    def test_cleanup_expired_dedup_entries(self):
        """Should cleanup expired dedup entries."""
        mgr = AlertDispatchManager(
            dedup_policy=DeduplicationPolicy(enabled=True, window_seconds=1)
        )
        
        # Add old entry
        mgr._dedup_entries["old"] = AlertDeduplicationEntry(
            dedup_key="old",
            sent_at=datetime.now() - timedelta(seconds=2),
        )
        
        # Add recent entry
        mgr._dedup_entries["new"] = AlertDeduplicationEntry(
            dedup_key="new",
            sent_at=datetime.now(),
        )
        
        removed = mgr.cleanup_expired_dedup_entries()
        
        assert removed == 1
        assert "old" not in mgr._dedup_entries
        assert "new" in mgr._dedup_entries
    
    def test_reset_circuit_breaker(self):
        """Should reset circuit breaker manually."""
        mgr = AlertDispatchManager(
            retry_policy=RetryPolicy(use_circuit_breaker=True)
        )
        
        breaker = CircuitBreakerState(provider="slack")
        breaker.is_open = True
        breaker.failure_count = 10
        mgr._circuit_breakers["slack"] = breaker
        
        was_open = mgr.reset_circuit_breaker("slack")
        
        assert was_open
        assert not mgr._circuit_breakers["slack"].is_open
        assert mgr._circuit_breaker_stat("slack").failure_count == 0
    
    def test_escalation_history(self):
        """Should track escalation history."""
        mgr = AlertDispatchManager()
        
        mgr._escalation_history["key1"] = [
            {
                "timestamp": "2026-06-27T10:00:00",
                "step": 0,
                "action": "add_channels",
            }
        ]
        
        history = mgr.get_escalation_history("key1")
        
        assert len(history) == 1
        assert history[0]["action"] == "add_channels"


class TestIntegration:
    """Integration tests with mocked dispatcher."""
    
    @patch('neuralbudget.alert_dispatch_advanced.AlertDispatcher')
    def test_dispatch_with_policies_success(self, mock_dispatcher_class):
        """Should dispatch successfully with all policies."""
        # Mock the dispatcher
        mock_dispatcher = Mock()
        mock_dispatcher.send_violation.return_value = AlertDispatchSummary(
            attempted=1,
            succeeded=1,
            failed=0,
            results=[AlertDispatchResult(provider="slack", ok=True)],
        )
        
        mgr = AlertDispatchManager(
            dispatcher=mock_dispatcher,
            dedup_policy=DeduplicationPolicy(enabled=True),
        )
        
        summary = mgr.dispatch_with_policies(
            mode="http",
            profile="strict",
            metric_data={},
            evaluation_result={"violation": True},
            alerts_config={"slack": {"webhook_url": "http://example.com"}},
        )
        
        assert summary.succeeded == 1
        assert summary.failed == 0
    
    @patch('neuralbudget.alert_dispatch_advanced.AlertDispatcher')
    def test_dispatch_respects_deduplication(self, mock_dispatcher_class):
        """Should respect deduplication window."""
        mock_dispatcher = Mock()
        
        mgr = AlertDispatchManager(
            dispatcher=mock_dispatcher,
            dedup_policy=DeduplicationPolicy(enabled=True, window_seconds=300),
        )
        
        config = {"slack": {"webhook_url": "http://example.com"}}
        
        # First dispatch
        mgr.dispatch_with_policies(
            mode="http",
            profile="strict",
            metric_data={},
            evaluation_result={"violation": True},
            alerts_config=config,
        )
        
        # Reset mock
        mock_dispatcher.reset_mock()
        
        # Second identical dispatch (should be deduplicated)
        mgr.dispatch_with_policies(
            mode="http",
            profile="strict",
            metric_data={},
            evaluation_result={"violation": True},
            alerts_config=config,
        )
        
        # Dispatcher should not be called
        mock_dispatcher.send_violation.assert_not_called()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
