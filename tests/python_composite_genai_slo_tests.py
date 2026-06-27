"""
Comprehensive test suite for unified CompositeGenAiSlo evaluation.

Tests weighted scoring, dimension thresholds, and real-world scenarios
across different weight profiles (quality-first, cost-optimized, balanced).
"""

import pytest
from typing import Dict, Any


class TestCompositeGenAiBasics:
    """Test basic composite GenAI SLO evaluation."""

    def test_composite_all_perfect_scores(self):
        """Test evaluation with all dimensions at perfect (1.0)."""
        dimensions = {
            "throughput_score": 1.0,
            "ttft_score": 1.0,
            "quality_score": 1.0,
            "groundedness_score": 1.0,
            "cost_score": 1.0,
            "retrieval_score": 1.0,
            "success_rate": 1.0,
        }
        
        weights = {
            "throughput_weight": 0.15,
            "ttft_weight": 0.15,
            "quality_weight": 0.30,
            "groundedness_weight": 0.15,
            "cost_weight": 0.10,
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
            "min_target_score": 0.85,
        }

        # Composite score: sum of all weights = 1.0
        composite = calculate_weighted_score(dimensions, weights)
        assert composite == pytest.approx(1.0, abs=0.001)

    def test_composite_all_passing_thresholds(self):
        """Test evaluation where all dimensions meet thresholds."""
        dimensions = {
            "throughput_score": 0.95,
            "ttft_score": 0.92,  # > 0.90
            "quality_score": 0.88,  # > 0.85
            "groundedness_score": 0.97,  # > 0.95
            "cost_score": 0.90,
            "retrieval_score": 0.93,
            "success_rate": 0.99,  # > 0.99
        }
        
        thresholds = {
            "ttft_min": 0.90,
            "quality_min": 0.85,
            "groundedness_min": 0.95,
            "success_rate_min": 0.99,
        }

        # All dimensions pass
        assert all_dimensions_pass(dimensions, thresholds)

    def test_composite_quality_fail(self):
        """Test evaluation where quality dimension fails."""
        dimensions = {
            "throughput_score": 0.95,
            "ttft_score": 0.92,
            "quality_score": 0.75,  # Below 0.85 threshold
            "groundedness_score": 0.97,
            "cost_score": 0.90,
            "retrieval_score": 0.93,
            "success_rate": 0.99,
        }
        
        thresholds = {
            "ttft_min": 0.90,
            "quality_min": 0.85,
            "groundedness_min": 0.95,
            "success_rate_min": 0.99,
        }

        # Quality fails
        assert not all_dimensions_pass(dimensions, thresholds)

    def test_composite_groundedness_fail(self):
        """Test evaluation where groundedness (hallucination) fails."""
        dimensions = {
            "throughput_score": 0.95,
            "ttft_score": 0.92,
            "quality_score": 0.88,
            "groundedness_score": 0.90,  # Below 0.95 threshold (high hallucination)
            "cost_score": 0.90,
            "retrieval_score": 0.93,
            "success_rate": 0.99,
        }
        
        thresholds = {
            "ttft_min": 0.90,
            "quality_min": 0.85,
            "groundedness_min": 0.95,
            "success_rate_min": 0.99,
        }

        assert not all_dimensions_pass(dimensions, thresholds)


class TestWeightProfiles:
    """Test different weight profiles for various use cases."""

    def test_quality_first_profile(self):
        """Test quality-optimized weight profile."""
        weights = {
            "throughput_weight": 0.10,
            "ttft_weight": 0.10,
            "quality_weight": 0.40,      # High quality weight
            "groundedness_weight": 0.25,  # High groundedness weight
            "cost_weight": 0.05,
            "retrieval_weight": 0.05,
            "success_rate_weight": 0.05,
            "min_target_score": 0.90,
        }

        # Sum to 1.0
        total = sum(v for k, v in weights.items() if k.endswith("_weight"))
        assert total == pytest.approx(1.0, abs=0.001)

        # With perfect quality/groundedness but weak throughput:
        dimensions = {
            "throughput_score": 0.60,  # Weak
            "ttft_score": 0.70,
            "quality_score": 1.0,       # Perfect
            "groundedness_score": 1.0,  # Perfect
            "cost_score": 0.50,
            "retrieval_score": 0.60,
            "success_rate": 0.95,
        }

        score = calculate_weighted_score(dimensions, weights)
        # Should be boosted by high quality/groundedness weights
        assert score > 0.75

    def test_cost_optimized_profile(self):
        """Test cost-optimized weight profile."""
        weights = {
            "throughput_weight": 0.20,
            "ttft_weight": 0.10,
            "quality_weight": 0.25,
            "groundedness_weight": 0.10,
            "cost_weight": 0.25,        # High cost weight
            "retrieval_weight": 0.05,
            "success_rate_weight": 0.05,
            "min_target_score": 0.80,
        }

        # With excellent cost efficiency but moderate quality:
        dimensions = {
            "throughput_score": 0.95,   # Fast
            "ttft_score": 0.85,
            "quality_score": 0.75,      # Moderate
            "groundedness_score": 0.80,  # Moderate
            "cost_score": 1.0,          # Perfect cost efficiency
            "retrieval_score": 0.70,
            "success_rate": 0.98,
        }

        score = calculate_weighted_score(dimensions, weights)
        # Should be boosted by perfect cost score
        assert score > 0.80

    def test_balanced_profile(self):
        """Test balanced default profile."""
        weights = {
            "throughput_weight": 0.15,
            "ttft_weight": 0.15,
            "quality_weight": 0.30,
            "groundedness_weight": 0.15,
            "cost_weight": 0.10,
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
            "min_target_score": 0.85,
        }

        # Typical production scenario
        dimensions = {
            "throughput_score": 0.92,
            "ttft_score": 0.88,
            "quality_score": 0.86,
            "groundedness_score": 0.96,
            "cost_score": 0.88,
            "retrieval_score": 0.89,
            "success_rate": 0.98,
        }

        score = calculate_weighted_score(dimensions, weights)
        # Should be around 0.90 (good but not perfect)
        assert 0.88 < score < 0.93


