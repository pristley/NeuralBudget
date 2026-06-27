//! LLM-as-Judge evaluator for reference-free quality SLOs.
//!
//! This module provides cached LLM-based evaluation of GenAI outputs without requiring
//! reference text. Evaluators score outputs on user-defined dimensions with deterministic
//! scoring and cost tracking.
//!
//! # Example
//!
//! ```ignore
//! use neuralbudget::genai_evaluator::{LlmJudgeEvaluator, LlmJudgeDimension, LlmProvider};
//!
//! let evaluator = LlmJudgeEvaluator::new(
//!     LlmProvider::OpenAI {
//!         api_key: "sk-...".to_string(),
//!         model: "gpt-4-mini".to_string(),
//!     },
//!     vec![
//!         LlmJudgeDimension {
//!             name: "correctness".to_string(),
//!             prompt: "Is this response correct? Score 1-5.".to_string(),
//!             weight: 0.5,
//!             threshold: 3.0,
//!             cost_per_call_usd: 0.0001,
//!         },
//!     ],
//! ).with_redis_cache("redis://localhost:6379", 3600)?;
//!
//! let result = evaluator.evaluate("What is 2+2?", "The answer is 4").await?;
//! println!("Score: {}", result.weighted_score);
//! println!("Cost: ${}", result.total_cost_usd);
//! ```

use crate::{NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmProvider {
    /// OpenAI API (GPT-4, GPT-4-mini)
    OpenAI { api_key: String, model: String },
    /// Anthropic API (Claude 3, etc.)
    Anthropic { api_key: String, model: String },
    /// Local model via LM Studio or similar
    Local { base_url: String, model: String },
}

impl LlmProvider {
    /// Get the model identifier
    pub fn model(&self) -> &str {
        match self {
            Self::OpenAI { model, .. } => model,
            Self::Anthropic { model, .. } => model,
            Self::Local { model, .. } => model,
        }
    }

    /// Get the API key (if applicable)
    pub fn api_key(&self) -> Option<&str> {
        match self {
            Self::OpenAI { api_key, .. } => Some(api_key),
            Self::Anthropic { api_key, .. } => Some(api_key),
            Self::Local { .. } => None,
        }
    }
}

/// Single evaluation dimension for LLM judge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmJudgeDimension {
    /// Dimension name (e.g., "correctness", "safety", "tone")
    pub name: String,
    /// Prompt template with {query} and {response} placeholders
    pub prompt: String,
    /// Weight in overall score calculation
    pub weight: f64,
    /// Minimum score to pass (1-5 typically)
    pub threshold: f64,
    /// Cost in USD per API call
    pub cost_per_call_usd: f64,
}

/// Redis cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub redis_url: String,
    pub ttl_seconds: u64,
}

/// Evaluation result for a single dimension
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionScore {
    pub name: String,
    pub score: f64,
    pub reasoning: Option<String>,
    pub pass: bool,
}

/// Overall evaluation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Timestamp of evaluation
    pub timestamp: i64,
    /// Cache key used for lookup
    pub cache_key: String,
    /// Whether result came from cache
    pub from_cache: bool,
    /// Individual dimension scores
    pub dimension_scores: Vec<DimensionScore>,
    /// Weighted overall score (0.0-1.0 normalized)
    pub weighted_score: f64,
    /// All dimensions passed their threshold
    pub pass: bool,
    /// Total cost in USD (0 if from cache)
    pub total_cost_usd: f64,
    /// Total tokens used (approximate)
    pub total_tokens: usize,
}

/// LLM-as-Judge evaluator
pub struct LlmJudgeEvaluator {
    pub provider: LlmProvider,
    pub dimensions: Vec<LlmJudgeDimension>,
    pub cache_config: Option<CacheConfig>,
    cache_client: Option<Arc<redis::aio::ConnectionManager>>,
    http_client: reqwest::Client,
}

impl LlmJudgeEvaluator {
    /// Create a new LLM judge evaluator
    pub fn new(provider: LlmProvider, dimensions: Vec<LlmJudgeDimension>) -> Self {
        Self {
            provider,
            dimensions,
            cache_config: None,
            cache_client: None,
            http_client: reqwest::Client::new(),
        }
    }

    /// Add Redis caching to the evaluator
    pub async fn with_redis_cache(mut self, redis_url: &str, ttl_seconds: u64) -> Result<Self> {
        let client = redis::Client::open(redis_url).map_err(|e| {
            NeuralBudgetError::ConfigError(format!("Redis connection failed: {}", e))
        })?;

        let manager = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| NeuralBudgetError::ConfigError(format!("Redis manager failed: {}", e)))?;

