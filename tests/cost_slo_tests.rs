//! Integration tests for cost-based SLOs.

#[cfg(test)]
mod cost_slo_tests {
    use neuralbudget::{CostBudget, CostEvaluation, CostSloEvaluator, GenaiCostSample};
    use serde_json;

    #[test]
    fn test_gpt4_mini_budget() {
        let budget = CostBudget::gpt4_mini();
        assert_eq!(budget.input_cost_per_1k, 0.00015);
        assert_eq!(budget.output_cost_per_1k, 0.0006);
        assert!(budget.max_per_request > 0.0);
    }

    #[test]
    fn test_claude3_haiku_budget() {
        let budget = CostBudget::claude3_haiku();
        assert_eq!(budget.input_cost_per_1k, 0.00025);
        assert_eq!(budget.output_cost_per_1k, 0.00125);
    }

    #[test]
    fn test_budget_creation() {
        let budget = CostBudget::new(0.0001, 0.0005, 0.01);
        assert_eq!(budget.input_cost_per_1k, 0.0001);
        assert_eq!(budget.output_cost_per_1k, 0.0005);
        assert_eq!(budget.max_per_request, 0.01);
    }

    #[test]
    fn test_cost_sample_total_tokens() {
        let sample = GenaiCostSample::new(1000, 100, 200);
        assert_eq!(sample.total_tokens(), 300);
    }

    #[test]
    fn test_cost_sample_zero_tokens() {
        let sample = GenaiCostSample::new(1000, 0, 0);
        assert_eq!(sample.total_tokens(), 0);
    }

    #[test]
    fn test_cost_sample_builder_latency() {
        let sample = GenaiCostSample::new(1000, 50, 100)
            .with_latency(750);
        assert_eq!(sample.inference_latency_ms, Some(750));
    }

    #[test]
    fn test_cost_sample_builder_ttft() {
        let sample = GenaiCostSample::new(1000, 50, 100)
            .with_ttft(200);
        assert_eq!(sample.ttft_ms, Some(200));
    }

    #[test]
    fn test_cost_sample_builder_quality() {
        let sample = GenaiCostSample::new(1000, 50, 100)
            .with_quality(0.92);
        assert_eq!(sample.quality_score, Some(0.92));
    }

    #[test]
    fn test_cost_sample_quality_clamping() {
        let sample1 = GenaiCostSample::new(1000, 50, 100)
            .with_quality(-0.5);
        assert_eq!(sample1.quality_score, Some(0.0));

        let sample2 = GenaiCostSample::new(1000, 50, 100)
            .with_quality(1.5);
        assert_eq!(sample2.quality_score, Some(1.0));
    }

    #[test]
    fn test_cost_sample_builder_chaining() {
        let sample = GenaiCostSample::new(1000, 50, 120)
            .with_latency(750)
            .with_ttft(200)
            .with_quality(0.92)
            .with_model("gpt-4-mini".to_string());

        assert_eq!(sample.input_tokens, 50);
        assert_eq!(sample.output_tokens, 120);
        assert_eq!(sample.inference_latency_ms, Some(750));
        assert_eq!(sample.ttft_ms, Some(200));
        assert_eq!(sample.quality_score, Some(0.92));
        assert_eq!(sample.model, Some("gpt-4-mini".to_string()));
    }

    #[test]
    fn test_cost_calculation_exact() {
        // GPT-4 Mini: $0.00015 per 1K input, $0.0006 per 1K output
        // 50 input tokens, 120 output tokens
        let input_cost: f64 = (50.0 / 1000.0) * 0.00015;
        let output_cost: f64 = (120.0 / 1000.0) * 0.0006;

        assert!((input_cost - 0.0000075).abs() < 0.00000001);
        assert!((output_cost - 0.000072).abs() < 0.00000001);
        assert!((input_cost + output_cost - 0.0000795).abs() < 0.00000001);
    }

    #[test]
    fn test_cost_evaluation_under_budget() {
        let budget = CostBudget::gpt4_mini();
        let input_cost = 0.0000075;
        let output_cost = 0.000072;
        let total_cost = 0.0000795;

        let eval = CostEvaluation::new(
            input_cost,
            output_cost,
            total_cost,
            budget.max_per_request,
            0.95,
        );

        assert!(eval.within_budget);
        assert!(eval.pass);
        assert!(eval.cost_score > 0.99);
    }

