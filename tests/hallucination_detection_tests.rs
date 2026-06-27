//! Integration tests for hallucination detection via groundedness evaluation.

#[cfg(test)]
mod hallucination_detection_tests {
    use neuralbudget::{
        Claim, ClaimExtractionConfig, ClaimExtractionMethod, Document, GroundednessEvaluator,
        GroundednessResult, HallucinationDetectionConfig, HallucinationExtractionMethod,
        HallucinationScoringMethod, ScoredClaim, SimilarityMethod,
    };
    use serde_json;

    #[test]
    fn test_claim_extraction_rule_based() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let response = "Exercise is healthy. It improves fitness. Sleep matters too.";
        let claims = evaluator.extract_claims_rule_based(response).unwrap();

        assert_eq!(claims.len(), 3);
        assert!(claims[0].text.contains("Exercise"));
        assert!(claims[1].text.contains("improves fitness"));
    }

    #[test]
    fn test_claim_extraction_filters_short_sentences() {
        let config = ClaimExtractionConfig {
            method: ClaimExtractionMethod::RuleBased,
            min_length: 20,
            max_claims: 10,
        };

        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        )
        .with_extraction_config(config);

        let response = "Hi. Regular exercise improves cardiovascular health.";
        let claims = evaluator.extract_claims_rule_based(response).unwrap();

        // "Hi" should be filtered out (too short)
        assert!(claims.iter().all(|c| c.text.len() >= 20));
    }

    #[test]
    fn test_claim_extraction_respects_max_claims() {
        let config = ClaimExtractionConfig {
            method: ClaimExtractionMethod::RuleBased,
            min_length: 1,
            max_claims: 2,
        };

        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        )
        .with_extraction_config(config);

        let response = "Claim one. Claim two. Claim three. Claim four.";
        let claims = evaluator.extract_claims_rule_based(response).unwrap();

        assert!(claims.len() <= 2);
    }

    #[test]
    fn test_token_overlap_exact_match() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let score = evaluator.token_overlap("exercise is healthy", "exercise is healthy");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_token_overlap_partial_match() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let score = evaluator.token_overlap("exercise is healthy", "exercise is good");
        // 2 matching tokens (exercise, is) / 4 total = 0.5
        assert!(score >= 0.4 && score <= 0.6);
    }

    #[test]
    fn test_token_overlap_no_match() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let score = evaluator.token_overlap("exercise", "weather");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_embedding_similarity_contains_keywords() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::EmbeddingSimilarity,
            0.5,
        );

        // All claim words in document
        let score = evaluator.embedding_similarity("exercise health", "exercise improves health");
        assert_eq!(score, 1.0);

        // Some words in document
        let score = evaluator.embedding_similarity("exercise benefits mood", "exercise helps health");
        assert!(score > 0.0 && score < 1.0);

        // No words in document
        let score = evaluator.embedding_similarity("exercise", "weather");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_tfidf_similarity() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TfIdf,
            0.5,
        );

        // Exact match should be high
        let score = evaluator.tfidf_similarity("exercise health", "exercise health benefits");
        assert!(score > 0.5);

        // No match should be zero
        let score = evaluator.tfidf_similarity("exercise", "weather");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_groundedness_all_claims_grounded() {
        let result = GroundednessResult {
            claims: vec![
                ScoredClaim {
                    claim: Claim {
                        text: "claim1".into(),
                        span: None,
                    },
                    similarity_score: 0.9,
                    grounded: true,
                    supporting_doc_index: Some(0),
                },
                ScoredClaim {
                    claim: Claim {
                        text: "claim2".into(),
                        span: None,
                    },
                    similarity_score: 0.85,
                    grounded: true,
                    supporting_doc_index: Some(0),
                },
            ],
            grounded_count: 2,
            hallucinated_count: 0,
            groundedness_score: 1.0,
            hallucination_rate: 0.0,
            pass: true,
        };

        assert_eq!(result.groundedness_score, 1.0);
        assert_eq!(result.hallucination_rate, 0.0);
        assert!(result.pass);
    }

    #[test]
    fn test_groundedness_half_claims_grounded() {
        let result = GroundednessResult {
            claims: vec![
                ScoredClaim {
                    claim: Claim {
                        text: "claim1".into(),
                        span: None,
                    },
                    similarity_score: 0.9,
                    grounded: true,
                    supporting_doc_index: Some(0),
                },
                ScoredClaim {
                    claim: Claim {
                        text: "claim2".into(),
                        span: None,
                    },
                    similarity_score: 0.1,
                    grounded: false,
                    supporting_doc_index: None,
                },
            ],
            grounded_count: 1,
            hallucinated_count: 1,
            groundedness_score: 0.5,
            hallucination_rate: 0.5,
            pass: false,
        };

        assert_eq!(result.groundedness_score, 0.5);
        assert_eq!(result.hallucination_rate, 0.5);
        assert!(!result.pass);
    }

    #[test]
    fn test_groundedness_no_claims() {
        let result = GroundednessResult {
            claims: vec![],
            grounded_count: 0,
            hallucinated_count: 0,
            groundedness_score: 1.0,  // No claims = no hallucinations
            hallucination_rate: 0.0,
            pass: true,
        };

        assert_eq!(result.groundedness_score, 1.0);
        assert!(result.pass);
    }

    #[test]
    fn test_scored_claim_structure() {
        let claim = ScoredClaim {
            claim: Claim {
                text: "Exercise improves health".into(),
                span: Some((0, 20)),
            },
            similarity_score: 0.85,
            grounded: true,
            supporting_doc_index: Some(0),
        };

        assert_eq!(claim.claim.text, "Exercise improves health");
        assert!(claim.grounded);
        assert_eq!(claim.similarity_score, 0.85);
        assert_eq!(claim.supporting_doc_index, Some(0));
    }

    #[test]
    fn test_document_structure() {
        let doc = Document {
            text: "Exercise strengthens the heart".into(),
            source: "health-guide.pdf".into(),
        };

        assert_eq!(doc.source, "health-guide.pdf");
        assert!(doc.text.contains("heart"));
    }

    #[test]
    fn test_claim_structure() {
        let claim = Claim {
            text: "Exercise is healthy".into(),
            span: Some((0, 18)),
        };

        assert_eq!(claim.text, "Exercise is healthy");
        assert_eq!(claim.span, Some((0, 18)));
    }

    #[test]
    fn test_hallucination_extraction_methods_serde() {
        use serde_json;

        let methods = vec![
            HallucinationExtractionMethod::RuleBased,
            HallucinationExtractionMethod::LlmBased,
        ];

        for method in methods {
            let json = serde_json::to_string(&method).unwrap();
            let deserialized: HallucinationExtractionMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(method, deserialized);
        }
    }

    #[test]
    fn test_hallucination_scoring_methods_serde() {
        use serde_json;

        let methods = vec![
            HallucinationScoringMethod::TokenOverlap,
            HallucinationScoringMethod::EmbeddingSimilarity,
            HallucinationScoringMethod::TfIdf,
        ];

        for method in methods {
            let json = serde_json::to_string(&method).unwrap();
            let deserialized: HallucinationScoringMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(method, deserialized);
        }
    }

    #[test]
    fn test_hallucination_detection_config_default() {
        let config = HallucinationDetectionConfig::default();

        assert!(!config.enabled);
        assert_eq!(
            config.extraction_method,
            HallucinationExtractionMethod::RuleBased
        );
        assert_eq!(
            config.scoring_method,
            HallucinationScoringMethod::TokenOverlap
        );
        assert_eq!(config.groundedness_threshold, 0.5);
        assert_eq!(config.min_groundedness_score, 0.75);
    }

    #[test]
    fn test_hallucination_detection_config_custom() {
        let config = HallucinationDetectionConfig {
            enabled: true,
            extraction_method: HallucinationExtractionMethod::LlmBased,
            scoring_method: HallucinationScoringMethod::EmbeddingSimilarity,
            groundedness_threshold: 0.6,
            min_groundedness_score: 0.8,
        };

        assert!(config.enabled);
        assert_eq!(
            config.extraction_method,
            HallucinationExtractionMethod::LlmBased
        );
        assert_eq!(
            config.scoring_method,
            HallucinationScoringMethod::EmbeddingSimilarity
        );
        assert_eq!(config.groundedness_threshold, 0.6);
        assert_eq!(config.min_groundedness_score, 0.8);
    }

    #[test]
    fn test_realistic_exercise_example() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let response = "Exercise improves cardiovascular health and increases muscle strength.";
        let claims = evaluator.extract_claims_rule_based(response).unwrap();

        let _docs = vec![
            Document {
                text: "Exercise strengthens the cardiovascular system".into(),
                source: "cardio.pdf".into(),
            },
            Document {
                text: "Resistance training builds muscle".into(),
                source: "fitness.pdf".into(),
            },
        ];

        // Verify claims were extracted
        assert!(claims.len() >= 1);
        assert!(claims.iter().any(|c| c.text.contains("cardiovascular")));
    }

    #[test]
    fn test_real_world_hallucination_scenario() {
        // Medical claim that should be grounded
        let correct_claim = "Vitamin C supports immune function";
        let doc = "Vitamin C plays a role in immune function";

        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let score = evaluator.token_overlap(correct_claim, doc);
        assert!(score > 0.5, "Medical claim should be grounded");

        // Hallucinated claim
        let hallucinated = "Vitamin C cures cancer";
        let score = evaluator.token_overlap(hallucinated, doc);
        assert!(score <= 0.5, "Hallucination should not be grounded");
    }
}
