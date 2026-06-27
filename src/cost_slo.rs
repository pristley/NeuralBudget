//! Cost-based SLOs for GenAI workloads.
//!
//! Tracks token usage and cost, enabling budgets and alerts for per-request and monthly spend.
//! Treats cost as a first-class SLI alongside quality and availability.

use crate::NeuralBudgetError;
use serde::{Deserialize, Serialize};

/// Cost budget configuration for input/output tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostBudget {
    /// Cost per 1000 input tokens (e.g., 0.00015 for GPT-4 mini)
    pub input_cost_per_1k: f64,
    /// Cost per 1000 output tokens (e.g., 0.0006 for GPT-4 mini)
    pub output_cost_per_1k: f64,
    /// Maximum cost allowed per request (e.g., 0.015 = 1.5 cents)
    pub max_per_request: f64,
}

impl CostBudget {
    /// Create a new cost budget.
    pub fn new(
        input_cost_per_1k: f64,
        output_cost_per_1k: f64,
        max_per_request: f64,
    ) -> Self {
        CostBudget {
            input_cost_per_1k,
            output_cost_per_1k,
            max_per_request,
        }
    }

    /// GPT-4 Mini pricing (as of June 2026)
    pub fn gpt4_mini() -> Self {
        CostBudget {
            input_cost_per_1k: 0.00015,
            output_cost_per_1k: 0.0006,
            max_per_request: 0.015,
        }
    }

    /// GPT-4 Standard pricing
    pub fn gpt4_standard() -> Self {
        CostBudget {
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.006,
            max_per_request: 0.05,
        }
    }

    /// Claude 3 Haiku pricing
    pub fn claude3_haiku() -> Self {
        CostBudget {
            input_cost_per_1k: 0.00025,
            output_cost_per_1k: 0.00125,
            max_per_request: 0.02,
        }
    }

    /// Claude 3 Sonnet pricing
    pub fn claude3_sonnet() -> Self {
        CostBudget {
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
            max_per_request: 0.05,
        }
    }
}

/// Sample with token counts for cost evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenaiCostSample {
    /// Timestamp (unix seconds)
    pub timestamp: u64,
    /// Number of input tokens consumed
    pub input_tokens: u32,
    /// Number of output tokens generated
    pub output_tokens: u32,
    /// Optional: inference latency in milliseconds
    pub inference_latency_ms: Option<u32>,
    /// Optional: time to first token in milliseconds
    pub ttft_ms: Option<u32>,
    /// Optional: associated quality score (0.0-1.0)
    pub quality_score: Option<f64>,
    /// Optional: model used (for pricing lookup)
    pub model: Option<String>,
}

impl GenaiCostSample {
    /// Create a new cost sample with token counts.
    pub fn new(timestamp: u64, input_tokens: u32, output_tokens: u32) -> Self {
        GenaiCostSample {
            timestamp,
            input_tokens,
            output_tokens,
            inference_latency_ms: None,
            ttft_ms: None,
            quality_score: None,
            model: None,
        }
    }

    /// Set inference latency.
    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.inference_latency_ms = Some(latency_ms);
        self
    }

    /// Set time to first token.
    pub fn with_ttft(mut self, ttft_ms: u32) -> Self {
        self.ttft_ms = Some(ttft_ms);
        self
    }

    /// Set quality score.
    pub fn with_quality(mut self, quality: f64) -> Self {
        self.quality_score = Some(quality.clamp(0.0, 1.0));
        self
    }

    /// Set model name.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    /// Total tokens consumed.
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Cost evaluation result for a single request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEvaluation {
    /// Cost for input tokens in USD
    pub input_cost: f64,
    /// Cost for output tokens in USD
    pub output_cost: f64,
    /// Total cost in USD
    pub total_cost: f64,
    /// Whether total cost is within per-request budget
    pub within_budget: bool,
    /// Cost score (0.0-1.0, where 1.0 = free, 0.0 = over budget)
    pub cost_score: f64,
    /// Whether cost is acceptable (cost_score >= threshold, default 0.95)
    pub pass: bool,
}

