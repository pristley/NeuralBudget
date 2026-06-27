"""Advanced alert dispatch system with retry, dedup, and escalation policies.

Extends the basic AlertDispatcher with:
- Retry policies (exponential backoff, jitter, circuit breaker)
- Deduplication (track sent alerts to avoid duplicates)
- Escalation policies (escalate unresolved alerts over time)

Example usage:
    dispatch_mgr = AlertDispatchManager(
        retry_policy=RetryPolicy(max_retries=3, initial_delay_ms=1000),
        dedup_policy=DeduplicationPolicy(enabled=True, window_seconds=300),
        escalation_policy=EscalationPolicy(
            enabled=True,
            steps=[
                EscalationStep(after_seconds=600, action={"add_channels": ["pagerduty"]}),
                EscalationStep(after_seconds=1800, action={"increase_severity": "critical"}),
            ]
        )
    )
    result = dispatch_mgr.dispatch_with_policies(
        mode="http",
        profile="strict",
        metric_data=...,
        evaluation_result=...,
        alerts_config=...
    )
"""

from __future__ import annotations

import hashlib
import json
import logging
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from typing import Any, Mapping, Optional
from collections import defaultdict

from neuralbudget.alerting import AlertDispatcher, AlertDispatchResult, AlertDispatchSummary


logger = logging.getLogger(__name__)


class EscalationAction(str, Enum):
    """Escalation actions for unresolved alerts."""
    
    ADD_CHANNELS = "add_channels"
    INCREASE_SEVERITY = "increase_severity"
    ADD_TAGS = "add_tags"
    MODIFY_CONFIG = "modify_config"
    FORCE_DISPATCH = "force_dispatch"


@dataclass(frozen=True)
class RetryPolicy:
    """Policy for retrying failed alert dispatches.
    
    Implements exponential backoff with jitter to avoid thundering herd.
    """
    
    # Maximum number of retry attempts
    max_retries: int = 3
    
    # Initial delay in milliseconds (doubles on each retry)
    initial_delay_ms: int = 100
    
    # Maximum delay in milliseconds (cap to prevent excessive waits)
    max_delay_ms: int = 30_000
    
    # Backoff multiplier for exponential backoff
    backoff_multiplier: float = 2.0
    
    # Add random jitter to delay (as percentage 0-100)
    jitter_percent: float = 10.0
    
    # If true, use circuit breaker to fail fast on repeated failures
    use_circuit_breaker: bool = True
    
    # Circuit breaker: number of failures before opening
    circuit_breaker_threshold: int = 5
    
    # Circuit breaker: duration to stay open in seconds
    circuit_breaker_open_seconds: int = 60

    def calculate_delay_ms(self, attempt: int) -> int:
        """Calculate delay for a given attempt number (0-indexed)."""
        if attempt < 0:
            return 0
        
        # Exponential backoff
        base_delay = min(
            self.initial_delay_ms * (self.backoff_multiplier ** attempt),
            self.max_delay_ms
        )
        
        # Add jitter
        if self.jitter_percent > 0:
            import random
            jitter = base_delay * (self.jitter_percent / 100.0) * random.random()
            base_delay += jitter
        
        return int(base_delay)


@dataclass(frozen=True)
class DeduplicationPolicy:
    """Policy for deduplicating alert dispatches.
    
    Tracks sent alerts to avoid duplicate notifications within a time window.
    """
    
    # Whether deduplication is enabled
    enabled: bool = True
    
    # Time window for deduplication in seconds
    window_seconds: int = 300  # 5 minutes
    
    # Strategy for generating dedup keys
    # 'content_hash': hash alert content (default)
    # 'custom': use custom dedup_key from config
    key_strategy: str = "content_hash"


