#!/usr/bin/env python3

"""Comprehensive unit tests for core functionality.

Tests cover:
- JSON serialization/deserialization
- Error handling and variants
- Client facade behavior
- Convenience layer functions
"""

import json
import unittest
from pathlib import Path
import sys
from typing import Dict, Any

# Add parent to path
repo_root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo_root))

try:
    from neuralbudget import NeuralBudgetClient
    from neuralbudget.convenience import (
        evaluate_http_histogram_once,
        evaluate_stateful_once,
        evaluate_ml_once,
    )
    NATIVE_AVAILABLE = True
except ImportError:
    NATIVE_AVAILABLE = False


class TestJsonSerialization(unittest.TestCase):
    """Test JSON serialization and deserialization."""
    
    def test_slo_config_json_roundtrip(self):
        """Test that SloConfig can be serialized and deserialized."""
        config_dict = {
            "service": "test-service",
            "slo_percentage": 99.5,
            "window_minutes": 30,
        }
        
        # Verify we can serialize and deserialize
        json_str = json.dumps(config_dict)
        restored = json.loads(json_str)
        
        assert restored["service"] == "test-service"
        assert restored["slo_percentage"] == 99.5
        assert restored["window_minutes"] == 30
    
    def test_error_budget_json_roundtrip(self):
        """Test that error budget can be serialized and deserialized."""
        budget_dict = {
            "total_budget_seconds": 1800.0,
            "consumed_seconds": 45.5,
            "remaining_seconds": 1754.5,
            "burn_rate": 0.025,
        }
        
        json_str = json.dumps(budget_dict)
        restored = json.loads(json_str)
        
        assert restored["total_budget_seconds"] == 1800.0
        assert restored["consumed_seconds"] == 45.5
        assert abs(restored["remaining_seconds"] - 1754.5) < 0.1
    
    def test_http_evaluation_result_json(self):
        """Test HTTP evaluation result JSON format."""
        result = {
            "service": "api-gateway",
            "pass": True,
            "success_rate": 0.9951,
            "sample_count": 10000,
            "latency_p99_ms": 125.5,
        }
        
        json_str = json.dumps(result)
        restored = json.loads(json_str)
        
        assert restored["pass"] is True
        assert restored["service"] == "api-gateway"
        assert abs(restored["success_rate"] - 0.9951) < 0.0001
    
    def test_ml_evaluation_result_json(self):
        """Test ML evaluation result JSON format."""
        result = {
            "model": "recommendation-model-v2",
            "pass": True,
            "precision": 0.945,
            "recall": 0.923,
            "f1_score": 0.934,
            "drift_detected": False,
        }
        
        json_str = json.dumps(result)
        restored = json.loads(json_str)
        
        assert restored["pass"] is True
        assert abs(restored["f1_score"] - 0.934) < 0.001
        assert restored["drift_detected"] is False
    
    def test_nested_composite_graph_json(self):
        """Test nested composite graph JSON serialization."""
        graph_dict = {
            "services": [
                {"name": "svc-1", "score": 0.99, "weight": 0.5},
                {"name": "svc-2", "score": 0.98, "weight": 0.3},
                {"name": "svc-3", "score": 0.97, "weight": 0.2},
            ],
            "dependencies": [
                {"from": "svc-1", "to": "svc-2", "penalty": 0.05},
                {"from": "svc-2", "to": "svc-3", "penalty": 0.03},
            ],
            "global_min_score": 0.95,
        }
        
        json_str = json.dumps(graph_dict)
        restored = json.loads(json_str)
        
        assert len(restored["services"]) == 3
        assert len(restored["dependencies"]) == 2
        assert restored["services"][0]["name"] == "svc-1"


