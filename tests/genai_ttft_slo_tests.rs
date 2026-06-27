// Comprehensive test suite for TTFT (Time to First Token) SLO evaluation

use neuralbudget::{
    evaluate_ttft_batch, evaluate_ttft_slo, GenaiStreamSample, TtftEvaluation, TtftSloParams,
};
use std::sync::atomic::{AtomicU32, Ordering};

fn create_sample(
    ttft_ms: f64,
    inter_token_ms: f64,
    total_tokens: u32,
    total_time_ms: f64,
) -> GenaiStreamSample {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);

    GenaiStreamSample {
        request_id: format!("req_{}", id),
        timestamp: 1000,
        time_to_first_token_ms: ttft_ms,
        inter_token_latency_ms: inter_token_ms,
        total_tokens,
        total_response_time_ms: total_time_ms,
        model: "gpt-4".to_string(),
        inter_token_latencies: None,
    }
}

#[test]
fn test_basic_ttft_pass() {
    let sample = create_sample(450.0, 45.0, 250, 13000.0);
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(eval.pass);
    assert!(eval.ttft_pass);
    assert!(eval.inter_token_pass);
    assert_eq!(eval.ttft_ms, 450.0);
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
fn test_both_exceed_thresholds() {
    let sample = create_sample(600.0, 70.0, 250, 18000.0);
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(!eval.pass);
    assert!(!eval.ttft_pass);
    assert!(!eval.inter_token_pass);
}

#[test]
fn test_ttft_utilization_90_percent() {
    let sample = create_sample(450.0, 45.0, 250, 13000.0);
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!((eval.details.ttft_utilization - 0.9).abs() < 0.001);
}

#[test]
fn test_inter_token_utilization() {
    let sample = create_sample(450.0, 40.0, 250, 13000.0);
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!((eval.details.inter_token_utilization - 0.8).abs() < 0.001);
}

#[test]
fn test_ttft_fraction_of_total_response() {
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
fn test_very_fast_ttft_chat_optimized() {
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
fn test_zero_total_response_time() {
    let sample = GenaiStreamSample {
        request_id: "zero_time".to_string(),
        timestamp: 1000,
        time_to_first_token_ms: 0.0,
        inter_token_latency_ms: 0.0,
        total_tokens: 0,
        total_response_time_ms: 0.0,
        model: "gpt-4".to_string(),
        inter_token_latencies: None,
    };

    let params = TtftSloParams::default();
    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(eval.pass);
    assert!(eval.details.tokens_per_second.is_finite());
}

#[test]
fn test_batch_evaluation_mixed_results() {
    let samples = vec![
        create_sample(450.0, 45.0, 250, 13000.0), // Pass
        create_sample(480.0, 48.0, 250, 13000.0), // Pass
        create_sample(420.0, 42.0, 250, 13000.0), // Pass
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
fn test_batch_percentile_p50_calculation() {
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
}

#[test]
fn test_batch_percentile_p99_calculation() {
    let samples = vec![
        create_sample(100.0, 20.0, 100, 5000.0),
        create_sample(200.0, 30.0, 100, 5000.0),
        create_sample(300.0, 40.0, 100, 5000.0),
        create_sample(400.0, 50.0, 100, 5000.0),
        create_sample(500.0, 60.0, 100, 5000.0),
    ];

    let params = TtftSloParams::default();
    let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

    // P99 should be near max
    assert!(batch_eval.ttft_p99_ms >= 400.0);
}

#[test]
fn test_batch_pass_rate_8_of_10() {
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
fn test_custom_strict_parameters() {
    let sample = create_sample(300.0, 60.0, 250, 13000.0);
    let params = TtftSloParams {
        ttft_threshold_ms: 250.0,
        ttft_percentile: 0.95,
        inter_token_latency_threshold_ms: 50.0,
        inter_token_percentile: 0.90,
    };

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(!eval.ttft_pass); // 300 > 250
    assert!(!eval.inter_token_pass); // 60 > 50
}

#[test]
fn test_custom_tolerant_parameters() {
    let sample = create_sample(800.0, 80.0, 250, 18000.0);
    let params = TtftSloParams {
        ttft_threshold_ms: 1000.0,
        ttft_percentile: 0.95,
        inter_token_latency_threshold_ms: 100.0,
        inter_token_percentile: 0.90,
    };

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(eval.pass);
    assert!(eval.ttft_pass);
    assert!(eval.inter_token_pass);
}

#[test]
fn test_serialization_stream_sample() {
    let sample = create_sample(450.0, 45.0, 250, 13000.0);

    let json = serde_json::to_string(&sample).unwrap();
    let deserialized: GenaiStreamSample = serde_json::from_str(&json).unwrap();

    assert_eq!(
        deserialized.time_to_first_token_ms,
        sample.time_to_first_token_ms
    );
    assert_eq!(
        deserialized.inter_token_latency_ms,
        sample.inter_token_latency_ms
    );
}

#[test]
fn test_serialization_ttft_evaluation() {
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
fn test_negative_ttft_invalid() {
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
    // Chat: fast TTFT critical for perceived responsiveness
    let sample = create_sample(250.0, 40.0, 500, 25000.0);
    let params = TtftSloParams {
        ttft_threshold_ms: 300.0, // Stricter for chat
        ttft_percentile: 0.99,
        inter_token_latency_threshold_ms: 50.0,
        inter_token_percentile: 0.95,
    };

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(eval.pass);
    assert!((eval.details.tokens_per_second - 20.0).abs() < 1.0);
}

#[test]
fn test_code_generation_scenario() {
    // Code generation: tolerates longer TTFT but needs steady throughput
    let samples = vec![
        create_sample(600.0, 30.0, 1000, 40000.0),
        create_sample(580.0, 32.0, 1000, 40000.0),
        create_sample(620.0, 28.0, 1000, 40000.0),
    ];

    let params = TtftSloParams {
        ttft_threshold_ms: 700.0,
        ttft_percentile: 0.95,
        inter_token_latency_threshold_ms: 40.0,
        inter_token_percentile: 0.99,
    };

    let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

    assert!(batch_eval.overall_pass_rate >= 0.8);
    assert!(batch_eval.avg_tokens_per_second > 20.0);
}

#[test]
fn test_summarization_long_sequences() {
    // Summarization: many tokens with moderate throughput
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
fn test_ttft_at_exact_threshold() {
    let sample = create_sample(500.0, 50.0, 250, 13000.0); // Exactly at threshold
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(eval.ttft_pass);
    assert!(eval.inter_token_pass);
}

#[test]
fn test_ttft_one_ms_over_threshold() {
    let sample = create_sample(501.0, 50.0, 250, 13000.0); // Just over threshold
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(!eval.ttft_pass);
}

#[test]
fn test_large_batch_1000_samples() {
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
    assert!(batch_eval.avg_inter_token_ms > 0.0);
    assert!(batch_eval.avg_tokens_per_second > 0.0);
}

#[test]
fn test_throughput_calculation_precision() {
    let sample = create_sample(500.0, 50.0, 100, 10000.0); // 100 tokens in 10 seconds
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!((eval.details.tokens_per_second - 10.0).abs() < 0.1);
}

#[test]
fn test_batch_average_calculations() {
    let samples = vec![
        create_sample(400.0, 40.0, 500, 25000.0),
        create_sample(500.0, 50.0, 500, 25000.0),
        create_sample(600.0, 60.0, 500, 25000.0),
    ];

    let params = TtftSloParams::default();
    let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

    assert!((batch_eval.avg_ttft_ms - 500.0).abs() < 1.0);
    assert!((batch_eval.avg_inter_token_ms - 50.0).abs() < 1.0);
}

#[test]
fn test_batch_overall_pass_rate() {
    let mut samples = Vec::new();
    for i in 0..100 {
        let ttft = if i < 90 { 450.0 } else { 600.0 }; // 90 pass, 10 fail
        let inter_token = 45.0;
        samples.push(create_sample(ttft, inter_token, 250, 13000.0));
    }

    let params = TtftSloParams::default();
    let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

    assert!((batch_eval.overall_pass_rate - 0.9).abs() < 0.01);
}

#[test]
fn test_very_long_response_stream() {
    // Test with very long stream (10k tokens)
    let sample = create_sample(800.0, 35.0, 10000, 350000.0);
    let params = TtftSloParams {
        ttft_threshold_ms: 1000.0,
        ttft_percentile: 0.99,
        inter_token_latency_threshold_ms: 40.0,
        inter_token_percentile: 0.95,
    };

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    assert!(eval.pass);
    assert!(eval.details.tokens_per_second > 25.0);
}

#[test]
fn test_real_world_chat_snapshot() {
    // Real-world: Chat responding to "What is machine learning?"
    let samples = vec![
        create_sample(280.0, 35.0, 150, 5250.0), // "What is..." response
        create_sample(320.0, 38.0, 200, 7600.0), // Longer response
        create_sample(250.0, 32.0, 180, 5760.0), // Quick response
        create_sample(300.0, 40.0, 220, 8800.0), // Detailed response
    ];

    let params = TtftSloParams {
        ttft_threshold_ms: 400.0,
        ttft_percentile: 0.99,
        inter_token_latency_threshold_ms: 50.0,
        inter_token_percentile: 0.95,
    };

    let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

    assert!(batch_eval.overall_pass_rate >= 0.75);
}

#[test]
fn test_inter_token_p95_percentile() {
    let mut samples = Vec::new();
    for i in 0..20 {
        let inter_token = 30.0 + (i as f64);
        samples.push(create_sample(400.0, inter_token, 500, 25000.0));
    }

    let params = TtftSloParams::default();
    let batch_eval = evaluate_ttft_batch(&samples, &params).unwrap();

    // P95 should be near the high end
    assert!(batch_eval.inter_token_p95_ms > 40.0);
}

#[test]
fn test_utilization_metrics_sum() {
    let sample = create_sample(450.0, 45.0, 250, 13000.0);
    let params = TtftSloParams::default();

    let eval = evaluate_ttft_slo(&sample, &params).unwrap();

    // Both utilizations should be < 1.0 (passing condition)
    assert!(eval.details.ttft_utilization < 1.0);
    assert!(eval.details.inter_token_utilization < 1.0);
}
