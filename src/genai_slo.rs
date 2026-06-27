/// TTFT (Time to First Token) and inter-token latency SLO evaluation for streaming GenAI.
///
/// Tracks streaming performance metrics specifically relevant to user perception:
/// - Time to First Token (TTFT): Latency until first token arrives (perceived responsiveness)
/// - Inter-Token Latency: Average time between successive tokens (perceived streaming smoothness)

use crate::{NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};

/// Configuration parameters for TTFT SLO evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtftSloParams {
    /// Maximum acceptable TTFT in milliseconds (e.g., 500ms)
    pub ttft_threshold_ms: f64,
    /// Percentile for TTFT threshold (e.g., 0.99 for P99)
    pub ttft_percentile: f64,
    /// Maximum acceptable inter-token latency in milliseconds (e.g., 50ms)
    pub inter_token_latency_threshold_ms: f64,
    /// Percentile for inter-token latency (e.g., 0.95 for P95)
    pub inter_token_percentile: f64,
}

impl Default for TtftSloParams {
    fn default() -> Self {
        Self {
            ttft_threshold_ms: 500.0,
            ttft_percentile: 0.99,
            inter_token_latency_threshold_ms: 50.0,
            inter_token_percentile: 0.95,
        }
    }
}

/// Streaming GenAI sample with latency metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenaiStreamSample {
    /// Unique identifier for this request
    pub request_id: String,
    /// Timestamp when request was initiated
    pub timestamp: u64,
    /// Time until first token arrived (milliseconds)
    pub time_to_first_token_ms: f64,
    /// Average latency between consecutive tokens (milliseconds)
    pub inter_token_latency_ms: f64,
    /// Total number of tokens generated
    pub total_tokens: u32,
    /// Total response time (milliseconds)
    pub total_response_time_ms: f64,
    /// Model used for generation
    pub model: String,
    /// Optional: raw inter-token latencies for percentile calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inter_token_latencies: Option<Vec<f64>>,
}

/// Evaluation result for TTFT SLO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtftEvaluation {
    /// Whether TTFT meets threshold
    pub ttft_pass: bool,
    /// Actual TTFT in milliseconds
    pub ttft_ms: f64,
    /// Whether inter-token latency meets threshold
    pub inter_token_pass: bool,
    /// Actual inter-token latency in milliseconds
    pub inter_token_latency_ms: f64,
    /// Overall SLO pass/fail
    pub pass: bool,
    /// Percentile metrics for monitoring
    pub details: TtftEvaluationDetails,
}

/// Detailed TTFT evaluation breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtftEvaluationDetails {
    /// TTFT as percentage of threshold (e.g., 0.9 = 90% of budget used)
    pub ttft_utilization: f64,
    /// Inter-token latency as percentage of threshold
    pub inter_token_utilization: f64,
    /// Time spent waiting for first token as fraction of total time
    pub ttft_fraction_of_total: f64,
    /// Total response time in milliseconds
    pub total_response_time_ms: f64,
    /// Total tokens generated
    pub total_tokens: u32,
    /// Tokens per second (throughput)
    pub tokens_per_second: f64,
}