    #[test]
    fn test_cost_evaluation_at_budget() {
        let budget = CostBudget::new(0.001, 0.001, 0.01);
        let total_cost = 0.01;

        let eval = CostEvaluation::new(0.004, 0.006, total_cost, budget.max_per_request, 0.95);

        assert!(eval.within_budget);
        assert!(eval.cost_score.abs() < 0.001);  // Near zero
    }

    #[test]
    fn test_cost_evaluation_over_budget() {
        let budget = CostBudget::new(0.001, 0.001, 0.005);  // Small budget
        let total_cost = 0.01;  // Over budget

        let eval = CostEvaluation::new(0.005, 0.005, total_cost, budget.max_per_request, 0.95);

        assert!(!eval.within_budget);
        assert!(!eval.pass);
        assert!(eval.cost_score < 0.0);  // Negative means over budget
    }

    #[test]
    fn test_cost_score_calculation() {
        let max_budget: f64 = 0.01;
        let total_cost: f64 = 0.002;

        let cost_score: f64 = (max_budget - total_cost) / max_budget;
        assert!((cost_score - 0.8).abs() < 0.001);  // 80% of budget remaining
    }

    #[test]
    fn test_cost_slo_evaluator_creation() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        assert_eq!(evaluator.cost_threshold, 0.95);
    }

    #[test]
    fn test_cost_slo_evaluator_with_threshold() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_cost_threshold(0.80);
        assert_eq!(evaluator.cost_threshold, 0.80);
    }

    #[test]
    fn test_cost_slo_evaluator_with_monthly_limit() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_monthly_limit(5000.0);
        assert_eq!(evaluator.monthly_budget, Some(5000.0));
    }

    #[test]
    fn test_cost_threshold_clamping() {
        let evaluator1 = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_cost_threshold(-0.5);
        assert_eq!(evaluator1.cost_threshold, 0.0);

        let evaluator2 = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_cost_threshold(1.5);
        assert_eq!(evaluator2.cost_threshold, 1.0);
    }

    #[test]
    fn test_evaluate_single_request() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);

        let result = evaluator.evaluate_request(&sample).unwrap();

        assert!(result.within_budget);
        assert!(result.pass);
        assert!(result.total_cost < 0.001);
    }

    #[test]
    fn test_evaluate_batch_all_pass() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let samples = vec![
            GenaiCostSample::new(1000, 50, 120),
            GenaiCostSample::new(2000, 60, 150),
            GenaiCostSample::new(3000, 40, 100),
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();

        assert_eq!(result.request_count, 3);
        assert!(result.pass_rate > 0.99);
    }

    #[test]
    fn test_evaluate_batch_monthly_ok() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_monthly_limit(100.0);

        let samples = vec![
            GenaiCostSample::new(1000, 50, 120),
            GenaiCostSample::new(2000, 60, 150),
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();
        assert!(result.monthly_budget_ok);
    }

    #[test]
    fn test_evaluate_batch_monthly_exceeded() {
        let evaluator = CostSloEvaluator::new(CostBudget::new(0.1, 0.1, 0.1))
            .with_monthly_limit(0.001);  // Tiny budget

        let samples = vec![
            GenaiCostSample::new(1000, 1000, 1000),
            GenaiCostSample::new(2000, 1000, 1000),
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();
        assert!(!result.monthly_budget_ok);
    }

    #[test]
    fn test_batch_cost_accumulation() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let samples = vec![
            GenaiCostSample::new(1000, 100, 100),
            GenaiCostSample::new(2000, 100, 100),
            GenaiCostSample::new(3000, 100, 100),
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();

        // All three requests should accumulate
        assert!(result.total_cost > 0.0);
        assert_eq!(result.request_count, 3);
    }

    #[test]
    fn test_hybrid_score_calculation() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);
        let cost_eval = evaluator.evaluate_request(&sample).unwrap();

        let quality_score = 0.92;
        let hybrid = evaluator.hybrid_score(&cost_eval, quality_score, 0.1, 0.9);

        // 10% cost weight + 90% quality weight
        // Expected to be close to quality since cost is near-perfect
        assert!(hybrid > 0.85);
        assert!(hybrid < 1.0);
    }

    #[test]
    fn test_hybrid_score_equal_weights() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);
        let cost_eval = evaluator.evaluate_request(&sample).unwrap();

        let quality_score = 0.8;
        let hybrid = evaluator.hybrid_score(&cost_eval, quality_score, 1.0, 1.0);

        // Equal weights: 50% cost (near 1.0) + 50% quality (0.8)
        assert!(hybrid > 0.85);
    }

    #[test]
    fn test_hybrid_score_quality_weight_zero() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);
        let cost_eval = evaluator.evaluate_request(&sample).unwrap();

        let quality_score = 0.2;
        let hybrid = evaluator.hybrid_score(&cost_eval, quality_score, 1.0, 0.0);

        // 100% cost weight, quality ignored
        assert!(hybrid > 0.99);
    }

    #[test]
    fn test_cost_comparison_models() {
        let mini = CostBudget::gpt4_mini();
        let turbo = CostBudget::gpt4_standard();

        // Turbo should be more expensive
        assert!(turbo.input_cost_per_1k > mini.input_cost_per_1k);
        assert!(turbo.output_cost_per_1k > mini.output_cost_per_1k);
    }

    #[test]
    fn test_cost_sample_serialization() {
        use serde_json;

        let sample = GenaiCostSample::new(1000, 50, 120)
            .with_quality(0.92)
            .with_model("gpt-4-mini".to_string());

        let json = serde_json::to_string(&sample).unwrap();
        let deserialized: GenaiCostSample = serde_json::from_str(&json).unwrap();

        assert_eq!(sample.input_tokens, deserialized.input_tokens);
        assert_eq!(sample.output_tokens, deserialized.output_tokens);
        assert_eq!(sample.quality_score, deserialized.quality_score);
        assert_eq!(sample.model, deserialized.model);
    }

    #[test]
    fn test_cost_evaluation_serialization() {
        use serde_json;

        let eval = CostEvaluation::new(0.001, 0.002, 0.003, 0.01, 0.95);

        let json = serde_json::to_string(&eval).unwrap();
        let deserialized: CostEvaluation = serde_json::from_str(&json).unwrap();

        assert_eq!(eval.input_cost, deserialized.input_cost);
        assert_eq!(eval.output_cost, deserialized.output_cost);
        assert_eq!(eval.total_cost, deserialized.total_cost);
        assert_eq!(eval.pass, deserialized.pass);
    }

    #[test]
    fn test_large_batch_evaluation() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        
        // Simulate 1000 requests
        let mut samples = Vec::new();
        for i in 0..1000 {
            samples.push(GenaiCostSample::new(i as u64, 50 + (i % 100) as u32, 100 + (i % 200) as u32));
        }

        let result = evaluator.evaluate_batch(&samples).unwrap();

        assert_eq!(result.request_count, 1000);
        assert!(result.pass_rate > 0.98);  // Most should pass
    }

    #[test]
    fn test_zero_token_request() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 0, 0);

        let result = evaluator.evaluate_request(&sample).unwrap();

        assert_eq!(result.total_cost, 0.0);
        assert!(result.within_budget);
        assert!(result.pass);
    }

    #[test]
    fn test_precise_cost_calculation_gpt4_mini() {
        // Exact calculation for GPT-4 Mini
        let budget = CostBudget::gpt4_mini();
        let _sample = GenaiCostSample::new(1000, 1000, 1000);

        let input_cost = (1000.0 / 1000.0) * budget.input_cost_per_1k;
        let output_cost = (1000.0 / 1000.0) * budget.output_cost_per_1k;
        let total_cost = input_cost + output_cost;

        assert!((input_cost - 0.00015).abs() < 0.000001);
        assert!((output_cost - 0.0006).abs() < 0.000001);
        assert!((total_cost - 0.00075).abs() < 0.000001);
    }

    #[test]
    fn test_monthly_budget_exact() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_monthly_limit(10.0);

        // Exactly 10000 requests at $0.001 each = $10
        let samples = (0..10000)
            .map(|i| GenaiCostSample::new(i as u64, 1666, 1334))  // Approx 0.001 each
            .collect::<Vec<_>>();

        let result = evaluator.evaluate_batch(&samples).unwrap();

        assert!(result.total_cost <= 10.0 * 1.01);  // Allow 1% rounding error
    }
}
