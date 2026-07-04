//! Similarity scorer trait and a global registry for scorer implementations.
//!
//! This module defines the `SimilarityScorer` trait and a thread-safe
//! `ScorerRegistry` that holds `Arc<dyn SimilarityScorer>` instances.
//!
//! Example:
//!
//! ```rust
//! use std::sync::Arc;
//! use neuralbudget::similarity_traits::{default_registry, ScorerRegistry, SimilarityScorer};
//! // let registry = default_registry();
//! // registry.register("bert".to_string(), Arc::new(BertScorer::new()))?;
//! ```

use crate::{NeuralBudgetError, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

/// Trait for a similarity scorer implementation.
pub trait SimilarityScorer: Send + Sync {
    /// Score a single reference/generated pair.
    fn score(&self, reference: &str, generated: &str) -> Result<f64>;

    /// Batch score multiple (reference, generated) pairs.
    fn batch_score(&self, pairs: Vec<(&str, &str)>) -> Result<Vec<f64>>;

    /// Name identifier for the scorer (static string).
    fn name(&self) -> &'static str;

    /// Optional model version string.
    fn model_version(&self) -> Option<String> {
        None
    }

    /// Optional cache hit metric if scorer implements caching.
    fn cache_hits(&self) -> Option<u64> {
        None
    }
}

/// Errors specific to scorer registry operations.
#[derive(Debug)]
pub enum ScorerRegistryError {
    DuplicateScorer(String),
    ScorerNotFound(String),
    InvalidDefault(String),
}

impl fmt::Display for ScorerRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateScorer(n) => write!(f, "scorer already registered: {}", n),
            Self::ScorerNotFound(n) => write!(f, "scorer not found: {}", n),
            Self::InvalidDefault(n) => write!(f, "invalid default scorer: {}", n),
        }
    }
}

impl std::error::Error for ScorerRegistryError {}

impl From<ScorerRegistryError> for NeuralBudgetError {
    fn from(e: ScorerRegistryError) -> Self {
        NeuralBudgetError::EvaluationError(format!("ScorerRegistry: {}", e))
    }
}

/// Thread-safe registry of scorers with a default scorer name.
pub struct ScorerRegistry {
    scorers: RwLock<HashMap<String, Arc<dyn SimilarityScorer>>>,
    default_scorer: RwLock<String>,
}

impl ScorerRegistry {
    /// Create a new empty registry with a default name.
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            scorers: RwLock::new(HashMap::new()),
            default_scorer: RwLock::new(default.into()),
        }
    }

    /// Register a scorer under `name`. Returns DuplicateScorer if present.
    pub fn register(&self, name: String, scorer: Arc<dyn SimilarityScorer>) -> Result<()> {
        let mut map = self.scorers.write().map_err(|e| NeuralBudgetError::EvaluationError(format!("lock poisoned: {}", e)))?;
        if map.contains_key(&name) {
            return Err(ScorerRegistryError::DuplicateScorer(name).into());
        }
        map.insert(name, scorer);
        Ok(())
    }

    /// Get a scorer by name.
    pub fn get_scorer(&self, name: &str) -> Result<Arc<dyn SimilarityScorer>> {
        let map = self.scorers.read().map_err(|e| NeuralBudgetError::EvaluationError(format!("lock poisoned: {}", e)))?;
        map.get(name)
            .cloned()
            .ok_or_else(|| ScorerRegistryError::ScorerNotFound(name.to_string()).into())
    }

    /// List registered scorer names.
    pub fn list_scorers(&self) -> Vec<String> {
        if let Ok(map) = self.scorers.read() {
            map.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Remove a scorer by name.
    pub fn remove_scorer(&self, name: &str) -> Result<()> {
        let mut map = self.scorers.write().map_err(|e| NeuralBudgetError::EvaluationError(format!("lock poisoned: {}", e)))?;
        if map.remove(name).is_some() {
            Ok(())
        } else {
            Err(ScorerRegistryError::ScorerNotFound(name.to_string()).into())
        }
    }

    /// Set default scorer name; returns error if scorer not registered.
    pub fn set_default(&self, name: &str) -> Result<()> {
        // validate existence
        {
            let map = self.scorers.read().map_err(|e| NeuralBudgetError::EvaluationError(format!("lock poisoned: {}", e)))?;
            if !map.contains_key(name) {
                return Err(ScorerRegistryError::InvalidDefault(name.to_string()).into());
            }
        }
        let mut def = self.default_scorer.write().map_err(|e| NeuralBudgetError::EvaluationError(format!("lock poisoned: {}", e)))?;
        *def = name.to_string();
        Ok(())
    }

    /// Get the current default scorer name.
    pub fn default_name(&self) -> String {
        self.default_scorer
            .read()
            .map(|r| r.clone())
            .unwrap_or_else(|_| "".to_string())
    }
}

/// Global default registry instance.
pub static DEFAULT_REGISTRY: Lazy<ScorerRegistry> = Lazy::new(|| ScorerRegistry::new("default"));

/// Return a static reference to the default registry.
pub fn default_registry() -> &'static ScorerRegistry {
    &DEFAULT_REGISTRY
}