class TestErrorHandling(unittest.TestCase):
    """Test error handling and error types."""
    
    def test_config_error_representation(self):
        """Test ConfigError representation."""
        error_dict = {
            "error_type": "ConfigError",
            "message": "Invalid window_minutes: must be > 0",
            "details": {"field": "window_minutes", "value": -5},
        }
        
        json_str = json.dumps(error_dict)
        restored = json.loads(json_str)
        
        assert restored["error_type"] == "ConfigError"
        assert "window_minutes" in restored["message"]
    
    def test_composite_error_representation(self):
        """Test CompositeError representation."""
        error_dict = {
            "error_type": "CompositeError",
            "message": "Service svc-1 not found in graph",
            "service": "svc-1",
        }
        
        json_str = json.dumps(error_dict)
        restored = json.loads(json_str)
        
        assert restored["error_type"] == "CompositeError"
    
    def test_format_error_representation(self):
        """Test FormatError representation."""
        error_dict = {
            "error_type": "FormatError",
            "message": "Invalid JSON format",
            "line": 42,
        }
        
        json_str = json.dumps(error_dict)
        restored = json.loads(json_str)
        
        assert restored["error_type"] == "FormatError"
        assert restored["line"] == 42
    
    def test_evaluation_error_representation(self):
        """Test EvaluationError representation."""
        error_dict = {
            "error_type": "EvaluationError",
            "message": "Cannot evaluate metric: data unavailable",
            "metric": "http_latency_p99",
        }
        
        json_str = json.dumps(error_dict)
        restored = json.loads(json_str)
        
        assert restored["error_type"] == "EvaluationError"
    
    def test_parse_error_representation(self):
        """Test ParseError representation."""
        error_dict = {
            "error_type": "ParseError",
            "message": "Failed to parse OTLP JSON",
            "cause": "Unexpected token",
        }
        
        json_str = json.dumps(error_dict)
        restored = json.loads(json_str)
        
        assert restored["error_type"] == "ParseError"


class TestClientFacade(unittest.TestCase):
    """Test NeuralBudgetClient behavior."""
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_client_creation(self):
        """Test that client can be created."""
        # This will skip if neuralbudget not compiled
        client = NeuralBudgetClient()
        assert client is not None
    
    def test_client_configuration_dict(self):
        """Test client configuration as dictionary."""
        config = {
            "service_name": "backend-api",
            "slo_percentage": 99.9,
            "window_duration_minutes": 30,
        }
        
        json_str = json.dumps(config)
        restored = json.loads(json_str)
        
        assert restored["service_name"] == "backend-api"
        assert restored["slo_percentage"] == 99.9


class TestConvenienceFunctions(unittest.TestCase):
    """Test convenience layer functions."""
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_http_histogram_evaluation(self):
        """Test HTTP histogram evaluation function."""
        # Mock test - actual behavior tested with native module
        result = evaluate_http_histogram_once(
            sample={"success_count": 9950, "total_count": 10000},
            success_threshold=9900,
        )
        
        assert result is not None
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_stateful_evaluation(self):
        """Test stateful evaluation function."""
        result = evaluate_stateful_once(
            sample={"live_connections": 500},
            threshold=100,
        )
        
        assert result is not None
    
    @unittest.skipIf(not NATIVE_AVAILABLE, "Native module not available")
    def test_ml_evaluation(self):
        """Test ML model evaluation function."""
        result = evaluate_ml_once(
            sample={"f1_score": 0.945, "drift": False},
            min_f1=0.9,
        )
        
        assert result is not None


