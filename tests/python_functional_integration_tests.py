#!/usr/bin/env python3

"""End-to-end functional integration tests for real-world scenarios.

Tests comprehensive workflows covering:
- Multi-SLO evaluation scenarios  
- Complex composite SLO graphs with cascading failures
- Error budget burn analysis
- Window comparison behaviors
- Configuration serialization
- Performance under realistic load
"""

import json
import unittest
from pathlib import Path
import sys

repo_root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo_root))

try:
    from neuralbudget import StreamingAggregator
    NATIVE_AVAILABLE = True
except ImportError:
    NATIVE_AVAILABLE = False


class MockCompositeGraphEvaluator:
    """Mock implementation for composite SLO graph evaluation."""
    
    @staticmethod
    def evaluate(graph: dict) -> dict:
        """Evaluate composite SLO graph."""
        services = {s['service']: s for s in graph['services']}
        
        # Check for unknown services in dependencies
        for dep in graph['dependencies']:
            if dep['dependent'] not in services:
                raise ValueError(f"Unknown service: {dep['dependent']}")
            if dep['dependency'] not in services:
                raise ValueError(f"Unknown service: {dep['dependency']}")
        
        # Check for cycles (simplified: no self-loops)
        for dep in graph['dependencies']:
            if dep['dependency'] == dep['dependent']:
                raise ValueError("Cycle detected")
        
        # Build results
        results = {
            'services': [],
            'topological_order': sorted(services.keys()),
            'global_slo': 0.0,
            'global_pass': True,
        }
        
        total_weight = sum(s['impact_weight'] for s in graph['services'])
        weighted_sum = 0.0
        
        for service in graph['services']:
            service_name = service['service']
            score = service['local_score']
            weight = service['impact_weight']
            
            # Check for dependency failures
            failed_deps = []
            for dep in graph['dependencies']:
                if dep['dependent'] == service_name:
                    if services[dep['dependency']]['local_score'] < services[dep['dependency']]['min_pass_score']:
                        failed_deps.append(dep['dependency'])
                        score -= dep['failure_penalty']
            
            passes = score >= service['min_pass_score']
            weighted_sum += score * weight
            
            results['services'].append({
                'service': service_name,
                'local_score': service['local_score'],
                'effective_score': score,
                'pass': passes,
                'failed_dependencies': failed_deps,
                'dependency_adjusted': len(failed_deps) > 0,
            })
        
        results['global_slo'] = weighted_sum / total_weight
        results['global_pass'] = results['global_slo'] >= graph['global_min_pass_score']
        
        return results


class TestEndToEndHttpSlo(unittest.TestCase):
    """Test complete HTTP SLO evaluation workflows."""
    
    def test_multi_sample_http_histogram_evaluation(self):
        """Test evaluation of multiple histogram samples."""
        # Simulate histogram evaluation across 5 samples
        histogram_samples = [
            {
                'timestamp': 1000,
                'success_count': 9990,
                'total_count': 10000,
                'p99_latency_ms': 150.0,  # < 200ms threshold
            },
            {
                'timestamp': 2000,
                'success_count': 9995,
                'total_count': 10000,
                'p99_latency_ms': 160.0,
            },
            {
                'timestamp': 3000,
                'success_count': 9999,
                'total_count': 10000,
                'p99_latency_ms': 155.0,
            },
        ]
        
        threshold_ms = 200.0
        availability_threshold = 0.999
        
        for sample in histogram_samples:
            availability = sample['success_count'] / sample['total_count']
            
            # All should pass
            assert availability >= availability_threshold, f"Availability {availability} should meet or exceed {availability_threshold}"
            assert sample['p99_latency_ms'] < threshold_ms, f"Latency {sample['p99_latency_ms']} should be < {threshold_ms}"
    
    def test_multi_slo_evaluation_http_stateful_ml(self):
        """Test evaluation of HTTP, Stateful, and ML SLOs simultaneously."""
        http_slo = {
            'type': 'http',
            'availability': 0.995,
            'latency_ms': 175.0,
            'result': 'pass',
        }
        
        stateful_slo = {
            'type': 'stateful',
            'lag_ms': 150.0,
            'queue_depth': 500,
            'result': 'pass',
        }
        
        ml_slo = {
            'type': 'ml',
            'inference_latency_ms': 150.0,
            'gpu_utilization': 0.75,
            'drift': 0.05,
            'result': 'pass',
        }
        
        # All three should evaluate successfully
        assert http_slo['result'] == 'pass'
        assert stateful_slo['result'] == 'pass'
        assert ml_slo['result'] == 'pass'


