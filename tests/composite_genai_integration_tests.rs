/// Comprehensive integration tests for unified Composite GenAI SLO evaluation.
///
/// Tests weighted scoring, dimension thresholds, and real-world scenarios
/// across different weight profiles.

#[cfg(test)]
mod composite_genai_integration_tests {
    use neuralbudget::{
        CompositeGenAiDimensions, CompositeGenAiEvaluation, CompositeGenAiThresholds,
        CompositeGenAiWeights, evaluate_composite_genai_slo,
    };

    fn create_dimensions(
        throughput: f64,
        ttft: f64,
        quality: f64,
        groundedness: f64,
        cost: f64,
        retrieval: f64,
        success: f64,
    ) -> CompositeGenAiDimensions {
        CompositeGenAiDimensions {
            throughput_score: throughput,
            ttft_score: ttft,
            quality_score: quality,
            groundedness_score: groundedness,
            cost_score: cost,
            retrieval_score: retrieval,
            success_rate: success,
        }
    }

    // ========================================================================
    // Basic Functionality Tests
    // ========================================================================

    #[test]
    fn test_composite_all_perfect_scores() {
        let dims = create_dimensions(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(eval.pass);
        assert_eq!(eval.composite_score, 1.0);
        assert!(eval.composite_pass);
        assert!(eval.all_dimensions_pass);
    }

    #[test]
    fn test_composite_all_minimum_pass() {
        let dims = create_dimensions(0.90, 0.90, 0.85, 0.95, 0.50, 0.50, 0.99);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(eval.pass);
        assert!(eval.dimension_pass_status.ttft_pass);
        assert!(eval.dimension_pass_status.quality_pass);
        assert!(eval.dimension_pass_status.groundedness_pass);
        assert!(eval.dimension_pass_status.success_rate_pass);
    }

    #[test]
    fn test_composite_quality_fail() {
        let dims = create_dimensions(0.95, 0.92, 0.75, 0.97, 0.90, 0.93, 0.99);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(!eval.pass);
        assert!(!eval.dimension_pass_status.quality_pass);
        assert!(eval.dimension_pass_status.ttft_pass);
    }

    #[test]
    fn test_composite_groundedness_fail() {
        let dims = create_dimensions(0.95, 0.92, 0.88, 0.90, 0.90, 0.93, 0.99);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(!eval.pass);
        assert!(!eval.dimension_pass_status.groundedness_pass);
    }

    #[test]
    fn test_composite_success_rate_fail() {
        let dims = create_dimensions(0.95, 0.92, 0.88, 0.97, 0.90, 0.93, 0.95);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(!eval.pass);
        assert!(!eval.dimension_pass_status.success_rate_pass);
    }

    // ========================================================================
    // Weight Profile Tests
    // ========================================================================

    #[test]
    fn test_quality_first_profile() {
        let dims = create_dimensions(0.6, 0.7, 1.0, 1.0, 0.5, 0.6, 0.95);

        let weights = CompositeGenAiWeights {
            throughput_weight: 0.10,
            ttft_weight: 0.10,
            quality_weight: 0.40,
            groundedness_weight: 0.25,
            cost_weight: 0.05,
            retrieval_weight: 0.05,
            success_rate_weight: 0.05,
            min_target_score: 0.90,
        };

        let thresholds = CompositeGenAiThresholds {
            ttft_min: 0.80,
            quality_min: 0.95,
            groundedness_min: 0.98,
            success_rate_min: 0.99,
        };

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Quality and groundedness are perfect (weights 0.40+0.25 = 0.65)
        // Should boost composite score despite weak throughput/cost
        assert!(eval.composite_score > 0.75);
    }

    #[test]
    fn test_cost_optimized_profile() {
        let dims = create_dimensions(0.95, 0.85, 0.75, 0.80, 1.0, 0.70, 0.98);

        let weights = CompositeGenAiWeights {
            throughput_weight: 0.20,
            ttft_weight: 0.10,
            quality_weight: 0.25,
            groundedness_weight: 0.10,
            cost_weight: 0.25,
            retrieval_weight: 0.05,
            success_rate_weight: 0.05,
            min_target_score: 0.80,
        };

        let thresholds = CompositeGenAiThresholds {
            ttft_min: 0.75,
            quality_min: 0.75,
            groundedness_min: 0.85,
            success_rate_min: 0.95,
        };

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Perfect cost score (weight 0.25) should boost overall score
        assert!(eval.composite_score > 0.80);
    }

    #[test]
    fn test_balanced_profile() {
        let dims = create_dimensions(0.92, 0.88, 0.86, 0.96, 0.88, 0.89, 0.98);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Typical production scenario should pass
        assert!(eval.pass);
        assert!(eval.composite_score > 0.88);
        assert!(eval.composite_score < 0.93);
    }

    // ========================================================================
    // Weighted Score Calculation Tests
    // ========================================================================

    #[test]
    fn test_score_calculation_equal_dimensions() {
        let dims = create_dimensions(0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8);

        let weights = CompositeGenAiWeights {
            throughput_weight: 1.0 / 7.0,
            ttft_weight: 1.0 / 7.0,
            quality_weight: 1.0 / 7.0,
            groundedness_weight: 1.0 / 7.0,
            cost_weight: 1.0 / 7.0,
            retrieval_weight: 1.0 / 7.0,
            success_rate_weight: 1.0 / 7.0,
            min_target_score: 0.85,
        };

        let thresholds = CompositeGenAiThresholds::default();
        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Equal scores with equal weights should give 0.8
        assert!((eval.composite_score - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_score_bounds_zero() {
        let dims = create_dimensions(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert_eq!(eval.composite_score, 0.0);
        assert!(!eval.composite_pass);
    }

    #[test]
    fn test_score_bounds_one() {
        let dims = create_dimensions(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert_eq!(eval.composite_score, 1.0);
    }

    #[test]
    fn test_quality_dominance() {
        // High quality vs low quality with quality weight 0.30
        let dims_high_quality = create_dimensions(0.5, 0.5, 1.0, 0.5, 0.5, 0.5, 0.5);
        let dims_low_quality = create_dimensions(0.5, 0.5, 0.0, 0.5, 0.5, 0.5, 0.5);

        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval_high = evaluate_composite_genai_slo(&dims_high_quality, &weights, &thresholds).unwrap();
        let eval_low = evaluate_composite_genai_slo(&dims_low_quality, &weights, &thresholds).unwrap();

        // High quality should significantly outperform
        let diff = eval_high.composite_score - eval_low.composite_score;
        assert!(diff > 0.25); // Quality weight 0.30 drives at least 0.30 difference
    }

    // ========================================================================
    // Real-World Scenario Tests
    // ========================================================================

    #[test]
    fn test_chat_assistant_typical() {
        let dims = create_dimensions(0.90, 0.95, 0.87, 0.96, 0.85, 0.88, 0.99);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(eval.pass);
        assert!(eval.composite_score > 0.90);
    }

    #[test]
    fn test_rag_system_with_perfect_retrieval() {
        let dims = create_dimensions(0.85, 0.80, 0.90, 0.98, 0.75, 0.95, 0.99);

        // RAG-optimized: higher retrieval and groundedness weights
        let weights = CompositeGenAiWeights {
            throughput_weight: 0.10,
            ttft_weight: 0.10,
            quality_weight: 0.25,
            groundedness_weight: 0.20,
            cost_weight: 0.10,
            retrieval_weight: 0.20,
            success_rate_weight: 0.05,
            min_target_score: 0.85,
        };

        let thresholds = CompositeGenAiThresholds::default();
        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Perfect retrieval and groundedness should boost score
        assert!(eval.composite_score > 0.88);
    }

    #[test]
    fn test_cost_constrained_service() {
        let dims = create_dimensions(0.88, 0.82, 0.80, 0.90, 0.95, 0.75, 0.98);

        let weights = CompositeGenAiWeights {
            throughput_weight: 0.20,
            ttft_weight: 0.10,
            quality_weight: 0.20,
            groundedness_weight: 0.10,
            cost_weight: 0.25,
            retrieval_weight: 0.10,
            success_rate_weight: 0.05,
            min_target_score: 0.80,
        };

        let thresholds = CompositeGenAiThresholds {
            ttft_min: 0.75,
            quality_min: 0.75,
            groundedness_min: 0.85,
            success_rate_min: 0.95,
        };

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Should pass despite moderate quality due to excellent cost
        assert!(eval.pass);
        assert!(eval.composite_score > 0.85);
    }

    #[test]
    fn test_high_volume_service() {
        let dims = create_dimensions(0.98, 0.92, 0.85, 0.94, 0.80, 0.85, 0.995);

        // Throughput-optimized: higher throughput weight for high-volume
        let weights = CompositeGenAiWeights {
            throughput_weight: 0.25,
            ttft_weight: 0.15,
            quality_weight: 0.25,
            groundedness_weight: 0.10,
            cost_weight: 0.15,
            retrieval_weight: 0.05,
            success_rate_weight: 0.05,
            min_target_score: 0.85,
        };

        let thresholds = CompositeGenAiThresholds::default();
        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(eval.pass);
        // High throughput should boost score
        assert!(eval.composite_score > 0.90);
    }

    // ========================================================================
    // Configuration Validation Tests
    // ========================================================================

    #[test]
    fn test_weights_sum_to_one() {
        let weights = CompositeGenAiWeights::default();
        let validation = weights.validate();

        assert!(validation.is_ok());
    }

    #[test]
    fn test_weights_invalid_sum() {
        let weights = CompositeGenAiWeights {
            throughput_weight: 0.20,
            ttft_weight: 0.20,
            quality_weight: 0.20,
            groundedness_weight: 0.20,
            cost_weight: 0.10,
            retrieval_weight: 0.10,
            success_rate_weight: 0.10,
            min_target_score: 0.85,
        };

        let validation = weights.validate();
        assert!(validation.is_err());
    }

    #[test]
    fn test_thresholds_valid() {
        let thresholds = CompositeGenAiThresholds::default();
        let validation = thresholds.validate();

        assert!(validation.is_ok());
    }

    #[test]
    fn test_dimension_validation() {
        let dims_invalid = CompositeGenAiDimensions {
            throughput_score: 1.5, // Invalid: > 1.0
            ttft_score: 0.5,
            quality_score: 0.5,
            groundedness_score: 0.5,
            cost_score: 0.5,
            retrieval_score: 0.5,
            success_rate: 0.5,
        };

        let validation = dims_invalid.validate();
        assert!(validation.is_err());
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_single_dimension_perfect() {
        let dims = create_dimensions(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Composite score should equal throughput weight (0.15)
        assert!((eval.composite_score - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_single_dimension_failure() {
        // Single failing dimension but composite score still high
        let dims = create_dimensions(0.95, 0.5, 0.95, 0.95, 0.95, 0.95, 0.95);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // TTFT is weak (0.5), should not pass due to dimension threshold
        assert!(!eval.pass);
        assert!(!eval.dimension_pass_status.ttft_pass);
    }

    #[test]
    fn test_composite_pass_with_weak_dimensions() {
        // Composite score passes but one dimension fails
        let dims = create_dimensions(0.95, 0.85, 0.95, 0.95, 0.95, 0.95, 0.95);
        let weights = CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds {
            ttft_min: 0.90, // TTFT will fail
            quality_min: 0.85,
            groundedness_min: 0.95,
            success_rate_min: 0.99,
        };

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // TTFT at 0.85 < 0.90 threshold, so dimension check fails
        assert!(!eval.all_dimensions_pass);
        assert!(!eval.pass); // Must pass all dimensions AND composite score
    }
}