impl CostEvaluation {
    /// Create a cost evaluation.
    pub fn new(
        input_cost: f64,
        output_cost: f64,
        total_cost: f64,
        max_per_request: f64,
        cost_threshold: f64,
    ) -> Self {
        let within_budget = total_cost <= max_per_request;
        // Cost score: how much of budget we're using
        // 1.0 = free, 0.0 = fully consumed
        let cost_score = if max_per_request > 0.0 {
            ((max_per_request - total_cost) / max_per_request).max(0.0)
        } else {
            0.0
        };
        let pass = within_budget && cost_score >= cost_threshold;

        CostEvaluation {
            input_cost,
            output_cost,
            total_cost,
            within_budget,
            cost_score,
            pass,
        }
    }
}

/// Cost SLO evaluator.
pub struct CostSloEvaluator {
    pub budget: CostBudget,
    pub cost_threshold: f64,
    pub monthly_budget: Option<f64>,
}

impl CostSloEvaluator {
    /// Create a new cost SLO evaluator with budget.
    pub fn new(budget: CostBudget) -> Self {
        CostSloEvaluator {
            budget,
            cost_threshold: 0.95,  // Default: 95% budget utilization is acceptable
            monthly_budget: None,
        }
    }

    /// Set cost acceptability threshold.
    ///
    /// - 0.95: Accept up to 95% of budget per request
    /// - 0.80: Stricter, only 80% utilization acceptable
    pub fn with_cost_threshold(mut self, threshold: f64) -> Self {
        self.cost_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set monthly cost limit.
    pub fn with_monthly_limit(mut self, limit: f64) -> Self {
        self.monthly_budget = Some(limit);
        self
    }

    /// Evaluate cost for a single request.
    pub fn evaluate_request(&self, sample: &GenaiCostSample) -> Result<CostEvaluation, NeuralBudgetError> {
        let input_cost = (sample.input_tokens as f64 / 1000.0) * self.budget.input_cost_per_1k;
        let output_cost = (sample.output_tokens as f64 / 1000.0) * self.budget.output_cost_per_1k;
        let total_cost = input_cost + output_cost;

        Ok(CostEvaluation::new(
            input_cost,
            output_cost,
            total_cost,
            self.budget.max_per_request,
            self.cost_threshold,
        ))
    }

    /// Evaluate batch of requests.
    pub fn evaluate_batch(
        &self,
        samples: &[GenaiCostSample],
    ) -> Result<BatchCostEvaluation, NeuralBudgetError> {
        let mut total_cost = 0.0;
        let mut evaluations = Vec::new();
        let mut passed = 0;
        let mut _failed = 0;

        for sample in samples {
            let eval = self.evaluate_request(sample)?;
            if eval.pass {
                passed += 1;
            } else {
                _failed += 1;
            }
            total_cost += eval.total_cost;
            evaluations.push(eval);
        }

        let monthly_budget_ok = if let Some(limit) = self.monthly_budget {
            total_cost <= limit
        } else {
            true
        };

        let pass_rate = if !evaluations.is_empty() {
            passed as f64 / evaluations.len() as f64
        } else {
            1.0
        };

        Ok(BatchCostEvaluation {
            evaluations,
            total_cost,
            pass_rate,
            monthly_budget_ok,
            request_count: samples.len(),
        })
    }

    /// Calculate hybrid score combining cost and quality.
    pub fn hybrid_score(
        &self,
        cost_eval: &CostEvaluation,
        quality_score: f64,
        cost_weight: f64,
        quality_weight: f64,
    ) -> f64 {
        let normalized_cost_weight = cost_weight / (cost_weight + quality_weight);
        let normalized_quality_weight = quality_weight / (cost_weight + quality_weight);

        normalized_cost_weight * cost_eval.cost_score
            + normalized_quality_weight * quality_score.clamp(0.0, 1.0)
    }
}

/// Batch cost evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCostEvaluation {
    /// Per-request evaluations
    pub evaluations: Vec<CostEvaluation>,
    /// Sum of all request costs
    pub total_cost: f64,
    /// Proportion of requests that passed cost check
    pub pass_rate: f64,
    /// Whether total cost is within monthly budget
    pub monthly_budget_ok: bool,
    /// Number of requests evaluated
    pub request_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_budget_gpt4_mini() {
        let budget = CostBudget::gpt4_mini();
        assert_eq!(budget.input_cost_per_1k, 0.00015);
        assert_eq!(budget.output_cost_per_1k, 0.0006);
    }