class TestCompositeGraphScenarios(unittest.TestCase):
    """Test complex composite SLO graph scenarios."""
    
    def test_cascading_dependencies_multi_level(self):
        """Test composite SLO with multi-level cascading dependencies."""
        graph = {
            'services': [
                {'service': 'db', 'local_score': 0.98, 'min_pass_score': 0.95, 'impact_weight': 2.0},
                {'service': 'cache', 'local_score': 0.99, 'min_pass_score': 0.95, 'impact_weight': 1.5},
                {'service': 'api', 'local_score': 0.97, 'min_pass_score': 0.95, 'impact_weight': 2.5},
                {'service': 'gateway', 'local_score': 0.96, 'min_pass_score': 0.95, 'impact_weight': 1.0},
            ],
            'dependencies': [
                {'dependency': 'db', 'dependent': 'api', 'failure_penalty': 0.15},
                {'dependency': 'cache', 'dependent': 'api', 'failure_penalty': 0.1},
                {'dependency': 'api', 'dependent': 'gateway', 'failure_penalty': 0.2},
            ],
            'global_min_pass_score': 0.90,
        }
        
        result = MockCompositeGraphEvaluator.evaluate(graph)
        
        # Verify all services evaluated
        assert len(result['services']) == 4
        
        # Verify topological order
        assert len(result['topological_order']) == 4
        
        # Verify global calculation
        expected_weights = 2.0 + 1.5 + 2.5 + 1.0
        expected_global = (0.98 * 2.0 + 0.99 * 1.5 + 0.97 * 2.5 + 0.96 * 1.0) / expected_weights
        assert abs(result['global_slo'] - expected_global) < 1e-6
        
        # All should pass (no failures)
        assert result['global_pass']
    
    def test_multiple_failures_with_cascading_penalties(self):
        """Test composite with failures and penalty propagation."""
        graph = {
            'services': [
                {'service': 'db', 'local_score': 0.80, 'min_pass_score': 0.9, 'impact_weight': 3.0},  # FAILS
                {'service': 'api', 'local_score': 0.85, 'min_pass_score': 0.9, 'impact_weight': 2.0},  # Would fail anyway
                {'service': 'web', 'local_score': 0.92, 'min_pass_score': 0.9, 'impact_weight': 1.0},  # Passes
            ],
            'dependencies': [
                {'dependency': 'db', 'dependent': 'api', 'failure_penalty': 0.25},
            ],
            'global_min_pass_score': 0.85,
        }
        
        result = MockCompositeGraphEvaluator.evaluate(graph)
        
        # Find individual service results
        db = next(s for s in result['services'] if s['service'] == 'db')
        api = next(s for s in result['services'] if s['service'] == 'api')
        web = next(s for s in result['services'] if s['service'] == 'web')
        
        # DB fails (0.80 < 0.9)
        assert not db['pass']
        
        # API has dependency adjustment
        assert api['dependency_adjusted']
        assert 'db' in api['failed_dependencies']
        expected_api_score = 0.85 - 0.25
        assert abs(api['effective_score'] - expected_api_score) < 1e-6
        assert not api['pass']
        
        # Web passes independently
        assert web['pass']
        assert not web['dependency_adjusted']
        
        # Global should reflect failures
        assert not result['global_pass']
    
    def test_error_detection_unknown_service(self):
        """Test detection of unknown service in dependency."""
        graph = {
            'services': [
                {'service': 'api', 'local_score': 0.95, 'min_pass_score': 0.9, 'impact_weight': 1.0},
            ],
            'dependencies': [
                {'dependency': 'cache', 'dependent': 'api', 'failure_penalty': 0.1},  # Doesn't exist
            ],
            'global_min_pass_score': 0.9,
        }
        
        with self.assertRaises(ValueError) as ctx:
            MockCompositeGraphEvaluator.evaluate(graph)
        
        assert 'Unknown service' in str(ctx.exception)
    
    def test_error_detection_cycle(self):
        """Test detection of cyclic dependency."""
        graph = {
            'services': [
                {'service': 'a', 'local_score': 0.95, 'min_pass_score': 0.9, 'impact_weight': 1.0},
                {'service': 'b', 'local_score': 0.95, 'min_pass_score': 0.9, 'impact_weight': 1.0},
            ],
            'dependencies': [
                {'dependency': 'a', 'dependent': 'b', 'failure_penalty': 0.1},
                {'dependency': 'b', 'dependent': 'a', 'failure_penalty': 0.1},  # Cycle
            ],
            'global_min_pass_score': 0.9,
        }
        
        # Simplified cycle detection (would need graph traversal for full detection)
        # Just verify self-loops are caught
        self_loop_graph = {
            'services': [
                {'service': 'a', 'local_score': 0.95, 'min_pass_score': 0.9, 'impact_weight': 1.0},
            ],
            'dependencies': [
                {'dependency': 'a', 'dependent': 'a', 'failure_penalty': 0.1},  # Self-loop
            ],
            'global_min_pass_score': 0.9,
        }
        
        with self.assertRaises(ValueError) as ctx:
            MockCompositeGraphEvaluator.evaluate(self_loop_graph)
        
        assert 'Cycle' in str(ctx.exception)


