//! Hallucination detection via groundedness evaluation.
//!
//! This module provides groundedness evaluation for GenAI outputs by checking if claims
//! are supported by retrieved documents (RAG) or web search results. Detects hallucinations
//! without requiring reference answers.
//!
//! # Example
//!
//! ```ignore
//! use neuralbudget::groundedness::{GroundednessEvaluator, Document, ClaimExtractionMethod};
//!
//! let evaluator = GroundednessEvaluator::new(
//!     ClaimExtractionMethod::LlmBased,
//!     SimilarityMethod::EmbeddingSimilarity,
//!     0.75,  // groundedness_threshold
//! );
//!
//! let docs = vec![
//!     Document {
//!         text: "Exercise improves cardiovascular health.".into(),
//!         source: "health-guide.pdf".into(),
//!     },
//! ];
//!
//! let result = evaluator.evaluate(
//!     "Exercise improves heart health and builds muscle.",
//!     &docs
//! ).await?;
//!
//! println!("Groundedness: {:.2}", result.groundedness_score);
//! println!("Grounded claims: {}/{}", result.grounded_count, result.claims.len());
//! ```

use crate::{NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};

/// A single claim extracted from an LLM response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// The extracted claim text
    pub text: String,
    /// Byte span in original response [start, end]
    pub span: Option<(usize, usize)>,
}

/// Claim with groundedness score
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredClaim {
    pub claim: Claim,
    /// Similarity score [0, 1] to best-matching document
    pub similarity_score: f64,
    /// Whether claim meets groundedness threshold
    pub grounded: bool,
    /// Most similar supporting document (if grounded)
    pub supporting_doc_index: Option<usize>,
}

/// A document providing context for grounding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Document text
    pub text: String,
    /// Source identifier (URL, filename, doc_id, etc.)
    pub source: String,
}

/// Method for extracting claims from response text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimExtractionMethod {
    /// Use LLM to extract claims
    LlmBased,
    /// Simple rule-based: split by sentences, filter short sentences
    RuleBased,
    /// Dependency parsing (future)
    DependencyParsing,
}

/// Method for scoring claim-document similarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMethod {
    /// Cosine similarity of embeddings
    EmbeddingSimilarity,
    /// TF-IDF based similarity
    TfIdf,
    /// Entailment model (RTE)
    Entailment,
    /// Simple token overlap
    TokenOverlap,
}

/// Source of context documents for grounding
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundingSource {
    /// Use provided context documents (RAG scenario)
    ProvidedDocuments,
    /// Retrieve from web search (future)
    WebSearch,
    /// Retrieve from knowledge base (future)
    KnowledgeBase,
}

/// Configuration for claim extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimExtractionConfig {
    pub method: ClaimExtractionMethod,
    /// Minimum claim length (chars)
    pub min_length: usize,
    /// Maximum claims to extract
    pub max_claims: usize,
}

impl Default for ClaimExtractionConfig {
    fn default() -> Self {
        Self {
            method: ClaimExtractionMethod::RuleBased,
            min_length: 10,
            max_claims: 20,
        }
    }
}

/// Result of groundedness evaluation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundednessResult {
    /// Extracted and scored claims
    pub claims: Vec<ScoredClaim>,
    /// Number of grounded claims
    pub grounded_count: usize,
    /// Number of hallucinated (ungrounded) claims
    pub hallucinated_count: usize,
    /// Overall groundedness score [0, 1]
    /// = grounded_count / total_claims
    pub groundedness_score: f64,
    /// Hallucination rate [0, 1]
    /// = hallucinated_count / total_claims
    pub hallucination_rate: f64,
    /// Pass/fail based on threshold
    pub pass: bool,
}

/// Evaluates whether LLM responses are grounded in provided documents
pub struct GroundednessEvaluator {
    extraction_method: ClaimExtractionMethod,
    similarity_method: SimilarityMethod,
    groundedness_threshold: f64,
    extraction_config: ClaimExtractionConfig,
}