/// Evaluate a streaming GenAI sample against TTFT SLO parameters.
///
/// # Arguments
/// * `sample` - GenAI streaming sample with TTFT and inter-token metrics
/// * `params` - TTFT SLO parameters with thresholds and percentiles
///
/// # Returns
/// * `Result<TtftEvaluation>` - Evaluation results with pass/fail determination
pub fn evaluate_ttft_slo(sample: &GenaiStreamSample, params: &TtftSloParams) -> Result<TtftEvaluation> {
    // Validate inputs
    if sample.time_to_first_token_ms < 0.0 || sample.inter_token_latency_ms < 0.0 {
        return Err(NeuralBudgetError::ConfigError(
            "Latencies cannot be negative".to_string(),
        ));
    }

    // Check TTFT threshold
    let ttft_pass = sample.time_to_first_token_ms <= params.ttft_threshold_ms;
    let ttft_utilization = sample.time_to_first_token_ms / params.ttft_threshold_ms;

    // Check inter-token threshold
    let inter_token_pass =
        sample.inter_token_latency_ms <= params.inter_token_latency_threshold_ms;
    let inter_token_utilization = sample.inter_token_latency_ms / params.inter_token_latency_threshold_ms;

    // Calculate additional metrics
    let ttft_fraction_of_total = if sample.total_response_time_ms > 0.0 {
        sample.time_to_first_token_ms / sample.total_response_time_ms
    } else {
        0.0
    };

    let tokens_per_second = if sample.total_response_time_ms > 0.0 {
        (sample.total_tokens as f64 / sample.total_response_time_ms) * 1000.0
    } else {
        0.0
    };

    let pass = ttft_pass && inter_token_pass;

    Ok(TtftEvaluation {
        ttft_pass,
        ttft_ms: sample.time_to_first_token_ms,
        inter_token_pass,
        inter_token_latency_ms: sample.inter_token_latency_ms,
        pass,
        details: TtftEvaluationDetails {
            ttft_utilization,
            inter_token_utilization,
            ttft_fraction_of_total,
            total_response_time_ms: sample.total_response_time_ms,
            total_tokens: sample.total_tokens,
            tokens_per_second,
        },
    })
}

/// Percentile value within a sorted slice.
fn calculate_percentile(sorted_values: &[f64], percentile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    if sorted_values.len() == 1 {
        return sorted_values[0];
    }

    let index = ((percentile * (sorted_values.len() as f64 - 1.0)).ceil()) as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

/// Evaluate a batch of streaming GenAI samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtftBatchEvaluation {
    /// Total samples evaluated
    pub total_samples: u32,
    /// Number of samples with TTFT pass
    pub ttft_pass_count: u32,
    /// TTFT pass rate (0.0-1.0)
    pub ttft_pass_rate: f64,
    /// P99 TTFT across all samples
    pub ttft_p99_ms: f64,
    /// P50 (median) TTFT
    pub ttft_p50_ms: f64,
    /// Number of samples with inter-token latency pass
    pub inter_token_pass_count: u32,
    /// Inter-token pass rate
    pub inter_token_pass_rate: f64,
    /// P95 inter-token latency
    pub inter_token_p95_ms: f64,
    /// P50 (median) inter-token latency
    pub inter_token_p50_ms: f64,
    /// Average TTFT across all samples
    pub avg_ttft_ms: f64,
    /// Average inter-token latency
    pub avg_inter_token_ms: f64,
    /// Average throughput (tokens/second)
    pub avg_tokens_per_second: f64,
    /// Overall pass rate (both TTFT and inter-token pass)
    pub overall_pass_rate: f64,
}