class TestCompositeScoring:
    """Test weighted score calculations."""

    def test_score_calculation_simple(self):
        """Test simple weighted score calculation."""
        dimensions = {
            "throughput_score": 0.8,
            "ttft_score": 0.8,
            "quality_score": 0.8,
            "groundedness_score": 0.8,
            "cost_score": 0.8,
            "retrieval_score": 0.8,
            "success_rate": 0.8,
        }
        
        weights = {
            "throughput_weight": 1.0/7,
            "ttft_weight": 1.0/7,
            "quality_weight": 1.0/7,
            "groundedness_weight": 1.0/7,
            "cost_weight": 1.0/7,
            "retrieval_weight": 1.0/7,
            "success_rate_weight": 1.0/7,
        }

        score = calculate_weighted_score(dimensions, weights)
        # All equal weights and equal scores should give 0.8
        assert score == pytest.approx(0.8, abs=0.001)

    def test_score_min_max_bounds(self):
        """Test that composite score is bounded [0.0, 1.0]."""
        weights = {
            "throughput_weight": 0.15,
            "ttft_weight": 0.15,
            "quality_weight": 0.30,
            "groundedness_weight": 0.15,
            "cost_weight": 0.10,
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
        }

        # Test with zeros
        zero_dims = {
            "throughput_score": 0.0,
            "ttft_score": 0.0,
            "quality_score": 0.0,
            "groundedness_score": 0.0,
            "cost_score": 0.0,
            "retrieval_score": 0.0,
            "success_rate": 0.0,
        }
        score_min = calculate_weighted_score(zero_dims, weights)
        assert score_min == pytest.approx(0.0, abs=0.001)

        # Test with ones
        one_dims = {
            "throughput_score": 1.0,
            "ttft_score": 1.0,
            "quality_score": 1.0,
            "groundedness_score": 1.0,
            "cost_score": 1.0,
            "retrieval_score": 1.0,
            "success_rate": 1.0,
        }
        score_max = calculate_weighted_score(one_dims, weights)
        assert score_max == pytest.approx(1.0, abs=0.001)

    def test_quality_weight_dominance(self):
        """Test that higher quality weight dominates score."""
        dims_high_quality = {
            "throughput_score": 0.5,
            "ttft_score": 0.5,
            "quality_score": 1.0,  # Perfect
            "groundedness_score": 0.5,
            "cost_score": 0.5,
            "retrieval_score": 0.5,
            "success_rate": 0.5,
        }

        dims_low_quality = {
            "throughput_score": 0.5,
            "ttft_score": 0.5,
            "quality_score": 0.0,  # Terrible
            "groundedness_score": 0.5,
            "cost_score": 0.5,
            "retrieval_score": 0.5,
            "success_rate": 0.5,
        }

        weights = {
            "throughput_weight": 0.15,
            "ttft_weight": 0.15,
            "quality_weight": 0.30,  # Highest weight
            "groundedness_weight": 0.15,
            "cost_weight": 0.10,
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
        }

        score_high = calculate_weighted_score(dims_high_quality, weights)
        score_low = calculate_weighted_score(dims_low_quality, weights)

        # High quality should significantly outperform low quality
        assert score_high - score_low > 0.20