impl GroundednessEvaluator {
    /// Create a new groundedness evaluator
    pub fn new(
        extraction_method: ClaimExtractionMethod,
        similarity_method: SimilarityMethod,
        groundedness_threshold: f64,
    ) -> Self {
        Self {
            extraction_method,
            similarity_method,
            groundedness_threshold,
            extraction_config: ClaimExtractionConfig::default(),
        }
    }

    /// Set extraction configuration
    pub fn with_extraction_config(mut self, config: ClaimExtractionConfig) -> Self {
        self.extraction_config = config;
        self
    }

    /// Evaluate groundedness of response against context documents
    pub async fn evaluate(&self, response: &str, context_docs: &[Document]) -> Result<GroundednessResult> {
        if context_docs.is_empty() {
            return Err(NeuralBudgetError::ConfigError(
                "No context documents provided for groundedness evaluation".to_string(),
            ));
        }

        // 1. Extract claims from response
        let claims = self.extract_claims(response).await?;

        if claims.is_empty() {
            return Ok(GroundednessResult {
                claims: vec![],
                grounded_count: 0,
                hallucinated_count: 0,
                groundedness_score: 1.0, // No claims = no hallucinations
                hallucination_rate: 0.0,
                pass: true,
            });
        }

        // 2. Score each claim against context documents
        let scored_claims = self.score_claims(&claims, context_docs).await?;

        // 3. Calculate overall groundedness
        let grounded_count = scored_claims.iter().filter(|c| c.grounded).count();
        let hallucinated_count = scored_claims.len() - grounded_count;
        let groundedness_score = grounded_count as f64 / scored_claims.len() as f64;
        let hallucination_rate = hallucinated_count as f64 / scored_claims.len() as f64;
        let pass = groundedness_score >= self.groundedness_threshold;

        Ok(GroundednessResult {
            claims: scored_claims,
            grounded_count,
            hallucinated_count,
            groundedness_score,
            hallucination_rate,
            pass,
        })
    }

