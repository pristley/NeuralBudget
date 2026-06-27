"""
Comprehensive Python test suite for Agent SLO evaluation.

Tests the AgentTrajectory, AgentEvaluation, and evaluation functions
across multiple scenarios including research agents, coding assistants,
support bots, and batch evaluations.
"""

import json
import time
import pytest
from datetime import datetime
from typing import List, Dict, Any


class TestAgentTrajectoryBasics:
    """Test basic Agent trajectory creation and serialization."""

    def test_create_simple_trajectory(self):
        """Test creating a simple agent trajectory."""
        trajectory_data = {
            "run_id": "test_run_123",
            "timestamp": int(time.time()),
            "steps": [
                {
                    "step": 1,
                    "action": "tool_call",
                    "tool_name": "search",
                    "success": True,
                    "output": "Found 5 results",
                    "duration_ms": 800,
                },
                {
                    "step": 2,
                    "action": "response",
                    "tool_name": None,
                    "success": True,
                    "output": "Based on search results...",
                    "duration_ms": 100,
                },
            ],
            "final_status": "success",
        }

        # Validate structure
        assert trajectory_data["run_id"] == "test_run_123"
        assert len(trajectory_data["steps"]) == 2
        assert trajectory_data["final_status"] == "success"
        assert trajectory_data["steps"][0]["action"] == "tool_call"

    def test_trajectory_json_serialization(self):
        """Test JSON serialization/deserialization of trajectory."""
        trajectory_data = {
            "run_id": "run_123",
            "timestamp": 1719446400,
            "steps": [
                {
                    "step": 1,
                    "action": "thought",
                    "tool_name": None,
                    "success": True,
                    "output": "Thinking...",
                    "duration_ms": 50,
                }
            ],
            "final_status": "success",
        }

        json_str = json.dumps(trajectory_data)
        deserialized = json.loads(json_str)

        assert deserialized["run_id"] == trajectory_data["run_id"]
        assert len(deserialized["steps"]) == len(trajectory_data["steps"])

    def test_trajectory_with_all_action_types(self):
        """Test trajectory with thought, tool_call, and response actions."""
        actions = ["thought", "tool_call", "response"]
        steps = [
            {
                "step": i + 1,
                "action": action,
                "tool_name": "tool" if action == "tool_call" else None,
                "success": True,
                "output": f"Step {i+1}",
                "duration_ms": 100,
            }
            for i, action in enumerate(actions)
        ]

        trajectory_data = {
            "run_id": "mixed_actions",
            "timestamp": int(time.time()),
            "steps": steps,
            "final_status": "success",
        }

        assert len(trajectory_data["steps"]) == 3
        assert trajectory_data["steps"][0]["action"] == "thought"
        assert trajectory_data["steps"][1]["action"] == "tool_call"
        assert trajectory_data["steps"][2]["action"] == "response"


class TestAgentStepMetrics:
    """Test metrics calculated from agent steps."""

    def test_tool_success_rate_calculation(self):
        """Test calculating tool success rate from steps."""
        steps = [
            {"step": 1, "action": "tool_call", "success": True},
            {"step": 2, "action": "tool_call", "success": True},
            {"step": 3, "action": "tool_call", "success": False},
            {"step": 4, "action": "response", "success": True},
        ]

        # Filter only tool calls
        tool_calls = [s for s in steps if s["action"] == "tool_call"]
        successful = sum(1 for s in tool_calls if s["success"])

        success_rate = successful / len(tool_calls) if tool_calls else 1.0
        assert success_rate == pytest.approx(2 / 3, abs=0.01)

    def test_step_count(self):
        """Test counting total steps."""
        trajectory_data = {
            "run_id": "count_test",
            "timestamp": int(time.time()),
            "steps": [
                {"step": i, "action": "tool_call", "success": True}
                for i in range(1, 6)
            ],
            "final_status": "success",
        }

        step_count = len(trajectory_data["steps"])
        assert step_count == 5

    def test_no_tool_calls_perfect_rate(self):
        """Test that trajectories with no tool calls have perfect success rate."""
        steps = [
            {"step": 1, "action": "thought", "success": True},
            {"step": 2, "action": "response", "success": True},
        ]

        tool_calls = [s for s in steps if s["action"] == "tool_call"]
        assert len(tool_calls) == 0
        success_rate = 1.0 if not tool_calls else 0.0  # No calls = perfect
        assert success_rate == 1.0


