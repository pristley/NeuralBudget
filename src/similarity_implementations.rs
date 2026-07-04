use crate::{Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::similarity_traits::SimilarityScorer;

/// A tiny example scorer used for tests and examples.
pub struct DummyScorer {
    version: Option<String>,
    cache_hits: AtomicU64,
}

impl DummyScorer {
    pub fn new() -> Self {
        Self {
            version: Some("0.1.0".to_string()),
            cache_hits: AtomicU64::new(0),
        }
    }
}

impl SimilarityScorer for DummyScorer {
    fn score(&self, reference: &str, generated: &str) -> Result<f64> {
        // trivial scoring: exact matches -> 1.0, otherwise 0.5
        if reference == generated {
            Ok(1.0)
        } else {
            Ok(0.5)
        }
    }

    fn batch_score(&self, pairs: Vec<(&str, &str)>) -> Result<Vec<f64>> {
        let mut out = Vec::with_capacity(pairs.len());
        for (r, g) in pairs {
            out.push(self.score(r, g)?);
        }
        Ok(out)
    }

    fn name(&self) -> &'static str {
        "dummy"
    }

    fn model_version(&self) -> Option<String> {
        self.version.clone()
    }

    fn cache_hits(&self) -> Option<u64> {
        Some(self.cache_hits.load(Ordering::SeqCst))
    }
}

pub fn arc_dummy() -> Arc<dyn SimilarityScorer> {
    Arc::new(DummyScorer::new())
}