@dataclass
class EscalationStep:
    """Single escalation step: trigger after N seconds."""
    
    # Seconds after initial alert before escalating
    after_seconds: int
    
    # Escalation action to take
    action: str  # 'add_channels', 'increase_severity', etc.
    
    # Action-specific configuration
    config: Mapping[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class EscalationPolicy:
    """Policy for escalating unresolved alerts.
    
    Automatically escalates alerts that remain unresolved after specified
    time intervals (e.g., add PagerDuty after 10m, page on-call after 30m).
    """
    
    # Whether escalation is enabled
    enabled: bool = True
    
    # Escalation steps
    steps: list[EscalationStep] = field(default_factory=list)
    
    # Maximum escalations per alert
    max_escalations: int = 10


@dataclass
class AlertDeduplicationEntry:
    """Track an alert that has been recently sent."""
    
    # Unique dedup key for this alert
    dedup_key: str
    
    # When the alert was first sent
    sent_at: datetime
    
    # Number of times this alert was deduplicated (prevented from re-sending)
    dedup_count: int = 0
    
    # Last escalation level applied
    escalation_level: int = 0
    
    # Metadata for escalation decisions
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class CircuitBreakerState:
    """Track circuit breaker state for a provider."""
    
    provider: str
    failure_count: int = 0
    last_failure_time: Optional[datetime] = None
    is_open: bool = False
    opened_at: Optional[datetime] = None


class AlertDispatchManager:
    """Advanced alert dispatch manager with retry, dedup, and escalation.
    
    Wraps the basic AlertDispatcher to add enterprise-grade reliability
    features for production SLO alerting.
    """
    
    def __init__(
        self,
        dispatcher: Optional[AlertDispatcher] = None,
        retry_policy: Optional[RetryPolicy] = None,
        dedup_policy: Optional[DeduplicationPolicy] = None,
        escalation_policy: Optional[EscalationPolicy] = None,
    ):
        """Initialize the alert dispatch manager.
        
        Args:
            dispatcher: Underlying AlertDispatcher (creates new if None)
            retry_policy: Retry configuration
            dedup_policy: Deduplication configuration
            escalation_policy: Escalation configuration
        """
        self.dispatcher = dispatcher or AlertDispatcher()
        self.retry_policy = retry_policy or RetryPolicy()
        self.dedup_policy = dedup_policy or DeduplicationPolicy()
        self.escalation_policy = escalation_policy or EscalationPolicy()
        
        # In-memory deduplication tracking
        self._dedup_entries: dict[str, AlertDeduplicationEntry] = {}
        
        # Circuit breaker state per provider
        self._circuit_breakers: dict[str, CircuitBreakerState] = {}
        
        # Escalation history
        self._escalation_history: dict[str, list[dict[str, Any]]] = defaultdict(list)

    def dispatch_with_policies(
        self,
        *,
        mode: str,
        profile: str | None,
        metric_data: Mapping[str, Any],
        evaluation_result: Mapping[str, Any],
        alerts_config: Mapping[str, Any],
    ) -> AlertDispatchSummary:
        """Dispatch alerts with retry, dedup, and escalation policies.
        
        Args:
            mode: SLO evaluation mode (e.g., 'http', 'stateful')
            profile: Optional SLO profile name
            metric_data: Raw metric data that triggered the alert
            evaluation_result: SLO evaluation result that triggered alert
            alerts_config: Alert configuration with provider settings
        
        Returns:
            AlertDispatchSummary with results and stats
        """
        # Generate dedup key
        dedup_key = self._generate_dedup_key(mode, profile, evaluation_result)
        
        # Check deduplication
        if self._should_skip_due_to_dedup(dedup_key):
            logger.info(f"Alert deduplicated (key={dedup_key})")
            return AlertDispatchSummary(
                attempted=0,
                succeeded=0,
                failed=0,
                results=[],
            )
        
        # Check circuit breakers and filter config
        filtered_config = self._apply_circuit_breaker_filters(alerts_config)
        
        if not self._has_any_enabled_providers(filtered_config):
            logger.warning("All providers filtered by circuit breaker")
            return AlertDispatchSummary(
                attempted=0,
                succeeded=0,
                failed=0,
                results=[],
            )
        
        # Dispatch with retries
        summary = self._dispatch_with_retries(
            mode=mode,
            profile=profile,
            metric_data=metric_data,
            evaluation_result=evaluation_result,
            alerts_config=filtered_config,
        )
        
        # Update dedup tracking and handle escalation
        self._update_dedup_tracking(dedup_key, summary, alerts_config)
        
        # Check for escalation
        escalated_summary = self._apply_escalation(
            dedup_key=dedup_key,
            mode=mode,
            profile=profile,
            metric_data=metric_data,
            evaluation_result=evaluation_result,
            alerts_config=alerts_config,
            initial_summary=summary,
        )
        
        # Update circuit breaker state
        self._update_circuit_breakers(summary)
        
        return escalated_summary

    def _dispatch_with_retries(
        self,
        *,
        mode: str,
        profile: str | None,
        metric_data: Mapping[str, Any],
        evaluation_result: Mapping[str, Any],
        alerts_config: Mapping[str, Any],
    ) -> AlertDispatchSummary:
        """Dispatch with retries on failure."""
        last_summary = None
        
        for attempt in range(self.retry_policy.max_retries + 1):
            try:
                summary = self.dispatcher.send_violation(
                    mode=mode,
                    profile=profile,
                    metric_data=metric_data,
                    result=evaluation_result,
                    alerts_config=alerts_config,
                )
                
                # Check if successful
                if summary.failed == 0:
                    return summary
                
                last_summary = summary
                
                # Determine if we should retry
                if attempt < self.retry_policy.max_retries:
                    delay_ms = self.retry_policy.calculate_delay_ms(attempt)
                    logger.warning(
                        f"Dispatch attempt {attempt + 1} failed "
                        f"({summary.failed}/{summary.attempted}), "
                        f"retrying in {delay_ms}ms"
                    )
                    time.sleep(delay_ms / 1000.0)
                else:
                    logger.error(
                        f"Dispatch failed after {attempt + 1} attempts: "
                        f"{summary.failed}/{summary.attempted} failed"
                    )
                    return summary
                    
            except Exception as exc:
                logger.error(f"Dispatch error on attempt {attempt + 1}: {exc}")
                if attempt < self.retry_policy.max_retries:
                    delay_ms = self.retry_policy.calculate_delay_ms(attempt)
                    time.sleep(delay_ms / 1000.0)
                else:
                    raise
        
        return last_summary or AlertDispatchSummary(
            attempted=0,
            succeeded=0,
            failed=0,
            results=[],
        )

    def _generate_dedup_key(
        self,
        mode: str,
        profile: str | None,
        evaluation_result: Mapping[str, Any],
    ) -> str:
        """Generate a deduplication key for an alert.
        
        By default, hashes the alert content to generate a stable key.
        """
        if self.dedup_policy.key_strategy == "custom":
            # Custom keys should be provided in the config
            return f"{mode}:{profile}:custom"
        
        # Content hash strategy: hash the alert content
        content = {
            "mode": mode,
            "profile": profile,
            "result": evaluation_result,
        }
        content_json = json.dumps(content, sort_keys=True)
        key_hash = hashlib.sha256(content_json.encode()).hexdigest()[:16]
        return f"{mode}:{profile}:{key_hash}"

    def _should_skip_due_to_dedup(self, dedup_key: str) -> bool:
        """Check if alert should be skipped due to deduplication."""
        if not self.dedup_policy.enabled:
            return False
        
        if dedup_key not in self._dedup_entries:
            return False
        
        entry = self._dedup_entries[dedup_key]
        age = (datetime.now() - entry.sent_at).total_seconds()
        
        if age > self.dedup_policy.window_seconds:
            # Dedup window expired, allow re-send
            del self._dedup_entries[dedup_key]
            return False
        
        # Within dedup window, skip
        entry.dedup_count += 1
        return True

    def _apply_circuit_breaker_filters(
        self,
        alerts_config: Mapping[str, Any],
    ) -> Mapping[str, Any]:
        """Filter providers based on circuit breaker state."""
        if not self.retry_policy.use_circuit_breaker:
            return alerts_config
        
        filtered = dict(alerts_config)
        now = datetime.now()
        
        for provider in ["slack", "pagerduty", "opsgenie"]:
            if provider not in self._circuit_breakers:
                continue
            
            breaker = self._circuit_breakers[provider]
            
            # Check if circuit should be reset
            if breaker.is_open and breaker.opened_at is not None:
                age = (now - breaker.opened_at).total_seconds()
                if age > self.retry_policy.circuit_breaker_open_seconds:
                    logger.info(f"Circuit breaker resetting for {provider}")
                    breaker.is_open = False
                    breaker.failure_count = 0
                    continue
            
            # Disable provider if circuit is open
            if breaker.is_open:
                logger.warning(f"Circuit breaker open for {provider}, disabling")
                if provider in filtered:
                    del filtered[provider]
        
        return filtered

    def _has_any_enabled_providers(self, alerts_config: Mapping[str, Any]) -> bool:
        """Check if any alert providers are enabled."""
        for provider in ["slack", "pagerduty", "opsgenie"]:
            if provider in alerts_config:
                config = alerts_config[provider]
                if isinstance(config, dict) and config:
                    return True
        return False

    def _update_dedup_tracking(
        self,
        dedup_key: str,
        summary: AlertDispatchSummary,
        alerts_config: Mapping[str, Any],
    ) -> None:
        """Update deduplication tracking after dispatch."""
        if not self.dedup_policy.enabled:
            return
        
        if summary.succeeded > 0:
            self._dedup_entries[dedup_key] = AlertDeduplicationEntry(
                dedup_key=dedup_key,
                sent_at=datetime.now(),
            )

    def _update_circuit_breakers(self, summary: AlertDispatchSummary) -> None:
        """Update circuit breaker state based on dispatch results."""
        if not self.retry_policy.use_circuit_breaker:
            return
        
        for result in summary.results:
            provider = result.provider
            
            if provider not in self._circuit_breakers:
                self._circuit_breakers[provider] = CircuitBreakerState(provider=provider)
            
            breaker = self._circuit_breakers[provider]
            now = datetime.now()
            
            if result.ok:
                # Success: reset failure count
                breaker.failure_count = 0
                if breaker.is_open:
                    logger.info(f"Circuit breaker healing for {provider}")
                    breaker.is_open = False
            else:
                # Failure: increment counter
                breaker.failure_count += 1
                breaker.last_failure_time = now
                
                # Check if threshold exceeded
                if (
                    breaker.failure_count >= self.retry_policy.circuit_breaker_threshold
                    and not breaker.is_open
                ):
                    logger.error(
                        f"Circuit breaker opening for {provider} "
                        f"(failures: {breaker.failure_count})"
                    )
                    breaker.is_open = True
                    breaker.opened_at = now

    def _apply_escalation(
        self,
        *,
        dedup_key: str,
        mode: str,
        profile: str | None,
        metric_data: Mapping[str, Any],
        evaluation_result: Mapping[str, Any],
        alerts_config: Mapping[str, Any],
        initial_summary: AlertDispatchSummary,
    ) -> AlertDispatchSummary:
        """Apply escalation policies if alert remains unresolved."""
        if not self.escalation_policy.enabled:
            return initial_summary
        
        if not self.escalation_policy.steps:
            return initial_summary
        
        if dedup_key not in self._dedup_entries:
            return initial_summary
        
        entry = self._dedup_entries[dedup_key]
        
        # Check which escalation steps are due
        now = datetime.now()
        age = (now - entry.sent_at).total_seconds()
        
        escalations_to_apply = []
        for i, step in enumerate(self.escalation_policy.steps):
            if (
                age >= step.after_seconds
                and i > entry.escalation_level
                and i < entry.escalation_level + self.escalation_policy.max_escalations
            ):
                escalations_to_apply.append(step)
        
        if not escalations_to_apply:
            return initial_summary
        
        logger.info(
            f"Escalating alert {dedup_key}: "
            f"{len(escalations_to_apply)} step(s) applicable"
        )
        
        escalated_config = dict(alerts_config)
        escalation_results = []
        
        for i, step in enumerate(escalations_to_apply):
            logger.info(f"Executing escalation step: {step.action}")
            
            # Apply escalation action
            escalated_config = self._apply_escalation_action(
                step=step,
                config=escalated_config,
            )
            
            # Record escalation
            self._escalation_history[dedup_key].append({
                "timestamp": now.isoformat(),
                "step": i,
                "action": step.action,
                "config": step.config,
            })
            
            entry.escalation_level = i + 1
        
        # Send escalated alerts
        if escalated_config != alerts_config:
            escalation_summary = self.dispatcher.send_violation(
                mode=mode,
                profile=profile,
                metric_data=metric_data,
                result=evaluation_result,
                alerts_config=escalated_config,
            )
            escalation_results.extend(escalation_summary.results)
        
        # Combine initial and escalation results
        all_results = list(initial_summary.results) + escalation_results
        return AlertDispatchSummary(
            attempted=len(all_results),
            succeeded=sum(1 for r in all_results if r.ok),
            failed=sum(1 for r in all_results if not r.ok),
            results=all_results,
        )

    def _apply_escalation_action(
        self,
        step: EscalationStep,
        config: Mapping[str, Any],
    ) -> dict[str, Any]:
        """Apply a single escalation action to alert config."""
        result = dict(config)
        
        if step.action == EscalationAction.ADD_CHANNELS:
            # Add additional notification channels
            channels = step.config.get("channels", [])
            for channel in channels:
                if channel == "pagerduty" and "pagerduty" not in result:
                    result["pagerduty"] = step.config.get("pagerduty_config", {
                        "severity": "critical",
                    })
                elif channel == "opsgenie" and "opsgenie" not in result:
                    result["opsgenie"] = step.config.get("opsgenie_config", {
                        "priority": "P1",
                    })
        
        elif step.action == EscalationAction.INCREASE_SEVERITY:
            # Increase severity in existing channels
            severity_map = {
                "low": "medium",
                "medium": "high",
                "high": "critical",
                "P4": "P3",
                "P3": "P2",
                "P2": "P1",
            }
            
            if "pagerduty" in result and isinstance(result["pagerduty"], dict):
                old_severity = result["pagerduty"].get("severity", "error")
                new_severity = severity_map.get(old_severity, "critical")
                result["pagerduty"] = dict(result["pagerduty"])
                result["pagerduty"]["severity"] = new_severity
            
            if "opsgenie" in result and isinstance(result["opsgenie"], dict):
                old_priority = result["opsgenie"].get("priority", "P3")
                new_priority = severity_map.get(old_priority, "P1")
                result["opsgenie"] = dict(result["opsgenie"])
                result["opsgenie"]["priority"] = new_priority
        
        elif step.action == EscalationAction.ADD_TAGS:
            # Add tags to alert
            tags = step.config.get("tags", [])
            for provider in ["opsgenie", "pagerduty"]:
                if provider in result and isinstance(result[provider], dict):
                    result[provider] = dict(result[provider])
                    existing_tags = result[provider].get("tags", [])
                    result[provider]["tags"] = list(set(existing_tags) | set(tags))
        
        elif step.action == EscalationAction.MODIFY_CONFIG:
            # Directly modify provider config
            provider = step.config.get("provider")
            modifications = step.config.get("modifications", {})
            if provider in result and isinstance(result[provider], dict):
                result[provider] = {**result[provider], **modifications}
        
        return result

    def get_dedup_stats(self) -> dict[str, Any]:
        """Get deduplication statistics."""
        return {
            "tracked_alerts": len(self._dedup_entries),
            "total_dedup_preventions": sum(
                e.dedup_count for e in self._dedup_entries.values()
            ),
            "entries": [
                {
                    "key": key,
                    "sent_at": entry.sent_at.isoformat(),
                    "dedup_count": entry.dedup_count,
                    "escalation_level": entry.escalation_level,
                }
                for key, entry in self._dedup_entries.items()
            ],
        }

    def get_circuit_breaker_stats(self) -> dict[str, Any]:
        """Get circuit breaker statistics."""
        return {
            "providers": [
                {
                    "provider": breaker.provider,
                    "is_open": breaker.is_open,
                    "failure_count": breaker.failure_count,
                    "last_failure": (
                        breaker.last_failure_time.isoformat()
                        if breaker.last_failure_time
                        else None
                    ),
                }
                for breaker in self._circuit_breakers.values()
            ],
        }

    def get_escalation_history(self, dedup_key: str) -> list[dict[str, Any]]:
        """Get escalation history for a specific alert."""
        return self._escalation_history.get(dedup_key, [])

    def cleanup_expired_dedup_entries(self) -> int:
        """Remove expired deduplication entries.
        
        Returns:
            Number of entries removed
        """
        now = datetime.now()
        expired_keys = [
            key
            for key, entry in self._dedup_entries.items()
            if (now - entry.sent_at).total_seconds() > self.dedup_policy.window_seconds
        ]
        
        for key in expired_keys:
            del self._dedup_entries[key]
        
        return len(expired_keys)

    def reset_circuit_breaker(self, provider: str) -> bool:
        """Manually reset circuit breaker for a provider.
        
        Returns:
            True if breaker was open, False otherwise
        """
        if provider not in self._circuit_breakers:
            return False
        
        breaker = self._circuit_breakers[provider]
        was_open = breaker.is_open
        breaker.is_open = False
        breaker.failure_count = 0
        breaker.opened_at = None
        
        if was_open:
            logger.info(f"Circuit breaker manually reset for {provider}")
        
        return was_open