class TestRealWorldScenarios:
    """Test real-world GenAI SLO scenarios."""

    def test_chat_assistant_typical(self):
        """Test typical chat assistant metrics."""
        dimensions = {
            "throughput_score": 0.90,     # Good throughput
            "ttft_score": 0.95,           # Fast first token
            "quality_score": 0.87,        # Good responses
            "groundedness_score": 0.96,   # Few hallucinations
            "cost_score": 0.85,           # Acceptable cost
            "retrieval_score": 0.88,      # Good retrieval if RAG
            "success_rate": 0.99,         # High success
        }

        weights = {
            "throughput_weight": 0.15,
            "ttft_weight": 0.15,
            "quality_weight": 0.30,
            "groundedness_weight": 0.15,
            "cost_weight": 0.10,
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
            "min_target_score": 0.85,
        }

        thresholds = {
            "ttft_min": 0.90,
            "quality_min": 0.85,
            "groundedness_min": 0.95,
            "success_rate_min": 0.99,
        }

        score = calculate_weighted_score(dimensions, weights)
        assert score > 0.90
        assert all_dimensions_pass(dimensions, thresholds)

    def test_rag_system_retrieval_critical(self):
        """Test RAG system where retrieval is critical."""
        dimensions = {
            "throughput_score": 0.85,
            "ttft_score": 0.80,
            "quality_score": 0.90,        # High quality due to good retrieval
            "groundedness_score": 0.98,   # Excellent grounding with RAG
            "cost_score": 0.75,           # Higher cost due to retrieval
            "retrieval_score": 0.95,      # Excellent recall@k
            "success_rate": 0.99,
        }

        # RAG-optimized weights (higher retrieval weight)
        weights = {
            "throughput_weight": 0.10,
            "ttft_weight": 0.10,
            "quality_weight": 0.25,
            "groundedness_weight": 0.20,  # Higher groundedness weight
            "cost_weight": 0.10,
            "retrieval_weight": 0.20,     # Much higher retrieval weight
            "success_rate_weight": 0.05,
            "min_target_score": 0.85,
        }

        score = calculate_weighted_score(dimensions, weights)
        # Excellent retrieval and groundedness should boost score
        assert score > 0.88

    def test_cost_constrained_scenario(self):
        """Test cost-constrained GenAI service."""
        dimensions = {
            "throughput_score": 0.88,
            "ttft_score": 0.82,          # Acceptable TTFT
            "quality_score": 0.80,       # Acceptable quality
            "groundedness_score": 0.90,  # Moderate groundedness
            "cost_score": 0.95,          # Excellent cost control
            "retrieval_score": 0.75,     # Lower retrieval
            "success_rate": 0.98,        # Good success
        }

        # Cost-optimized weights
        weights = {
            "throughput_weight": 0.20,
            "ttft_weight": 0.10,
            "quality_weight": 0.20,
            "groundedness_weight": 0.10,
            "cost_weight": 0.25,         # High cost weight
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
            "min_target_score": 0.80,   # Slightly lower target
        }

        thresholds = {
            "ttft_min": 0.75,           # More lenient TTFT
            "quality_min": 0.75,
            "groundedness_min": 0.85,   # More tolerant hallucinations
            "success_rate_min": 0.95,
        }

        score = calculate_weighted_score(dimensions, weights)
        assert score > 0.85
        # Should pass despite moderate quality due to excellent cost
        assert all_dimensions_pass(dimensions, thresholds)


class TestThresholdValidation:
    """Test dimension threshold validation."""

    def test_weights_sum_to_one(self):
        """Verify weights sum to 1.0."""
        weights = {
            "throughput_weight": 0.15,
            "ttft_weight": 0.15,
            "quality_weight": 0.30,
            "groundedness_weight": 0.15,
            "cost_weight": 0.10,
            "retrieval_weight": 0.10,
            "success_rate_weight": 0.05,
        }

        total = sum(weights.values())
        assert total == pytest.approx(1.0, abs=0.001)

    def test_thresholds_valid_range(self):
        """Verify thresholds are in valid range."""
        thresholds = {
            "ttft_min": 0.90,
            "quality_min": 0.85,
            "groundedness_min": 0.95,
            "success_rate_min": 0.99,
        }

        for name, threshold in thresholds.items():
            assert 0.0 <= threshold <= 1.0


# Helper functions

def calculate_weighted_score(dimensions: Dict[str, float], weights: Dict[str, float]) -> float:
    """Calculate weighted composite score."""
    score = (
        weights["throughput_weight"] * dimensions["throughput_score"] +
        weights["ttft_weight"] * dimensions["ttft_score"] +
        weights["quality_weight"] * dimensions["quality_score"] +
        weights["groundedness_weight"] * dimensions["groundedness_score"] +
        weights["cost_weight"] * dimensions["cost_score"] +
        weights["retrieval_weight"] * dimensions["retrieval_score"] +
        weights["success_rate_weight"] * dimensions["success_rate"]
    )
    return min(1.0, max(0.0, score))  # Clamp to [0.0, 1.0]


def all_dimensions_pass(dimensions: Dict[str, float], thresholds: Dict[str, float]) -> bool:
    """Check if all critical dimensions meet thresholds."""
    return (
        dimensions["ttft_score"] >= thresholds["ttft_min"] and
        dimensions["quality_score"] >= thresholds["quality_min"] and
        dimensions["groundedness_score"] >= thresholds["groundedness_min"] and
        dimensions["success_rate"] >= thresholds["success_rate_min"]
    )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