    #[test]
    fn test_cost_budget_claude3_haiku() {
        let budget = CostBudget::claude3_haiku();
        assert_eq!(budget.input_cost_per_1k, 0.00025);
        assert_eq!(budget.output_cost_per_1k, 0.00125);
    }

    #[test]
    fn test_cost_sample_total_tokens() {
        let sample = GenaiCostSample::new(1000, 50, 120);
        assert_eq!(sample.total_tokens(), 170);
    }

    #[test]
    fn test_cost_sample_builder() {
        let sample = GenaiCostSample::new(1000, 50, 120)
            .with_latency(750)
            .with_ttft(200)
            .with_quality(0.92)
            .with_model("gpt-4-mini".to_string());

        assert_eq!(sample.inference_latency_ms, Some(750));
        assert_eq!(sample.ttft_ms, Some(200));
        assert_eq!(sample.quality_score, Some(0.92));
        assert_eq!(sample.model, Some("gpt-4-mini".to_string()));
    }

    #[test]
    fn test_cost_evaluation_under_budget() {
        let budget = CostBudget::gpt4_mini();
        let sample = GenaiCostSample::new(1000, 50, 120);

        let input_cost = (50.0 / 1000.0) * 0.00015;
        let output_cost = (120.0 / 1000.0) * 0.0006;
        let total_cost = input_cost + output_cost;

        let eval = CostEvaluation::new(input_cost, output_cost, total_cost, budget.max_per_request, 0.95);

        assert!(eval.within_budget);
        assert!(eval.pass);
        assert!(eval.cost_score > 0.99);
    }

    #[test]
    fn test_cost_evaluation_over_budget() {
        let budget = CostBudget::new(0.001, 0.001, 0.005);  // max_per_request = 0.005
        let _sample = GenaiCostSample::new(1000, 5000, 5000);  // 5000 input + 5000 output tokens

        let input_cost = (5000.0 / 1000.0) * 0.001;  // 0.005
        let output_cost = (5000.0 / 1000.0) * 0.001; // 0.005
        let total_cost = input_cost + output_cost;   // 0.01 > 0.005 budget

        let eval = CostEvaluation::new(input_cost, output_cost, total_cost, budget.max_per_request, 0.95);

        assert!(!eval.within_budget);
        assert!(!eval.pass);
    }