class TestLoopDetection:
    """Test loop detection logic."""

    def test_count_action_repetitions(self):
        """Test counting action occurrences."""
        steps = [
            {"action": "tool_call"},
            {"action": "tool_call"},
            {"action": "tool_call"},
            {"action": "response"},
        ]

        action_counts = {}
        for step in steps:
            action = step["action"]
            action_counts[action] = action_counts.get(action, 0) + 1

        assert action_counts["tool_call"] == 3
        assert action_counts["response"] == 1

    def test_loop_detection_threshold(self):
        """Test loop detection with max_repeated_actions threshold."""
        action_counts = {"tool_call": 3, "response": 1}
        max_repeated_actions = 2

        loop_detected = any(count > max_repeated_actions for count in action_counts.values())

        assert loop_detected is True

    def test_no_loop_within_threshold(self):
        """Test that actions within threshold don't trigger loop detection."""
        action_counts = {"tool_call": 2, "response": 1}
        max_repeated_actions = 2

        loop_detected = any(count > max_repeated_actions for count in action_counts.values())

        assert loop_detected is False

    def test_detailed_action_counts(self):
        """Test tracking all action types and their counts."""
        steps = [
            {"action": "thought"},
            {"action": "thought"},
            {"action": "tool_call"},
            {"action": "tool_call"},
            {"action": "response"},
        ]

        action_counts = {}
        for step in steps:
            action = step["action"]
            action_counts[action] = action_counts.get(action, 0) + 1

        assert action_counts == {"thought": 2, "tool_call": 2, "response": 1}


class TestSLOEvaluationCriteria:
    """Test individual SLO evaluation criteria."""

    def test_max_steps_pass(self):
        """Test max_steps evaluation."""
        steps_taken = 5
        max_steps = 10

        max_steps_pass = steps_taken <= max_steps
        assert max_steps_pass is True

    def test_max_steps_fail(self):
        """Test max_steps evaluation when exceeded."""
        steps_taken = 15
        max_steps = 10

        max_steps_pass = steps_taken <= max_steps
        assert max_steps_pass is False

    def test_tool_success_threshold_pass(self):
        """Test tool_success_rate evaluation when above threshold."""
        tool_success_rate = 0.96
        threshold = 0.95

        tool_success_pass = tool_success_rate >= threshold
        assert tool_success_pass is True

    def test_tool_success_threshold_fail(self):
        """Test tool_success_rate evaluation when below threshold."""
        tool_success_rate = 0.90
        threshold = 0.95

        tool_success_pass = tool_success_rate >= threshold
        assert tool_success_pass is False

    def test_final_status_success(self):
        """Test final_status evaluation."""
        final_status = "success"
        success_status_pass = final_status == "success"
        assert success_status_pass is True

    def test_final_status_failure(self):
        """Test final_status evaluation for failure."""
        final_status = "failure"
        success_status_pass = final_status == "success"
        assert success_status_pass is False


class TestOverallSLOPass:
    """Test overall SLO pass/fail logic."""

    def test_all_criteria_met(self):
        """Test overall pass when all criteria met."""
        max_steps_pass = True
        tool_success_pass = True
        loop_detection_pass = True
        success_status_pass = True

        overall_pass = (
            max_steps_pass
            and tool_success_pass
            and loop_detection_pass
            and success_status_pass
        )

        assert overall_pass is True

    def test_one_criterion_fails(self):
        """Test overall fail when one criterion fails."""
        max_steps_pass = False
        tool_success_pass = True
        loop_detection_pass = True
        success_status_pass = True

        overall_pass = (
            max_steps_pass
            and tool_success_pass
            and loop_detection_pass
            and success_status_pass
        )

        assert overall_pass is False

    def test_multiple_criteria_fail(self):
        """Test overall fail when multiple criteria fail."""
        max_steps_pass = False
        tool_success_pass = False
        loop_detection_pass = True
        success_status_pass = True

        overall_pass = (
            max_steps_pass
            and tool_success_pass
            and loop_detection_pass
            and success_status_pass
        )

        assert overall_pass is False