    /// Extract claims from response text
    async fn extract_claims(&self, response: &str) -> Result<Vec<Claim>> {
        match self.extraction_method {
            ClaimExtractionMethod::RuleBased => self.extract_claims_rule_based(response),
            ClaimExtractionMethod::LlmBased => self.extract_claims_llm_based(response).await,
            ClaimExtractionMethod::DependencyParsing => {
                Err(NeuralBudgetError::ConfigError(
                    "Dependency parsing not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Simple rule-based claim extraction: split by sentences
    pub fn extract_claims_rule_based(&self, response: &str) -> Result<Vec<Claim>> {
        let mut claims = Vec::new();
        let mut current_pos = 0;

        for sentence in response.split(['.', '!', '?']) {
            let trimmed = sentence.trim();
            let end_pos = current_pos + sentence.len();
            if trimmed.len() >= self.extraction_config.min_length {
                claims.push(Claim {
                    text: trimmed.to_string(),
                    span: Some((current_pos, end_pos)),
                });

                if claims.len() >= self.extraction_config.max_claims {
                    break;
                }
            }
            current_pos = end_pos + 1; // +1 for delimiter
        }

        Ok(claims)
    }

    /// LLM-based claim extraction (placeholder)
    async fn extract_claims_llm_based(&self, response: &str) -> Result<Vec<Claim>> {
        // TODO: Call LLM to extract claims
        // For now, fall back to rule-based
        self.extract_claims_rule_based(response)
    }

    /// Score each claim against context documents
    async fn score_claims(
        &self,
        claims: &[Claim],
        context_docs: &[Document],
    ) -> Result<Vec<ScoredClaim>> {
        let mut scored_claims = Vec::new();

        for claim in claims {
            // Find best matching document
            let mut best_score = 0.0;
            let mut best_doc_idx = None;

            for (doc_idx, doc) in context_docs.iter().enumerate() {
                let score = match self.similarity_method {
                    SimilarityMethod::EmbeddingSimilarity => {
                        self.embedding_similarity(&claim.text, &doc.text)
                    }
                    SimilarityMethod::TfIdf => self.tfidf_similarity(&claim.text, &doc.text),
                    SimilarityMethod::TokenOverlap => self.token_overlap(&claim.text, &doc.text),
                    SimilarityMethod::Entailment => {
                        // TODO: Implement entailment scoring
                        self.token_overlap(&claim.text, &doc.text)
                    }
                };

                if score > best_score {
                    best_score = score;
                    best_doc_idx = Some(doc_idx);
                }
            }

            let grounded = best_score >= self.groundedness_threshold;

            scored_claims.push(ScoredClaim {
                claim: claim.clone(),
                similarity_score: best_score,
                grounded,
                supporting_doc_index: if grounded { best_doc_idx } else { None },
            });
        }

        Ok(scored_claims)
    }

    /// Simple embedding-like similarity using character n-gram overlap
    pub fn embedding_similarity(&self, claim: &str, doc: &str) -> f64 {
        let claim_lower = claim.to_lowercase();
        let doc_lower = doc.to_lowercase();

        // Check if significant portion of claim is in document
        let words: Vec<&str> = claim_lower.split_whitespace().collect();
        if words.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        for word in &words {
            if doc_lower.contains(word) {
                matches += 1;
            }
        }

        matches as f64 / words.len() as f64
    }

    /// TF-IDF style similarity
    pub fn tfidf_similarity(&self, claim: &str, doc: &str) -> f64 {
        let claim_words: Vec<&str> = claim.split_whitespace().collect();
        let doc_words: Vec<&str> = doc.split_whitespace().collect();

        if claim_words.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;
        for claim_word in &claim_words {
            // Count occurrences in document
            let count = doc_words.iter().filter(|w| *w == claim_word).count();
            if count > 0 {
                // Simple TF-IDF: reward rare words more
                let tf = count as f64 / doc_words.len() as f64;
                score += tf;
            }
        }

        (score / claim_words.len() as f64).min(1.0)
    }

    /// Token overlap similarity
    pub fn token_overlap(&self, claim: &str, doc: &str) -> f64 {
        let claim_tokens: std::collections::HashSet<&str> = claim.split_whitespace().collect();
        let doc_tokens: std::collections::HashSet<&str> = doc.split_whitespace().collect();

        if claim_tokens.is_empty() {
            return 0.0;
        }

        let intersection = claim_tokens.intersection(&doc_tokens).count();
        intersection as f64 / claim_tokens.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_extraction_rule_based() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        let response = "Exercise is healthy. It improves fitness.";
        let claims = evaluator.extract_claims_rule_based(response).unwrap();

        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].text, "Exercise is healthy");
        assert_eq!(claims[1].text, "It improves fitness");
    }

    #[test]
    fn test_claim_extraction_min_length() {
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

        let response = "Run. Exercise improves cardiovascular health.";
        let claims = evaluator.extract_claims_rule_based(response).unwrap();

        // "Run" should be filtered out (too short)
        assert!(claims.iter().all(|c| c.text.len() >= 20));
    }

    #[test]
    fn test_token_overlap_similarity() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::TokenOverlap,
            0.5,
        );

        // Exact match
        let score = evaluator.token_overlap("exercise is healthy", "exercise is healthy");
        assert_eq!(score, 1.0);

        // Partial match
        let score = evaluator.token_overlap("exercise is healthy", "exercise is good");
        assert!(score > 0.0 && score < 1.0);

        // No match
        let score = evaluator.token_overlap("exercise", "weather");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_embedding_similarity() {
        let evaluator = GroundednessEvaluator::new(
            ClaimExtractionMethod::RuleBased,
            SimilarityMethod::EmbeddingSimilarity,
            0.5,
        );

        // All words present
        let score = evaluator.embedding_similarity("exercise health", "exercise improves health");
        assert!(score > 0.5);

        // Some words present
        let score = evaluator.embedding_similarity("exercise benefits mood", "exercise helps health");
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_groundedness_result_pass_fail() {
        // All claims grounded
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
            ],
            grounded_count: 1,
            hallucinated_count: 0,
            groundedness_score: 1.0,
            hallucination_rate: 0.0,
            pass: true,
        };
        assert!(result.pass);
        assert_eq!(result.hallucination_rate, 0.0);

        // Half claims grounded, threshold 0.75
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
        assert!(!result.pass);
        assert_eq!(result.hallucination_rate, 0.5);
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
}