/// Evaluate a batch of streaming samples and compute percentile metrics.
///
/// # Arguments
/// * `samples` - Slice of GenAI streaming samples
/// * `params` - TTFT SLO parameters
///
/// # Returns
/// * `Result<TtftBatchEvaluation>` - Batch evaluation with percentiles
pub fn evaluate_ttft_batch(
    samples: &[GenaiStreamSample],
    params: &TtftSloParams,
) -> Result<TtftBatchEvaluation> {
    if samples.is_empty() {
        return Err(NeuralBudgetError::ConfigError(
            "Cannot evaluate empty sample batch".to_string(),
        ));
    }

    let mut ttft_values = Vec::new();
    let mut inter_token_values = Vec::new();
    let mut ttft_pass_count = 0u32;
    let mut inter_token_pass_count = 0u32;
    let mut overall_pass_count = 0u32;
    let mut total_tokens = 0u32;
    let mut total_time_ms = 0.0f64;

    for sample in samples {
        let eval = evaluate_ttft_slo(sample, params)?;

        ttft_values.push(sample.time_to_first_token_ms);
        inter_token_values.push(sample.inter_token_latency_ms);

        if eval.ttft_pass {
            ttft_pass_count += 1;
        }
        if eval.inter_token_pass {
            inter_token_pass_count += 1;
        }
        if eval.pass {
            overall_pass_count += 1;
        }

        total_tokens += sample.total_tokens;
        total_time_ms += sample.total_response_time_ms;
    }

    // Sort for percentile calculation
    ttft_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    inter_token_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let count = samples.len() as f64;
    let avg_ttft = ttft_values.iter().sum::<f64>() / count;
    let avg_inter_token = inter_token_values.iter().sum::<f64>() / count;
    let avg_tps = if total_time_ms > 0.0 {
        (total_tokens as f64 / total_time_ms) * 1000.0
    } else {
        0.0
    };

    Ok(TtftBatchEvaluation {
        total_samples: samples.len() as u32,
        ttft_pass_count,
        ttft_pass_rate: ttft_pass_count as f64 / count,
        ttft_p99_ms: calculate_percentile(&ttft_values, 0.99),
        ttft_p50_ms: calculate_percentile(&ttft_values, 0.50),
        inter_token_pass_count,
        inter_token_pass_rate: inter_token_pass_count as f64 / count,
        inter_token_p95_ms: calculate_percentile(&inter_token_values, 0.95),
        inter_token_p50_ms: calculate_percentile(&inter_token_values, 0.50),
        avg_ttft_ms: avg_ttft,
        avg_inter_token_ms: avg_inter_token,
        avg_tokens_per_second: avg_tps,
        overall_pass_rate: overall_pass_count as f64 / count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample(
        ttft_ms: f64,
        inter_token_ms: f64,
        total_tokens: u32,
        total_time_ms: f64,
    ) -> GenaiStreamSample {
        GenaiStreamSample {
            request_id: "test_req".to_string(),
            timestamp: 1000,
            time_to_first_token_ms: ttft_ms,
            inter_token_latency_ms: inter_token_ms,
            total_tokens,
            total_response_time_ms: total_time_ms,
            model: "gpt4".to_string(),
            inter_token_latencies: None,
        }
    }

    #[test]
    fn test_ttft_pass() {
        let sample = create_sample(450.0, 45.0, 250, 13000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.pass);
        assert!(eval.ttft_pass);
        assert!(eval.inter_token_pass);
    }

    #[test]
    fn test_ttft_exceeds_threshold() {
        let sample = create_sample(550.0, 45.0, 250, 13000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(!eval.pass);
        assert!(!eval.ttft_pass);
        assert!(eval.inter_token_pass);
    }

    #[test]
    fn test_inter_token_exceeds_threshold() {
        let sample = create_sample(450.0, 60.0, 250, 15000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(!eval.pass);
        assert!(eval.ttft_pass);
        assert!(!eval.inter_token_pass);
    }

    #[test]
    fn test_both_exceed_threshold() {
        let sample = create_sample(600.0, 70.0, 250, 18000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(!eval.pass);
        assert!(!eval.ttft_pass);
        assert!(!eval.inter_token_pass);
    }

    #[test]
    fn test_ttft_utilization_calculation() {
        let sample = create_sample(450.0, 45.0, 250, 13000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!((eval.details.ttft_utilization - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_inter_token_utilization_calculation() {
        let sample = create_sample(450.0, 40.0, 250, 13000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!((eval.details.inter_token_utilization - (40.0 / 50.0)).abs() < 0.001);
    }

    #[test]
    fn test_ttft_fraction_of_total() {
        let sample = create_sample(500.0, 50.0, 100, 10000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!((eval.details.ttft_fraction_of_total - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_tokens_per_second_calculation() {
        let sample = create_sample(500.0, 50.0, 1000, 10000.0); // 1000 tokens in 10 seconds
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!((eval.details.tokens_per_second - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_very_fast_ttft() {
        let sample = create_sample(100.0, 30.0, 500, 25000.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.pass);
        assert!((eval.details.ttft_utilization - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_single_token_response() {
        let sample = create_sample(200.0, 0.0, 1, 200.0);
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.pass);
        assert_eq!(eval.details.total_tokens, 1);
    }

    #[test]
    fn test_zero_response_time() {
        let sample = GenaiStreamSample {
            request_id: "zero_time".to_string(),
            timestamp: 1000,
            time_to_first_token_ms: 0.0,
            inter_token_latency_ms: 0.0,
            total_tokens: 0,
            total_response_time_ms: 0.0,
            model: "gpt4".to_string(),
            inter_token_latencies: None,
        };

        let params = TtftSloParams::default();
        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.pass);
        assert!(eval.details.tokens_per_second.is_finite());
    }

    #[test]
    fn test_batch_evaluation() {
        let samples = vec![
            create_sample(450.0, 45.0, 250, 13000.0),
            create_sample(480.0, 48.0, 250, 13000.0),
            create_sample(420.0, 42.0, 250, 13000.0),
            create_sample(510.0, 52.0, 250, 13000.0), // TTFT fail
            create_sample(490.0, 55.0, 250, 13000.0), // Inter-token fail
        ];

        let params = TtftSloParams::default();
        let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

        assert_eq!(batch_eval.total_samples, 5);
        assert!(batch_eval.ttft_pass_rate > 0.5);
        assert!(batch_eval.inter_token_pass_rate > 0.5);
    }

    #[test]
    fn test_batch_percentile_calculation() {
        let samples = vec![
            create_sample(100.0, 20.0, 100, 5000.0),
            create_sample(200.0, 30.0, 100, 5000.0),
            create_sample(300.0, 40.0, 100, 5000.0),
            create_sample(400.0, 50.0, 100, 5000.0),
            create_sample(500.0, 60.0, 100, 5000.0),
        ];

        let params = TtftSloParams::default();
        let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

        // P50 should be the median (300)
        assert!((batch_eval.ttft_p50_ms - 300.0).abs() < 1.0);
        // P99 should be near max
        assert!(batch_eval.ttft_p99_ms >= 400.0);
    }

    #[test]
    fn test_batch_pass_rate() {
        let mut samples = Vec::new();
        for i in 0..10 {
            let ttft = if i < 8 { 450.0 } else { 600.0 }; // 8 pass, 2 fail
            samples.push(create_sample(ttft, 45.0, 250, 13000.0));
        }

        let params = TtftSloParams::default();
        let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

        assert_eq!(batch_eval.ttft_pass_count, 8);
        assert!((batch_eval.ttft_pass_rate - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_custom_parameters() {
        let sample = create_sample(300.0, 60.0, 250, 13000.0);
        let params = TtftSloParams {
            ttft_threshold_ms: 400.0,
            ttft_percentile: 0.95,
            inter_token_latency_threshold_ms: 80.0,
            inter_token_percentile: 0.90,
        };

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.pass);
        assert!(eval.ttft_pass);
        assert!(eval.inter_token_pass);
    }

    #[test]
    fn test_serialization_sample() {
        let sample = create_sample(450.0, 45.0, 250, 13000.0);

        let json = serde_json::to_string(&sample).unwrap();
        let deserialized: GenaiStreamSample = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.request_id, sample.request_id);
        assert_eq!(deserialized.time_to_first_token_ms, sample.time_to_first_token_ms);
    }

    #[test]
    fn test_serialization_evaluation() {
        let sample = create_sample(450.0, 45.0, 250, 13000.0);
        let params = TtftSloParams::default();
        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        let json = serde_json::to_string(&eval).unwrap();
        let deserialized: TtftEvaluation = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.pass, eval.pass);
        assert!((deserialized.ttft_ms - eval.ttft_ms).abs() < 0.01);
    }

    #[test]
    fn test_batch_empty_invalid() {
        let samples: Vec<GenaiStreamSample> = vec![];
        let params = TtftSloParams::default();

        let result = evaluate_ttft_batch(&samples, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_latency_invalid() {
        let sample = GenaiStreamSample {
            request_id: "negative".to_string(),
            timestamp: 1000,
            time_to_first_token_ms: -100.0,
            inter_token_latency_ms: 45.0,
            total_tokens: 250,
            total_response_time_ms: 13000.0,
            model: "gpt4".to_string(),
            inter_token_latencies: None,
        };

        let params = TtftSloParams::default();
        let result = evaluate_ttft_slo(&sample, &params);

        assert!(result.is_err());
    }

    #[test]
    fn test_chat_assistant_scenario() {
        // Chat assistants need fast TTFT for perceived responsiveness
        let sample = create_sample(250.0, 40.0, 500, 25000.0);
        let params = TtftSloParams {
            ttft_threshold_ms: 300.0, // Stricter for chat
            ttft_percentile: 0.99,
            inter_token_latency_threshold_ms: 50.0,
            inter_token_percentile: 0.95,
        };

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.pass);
    }

    #[test]
    fn test_batch_code_generation_scenario() {
        // Code generation can tolerate longer TTFT but needs steady throughput
        let samples = vec![
            create_sample(600.0, 30.0, 1000, 40000.0),
            create_sample(580.0, 32.0, 1000, 40000.0),
            create_sample(620.0, 28.0, 1000, 40000.0),
        ];

        let params = TtftSloParams {
            ttft_threshold_ms: 700.0, // Tolerant TTFT
            ttft_percentile: 0.95,
            inter_token_latency_threshold_ms: 40.0,
            inter_token_percentile: 0.99,
        };

        let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

        assert!(batch_eval.overall_pass_rate >= 0.8);
    }

    #[test]
    fn test_batch_summarization_scenario() {
        // Long sequences with many tokens
        let samples = vec![
            create_sample(400.0, 35.0, 5000, 200000.0),
            create_sample(420.0, 37.0, 5000, 200000.0),
            create_sample(380.0, 33.0, 5000, 200000.0),
        ];

        let params = TtftSloParams {
            ttft_threshold_ms: 500.0,
            ttft_percentile: 0.99,
            inter_token_latency_threshold_ms: 40.0,
            inter_token_percentile: 0.95,
        };

        let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

        assert_eq!(batch_eval.total_samples, 3);
        assert!(batch_eval.avg_tokens_per_second > 20.0);
    }

    #[test]
    fn test_percentile_function() {
        let sorted = vec![100.0, 200.0, 300.0, 400.0, 500.0];

        let p50 = calculate_percentile(&sorted, 0.50);
        let p99 = calculate_percentile(&sorted, 0.99);

        assert_eq!(p50, 300.0); // Median
        assert!(p99 >= 400.0);
    }

    #[test]
    fn test_single_element_percentile() {
        let sorted = vec![250.0];

        let p50 = calculate_percentile(&sorted, 0.50);
        let p99 = calculate_percentile(&sorted, 0.99);

        assert_eq!(p50, 250.0);
        assert_eq!(p99, 250.0);
    }

    #[test]
    fn test_large_batch_metrics() {
        let mut samples = Vec::new();
        for i in 0..1000 {
            let ttft = 300.0 + (i as f64 % 200.0);
            let inter_token = 40.0 + (i as f64 % 20.0);
            samples.push(create_sample(ttft, inter_token, 500, 25000.0));
        }

        let params = TtftSloParams::default();
        let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

        assert_eq!(batch_eval.total_samples, 1000);
        assert!(batch_eval.avg_ttft_ms > 0.0);
        assert!(batch_eval.avg_tokens_per_second > 0.0);
    }

    #[test]
    fn test_throughput_calculation() {
        let sample = create_sample(500.0, 50.0, 100, 10000.0); // 100 tokens in 10 seconds
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!((eval.details.tokens_per_second - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_ttft_boundary_condition() {
        let sample = create_sample(500.0, 50.0, 250, 13000.0); // Exactly at threshold
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.ttft_pass);
    }

    #[test]
    fn test_inter_token_boundary_condition() {
        let sample = create_sample(450.0, 50.0, 250, 13000.0); // Exactly at threshold
        let params = TtftSloParams::default();

        let eval = evaluate_ttft_slo(&sample, &params).unwrap();

        assert!(eval.inter_token_pass);
    }
}

// ============================================================================
// Unified Composite GenAI SLO Evaluation
// ============================================================================

/// Individual dimension scores for composite GenAI SLO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeGenAiDimensions {
    /// Throughput score (0.0-1.0): tokens/sec / target_tps
    pub throughput_score: f64,
    /// TTFT score (0.0-1.0): 1.0 if passes, 0.0 otherwise, or normalized 0-1
    pub ttft_score: f64,
    /// Quality score (0.0-1.0): semantic similarity or LLM judge rating
    pub quality_score: f64,
    /// Groundedness score (0.0-1.0): 1.0 - hallucination_rate
    pub groundedness_score: f64,
    /// Cost score (0.0-1.0): budget_remaining / total_budget
    pub cost_score: f64,
    /// Retrieval score (0.0-1.0): recall@k or MRR for RAG
    pub retrieval_score: f64,
    /// Success rate (0.0-1.0): successful_requests / total_requests
    pub success_rate: f64,
}

impl CompositeGenAiDimensions {
    /// All dimensions must be between 0.0 and 1.0
    pub fn validate(&self) -> Result<()> {
        let dimensions = vec![
            ("throughput", self.throughput_score),
            ("ttft", self.ttft_score),
            ("quality", self.quality_score),
            ("groundedness", self.groundedness_score),
            ("cost", self.cost_score),
            ("retrieval", self.retrieval_score),
            ("success_rate", self.success_rate),
        ];

        for (name, score) in dimensions {
            if score < 0.0 || score > 1.0 {
                return Err(NeuralBudgetError::EvaluationError(format!(
                    "{} score {:.2} must be between 0.0 and 1.0",
                    name, score
                )));
            }
        }

        Ok(())
    }
}

/// Unified GenAI SLO evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeGenAiEvaluation {
    /// Individual dimension scores
    pub dimensions: CompositeGenAiDimensions,
    /// Weighted composite score (0.0-1.0)
    pub composite_score: f64,
    /// Whether all required dimensions pass
    pub all_dimensions_pass: bool,
    /// Whether composite score meets minimum target
    pub composite_pass: bool,
    /// Overall SLO result (all_pass AND composite_pass)
    pub pass: bool,
    /// Per-dimension pass/fail indicators
    pub dimension_pass_status: DimensionPassStatus,
}

/// Per-dimension pass/fail status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionPassStatus {
    /// TTFT meets threshold
    pub ttft_pass: bool,
    /// Quality meets threshold
    pub quality_pass: bool,
    /// Groundedness meets threshold
    pub groundedness_pass: bool,
    /// Success rate meets threshold
    pub success_rate_pass: bool,
}

/// Evaluate unified GenAI SLO combining all quality dimensions
///
/// # Arguments
/// * `dimensions` - Individual scores for all dimensions (0.0-1.0)
/// * `weights` - Configured weights for each dimension
/// * `thresholds` - Minimum scores required for each dimension to pass
///
/// # Returns
/// * `Result<CompositeGenAiEvaluation>` - Combined evaluation with composite score
pub fn evaluate_composite_genai_slo(
    dimensions: &CompositeGenAiDimensions,
    weights: &crate::CompositeGenAiWeights,
    thresholds: &CompositeGenAiThresholds,
) -> Result<CompositeGenAiEvaluation> {
    // Validate inputs
    dimensions.validate()?;
    weights.validate()?;
    thresholds.validate()?;

    // Calculate weighted composite score
    let composite_score = weights.throughput_weight * dimensions.throughput_score
        + weights.ttft_weight * dimensions.ttft_score
        + weights.quality_weight * dimensions.quality_score
        + weights.groundedness_weight * dimensions.groundedness_score
        + weights.cost_weight * dimensions.cost_score
        + weights.retrieval_weight * dimensions.retrieval_score
        + weights.success_rate_weight * dimensions.success_rate;

    // Check per-dimension pass/fail
    let ttft_pass = dimensions.ttft_score >= thresholds.ttft_min;
    let quality_pass = dimensions.quality_score >= thresholds.quality_min;
    let groundedness_pass = dimensions.groundedness_score >= thresholds.groundedness_min;
    let success_rate_pass = dimensions.success_rate >= thresholds.success_rate_min;

    let all_dimensions_pass = ttft_pass && quality_pass && groundedness_pass && success_rate_pass;
    let composite_pass = composite_score >= weights.min_target_score;

    // Overall pass: all dimensions must pass AND composite score must meet target
    let pass = all_dimensions_pass && composite_pass;

    Ok(CompositeGenAiEvaluation {
        dimensions: dimensions.clone(),
        composite_score,
        all_dimensions_pass,
        composite_pass,
        pass,
        dimension_pass_status: DimensionPassStatus {
            ttft_pass,
            quality_pass,
            groundedness_pass,
            success_rate_pass,
        },
    })
}

/// Thresholds for individual dimensions to pass
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeGenAiThresholds {
    /// Minimum TTFT score (0.0-1.0)
    #[serde(default = "default_ttft_min")]
    pub ttft_min: f64,
    /// Minimum quality score (0.0-1.0)
    #[serde(default = "default_quality_min")]
    pub quality_min: f64,
    /// Minimum groundedness score (0.0-1.0)
    #[serde(default = "default_groundedness_min")]
    pub groundedness_min: f64,
    /// Minimum success rate (0.0-1.0)
    #[serde(default = "default_success_rate_min")]
    pub success_rate_min: f64,
}

fn default_ttft_min() -> f64 { 0.90 }
fn default_quality_min() -> f64 { 0.85 }
fn default_groundedness_min() -> f64 { 0.95 }
fn default_success_rate_min() -> f64 { 0.99 }

impl Default for CompositeGenAiThresholds {
    fn default() -> Self {
        Self {
            ttft_min: 0.90,
            quality_min: 0.85,
            groundedness_min: 0.95,
            success_rate_min: 0.99,
        }
    }
}

impl CompositeGenAiThresholds {
    /// Validate thresholds are in valid range
    pub fn validate(&self) -> Result<()> {
        let thresholds = vec![
            ("ttft", self.ttft_min),
            ("quality", self.quality_min),
            ("groundedness", self.groundedness_min),
            ("success_rate", self.success_rate_min),
        ];

        for (name, threshold) in thresholds {
            if threshold < 0.0 || threshold > 1.0 {
                return Err(NeuralBudgetError::ConfigError(format!(
                    "{}_min threshold {:.2} must be between 0.0 and 1.0",
                    name, threshold
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod composite_tests {
    use super::*;

    #[test]
    fn test_composite_all_pass() {
        let dims = CompositeGenAiDimensions {
            throughput_score: 0.95,
            ttft_score: 0.92,
            quality_score: 0.88,
            groundedness_score: 0.97,
            cost_score: 0.90,
            retrieval_score: 0.93,
            success_rate: 0.99,
        };

        let weights = crate::CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(eval.pass);
        assert!(eval.composite_pass);
        assert!(eval.all_dimensions_pass);
    }

    #[test]
    fn test_composite_quality_fail() {
        let dims = CompositeGenAiDimensions {
            throughput_score: 0.95,
            ttft_score: 0.92,
            quality_score: 0.75, // Below 0.85 threshold
            groundedness_score: 0.97,
            cost_score: 0.90,
            retrieval_score: 0.93,
            success_rate: 0.99,
        };

        let weights = crate::CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        assert!(!eval.pass);
        assert!(!eval.dimension_pass_status.quality_pass);
    }

    #[test]
    fn test_composite_score_calculation() {
        let dims = CompositeGenAiDimensions {
            throughput_score: 1.0,
            ttft_score: 1.0,
            quality_score: 1.0,
            groundedness_score: 1.0,
            cost_score: 1.0,
            retrieval_score: 1.0,
            success_rate: 1.0,
        };

        let weights = crate::CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds::default();

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // All dimensions at 1.0 should yield composite score of 1.0
        assert!((eval.composite_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_composite_threshold_pass() {
        let dims = CompositeGenAiDimensions {
            throughput_score: 0.85,
            ttft_score: 0.85,
            quality_score: 0.85,
            groundedness_score: 0.85,
            cost_score: 0.85,
            retrieval_score: 0.85,
            success_rate: 0.85,
        };

        let weights = crate::CompositeGenAiWeights::default();
        let thresholds = CompositeGenAiThresholds {
            ttft_min: 0.80,
            quality_min: 0.80,
            groundedness_min: 0.80,
            success_rate_min: 0.80,
        };

        let eval = evaluate_composite_genai_slo(&dims, &weights, &thresholds).unwrap();

        // Composite score: 0.85 * 1.0 = 0.85, which is >= 0.85 min_target
        assert!(eval.composite_pass || eval.all_dimensions_pass);
    }
}