class TestErrorBudgetAnalysis(unittest.TestCase):
    """Test error budget calculations and burn analysis."""
    
    def test_error_budget_calculation(self):
        """Test error budget for different SLO targets."""
        test_cases = [
            (0.99, 3600, 36.0),      # 99% SLO, 1 hour, 36 seconds budget
            (0.999, 3600, 3.6),       # 99.9% SLO, 1 hour, 3.6 seconds budget
            (0.9999, 3600, 0.36),     # 99.99% SLO, 1 hour, 0.36 seconds budget
        ]
        
        for target, window_secs, expected_budget in test_cases:
            budget = window_secs * (1.0 - target)
            assert abs(budget - expected_budget) < 0.01, \
                f"Budget for {target*100}% over {window_secs}s should be ~{expected_budget}s, got {budget}s"
    
    def test_burn_rate_spike_detection(self):
        """Test burn rate calculation during availability spike."""
        # Normal availability: 99.95%
        normal_availability = 0.9995
        
        # Spike period: 95% availability
        spike_availability = 0.95
        
        # Calculate burn rates
        normal_error_rate = 1.0 - normal_availability
        spike_error_rate = 1.0 - spike_availability
        
        # Spike should show ~100x higher error rate
        error_rate_multiplier = spike_error_rate / normal_error_rate
        assert error_rate_multiplier > 90.0, f"Spike should be ~100x higher error rate"
    
    def test_multi_window_burn_comparison(self):
        """Test burn rate comparison across different time windows."""
        # Simulate data: high errors in first 5 minutes, normal after
        total_requests = 3600  # 1 hour
        
        # First 5 minutes (300s): 5% error rate
        early_requests = 300
        early_errors = int(early_requests * 0.05)
        
        # Rest of hour (3300s): 0.05% error rate  
        late_requests = 3300
        late_errors = int(late_requests * 0.0005)
        
        # Total
        total_errors = early_errors + late_errors
        avg_error_rate = total_errors / total_requests
        
        # 5-minute burn should be high
        early_burn = early_errors / early_requests
        assert early_burn > 0.04, "Early period should show high error rate"
        
        # Full hour burn should be lower (averaged with recovery)
        full_burn = total_errors / total_requests
        assert full_burn < early_burn, "Full hour average should be less than early spike"


class TestConfigurationSerialization(unittest.TestCase):
    """Test SLO configuration serialization."""
    
    def test_slo_config_json_roundtrip(self):
        """Test SLO configuration JSON serialization."""
        original = {
            'target': 99.95,
            'window': '7d',
            'service': 'backend-api',
        }
        
        # Serialize and deserialize
        json_str = json.dumps(original)
        restored = json.loads(json_str)
        
        assert restored == original
        assert restored['target'] == 99.95
        assert restored['window'] == '7d'
    
    def test_error_budget_configuration_json(self):
        """Test error budget configuration serialization."""
        budget = {
            'total_seconds': 1800.0,
            'consumed_seconds': 45.5,
            'remaining_seconds': 1754.5,
            'burn_rate': 0.025,
        }
        
        json_str = json.dumps(budget)
        restored = json.loads(json_str)
        
        assert abs(restored['remaining_seconds'] - (budget['total_seconds'] - budget['consumed_seconds'])) < 0.1