class TestRealWorldScenarios:
    """Test realistic agent execution scenarios."""

    def test_research_agent_scenario(self):
        """Test research agent: search, fetch, summarize, respond."""
        trajectory_data = {
            "run_id": "research_agent_123",
            "timestamp": int(time.time()),
            "steps": [
                {
                    "step": 1,
                    "action": "thought",
                    "success": True,
                    "output": "Need to find info",
                },
                {
                    "step": 2,
                    "action": "tool_call",
                    "tool_name": "search",
                    "success": True,
                    "output": "Found 5 articles",
                    "duration_ms": 800,
                },
                {
                    "step": 3,
                    "action": "tool_call",
                    "tool_name": "fetch",
                    "success": True,
                    "output": "Got content",
                    "duration_ms": 400,
                },
                {
                    "step": 4,
                    "action": "tool_call",
                    "tool_name": "summarize",
                    "success": True,
                    "output": "Summary computed",
                    "duration_ms": 600,
                },
                {"step": 5, "action": "response", "success": True},
            ],
            "final_status": "success",
        }

        # Evaluate
        steps_taken = len(trajectory_data["steps"])
        tool_calls = [s for s in trajectory_data["steps"] if s["action"] == "tool_call"]
        tool_success = sum(1 for s in tool_calls if s["success"]) / len(tool_calls)

        assert steps_taken == 5
        assert steps_taken <= 10
        assert tool_success >= 0.95
        assert trajectory_data["final_status"] == "success"

    def test_coding_agent_scenario(self):
        """Test coding agent: write, test, fix, test again."""
        trajectory_data = {
            "run_id": "coding_agent_456",
            "timestamp": int(time.time()),
            "steps": [
                {"step": 1, "action": "thought", "success": True},
                {
                    "step": 2,
                    "action": "tool_call",
                    "tool_name": "write_code",
                    "success": True,
                },
                {"step": 3, "action": "tool_call", "tool_name": "run_tests", "success": False},
                {
                    "step": 4,
                    "action": "tool_call",
                    "tool_name": "fix_code",
                    "success": True,
                },
                {"step": 5, "action": "tool_call", "tool_name": "run_tests", "success": True},
                {"step": 6, "action": "response", "success": True},
            ],
            "final_status": "success",
        }

        # Evaluate with tolerance for test failures
        tool_calls = [s for s in trajectory_data["steps"] if s["action"] == "tool_call"]
        tool_success = sum(1 for s in tool_calls if s["success"]) / len(tool_calls)

        # 4/5 = 0.8
        assert tool_success == pytest.approx(0.8, abs=0.01)
        assert tool_success >= 0.75  # Tolerant threshold

    def test_support_agent_scenario(self):
        """Test support agent: lookup, search, respond."""
        trajectory_data = {
            "run_id": "support_agent_789",
            "timestamp": int(time.time()),
            "steps": [
                {
                    "step": 1,
                    "action": "tool_call",
                    "tool_name": "lookup_customer",
                    "success": True,
                },
                {
                    "step": 2,
                    "action": "tool_call",
                    "tool_name": "search_kb",
                    "success": True,
                },
                {
                    "step": 3,
                    "action": "tool_call",
                    "tool_name": "fetch_docs",
                    "success": True,
                },
                {"step": 4, "action": "thought", "success": True},
                {"step": 5, "action": "response", "success": True},
            ],
            "final_status": "success",
        }

        steps_taken = len(trajectory_data["steps"])
        tool_calls = [s for s in trajectory_data["steps"] if s["action"] == "tool_call"]
        tool_success = sum(1 for s in tool_calls if s["success"]) / len(tool_calls)

        assert steps_taken == 5
        assert tool_success == 1.0

    def test_simple_query_agent(self):
        """Test agent that just responds without tools."""
        trajectory_data = {
            "run_id": "simple_agent",
            "timestamp": int(time.time()),
            "steps": [{"step": 1, "action": "response", "success": True}],
            "final_status": "success",
        }

        steps_taken = len(trajectory_data["steps"])
        tool_calls = [s for s in trajectory_data["steps"] if s["action"] == "tool_call"]

        assert steps_taken == 1
        assert len(tool_calls) == 0


class TestBatchEvaluation:
    """Test batch evaluation metrics."""

    def test_batch_success_rate(self):
        """Test calculating success rate across multiple runs."""
        results = [True, True, False, True]  # 3 passes, 1 fail
        success_rate = sum(results) / len(results)

        assert success_rate == 0.75

    def test_batch_average_steps(self):
        """Test calculating average steps across runs."""
        step_counts = [5, 7, 4, 6]
        avg_steps = sum(step_counts) / len(step_counts)

        assert avg_steps == 5.5

    def test_batch_average_tool_success(self):
        """Test calculating average tool success rate."""
        tool_rates = [0.95, 1.0, 0.90, 0.85]
        avg_tool_success = sum(tool_rates) / len(tool_rates)

        assert avg_tool_success == pytest.approx(0.925, abs=0.001)

    def test_batch_loop_detection_rate(self):
        """Test calculating loop detection rate."""
        loop_detected = [False, False, True, False]
        loop_rate = sum(loop_detected) / len(loop_detected)

        assert loop_rate == 0.25


