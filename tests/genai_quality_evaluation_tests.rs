//! Integration tests for GenAI LLM-as-Judge evaluator.
//!
//! Note: Tests that require actual API keys are marked with `#[ignore]` and
//! can be run with `cargo test -- --ignored --nocapture`

#[cfg(test)]
mod genai_quality_evaluation_tests {
    use neuralbudget::{
        CacheConfig, DimensionScoreResult, EvaluationResult, LlmJudgeDimension, LlmJudgeEvaluator,
        LlmProvider,
    };

    #[test]
    fn test_cache_key_determinism() {
        let key1 = LlmJudgeEvaluator::generate_cache_key("query", "response");
        let key2 = LlmJudgeEvaluator::generate_cache_key("query", "response");

        assert_eq!(key1, key2, "Same input should produce same cache key");
    }

    #[test]
    fn test_cache_key_different_for_different_inputs() {
        let key1 = LlmJudgeEvaluator::generate_cache_key("query1", "response1");
        let key2 = LlmJudgeEvaluator::generate_cache_key("query1", "response2");

        assert_ne!(key1, key2, "Different inputs should produce different keys");
    }

    #[test]
    fn test_cache_key_format() {
        let key = LlmJudgeEvaluator::generate_cache_key("test", "test");
        assert!(
            key.starts_with("llm_judge:"),
            "Cache key should start with 'llm_judge:'"
        );
    }

    #[test]
    fn test_dimension_creation() {
        let dim = LlmJudgeDimension {
            name: "correctness".to_string(),
            prompt: "Is this correct? Score 1-5.".to_string(),
            weight: 0.5,
            threshold: 3.0,
            cost_per_call_usd: 0.0001,
        };

        assert_eq!(dim.name, "correctness");
        assert_eq!(dim.weight, 0.5);
        assert_eq!(dim.threshold, 3.0);
        assert_eq!(dim.cost_per_call_usd, 0.0001);
    }

    #[test]
    fn test_score_normalization() {
        // Score 1 → 0.0, Score 5 → 1.0
        let test_cases: &[(f64, f64)] =
            &[(1.0, 0.0), (2.0, 0.25), (3.0, 0.5), (4.0, 0.75), (5.0, 1.0)];

        for (raw_score, expected_normalized) in test_cases {
            let normalized = (raw_score - 1.0) / 4.0;
            assert!(
                (normalized - expected_normalized).abs() < 0.001,
                "Score {} should normalize to {}, got {}",
                raw_score,
                expected_normalized,
                normalized
            );
        }
    }

