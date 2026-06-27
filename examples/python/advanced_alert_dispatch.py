#!/usr/bin/env python3
"""Advanced alert dispatch examples with retry, dedup, and escalation.

Demonstrates the enterprise-grade alert delivery system with:
- Automatic retries with exponential backoff and jitter
- Deduplication to prevent alert storms
- Escalation policies to ensure critical issues get attention
"""

import json
import logging
import time
from datetime import datetime, timedelta

from neuralbudget import NeuralBudgetClient
from neuralbudget.alert_dispatch_advanced import (
    AlertDispatchManager,
    RetryPolicy,
    DeduplicationPolicy,
    EscalationPolicy,
    EscalationStep,
    EscalationAction,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


def example_1_basic_retry_policy():
    """Example 1: Basic retry with exponential backoff."""
    print("\n=== Example 1: Retry with Exponential Backoff ===\n")
    
    retry_policy = RetryPolicy(
        max_retries=3,
        initial_delay_ms=100,
        max_delay_ms=10_000,
        backoff_multiplier=2.0,
        jitter_percent=10.0,
    )
    
    print("Retry configuration:")
    print(f"  Max retries: {retry_policy.max_retries}")
    print(f"  Initial delay: {retry_policy.initial_delay_ms}ms")
    print(f"  Max delay: {retry_policy.max_delay_ms}ms")
    print(f"  Backoff multiplier: {retry_policy.backoff_multiplier}x")
    print(f"  Jitter: {retry_policy.jitter_percent}%")
    
    print("\nCalculated delays per attempt:")
    for attempt in range(retry_policy.max_retries + 1):
        delay = retry_policy.calculate_delay_ms(attempt)
        print(f"  Attempt {attempt}: {delay}ms")


def example_2_deduplication():
    """Example 2: Deduplication to prevent alert storms."""
    print("\n=== Example 2: Deduplication ===\n")
    
    dedup_policy = DeduplicationPolicy(
        enabled=True,
        window_seconds=300,  # 5 minutes
        key_strategy="content_hash",
    )
    
    print("Deduplication configuration:")
    print(f"  Enabled: {dedup_policy.enabled}")
    print(f"  Window: {dedup_policy.window_seconds}s (5 minutes)")
    print(f"  Strategy: {dedup_policy.key_strategy}")
    
    print("\nBehavior:")
    print("  1. First alert triggered at T=0s → SENT")
    print("  2. Identical alert at T=60s → DEDUPLICATED (same content)")
    print("  3. Identical alert at T=120s → DEDUPLICATED")
    print("  4. Identical alert at T=400s → SENT (window expired)")


def example_3_circuit_breaker():
    """Example 3: Circuit breaker for provider reliability."""
    print("\n=== Example 3: Circuit Breaker ===\n")
    
    retry_policy = RetryPolicy(
        use_circuit_breaker=True,
        circuit_breaker_threshold=5,  # Open after 5 failures
        circuit_breaker_open_seconds=60,  # Stay open for 60s
    )
    
    print("Circuit breaker configuration:")
    print(f"  Threshold: {retry_policy.circuit_breaker_threshold} failures")
    print(f"  Open duration: {retry_policy.circuit_breaker_open_seconds}s")
    
    print("\nFlow:")
    print("  1. Failures 1-4: Retry normally")
    print("  2. Failure 5: Circuit opens (fail fast)")
    print("  3. Subsequent requests fail immediately")
    print("  4. After 60s: Circuit half-open, try recovery")
    print("  5. Success: Circuit closes, normal operation resumes")


def example_4_escalation_policy():
    """Example 4: Escalation for unresolved alerts."""
    print("\n=== Example 4: Escalation Policies ===\n")
    
    escalation_policy = EscalationPolicy(
        enabled=True,
        steps=[
            EscalationStep(
                after_seconds=300,  # After 5 minutes
                action=EscalationAction.ADD_CHANNELS,
                config={
                    "channels": ["pagerduty"],
                    "pagerduty_config": {"severity": "error"},
                }
            ),
            EscalationStep(
                after_seconds=900,  # After 15 minutes
                action=EscalationAction.INCREASE_SEVERITY,
                config={}
            ),
            EscalationStep(
                after_seconds=1800,  # After 30 minutes
                action=EscalationAction.ADD_CHANNELS,
                config={
                    "channels": ["opsgenie"],
                    "opsgenie_config": {"priority": "P1"},
                }
            ),
        ],
        max_escalations=10,
    )
    
    print("Escalation timeline:")
    for i, step in enumerate(escalation_policy.steps):
        minutes = step.after_seconds // 60
        print(f"  T+{minutes}m: {step.action}")
        if step.action == EscalationAction.ADD_CHANNELS:
            channels = step.config.get("channels", [])
            print(f"           Add: {', '.join(channels)}")
        elif step.action == EscalationAction.INCREASE_SEVERITY:
            print(f"           Escalate to critical")


def example_5_full_dispatch_flow():
    """Example 5: Complete dispatch flow with all policies."""
    print("\n=== Example 5: Full Dispatch Flow ===\n")
    
    # Configure policies
    retry_policy = RetryPolicy(
        max_retries=2,
        initial_delay_ms=100,
        backoff_multiplier=2.0,
    )
    
    dedup_policy = DeduplicationPolicy(
        enabled=True,
        window_seconds=300,
    )
    
    escalation_policy = EscalationPolicy(
        enabled=True,
        steps=[
            EscalationStep(
                after_seconds=300,
                action=EscalationAction.ADD_CHANNELS,
                config={"channels": ["pagerduty"]},
            ),
        ],
    )
    
    # Create manager
    dispatch_mgr = AlertDispatchManager(
        retry_policy=retry_policy,
        dedup_policy=dedup_policy,
        escalation_policy=escalation_policy,
    )
    
    print("Dispatch manager created with:")
    print(f"  Retry: max={retry_policy.max_retries}, backoff={retry_policy.backoff_multiplier}x")
    print(f"  Dedup: enabled, window={dedup_policy.window_seconds}s")
    print(f"  Escalation: {len(escalation_policy.steps)} steps")
    
    print("\nSimulated dispatch sequence:")
    print("  [T+0s]   Alert triggered")
    print("  [T+0s]   → Dedup check: PASS (new alert)")
    print("  [T+0s]   → Circuit breaker: PASS (all healthy)")
    print("  [T+0s]   → Send attempt 1: FAIL (timeout)")
    print("  [T+0.1s] → Backoff 100ms, retry attempt 2: FAIL")
    print("  [T+0.3s] → Backoff 200ms, retry attempt 3: SUCCESS ✓")
    print("  [T+0.5s] → Alert dispatched to Slack")
    print("\n  [T+60s]  Identical alert triggered")
    print("  [T+60s]  → Dedup check: SKIP (within 300s window)")
    print("  [T+60s]  → Alert prevention: 1 dedup prevented")
    print("\n  [T+400s] Same alert triggered again")
    print("  [T+400s] → Dedup check: PASS (window expired)")
    print("  [T+400s] → Send successful ✓")
    print("\n  [T+420s] Unresolved for 20m → Escalation trigger")
    print("  [T+420s] → Add PagerDuty channel")
    print("  [T+420s] → Send escalated alert to PagerDuty ✓")


def example_6_monitoring_dispatch_health():
    """Example 6: Monitor dispatch health and statistics."""
    print("\n=== Example 6: Monitoring Dispatch Health ===\n")
    
    dispatch_mgr = AlertDispatchManager(
        retry_policy=RetryPolicy(use_circuit_breaker=True),
        dedup_policy=DeduplicationPolicy(enabled=True),
        escalation_policy=EscalationPolicy(enabled=True),
    )
    
    # Simulate some activity
    print("Simulated dispatch statistics:\n")
    
    # Dedup stats
    dedup_stats = {
        "tracked_alerts": 23,
        "total_dedup_preventions": 156,
        "entries": [
            {
                "key": "http:strict:abc123",
                "sent_at": "2026-06-27T10:30:00",
                "dedup_count": 8,
                "escalation_level": 1,
            },
            {
                "key": "stateful:default:def456",
                "sent_at": "2026-06-27T10:45:00",
                "dedup_count": 12,
                "escalation_level": 0,
            },
        ]
    }
    
    print("Deduplication Statistics:")
    print(f"  Tracked alerts: {dedup_stats['tracked_alerts']}")
    print(f"  Prevention events: {dedup_stats['total_dedup_preventions']}")
    print(f"  Examples:")
    for entry in dedup_stats["entries"][:2]:
        print(f"    - {entry['key']}: {entry['dedup_count']} prevented sends")
    
    # Circuit breaker stats
    cb_stats = {
        "providers": [
            {
                "provider": "slack",
                "is_open": False,
                "failure_count": 0,
                "last_failure": None,
            },
            {
                "provider": "pagerduty",
                "is_open": False,
                "failure_count": 2,
                "last_failure": "2026-06-27T10:52:00",
            },
            {
                "provider": "opsgenie",
                "is_open": True,
                "failure_count": 5,
                "last_failure": "2026-06-27T10:58:00",
            },
        ]
    }
    
    print("\nCircuit Breaker Statistics:")
    for provider in cb_stats["providers"]:
        status = "OPEN ⚠️" if provider["is_open"] else "CLOSED ✓"
        print(f"  {provider['provider']}: {status}")
        print(f"    Failures: {provider['failure_count']}")
        if provider["last_failure"]:
            print(f"    Last: {provider['last_failure']}")
    
    # Escalation history
    escalation_history = [
        {
            "timestamp": "2026-06-27T10:35:00",
            "step": 0,
            "action": "add_channels",
            "config": {"channels": ["pagerduty"]},
        },
        {
            "timestamp": "2026-06-27T10:45:00",
            "step": 1,
            "action": "increase_severity",
            "config": {},
        },
    ]
    
    print("\nEscalation History (for alert http:strict:abc123):")
    for event in escalation_history:
        print(f"  {event['timestamp']}: {event['action']}")


def example_7_custom_retry_strategy():
    """Example 7: Custom retry strategy for critical services."""
    print("\n=== Example 7: Custom Retry Strategy ===\n")
    
    # Aggressive retry for critical payment service
    critical_retry = RetryPolicy(
        max_retries=5,
        initial_delay_ms=50,
        max_delay_ms=60_000,
        backoff_multiplier=3.0,  # Faster escalation
        jitter_percent=5.0,       # Less jitter
        use_circuit_breaker=False,  # Always try for critical services
    )
    
    # Conservative retry for best-effort service
    best_effort_retry = RetryPolicy(
        max_retries=1,
        initial_delay_ms=200,
        max_delay_ms=5_000,
        backoff_multiplier=1.5,
        jitter_percent=20.0,
        use_circuit_breaker=True,
    )
    
    print("Critical Service Retry (e.g., payment):")
    print(f"  Max retries: {critical_retry.max_retries}")
    print(f"  Backoff: {critical_retry.backoff_multiplier}x (faster escalation)")
    print(f"  Circuit breaker: DISABLED (must try hard)")
    
    print("\nBest-Effort Service Retry:")
    print(f"  Max retries: {best_effort_retry.max_retries}")
    print(f"  Backoff: {best_effort_retry.backoff_multiplier}x (gentle)")
    print(f"  Circuit breaker: ENABLED (fail fast)")


def example_8_dedup_configuration_strategies():
    """Example 8: Different deduplication strategies."""
    print("\n=== Example 8: Deduplication Strategies ===\n")
    
    strategies = [
        {
            "name": "Strict (10s window)",
            "config": DeduplicationPolicy(
                enabled=True,
                window_seconds=10,
                key_strategy="content_hash",
            ),
            "use_case": "High-volume services, can tolerate message loss",
        },
        {
            "name": "Balanced (5m window)",
            "config": DeduplicationPolicy(
                enabled=True,
                window_seconds=300,
                key_strategy="content_hash",
            ),
            "use_case": "Typical SLO monitoring, balanced approach",
        },
        {
            "name": "Lenient (30m window)",
            "config": DeduplicationPolicy(
                enabled=True,
                window_seconds=1800,
                key_strategy="content_hash",
            ),
            "use_case": "Critical services, avoid alert storms",
        },
        {
            "name": "Disabled",
            "config": DeduplicationPolicy(enabled=False),
            "use_case": "Important events that should never be deduplicated",
        },
    ]
    
    print("Deduplication Strategy Comparison:\n")
    for strategy in strategies:
        print(f"Strategy: {strategy['name']}")
        if strategy['config'].enabled:
            print(f"  Window: {strategy['config'].window_seconds}s")
        print(f"  Use Case: {strategy['use_case']}")
        print()


def example_9_escalation_scenarios():
    """Example 9: Real-world escalation scenarios."""
    print("\n=== Example 9: Real-World Escalation Scenarios ===\n")
    
    print("Scenario 1: Payment Service (Aggressive)")
    print("  T+0m   → Slack notification")
    print("  T+5m   → Add PagerDuty (medium severity)")
    print("  T+10m  → Increase to high severity")
    print("  T+15m  → Add OpsGenie + executive team")
    print("  → Ensures quick response for revenue-impacting issues\n")
    
    print("Scenario 2: Batch Processing (Moderate)")
    print("  T+0m   → Slack notification")
    print("  T+30m  → Add PagerDuty (low severity)")
    print("  T+60m  → Increase severity + add OpsGenie")
    print("  → Balances urgency with alert fatigue\n")
    
    print("Scenario 3: Monitoring Service (Conservative)")
    print("  T+0m   → Slack notification only")
    print("  T+120m → Add PagerDuty (low severity)")
    print("  T+360m → Add OpsGenie for 24h+ outages")
    print("  → Prioritizes other alerts, handles gracefully\n")


def example_10_integration_with_client():
    """Example 10: Integration with NeuralBudgetClient."""
    print("\n=== Example 10: Integration with NeuralBudgetClient ===\n")
    
    print("Integration pattern:\n")
    print("""
    # 1. Create client
    client = NeuralBudgetClient().load_config("config.json")
    
    # 2. Create advanced dispatch manager
    dispatch_mgr = AlertDispatchManager(
        retry_policy=RetryPolicy(max_retries=3),
        dedup_policy=DeduplicationPolicy(enabled=True),
        escalation_policy=EscalationPolicy(enabled=True, steps=[...]),
    )
    
    # 3. Evaluate SLO
    result = client.evaluate(metric_data)
    
    # 4. If alert needed, use advanced dispatch
    if result.get("violation"):
        summary = dispatch_mgr.dispatch_with_policies(
            mode="http",
            profile="strict",
            metric_data=metric_data,
            evaluation_result=result,
            alerts_config=alerts_config,
        )
        
        # 5. Monitor health
        dedup_stats = dispatch_mgr.get_dedup_stats()
        cb_stats = dispatch_mgr.get_circuit_breaker_stats()
    """)


if __name__ == "__main__":
    print("╔════════════════════════════════════════════════════════════╗")
    print("║  Advanced Alert Dispatch Examples                          ║")
    print("║  Retry • Deduplication • Escalation • Circuit Breaker      ║")
    print("╚════════════════════════════════════════════════════════════╝")
    
    example_1_basic_retry_policy()
    example_2_deduplication()
    example_3_circuit_breaker()
    example_4_escalation_policy()
    example_5_full_dispatch_flow()
    example_6_monitoring_dispatch_health()
    example_7_custom_retry_strategy()
    example_8_dedup_configuration_strategies()
    example_9_escalation_scenarios()
    example_10_integration_with_client()
    
    print("\n✓ All examples completed!\n")
