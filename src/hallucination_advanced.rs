//! Advanced hallucination detection utilities.
//!
//! Provides lightweight, rule-based approximations for entailment, contradiction,
//! and ungroundedness detection suitable for integration into SLO pipelines.

use crate::{NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Types of hallucination detected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HallucinationScore {
    Entailment,
    Contradiction,
    Ungroundedness,
    HighConfidence,
    FactuallyCorrect,
}

/// Result metrics from hallucination analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationMetrics {
    pub factual_consistency: f64, // 0.0..1.0
    pub hallucination_types: Vec<HallucinationScore>,
    pub confidence: f64, // 0.0..1.0
    pub explanation: String,
    pub conflicting_entities: Vec<String>,
}

impl HallucinationMetrics {
    pub fn new() -> Self {
        Self {
            factual_consistency: 1.0,
            hallucination_types: Vec::new(),
            confidence: 1.0,
            explanation: String::new(),
            conflicting_entities: Vec::new(),
        }
    }
}

/// Hallucination detector with optional RAG context and entity extractor.
pub struct HallucinationDetector {
    reference: String,
    rag_context: Option<String>,
    entity_extractor: Option<Box<dyn Fn(&str) -> Vec<String>>>,
}

impl HallucinationDetector {
    /// Create a new detector with a reference text.
    pub fn new(reference: &str) -> Self {
        Self {
            reference: reference.to_string(),
            rag_context: None,
            entity_extractor: None,
        }
    }

    /// Set RAG context used for grounding checks.
    pub fn set_rag_context(&mut self, ctx: &str) {
        self.rag_context = Some(ctx.to_string());
    }

    /// Set a custom entity extractor function. If not set, a basic heuristic is used.
    pub fn set_entity_extractor<F>(&mut self, f: F)
    where
        F: 'static + Fn(&str) -> Vec<String>,
    {
        self.entity_extractor = Some(Box::new(f));
    }

    /// Analyze a generated text and produce hallucination metrics.
    pub fn analyze(&self, generated: &str) -> Result<HallucinationMetrics> {
        if self.reference.trim().is_empty() {
            return Err(NeuralBudgetError::ConfigError(
                "reference text required for analysis".to_string(),
            ));
        }

        let mut metrics = HallucinationMetrics::new();

        // extract entities
        let gen_entities = self.extract_entities(generated);
        let ref_entities = self.extract_entities(&self.reference);
        let rag_entities = if let Some(ctx) = &self.rag_context {
            self.extract_entities(ctx)
        } else {
            Vec::new()
        };

        // contradictions
        let contradictions = self.detect_contradictions(&ref_entities, &gen_entities, generated);
        if !contradictions.is_empty() {
            metrics.hallucination_types.push(HallucinationScore::Contradiction);
            metrics.conflicting_entities = contradictions.clone();
        }

        // ungroundedness: entities in generated not present in ref or RAG
        let missing = self.detect_ungroundedness(&gen_entities, &ref_entities, &rag_entities);
        if !missing.is_empty() {
            metrics.hallucination_types.push(HallucinationScore::Ungroundedness);
        }

        // entailment approximation
        let entailment_score = self.score_entailment(&ref_entities, &gen_entities, generated);
        if entailment_score > 0.7 && contradictions.is_empty() && missing.is_empty() {
            metrics.hallucination_types.push(HallucinationScore::Entailment);
        }

        // high confidence heuristic: if text contains hedging words, reduce confidence
        let mut confidence = 1.0_f64;
        if generated.to_lowercase().contains("i think")
            || generated.to_lowercase().contains("maybe")
            || generated.to_lowercase().contains("might")
        {
            confidence *= 0.8;
        }

        // compute hallucination score: contradictions penalize more than ungroundedness
        let contradiction_penalty = (metrics.conflicting_entities.len() as f64) * 0.7;
        let ungrounded_penalty = (missing.len() as f64) * 0.3;
        let raw_penalty = (contradiction_penalty + ungrounded_penalty).min(1.0);
        let factual_consistency = ((1.0 - raw_penalty) * confidence).clamp(0.0, 1.0);

        // explanation
        let mut explanation = Vec::new();
        if !metrics.conflicting_entities.is_empty() {
            explanation.push(format!(
                "Contradictions on: {}",
                metrics.conflicting_entities.join(", ")
            ));
        }
        if !missing.is_empty() {
            explanation.push(format!("Ungrounded entities: {}", missing.join(", ")));
        }
        if explanation.is_empty() {
            explanation.push("No hallucinations detected".to_string());
            metrics.hallucination_types.push(HallucinationScore::FactuallyCorrect);
        }

        metrics.factual_consistency = factual_consistency;
        metrics.confidence = confidence;
        metrics.explanation = explanation.join("; ");

        Ok(metrics)
    }