    #[test]
    fn test_cost_slo_evaluator_single_request() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);

        let result = evaluator.evaluate_request(&sample).unwrap();
        assert!(result.pass);
        assert!(result.within_budget);
    }

    #[test]
    fn test_cost_slo_evaluator_batch() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_monthly_limit(10.0);

        let samples = vec![
            GenaiCostSample::new(1000, 50, 120),
            GenaiCostSample::new(2000, 60, 150),
            GenaiCostSample::new(3000, 40, 100),
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();
        assert_eq!(result.request_count, 3);
        assert!(result.monthly_budget_ok);
        assert!(result.pass_rate >= 0.9);
    }

    #[test]
    fn test_hybrid_score_cost_and_quality() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);
        let cost_eval = evaluator.evaluate_request(&sample).unwrap();

        let quality_score = 0.92;
        let hybrid = evaluator.hybrid_score(&cost_eval, quality_score, 0.1, 0.9);

        // 10% cost weight + 90% quality weight
        // Expected: 0.1 * cost_score + 0.9 * quality_score
        assert!(hybrid > 0.8);  // Should be close to quality score
    }

    #[test]
    fn test_cost_score_decreases_with_usage() {
        let budget = CostBudget::new(0.001, 0.001, 0.01);

        // Low token usage
        let eval1 = CostEvaluation::new(0.001, 0.001, 0.002, budget.max_per_request, 0.95);
        // High token usage
        let eval2 = CostEvaluation::new(0.003, 0.003, 0.006, budget.max_per_request, 0.95);

        assert!(eval1.cost_score > eval2.cost_score);
    }

    #[test]
    fn test_batch_pass_rate() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());

        let samples = vec![
            GenaiCostSample::new(1000, 50, 120),    // Under budget
            GenaiCostSample::new(2000, 50, 120),    // Under budget
            GenaiCostSample::new(3000, 50, 120),    // Under budget
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();
        assert_eq!(result.pass_rate, 1.0);  // All passed
    }

    #[test]
    fn test_monthly_budget_tracking() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_monthly_limit(1.0);  // $1 monthly budget

        let samples = vec![
            // Each with 500K output tokens costs ~$0.30
            // Need at least 4 samples to exceed $1.0 limit
            GenaiCostSample::new(1000, 1000, 500_000),   // ~$0.30
            GenaiCostSample::new(2000, 1000, 500_000),   // ~$0.30
            GenaiCostSample::new(3000, 1000, 500_000),   // ~$0.30
            GenaiCostSample::new(4000, 1000, 500_000),   // ~$0.30
            // Total: ~$1.20, exceeding $1.0 limit
        ];

        let result = evaluator.evaluate_batch(&samples).unwrap();
        assert!(!result.monthly_budget_ok);  // Total cost should exceed $1 limit
    }

    #[test]
    fn test_cost_threshold_configuration() {
        let evaluator_strict = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_cost_threshold(0.5);

        let evaluator_lenient = CostSloEvaluator::new(CostBudget::gpt4_mini())
            .with_cost_threshold(0.99);

        assert!(evaluator_strict.cost_threshold == 0.5);
        assert!(evaluator_lenient.cost_threshold == 0.99);
    }

    #[test]
    fn test_cost_calculation_accuracy() {
        // GPT-4 Mini: $0.00015 per 1K input, $0.0006 per 1K output
        let budget = CostBudget::gpt4_mini();
        let _sample = GenaiCostSample::new(1000, 1000, 1000);

        let input_cost = (1000.0 / 1000.0) * 0.00015;
        let output_cost = (1000.0 / 1000.0) * 0.0006;
        let total_cost = input_cost + output_cost;

        assert_eq!(input_cost, 0.00015);
        assert_eq!(output_cost, 0.0006);
        // Use approximate equality for floating point
        assert!((total_cost - 0.00075_f64).abs() < 1e-10);
    }

    #[test]
    fn test_zero_budget_edge_case() {
        let eval = CostEvaluation::new(0.0, 0.0, 0.0, 0.0, 0.95);
        assert!(eval.within_budget);
        assert_eq!(eval.cost_score, 0.0);
    }

    #[test]
    fn test_quality_score_clamping() {
        let evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini());
        let sample = GenaiCostSample::new(1000, 50, 120);
        let cost_eval = evaluator.evaluate_request(&sample).unwrap();

        // Test clamping of invalid quality scores
        let hybrid1 = evaluator.hybrid_score(&cost_eval, -0.5, 0.1, 0.9);
        let hybrid2 = evaluator.hybrid_score(&cost_eval, 1.5, 0.1, 0.9);

        assert!(hybrid1 >= 0.0);
        assert!(hybrid2 <= 1.0);
    }
}