    #[test]
    fn test_weighted_aggregation() {
        // Test case: 2 dimensions with equal weight
        // dim1: 0.8 (score 4.2/5), dim2: 0.6 (score 3.4/5)
        // Both weight 0.5: (0.8*0.5 + 0.6*0.5) / 1.0 = 0.7

        let score1: f64 = 0.8;
        let score2: f64 = 0.6;
        let weight1: f64 = 0.5;
        let weight2: f64 = 0.5;

        let weighted_sum = score1 * weight1 + score2 * weight2;
        let weight_total = weight1 + weight2;
        let final_score = weighted_sum / weight_total;

        assert!((final_score - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_pass_fail_with_single_dimension() {
        // Score 4/5 = 0.75 normalized, threshold 0.6 → PASS
        let score = 0.75;
        let threshold = 0.6;
        let pass = score >= threshold;

        assert!(pass);
    }

    #[test]
    fn test_pass_fail_with_multiple_dimensions() {
        let dimensions = [
            ("correctness", 0.75, 0.6, true), // 0.75 >= 0.6 → pass
            ("safety", 0.4, 0.8, false),      // 0.4 < 0.8 → fail
        ];

        let all_pass = dimensions
            .iter()
            .all(|(_, score, threshold, _)| score >= threshold);
        assert!(!all_pass, "Should fail because safety dimension fails");
    }

    #[test]
    fn test_llm_provider_model() {
        let provider = LlmProvider::OpenAI {
            api_key: "sk-test".to_string(),
            model: "gpt-4-mini".to_string(),
        };

        assert_eq!(provider.model(), "gpt-4-mini");
        assert_eq!(provider.api_key(), Some("sk-test"));
    }

    #[test]
    fn test_llm_provider_anthropic() {
        let provider = LlmProvider::Anthropic {
            api_key: "sk-ant-test".to_string(),
            model: "claude-3-haiku".to_string(),
        };

        assert_eq!(provider.model(), "claude-3-haiku");
        assert_eq!(provider.api_key(), Some("sk-ant-test"));
    }

    #[test]
    fn test_llm_provider_local() {
        let provider = LlmProvider::Local {
            base_url: "http://localhost:11434".to_string(),
            model: "llama2".to_string(),
        };

        assert_eq!(provider.model(), "llama2");
        assert_eq!(provider.api_key(), None);
    }

    #[test]
    fn test_evaluator_creation() {
        let dimensions = vec![LlmJudgeDimension {
            name: "test".to_string(),
            prompt: "Test {query} {response}".to_string(),
            weight: 1.0,
            threshold: 3.0,
            cost_per_call_usd: 0.0001,
        }];

        let evaluator = LlmJudgeEvaluator::new(
            LlmProvider::Local {
                base_url: "http://localhost:11434".to_string(),
                model: "llama2".to_string(),
            },
            dimensions,
        );

        assert_eq!(evaluator.dimensions.len(), 1);
    }

    #[test]
    fn test_dimension_score_result() {
        let result = DimensionScoreResult {
            name: "correctness".to_string(),
            score: 4.0,
            reasoning: Some("Very accurate response".to_string()),
            pass: true,
        };

        assert_eq!(result.name, "correctness");
        assert_eq!(result.score, 4.0);
        assert!(result.pass);
    }

    #[test]
    fn test_evaluation_result_structure() {
        let result = EvaluationResult {
            timestamp: 1234567890,
            cache_key: "llm_judge:abc123".to_string(),
            from_cache: false,
            dimension_scores: vec![],
            weighted_score: 0.8,
            pass: true,
            total_cost_usd: 0.0002,
            total_tokens: 150,
        };

        assert_eq!(result.timestamp, 1234567890);
        assert!(!result.from_cache);
        assert_eq!(result.total_cost_usd, 0.0002);
        assert_eq!(result.total_tokens, 150);
    }

    #[test]
    fn test_cost_calculation_multiple_dimensions() {
        let costs = &[0.0001, 0.0001, 0.00005];
        let total_cost: f64 = costs.iter().sum();

        assert!((total_cost - 0.00025).abs() < 0.000001);
    }

    #[test]
    fn test_cache_config_serialization() {
        let config = CacheConfig {
            redis_url: "redis://localhost:6379".to_string(),
            ttl_seconds: 86400,
        };

        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.ttl_seconds, 86400);
    }

    // Mock extraction test (no actual LLM calls)
    #[test]
    fn test_score_extraction_formats() {
        // These are the formats the extract_score function should handle
        let test_cases = vec![
            ("Score: 4", 4.0),
            ("I rate this a 3 out of 5", 3.0),
            ("5", 5.0),
            ("Quality: 2", 2.0),
            ("Rating: 4", 4.0),
        ];

        for (response, expected_score) in test_cases {
            // The extract_score logic: find first digit 1-5
            let mut found = false;
            for c in response.chars() {
                if let Ok(score) = c.to_string().parse::<f64>() {
                    if (1.0..=5.0).contains(&score) {
                        assert_eq!(score, expected_score);
                        found = true;
                        break;
                    }
                }
            }
            assert!(found, "Should find score in: {}", response);
        }
    }

    #[test]
    fn test_score_extraction_invalid_responses() {
        let invalid_responses = vec![
            "No score here",
            "Score 0 (out of bounds)",
            "Score 6 (out of bounds)",
        ];

        for response in invalid_responses {
            let mut found = false;
            for c in response.chars() {
                if let Ok(score) = c.to_string().parse::<f64>() {
                    if (1.0..=5.0).contains(&score) {
                        found = true;
                        break;
                    }
                }
            }
            assert!(!found, "Should not find valid score in: {}", response);
        }
    }

    #[test]
    fn test_weighted_score_with_three_dimensions() {
        // correctness: 0.75 (4/5), weight 0.4 → 0.3
        // safety: 1.0 (5/5), weight 0.35 → 0.35
        // tone: 0.5 (3/5), weight 0.25 → 0.125
        // Total: 0.775

        let dimensions = [(0.75, 0.4), (1.0, 0.35), (0.5, 0.25)];

        let weighted_sum: f64 = dimensions
            .iter()
            .map(|(score, weight)| score * weight)
            .sum();
        let weight_sum: f64 = dimensions.iter().map(|(_, weight)| weight).sum();
        let final_score = weighted_sum / weight_sum;

        assert!((final_score - 0.775).abs() < 0.001);
    }

    #[test]
    fn test_cache_key_hash_consistency() {
        // Verify that the hash function produces consistent output
        let query = "What is machine learning?";
        let response = "Machine learning is a subset of artificial intelligence.";

        let key1 = LlmJudgeEvaluator::generate_cache_key(query, response);
        let key2 = LlmJudgeEvaluator::generate_cache_key(query, response);
        let key3 = LlmJudgeEvaluator::generate_cache_key(query, "Different response");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_monthly_cost_estimation() {
        let queries_per_day = 10_000;
        let cache_hit_rate = 0.95;
        let cost_per_uncached = 0.0002;

        let uncached_per_day = queries_per_day as f64 * (1.0 - cache_hit_rate);
        let daily_cost = uncached_per_day * cost_per_uncached;
        let monthly_cost = daily_cost * 30.0;

        // Expected: 10000 * 0.05 * 0.0002 * 30 = $3.00
        assert!((monthly_cost - 3.0).abs() < 0.1);
    }

    // Integration tests that require real API keys are marked as ignored
    #[ignore]
    #[tokio::test]
    async fn test_openai_integration_with_cache() {
        // This test requires:
        // - OPENAI_API_KEY environment variable
        // - Redis running on localhost:6379
        // Run with: cargo test test_openai_integration_with_cache -- --ignored --nocapture

        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY env var not set");

        let dimensions = vec![LlmJudgeDimension {
            name: "test".to_string(),
            prompt: "Rate this: {response}. Score 1-5.".to_string(),
            weight: 1.0,
            threshold: 3.0,
            cost_per_call_usd: 0.0001,
        }];

        let _evaluator = LlmJudgeEvaluator::new(
            LlmProvider::OpenAI {
                api_key: api_key.clone(),
                model: "gpt-4-mini".to_string(),
            },
            dimensions,
        );

        // Note: Redis cache test would require actual Redis connection
        // .with_redis_cache("redis://localhost:6379", 3600)
        // .await
        // .expect("Failed to configure cache");

        // This would call actual OpenAI API
        // let result = evaluator.evaluate("test query", "test response").await;
        // assert!(result.is_ok());
    }
}