    /// Detect contradictions between reference entities and generated entities.
    fn detect_contradictions(&self, ref_entities: &[String], gen_entities: &[String], generated_text: &str) -> Vec<String> {
        let mut conflicts = Vec::new();
        let negations = vec!["not", "no", "never", "doesn't", "does not", "didn't", "without"];
        let gen_lower = generated_text.to_lowercase();
        let ref_set: HashSet<String> = ref_entities.iter().map(|s| s.to_lowercase()).collect();

        for ge in gen_entities {
            let gl = ge.to_lowercase();
            if ref_set.contains(&gl) {
                // look for negation near entity in generated text
                for neg in &negations {
                    let pattern1 = format!("{} {}", neg, gl);
                    let pattern2 = format!("{} {}", gl, neg);
                    if gen_lower.contains(&pattern1) || gen_lower.contains(&pattern2) {
                        conflicts.push(ge.clone());
                        break;
                    }
                }
            }
        }
        conflicts
    }

    /// Detect ungrounded entities present in generated but not in reference or RAG context.
    fn detect_ungroundedness(&self, gen_entities: &[String], ref_entities: &[String], rag_entities: &[String]) -> Vec<String> {
        let ref_set: HashSet<String> = ref_entities.iter().map(|s| s.to_lowercase()).collect();
        let rag_set: HashSet<String> = rag_entities.iter().map(|s| s.to_lowercase()).collect();
        let mut missing = Vec::new();
        for ge in gen_entities {
            let gl = ge.to_lowercase();
            if !ref_set.contains(&gl) && !rag_set.contains(&gl) {
                missing.push(ge.clone());
            }
        }
        missing
    }

    /// Approximate entailment score between reference and generated.
    fn score_entailment(&self, ref_entities: &[String], gen_entities: &[String], generated_text: &str) -> f64 {
        let ref_set: HashSet<String> = ref_entities.iter().map(|s| s.to_lowercase()).collect();
        let gen_set: HashSet<String> = gen_entities.iter().map(|s| s.to_lowercase()).collect();
        let common = ref_set.intersection(&gen_set).count() as f64;
        let ref_count = ref_set.len() as f64;
        let entity_score = if ref_count > 0.0 { common / ref_count } else { 0.0 };

        // token overlap
        let ref_tokens: HashSet<String> = self.token_set(&self.reference);
        let gen_tokens: HashSet<String> = self.token_set(generated_text);
        let inter = ref_tokens.intersection(&gen_tokens).count() as f64;
        let union = ref_tokens.union(&gen_tokens).count() as f64;
        let token_overlap = if union > 0.0 { inter / union } else { 0.0 };

        // weighted combination
        (0.6 * entity_score) + (0.4 * token_overlap)
    }

    /// Extract entities using either the provided extractor or a simple heuristic.
    fn extract_entities(&self, text: &str) -> Vec<String> {
        if let Some(extractor) = &self.entity_extractor {
            return extractor(text);
        }
        // naive extractor: words starting with uppercase and length>1
        text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .filter(|w| {
                w.chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            })
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .collect()
    }

    fn token_set(&self, text: &str) -> HashSet<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entailment_all_facts_present() {
        let ref_text = "Paris is the capital of France. The Eiffel Tower is in Paris.";
        let gen = "The Eiffel Tower stands in Paris, the capital of France.";
        let d = HallucinationDetector::new(ref_text);
        let m = d.analyze(gen).unwrap();
        assert!(m.hallucination_types.contains(&HallucinationScore::Entailment));
        assert!(m.factual_consistency > 0.5);
    }

    #[test]
    fn test_contradiction_detection() {
        let ref_text = "The patient has diabetes.";
        let gen = "The patient does not have diabetes.";
        let d = HallucinationDetector::new(ref_text);
        let m = d.analyze(gen).unwrap();
        assert!(m.hallucination_types.contains(&HallucinationScore::Contradiction));
        assert!(m.conflicting_entities.contains(&"diabetes".to_string()) || !m.conflicting_entities.is_empty());
    }

    #[test]
    fn test_ungroundedness_detection() {
        let ref_text = "The company was founded in 1999 in Berlin.";
        let gen = "The company was founded in 1999 in Berlin. It operates offices in Reykjavik.";
        let mut d = HallucinationDetector::new(ref_text);
        d.set_rag_context("Berlin is the capital of Germany.");
        let m = d.analyze(gen).unwrap();
        assert!(m.hallucination_types.contains(&HallucinationScore::Ungroundedness));
    }

    #[test]
    fn test_empty_reference_error() {
        let d = HallucinationDetector::new("");
        let err = d.analyze("some text");
        assert!(err.is_err());
    }
}
