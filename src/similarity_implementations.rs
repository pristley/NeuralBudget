use crate::Result;
use log::{debug, warn};
use lru::LruCache;
use reqwest::blocking::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::similarity_traits::SimilarityScorer;

fn hash_to_string(a: &str, b: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(a.as_bytes());
    hasher.update(b.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Stateless cosine-like scorer using simple word-overlap vectors.
pub struct CosineSimScorer;

impl CosineSimScorer {
    pub fn new() -> Self {
        Self {}
    }

    fn tokenize(s: &str) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        for w in s
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
        {
            *m.entry(w.to_lowercase()).or_insert(0.0) += 1.0;
        }
        m
    }

    fn cosine(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
        let mut dot = 0.0;
        let mut na = 0.0;
        let mut nb = 0.0;
        for (k, va) in a.iter() {
            na += va * va;
            if let Some(vb) = b.get(k) {
                dot += va * vb;
            }
        }
        for vb in b.values() {
            nb += vb * vb;
        }
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na.sqrt() * nb.sqrt())
        }
    }
}

impl SimilarityScorer for CosineSimScorer {
    fn score(&self, reference: &str, generated: &str) -> Result<f64> {
        let a = CosineSimScorer::tokenize(reference);
        let b = CosineSimScorer::tokenize(generated);
        Ok(CosineSimScorer::cosine(&a, &b))
    }

    fn batch_score(&self, pairs: Vec<(&str, &str)>) -> Result<Vec<f64>> {
        let mut out = Vec::with_capacity(pairs.len());
        for (r, g) in pairs {
            out.push(self.score(r, g)?);
        }
        Ok(out)
    }

    fn name(&self) -> &'static str {
        "cosine_simple"
    }
}

impl Clone for CosineSimScorer {
    fn clone(&self) -> Self {
        Self::new()
    }
}

/// HTTP-backed scorer with LRU cache. Falls back to `CosineSimScorer` on errors.
pub struct BertScorer {
    pub http_endpoint: String,
    pub model_name: String,
    pub timeout_secs: u64,
    pub cache: Arc<Mutex<LruCache<String, f64>>>,
    pub cache_hits: AtomicU64,
    client: Client,
    fallback: CosineSimScorer,
}

impl BertScorer {
    pub fn new(endpoint: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        Self {
            http_endpoint: endpoint,
            model_name: model,
            timeout_secs: 5,
            cache: Arc::new(Mutex::new(LruCache::new(1000))),
            cache_hits: AtomicU64::new(0),
            client,
            fallback: CosineSimScorer::new(),
        }
    }

    fn fetch_similarity(&self, reference: &str, generated: &str) -> Result<f64> {
        let payload = serde_json::json!({"text1": reference, "text2": generated, "model": self.model_name});
        let resp = self.client.post(&self.http_endpoint).json(&payload).send();
        match resp {
            Ok(r) => {
                let v: Value = r.json().map_err(|e| crate::NeuralBudgetError::FormatError(format!("invalid json: {}", e)))?;
                if let Some(sim) = v.get("similarity") {
                    if let Some(f) = sim.as_f64() {
                        return Ok(f);
                    }
                }
                Err(crate::NeuralBudgetError::FormatError("missing similarity field".to_string()))
            }
            Err(e) => Err(crate::NeuralBudgetError::EvaluationError(format!("http error: {}", e))),
        }
    }
}

impl SimilarityScorer for BertScorer {
    fn score(&self, reference: &str, generated: &str) -> Result<f64> {
        let key = hash_to_string(reference, generated);
        if let Ok(mut c) = self.cache.lock() {
            if let Some(v) = c.get(&key) {
                self.cache_hits.fetch_add(1, Ordering::SeqCst);
                debug!("cache hit for key {}", key);
                return Ok(*v);
            }
        }

        match self.fetch_similarity(reference, generated) {
            Ok(sim) => {
                if let Ok(mut c) = self.cache.lock() {
                    c.put(key.clone(), sim);
                }
                Ok(sim)
            }
            Err(e) => {
                warn!("BertScorer http failed {}, falling back to cosine", e);
                let f = self.fallback.score(reference, generated)?;
                if let Ok(mut c) = self.cache.lock() {
                    c.put(key, f);
                }
                Ok(f)
            }
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
        "bert_scorer"
    }

    fn model_version(&self) -> Option<String> {
        Some(self.model_name.clone())
    }

    fn cache_hits(&self) -> Option<u64> {
        Some(self.cache_hits.load(Ordering::SeqCst))
    }
}

impl Clone for BertScorer {
    fn clone(&self) -> Self {
        Self {
            http_endpoint: self.http_endpoint.clone(),
            model_name: self.model_name.clone(),
            timeout_secs: self.timeout_secs,
            cache: Arc::clone(&self.cache),
            cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::SeqCst)),
            client: self.client.clone(),
            fallback: self.fallback.clone(),
        }
    }
}

/// RobertaScorer mirrors BertScorer but with a different default model name.
pub struct RobertaScorer {
    inner: BertScorer,
}

impl RobertaScorer {
    pub fn new(endpoint: String) -> Self {
        Self {
            inner: BertScorer::new(endpoint, "all-roberta-large-v1".to_string()),
        }
    }
}

impl SimilarityScorer for RobertaScorer {
    fn score(&self, reference: &str, generated: &str) -> Result<f64> {
        self.inner.score(reference, generated)
    }

    fn batch_score(&self, pairs: Vec<(&str, &str)>) -> Result<Vec<f64>> {
        self.inner.batch_score(pairs)
    }

    fn name(&self) -> &'static str {
        "roberta_scorer"
    }

    fn model_version(&self) -> Option<String> {
        self.inner.model_version()
    }

    fn cache_hits(&self) -> Option<u64> {
        self.inner.cache_hits()
    }
}

impl Clone for RobertaScorer {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Keep the previous DummyScorer available for examples/tests.
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;

    #[test]
    fn test_bert_cache_and_fallback() {
        let _m = mock("POST", "/embed")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"similarity": 0.85}"#)
            .create();

        let endpoint = format!("{}{}", &mockito::server_url(), "/embed");
        let scorer = BertScorer::new(endpoint, "mymodel".to_string());
        let res = scorer.score("hello world", "hello world").unwrap();
        assert!(res > 0.8 && res <= 1.0);

        // second call should hit cache
        let res2 = scorer.score("hello world", "hello world").unwrap();
        assert_eq!(res, res2);
    }

    #[test]
    fn test_fetch_failure_fallback() {
        // point to a non-routable address to force error and fallback
        let scorer = BertScorer::new("http://127.0.0.1:9/embed".to_string(), "mymodel".to_string());
        let res = scorer.score("a quick fox", "the quick fox").unwrap();
        // cosine fallback should return > 0
        assert!(res >= 0.0 && res <= 1.0);
    }
}