        self.cache_config = Some(CacheConfig {
            redis_url: redis_url.to_string(),
            ttl_seconds,
        });
        self.cache_client = Some(Arc::new(manager));

        Ok(self)
    }

    /// Generate a cache key from query and response
    pub fn generate_cache_key(query: &str, response: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hasher.update("|".as_bytes());
        hasher.update(response.as_bytes());
        format!("llm_judge:{:x}", hasher.finalize())
    }

    /// Check cache for existing evaluation
    async fn check_cache(&self, cache_key: &str) -> Result<Option<EvaluationResult>> {
        if let Some(manager) = &self.cache_client {
            let mut conn = manager.as_ref().clone();
            match redis::cmd("GET")
                .arg(cache_key)
                .query_async::<_, String>(&mut conn)
                .await
            {
                Ok(data) => {
                    if let Ok(result) = serde_json::from_str::<EvaluationResult>(&data) {
                        return Ok(Some(result));
                    }
                }
                Err(_) => {
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }

    /// Store evaluation result in cache
    async fn store_cache(&self, cache_key: &str, result: &EvaluationResult) -> Result<()> {
        if let Some(manager) = &self.cache_client {
            if let Some(config) = &self.cache_config {
                let json = serde_json::to_string(result).map_err(|e| {
                    NeuralBudgetError::FormatError(format!("JSON serialization failed: {}", e))
                })?;

                let mut conn = manager.as_ref().clone();
                redis::cmd("SET")
                    .arg(cache_key)
                    .arg(&json)
                    .arg("EX")
                    .arg(config.ttl_seconds)
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(|e| {
                        NeuralBudgetError::ConfigError(format!("Cache storage failed: {}", e))
                    })?;
            }
        }
        Ok(())
    }

    /// Evaluate a query-response pair
    pub async fn evaluate(&self, query: &str, response: &str) -> Result<EvaluationResult> {
        let cache_key = Self::generate_cache_key(query, response);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Check cache
        if let Some(mut cached) = self.check_cache(&cache_key).await? {
            cached.from_cache = true;
            return Ok(cached);
        }

        // Evaluate each dimension
        let mut dimension_scores = Vec::new();
        let mut total_cost = 0.0;
        let mut total_tokens = 0;
        let mut score_sum = 0.0;
        let mut weight_sum = 0.0;

        for dimension in &self.dimensions {
            // Replace placeholders in prompt
            let prompt = dimension
                .prompt
                .replace("{query}", query)
                .replace("{response}", response);

            // Call LLM
            let (score, tokens) = self.call_llm(&prompt, dimension).await?;

            total_cost += dimension.cost_per_call_usd;
            total_tokens += tokens;

            // Normalize score to 0-1
            let normalized_score = (score.clamp(1.0, 5.0) - 1.0) / 4.0;

            let pass = score >= dimension.threshold;
            dimension_scores.push(DimensionScore {
                name: dimension.name.clone(),
                score,
                reasoning: None,
                pass,
            });

            score_sum += normalized_score * dimension.weight;
            weight_sum += dimension.weight;
        }

        let weighted_score = if weight_sum > 0.0 {
            score_sum / weight_sum
        } else {
            0.0
        };

        let pass = dimension_scores.iter().all(|d| d.pass);

        let result = EvaluationResult {
            timestamp,
            cache_key: cache_key.clone(),
            from_cache: false,
            dimension_scores,
            weighted_score,
            pass,
            total_cost_usd: total_cost,
            total_tokens,
        };

        // Store in cache
        self.store_cache(&cache_key, &result).await.ok();

        Ok(result)
    }

    /// Call the LLM and extract score
    async fn call_llm(&self, prompt: &str, _dimension: &LlmJudgeDimension) -> Result<(f64, usize)> {
        match &self.provider {
            LlmProvider::OpenAI { api_key, model } => {
                self.call_openai(api_key, model, prompt).await
            }
            LlmProvider::Anthropic { api_key, model } => {
                self.call_anthropic(api_key, model, prompt).await
            }
            LlmProvider::Local { base_url, model } => {
                self.call_local(base_url, model, prompt).await
            }
        }
    }

    /// Call OpenAI API
    async fn call_openai(&self, api_key: &str, model: &str, prompt: &str) -> Result<(f64, usize)> {
        #[derive(Serialize)]
        struct OpenAIRequest {
            model: String,
            messages: Vec<OpenAIMessage>,
            temperature: f32,
            max_tokens: i32,
        }

        #[derive(Serialize, Deserialize)]
        struct OpenAIMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
            usage: OpenAIUsage,
        }

        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIMessage,
        }

        #[derive(Deserialize)]
        struct OpenAIUsage {
            total_tokens: usize,
        }

        let request = OpenAIRequest {
            model: model.to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
            max_tokens: 100,
        };

        let response = self
            .http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                NeuralBudgetError::EvaluationError(format!("OpenAI API call failed: {}", e))
            })?;

        let data: OpenAIResponse = response.json().await.map_err(|e| {
            NeuralBudgetError::FormatError(format!("Failed to parse OpenAI response: {}", e))
        })?;

        let content = &data.choices[0].message.content;
        let score = self.extract_score(content)?;

        Ok((score, data.usage.total_tokens))
    }

    /// Call Anthropic API
    async fn call_anthropic(
        &self,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Result<(f64, usize)> {
        #[derive(Serialize)]
        struct AnthropicRequest {
            model: String,
            max_tokens: i32,
            messages: Vec<AnthropicMessage>,
        }

        #[derive(Serialize)]
        struct AnthropicMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
            usage: AnthropicUsage,
        }

        #[derive(Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        #[derive(Deserialize)]
        struct AnthropicUsage {
            input_tokens: usize,
            output_tokens: usize,
        }

        let request = AnthropicRequest {
            model: model.to_string(),
            max_tokens: 100,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                NeuralBudgetError::EvaluationError(format!("Anthropic API call failed: {}", e))
            })?;

        let data: AnthropicResponse = response.json().await.map_err(|e| {
            NeuralBudgetError::FormatError(format!("Failed to parse Anthropic response: {}", e))
        })?;

        let content = &data.content[0].text;
        let score = self.extract_score(content)?;
        let total_tokens = data.usage.input_tokens + data.usage.output_tokens;

        Ok((score, total_tokens))
    }

    /// Call local LLM endpoint
    async fn call_local(&self, base_url: &str, model: &str, prompt: &str) -> Result<(f64, usize)> {
        #[derive(Serialize)]
        struct LocalRequest {
            model: String,
            prompt: String,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct LocalResponse {
            response: String,
        }

        let request = LocalRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let url = format!("{}/api/generate", base_url);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                NeuralBudgetError::EvaluationError(format!("Local LLM API call failed: {}", e))
            })?;

        let data: LocalResponse = response.json().await.map_err(|e| {
            NeuralBudgetError::FormatError(format!("Failed to parse local LLM response: {}", e))
        })?;

        let score = self.extract_score(&data.response)?;

        // Estimate tokens
        let tokens = (data.response.len() / 4).max(1);

        Ok((score, tokens))
    }

    /// Extract numeric score from LLM response
    fn extract_score(&self, response: &str) -> Result<f64> {
        // Try to find a number between 1 and 5
        for word in response.split_whitespace() {
            if let Ok(score) = word.parse::<f64>() {
                if (1.0..=5.0).contains(&score) {
                    return Ok(score);
                }
            }
        }

        // Try to find a number anywhere in the response
        for ch in response.chars() {
            if let Ok(score) = ch.to_string().parse::<f64>() {
                if (1.0..=5.0).contains(&score) {
                    return Ok(score);
                }
            }
        }

        Err(NeuralBudgetError::EvaluationError(
            "Could not extract score from LLM response".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key1 = LlmJudgeEvaluator::generate_cache_key("query1", "response1");
        let key2 = LlmJudgeEvaluator::generate_cache_key("query1", "response1");
        let key3 = LlmJudgeEvaluator::generate_cache_key("query1", "response2");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert!(key1.starts_with("llm_judge:"));
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
    }

    #[test]
    fn test_score_extraction() {
        let evaluator = LlmJudgeEvaluator::new(
            LlmProvider::Local {
                base_url: "http://localhost:11434".to_string(),
                model: "llama2".to_string(),
            },
            vec![],
        );

        assert_eq!(evaluator.extract_score("Score: 4").unwrap(), 4.0);
        assert_eq!(
            evaluator.extract_score("I give this a 3 out of 5").unwrap(),
            3.0
        );
        assert!(evaluator.extract_score("no score here").is_err());
    }

    #[test]
    fn test_llm_provider_model() {
        let provider = LlmProvider::OpenAI {
            api_key: "key".to_string(),
            model: "gpt-4-mini".to_string(),
        };
        assert_eq!(provider.model(), "gpt-4-mini");
        assert_eq!(provider.api_key(), Some("key"));
    }
}
