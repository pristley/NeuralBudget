//! Lightweight SHAP approximation (no external ML libraries).
//!
//! Provides `ShapExplainer` and `ShapValue` which compute simple additive
//! attributions from feature importances and a baseline value.

use crate::{NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShapValue {
    pub feature: String,
    pub value: f64,
    pub confidence: f64,
}

pub struct ShapExplainer {
    pub feature_importances: Vec<(String, f64)>,
    pub baseline_value: f64,
}

impl ShapExplainer {
    pub fn new(importances: Vec<(String, f64)>, baseline: f64) -> Self {
        Self {
            feature_importances: importances,
            baseline_value: baseline,
        }
    }

    /// Explain with implicit prediction = baseline_value (returns zeros)
    pub fn explain(&self) -> Result<Vec<ShapValue>> {
        self.explain_with_context(self.baseline_value)
    }

    pub fn explain_with_context(&self, prediction: f64) -> Result<Vec<ShapValue>> {
        if self.feature_importances.is_empty() {
            return Ok(Vec::new());
        }

        // validate importances and sum
        let mut sum = 0.0f64;
        for (_, v) in &self.feature_importances {
            if !v.is_finite() {
                return Err(NeuralBudgetError::EvaluationError("importance contains non-finite value".to_string()));
            }
            sum += *v;
        }
        if sum.abs() < 1e-12 {
            return Err(NeuralBudgetError::EvaluationError("sum of importances is zero".to_string()));
        }

        let delta = prediction - self.baseline_value;

        // compute variance of importances for uncertainty estimate
        let mean_imp = sum / (self.feature_importances.len() as f64);
        let var_imp = self
            .feature_importances
            .iter()
            .map(|(_, v)| (v - mean_imp) * (v - mean_imp))
            .sum::<f64>()
            / (self.feature_importances.len() as f64);

        let mut out: Vec<ShapValue> = Vec::new();
        for (feat, imp) in &self.feature_importances {
            let rel = imp / sum;
            let contribution = rel * delta;
            // uncertainty increases with variance and decreases with importance
            let uncertainty = (var_imp.sqrt()) * (1.0 - (imp.abs() / (sum.abs()))).abs();
            let confidence = 1.0 / (1.0 + uncertainty);
            let conf = if !confidence.is_finite() { 0.85 } else { confidence.clamp(0.0, 1.0) };
            out.push(ShapValue {
                feature: feat.clone(),
                value: contribution,
                confidence: conf.max(0.01), // avoid zero
            });
        }

        // sort by absolute contribution desc
        out.sort_by(|a, b| b.value.abs().partial_cmp(&a.value.abs()).unwrap());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shap_sums_to_delta() -> Result<()> {
        let imps = vec![("f1".to_string(), 0.6), ("f2".to_string(), 0.3), ("f3".to_string(), 0.1)];
        let expl = ShapExplainer::new(imps, 0.5);
        let vals = expl.explain_with_context(0.8)?;
        let sum: f64 = vals.iter().map(|v| v.value).sum();
        assert!((sum - 0.3).abs() < 1e-8);
        Ok(())
    }

    #[test]
    fn test_uniform_importances() -> Result<()> {
        let imps = vec![("a".to_string(), 1.0), ("b".to_string(), 1.0), ("c".to_string(), 1.0)];
        let expl = ShapExplainer::new(imps, 0.0);
        let vals = expl.explain_with_context(0.9)?;
        // each should be roughly 0.3
        for v in vals {
            assert!((v.value - 0.3).abs() < 1e-8);
        }
        Ok(())
    }

    #[test]
    fn test_zero_importance_error() {
        let imps = vec![("a".to_string(), 0.0), ("b".to_string(), 0.0)];
        let expl = ShapExplainer::new(imps, 0.5);
        let res = expl.explain_with_context(0.6);
        assert!(res.is_err());
    }
}