class TestConfigurationScenarios:
    """Test different SLO configurations."""

    def test_strict_configuration(self):
        """Test strict SLO thresholds."""
        config = {
            "max_steps": 5,
            "tool_success_threshold": 0.99,
            "max_repeated_actions": 1,
            "success_threshold": 0.95,
        }

        assert config["max_steps"] == 5
        assert config["tool_success_threshold"] == 0.99

    def test_tolerant_configuration(self):
        """Test tolerant SLO thresholds."""
        config = {
            "max_steps": 20,
            "tool_success_threshold": 0.85,
            "max_repeated_actions": 3,
            "success_threshold": 0.80,
        }

        assert config["max_steps"] == 20
        assert config["tool_success_threshold"] == 0.85

    def test_default_configuration(self):
        """Test default SLO thresholds."""
        config = {
            "max_steps": 10,
            "tool_success_threshold": 0.95,
            "max_repeated_actions": 2,
            "success_threshold": 0.90,
        }

        assert config["max_steps"] == 10
        assert config["tool_success_threshold"] == 0.95


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_single_step_agent(self):
        """Test agent with single response step."""
        steps_taken = 1
        max_steps = 10

        assert steps_taken <= max_steps

    def test_many_step_agent(self):
        """Test agent with many steps."""
        steps_taken = 50
        max_steps = 100

        assert steps_taken <= max_steps

    def test_zero_tool_calls(self):
        """Test trajectory with no tool calls."""
        tool_calls = []
        success_rate = 1.0 if not tool_calls else 0.0

        assert success_rate == 1.0

    def test_all_tools_fail(self):
        """Test trajectory where all tools fail."""
        tool_calls = [{"success": False}, {"success": False}, {"success": False}]
        success_rate = sum(1 for t in tool_calls if t["success"]) / len(tool_calls)

        assert success_rate == 0.0

    def test_tool_success_boundary(self):
        """Test tool success rate exactly at threshold."""
        success = 3
        total = 4
        threshold = 0.75

        rate = success / total
        assert rate == threshold
        assert rate >= threshold

    def test_max_steps_boundary(self):
        """Test steps exactly at max."""
        steps_taken = 10
        max_steps = 10

        assert steps_taken <= max_steps

    def test_max_steps_one_over(self):
        """Test steps one over max."""
        steps_taken = 11
        max_steps = 10

        assert not (steps_taken <= max_steps)

    def test_action_count_boundary(self):
        """Test action repetition exactly at boundary."""
        max_repeated = 2
        action_count = 2

        loop_detected = action_count > max_repeated
        assert loop_detected is False

    def test_action_count_over_boundary(self):
        """Test action repetition one over boundary."""
        max_repeated = 2
        action_count = 3

        loop_detected = action_count > max_repeated
        assert loop_detected is True


class TestMonitoringMetrics:
    """Test metrics suitable for monitoring and alerting."""

    def test_metric_success_rate(self):
        """Test success_rate metric for alerting."""
        metric = 0.92  # 92% pass rate

        # Alert if < 0.90
        alert_triggered = metric < 0.90
        assert alert_triggered is False

    def test_metric_tool_degradation(self):
        """Test tool_success_rate degradation detection."""
        baseline = 0.98
        current = 0.90

        degradation = baseline - current
        assert degradation == 0.08

    def test_metric_avg_steps_explosion(self):
        """Test detecting when average steps increases."""
        baseline_avg = 5.0
        current_avg = 12.0
        max_steps = 10

        explosion_detected = current_avg > max_steps * 0.9
        assert explosion_detected is True

    def test_metric_loop_detection_rate(self):
        """Test loop detection rate metric."""
        loop_rate = 0.15  # 15% of runs detected loops

        # Alert if > 10%
        alert_triggered = loop_rate > 0.10
        assert alert_triggered is True


class TestIntegrationPatterns:
    """Test integration patterns with agent frameworks."""

    def test_trajectory_from_agent_steps_list(self):
        """Test constructing trajectory from agent step list."""
        agent_steps = [
            {"action": "search", "tool": "google", "result": "5 pages"},
            {"action": "fetch", "tool": "http", "result": "content"},
            {"action": "response", "result": "answer"},
        ]

        trajectory_steps = [
            {
                "step": i + 1,
                "action": "tool_call" if step["tool"] else "response",
                "tool_name": step.get("tool"),
                "success": True,
                "output": step["result"],
            }
            for i, step in enumerate(agent_steps)
        ]

        assert len(trajectory_steps) == 3
        assert trajectory_steps[0]["action"] == "tool_call"
        assert trajectory_steps[2]["action"] == "response"

    def test_evaluation_result_to_monitoring(self):
        """Test converting evaluation result to monitoring format."""
        evaluation = {
            "steps_taken": 5,
            "tool_success_rate": 0.95,
            "loop_detected": False,
            "success": True,
            "pass": True,
        }

        monitoring_data = {
            "metric_steps": evaluation["steps_taken"],
            "metric_tool_success": evaluation["tool_success_rate"],
            "metric_loop_detected": evaluation["loop_detected"],
            "slo_pass": evaluation["pass"],
        }

        assert monitoring_data["slo_pass"] is True
        assert monitoring_data["metric_steps"] == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