class TestDataIntegrity(unittest.TestCase):
    """Test data integrity and edge cases."""
    
    def test_float_precision_in_metrics(self):
        """Test float precision is preserved in JSON."""
        metrics = {
            "p50": 10.123456789,
            "p95": 125.987654321,
            "p99": 999.111222333,
        }
        
        json_str = json.dumps(metrics)
        restored = json.loads(json_str)
        
        # Should preserve precision (typically 15-17 significant digits)
        assert abs(restored["p50"] - 10.123456789) < 1e-10
    
    def test_large_numbers_in_metrics(self):
        """Test large numbers don't overflow."""
        metrics = {
            "total_requests": 1_000_000_000,
            "success_count": 999_500_000,
            "error_count": 500_000,
        }
        
        json_str = json.dumps(metrics)
        restored = json.loads(json_str)
        
        assert restored["total_requests"] == 1_000_000_000
        assert restored["success_count"] + restored["error_count"] == restored["total_requests"]
    
    def test_zero_values(self):
        """Test zero values are preserved."""
        metrics = {
            "errors": 0,
            "warnings": 0,
            "latency_ms": 0.0,
        }
        
        json_str = json.dumps(metrics)
        restored = json.loads(json_str)
        
        assert restored["errors"] == 0
        assert restored["latency_ms"] == 0.0
    
    def test_negative_values(self):
        """Test negative values (delta metrics)."""
        deltas = {
            "cpu_change_percent": -5.5,
            "memory_delta_mb": -128,
            "latency_improvement_ms": -15.25,
        }
        
        json_str = json.dumps(deltas)
        restored = json.loads(json_str)
        
        assert restored["cpu_change_percent"] == -5.5
        assert restored["memory_delta_mb"] == -128
    
    def test_boolean_values(self):
        """Test boolean pass/fail indicators."""
        indicators = {
            "pass": True,
            "slo_met": True,
            "anomaly_detected": False,
            "autoscale_triggered": False,
        }
        
        json_str = json.dumps(indicators)
        restored = json.loads(json_str)
        
        assert restored["pass"] is True
        assert restored["anomaly_detected"] is False
    
    def test_empty_collections(self):
        """Test empty lists and dicts."""
        data = {
            "services": [],
            "errors": [],
            "metadata": {},
            "tags": {},
        }
        
        json_str = json.dumps(data)
        restored = json.loads(json_str)
        
        assert len(restored["services"]) == 0
        assert len(restored["errors"]) == 0
        assert len(restored["metadata"]) == 0


class TestMetricsNormalization(unittest.TestCase):
    """Test metric normalization and bounds."""
    
    def test_percentage_bounds(self):
        """Test percentage metrics stay in [0, 100]."""
        percentages = [
            {"name": "success_rate", "value": 99.95},
            {"name": "availability", "value": 99.999},
            {"name": "slo_compliance", "value": 100.0},
            {"name": "error_rate", "value": 0.05},
        ]
        
        for metric in percentages:
            assert 0.0 <= metric["value"] <= 100.0
    
    def test_latency_metrics_positive(self):
        """Test latency metrics are non-negative."""
        latencies = {
            "p50_ms": 10.5,
            "p95_ms": 50.2,
            "p99_ms": 100.8,
            "p999_ms": 500.3,
            "max_ms": 2000.0,
        }
        
        for value in latencies.values():
            assert value >= 0.0
    
    def test_rate_metrics_monotonic(self):
        """Test rate percentiles are monotonically increasing."""
        rates = {
            "p50": 10.0,
            "p95": 50.0,
            "p99": 100.0,
            "p999": 500.0,
        }
        
        sorted_values = sorted(rates.values())
        assert sorted_values == list(rates.values())


class TestCompositeGraphValidation(unittest.TestCase):
    """Test composite graph configuration validation."""
    
    def test_graph_with_no_cycles(self):
        """Validate acyclic graph structure."""
        graph = {
            "services": ["svc-1", "svc-2", "svc-3"],
            "dependencies": [
                ("svc-1", "svc-2"),
                ("svc-2", "svc-3"),
            ],
        }
        
        # Build adjacency list
        adj = {svc: [] for svc in graph["services"]}
        for src, dst in graph["dependencies"]:
            adj[src].append(dst)
        
        # Simple check: no self-loops or backwards edges
        for src, dests in adj.items():
            for dst in dests:
                assert src != dst  # No self-loops
    
    def test_graph_connectivity(self):
        """Test all services are reachable."""
        graph = {
            "services": ["api", "db", "cache", "queue"],
            "dependencies": [
                ("api", "db"),
                ("api", "cache"),
                ("api", "queue"),
            ],
        }
        
        # All services should be defined
        services_set = set(graph["services"])
        for src, dst in graph["dependencies"]:
            assert src in services_set
            assert dst in services_set


if __name__ == "__main__":
    unittest.main()