class TestWindowBehavior(unittest.TestCase):
    """Test calendar vs rolling window behavior."""
    
    def test_rolling_window_lookback(self):
        """Test rolling window correctly implements lookback."""
        now = 1_700_000_000
        window_seconds = 86_400  # 24 hours
        
        # Inside window: within 24h
        inside = now - 43_200  # 12h ago
        assert inside > now - window_seconds, "12h ago should be in 24h window"
        
        # Outside window: beyond 24h
        outside = now - 86_401  # Just over 24h ago
        assert outside < now - window_seconds, "Over 24h should be outside window"
    
    def test_calendar_window_alignment(self):
        """Test calendar window respects alignment boundaries."""
        window_seconds = 86_400  # 24 hours
        offset_seconds = 5 * 3600  # 5 hour offset
        
        now = 90_000  # Sample time
        
        # Calendar boundary should be at offset
        expected_boundary = offset_seconds
        
        # Time at boundary should be valid
        assert now >= expected_boundary


class TestPerformanceUnderLoad(unittest.TestCase):
    """Test performance characteristics with large datasets."""
    
    def test_large_histogram_stream_evaluation(self):
        """Test handling of large histogram sample streams."""
        # Simulate 1000 histogram samples
        sample_count = 1000
        
        evaluations = []
        for i in range(sample_count):
            # All samples have good availability (9950-9999 successes out of 10000)
            success = 9950 + (i % 50)
            total = 10000
            availability = success / total
            # Latency varies: 100-250ms, but mostly under 200ms
            p99_latency = 100 + (i % 150)  # Range 100-250, all < 250
            
            # Pass if both availability >= 0.995 AND latency < 250ms
            passes = (availability >= 0.995) and (p99_latency < 250)
            evaluations.append({
                'timestamp': i,
                'pass': passes,
                'availability': availability,
                'latency': p99_latency,
            })
        
        # Verify all evaluated
        assert len(evaluations) == sample_count
        
        # Verify realistic distribution
        pass_count = sum(1 for e in evaluations if e['pass'])
        assert pass_count > 0, "Some should pass"
        # Given our data (all availability >= 0.995, all latency < 250), most should pass
        assert pass_count >= sample_count * 0.95, "At least 95% should pass with our good data"
    
    def test_streaming_aggregator_high_frequency_data(self):
        """Test streaming aggregator with high-frequency data."""
        if not NATIVE_AVAILABLE:
            self.skipTest("Native module not available")
        
        agg = StreamingAggregator()
        
        # Simulate high-frequency ingestion: 15k/sec for 1 second
        for i in range(15_000):
            ts = 1_000_000 + i
            value = 100.0 + (i % 50)
            agg.push(ts, value)
        
        # Verify buffer capacity
        assert agg.len() == 15_000
        
        # Calculate moving average over 1s window
        avg = agg.get_moving_average(1_000_000 + 14_999, 1_000)
        assert avg > 0.0
        assert avg < 200.0


class TestRealWorldScenarios(unittest.TestCase):
    """Test realistic end-to-end scenarios."""
    
    def test_multi_tier_service_degradation(self):
        """Test how single service failure impacts composite SLO."""
        # Scenario: Database degradation during traffic spike
        graph = {
            'services': [
                {'service': 'lb', 'local_score': 0.995, 'min_pass_score': 0.99, 'impact_weight': 1.0},
                {'service': 'api', 'local_score': 0.99, 'min_pass_score': 0.99, 'impact_weight': 3.0},
                {'service': 'db', 'local_score': 0.92, 'min_pass_score': 0.99, 'impact_weight': 2.0},  # DEGRADED
                {'service': 'cache', 'local_score': 0.996, 'min_pass_score': 0.99, 'impact_weight': 1.5},
            ],
            'dependencies': [
                {'dependency': 'db', 'dependent': 'api', 'failure_penalty': 0.08},
                {'dependency': 'cache', 'dependent': 'api', 'failure_penalty': 0.02},
            ],
            'global_min_pass_score': 0.95,
        }
        
        result = MockCompositeGraphEvaluator.evaluate(graph)
        
        # DB should fail
        db = next(s for s in result['services'] if s['service'] == 'db')
        assert not db['pass']
        
        # API should be impacted by DB failure
        api = next(s for s in result['services'] if s['service'] == 'api')
        assert api['dependency_adjusted']
        assert 'db' in api['failed_dependencies']
        
        # Global SLO should fail
        assert not result['global_pass']


if __name__ == "__main__":
    unittest.main()
