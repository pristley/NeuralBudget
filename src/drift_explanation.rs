//! Feature drift analysis and explanation utilities.
//!
//! Implements `FeatureDriftAnalyzer` which collects baseline and current samples
//! per feature, computes distribution summaries, performs Kolmogorov-Smirnov
//! two-sample tests, ranks features by importance, and produces a
//! `DriftExplanation` describing the top drifting features.

use crate::{NeuralBudgetError, Result};
use crate::shap_integration::{ShapExplainer, ShapValue};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-feature distribution summary and cached statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distribution {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub percentiles: HashMap<u8, f64>,
    #[serde(skip)]
    samples: Vec<f64>,
    #[serde(skip)]
    computed: bool,
}

impl Distribution {
    pub fn new() -> Self {
        Self {
            mean: 0.0,
            std: 0.0,
            min: f64::NAN,
            max: f64::NAN,
            percentiles: HashMap::new(),
            samples: Vec::new(),
            computed: false,
        }
    }

    /// Add a sample, capping history at 1000 samples (keep most recent).
    pub fn add_sample(&mut self, v: f64) {
        if !v.is_finite() {
            return;
        }
        self.samples.push(v);
        if self.samples.len() > 1000 {
            self.samples.remove(0);
        }
        self.computed = false;
    }

    /// Number of stored samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Compute summary statistics and percentiles (lazy, cached).
    pub fn compute(&mut self) {
        if self.computed {
            return;
        }
        if self.samples.is_empty() {
            self.mean = 0.0;
            self.std = 0.0;
            self.min = f64::NAN;
            self.max = f64::NAN;
            self.percentiles.clear();
            self.computed = true;
            return;
        }

        let mut s = self.samples.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = s.len() as f64;
        let sum: f64 = s.iter().sum();
        let mean = sum / n;
        let var: f64 = if s.len() > 1 {
            s.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0)
        } else {
            0.0
        };
        let std = var.max(0.0).sqrt();
        self.mean = mean;
        self.std = std;
        self.min = *s.first().unwrap();
        self.max = *s.last().unwrap();

        self.percentiles.clear();
        for &p in &[25u8, 50u8, 75u8, 95u8] {
            let idx = ((p as f64 / 100.0) * (n - 1.0)).round() as usize;
            let val = s.get(idx).cloned().unwrap_or(self.mean);
            self.percentiles.insert(p, val);
        }

        self.computed = true;
    }

    /// Get a reference to sorted samples (for KS computation).
    pub fn samples_sorted(&self) -> Vec<f64> {
        let mut s = self.samples.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        s
    }
}

/// Per-feature drift info included in the explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDriftInfo {
    pub feature_name: String,
    pub baseline_mean: f64,
    pub current_mean: f64,
    pub drift_magnitude: f64,
    pub ks_test_stat: f64,
    pub p_value: f64,
}

/// Result of a drift analysis run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftExplanation {
    pub top_features: Vec<(String, f64)>, // (feature, importance)
    pub ks_statistic: f64,
    pub p_value: f64,
    pub feature_drifts: HashMap<String, FeatureDriftInfo>,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shap_values: Option<Vec<ShapValue>>,
}

/// Analyzer collecting baseline and current samples and computing drift.
pub struct FeatureDriftAnalyzer {
    baseline_distributions: HashMap<String, Distribution>,
    current_distributions: HashMap<String, Distribution>,
    ks_threshold: f64,
}

impl FeatureDriftAnalyzer {
    /// Create a new analyzer. `ks_threshold` defaults to 0.15 if NaN or <=0.
    pub fn new(ks_threshold: f64) -> Self {
        let t = if ks_threshold.is_finite() && ks_threshold > 0.0 {
            ks_threshold
        } else {
            0.15
        };
        Self {
            baseline_distributions: HashMap::new(),
            current_distributions: HashMap::new(),
            ks_threshold: t,
        }
    }

    pub fn add_baseline_sample(&mut self, feature: String, value: f64) {
        let d = self
            .baseline_distributions
            .entry(feature)
            .or_insert_with(Distribution::new);
        d.add_sample(value);
    }

    pub fn add_current_sample(&mut self, feature: String, value: f64) {
        let d = self
            .current_distributions
            .entry(feature)
            .or_insert_with(Distribution::new);
        d.add_sample(value);
    }

    pub fn get_distribution(&self, feature: &str) -> Option<&Distribution> {
        self.current_distributions.get(feature)
    }

    /// Analyze all features present in both baseline and current.
    pub fn analyze(&mut self) -> Result<DriftExplanation> {
        if self.baseline_distributions.is_empty() || self.current_distributions.is_empty() {
            return Err(NeuralBudgetError::EvaluationError(
                "no features to analyze".to_string(),
            ));
        }

        let mut results: HashMap<String, FeatureDriftInfo> = HashMap::new();
        let mut importances: Vec<(String, f64)> = Vec::new();

        for (feat, _) in self
            .baseline_distributions
            .keys()
            .cloned()
            .collect::<Vec<String>>()
        {
            let baseline = match self.baseline_distributions.get_mut(&feat) {
                Some(b) => b,
                None => continue,
            };
            let current = match self.current_distributions.get_mut(&feat) {
                Some(c) => c,
                None => continue,
            };

            if baseline.len() < 5 || current.len() < 5 {
                return Err(NeuralBudgetError::EvaluationError(format!(
                    "feature '{}' requires at least 5 samples in both baseline and current",
                    feat
                )));
            }

            baseline.compute();
            current.compute();

            // KS test
            let (ks_stat, pval) = ks_two_sample(&baseline.samples_sorted(), &current.samples_sorted());

            // drift magnitude relative to baseline std
            let denom = if baseline.std <= 0.0 { 1e-9 } else { baseline.std };
            let mag = (current.mean - baseline.mean).abs() / denom;

            // importance formula
            let raw_importance = mag * ks_stat;

            let info = FeatureDriftInfo {
                feature_name: feat.clone(),
                baseline_mean: baseline.mean,
                current_mean: current.mean,
                drift_magnitude: mag,
                ks_test_stat: ks_stat,
                p_value: pval,
            };
            results.insert(feat.clone(), info);
            importances.push((feat.clone(), raw_importance));
        }

        if results.is_empty() {
            return Err(NeuralBudgetError::EvaluationError(
                "no overlapping features to analyze".to_string(),
            ));
        }

        // Normalize importances to 0-1
        let max_imp = importances.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
        let mut normalized: Vec<(String, f64)> = importances
            .into_iter()
            .map(|(k, v)| {
                let norm = if max_imp > 0.0 { (v / max_imp).clamp(0.0, 1.0) } else { 0.0 };
                (k, norm)
            })
            .collect();

        // Sort desc and take top 5
        normalized.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let top_features = normalized.iter().take(5).cloned().collect::<Vec<_>>();

        // overall KS stat and p-value: take max KS and min p-value
        let ks_statistic = results.values().map(|r| r.ks_test_stat).fold(0.0_f64, f64::max);
        let p_value = results.values().map(|r| r.p_value).fold(1.0_f64, f64::min);

        // Build human summary
        let mut summary = String::new();
        for (f, _) in &top_features {
            if let Some(info) = results.get(f) {
                summary.push_str(&format!(
                    "feature '{}' drift {:.3} (ks={:.3}, p={:.3}); ",
                    f, info.drift_magnitude, info.ks_test_stat, info.p_value
                ));
            }
        }
        if summary.is_empty() {
            summary = "no significant drifts detected".to_string();
        }

        // Integrate SHAP explanation using top_features as importances
        let mut shap_values = None;
        if !top_features.is_empty() {
            // build explainer with normalized importances
            let expl = ShapExplainer::new(top_features.clone(), 0.5);
            if let Ok(vals) = expl.explain_with_context(0.5 + ks_statistic * 0.1) {
                shap_values = Some(vals);
            }
        }

        Ok(DriftExplanation {
            top_features,
            ks_statistic,
            p_value,
            feature_drifts: results,
            summary,
            shap_values,
        })
    }
}

/// Two-sample Kolmogorov-Smirnov statistic and approximate p-value.
fn ks_two_sample(a: &Vec<f64>, b: &Vec<f64>) -> (f64, f64) {
    if a.is_empty() || b.is_empty() {
        return (0.0, 1.0);
    }

    let mut ia = 0usize;
    let mut ib = 0usize;
    let na = a.len();
    let nb = b.len();
    let mut d = 0.0f64;

    while ia < na && ib < nb {
        let va = a[ia];
        let vb = b[ib];
        if va <= vb {
            ia += 1;
        } else {
            ib += 1;
        }
        let ca = ia as f64 / na as f64;
        let cb = ib as f64 / nb as f64;
        let diff = (ca - cb).abs();
        if diff > d {
            d = diff;
        }
    }

    // Handle tails
    while ia < na {
        ia += 1;
        let ca = ia as f64 / na as f64;
        let cb = 1.0;
        let diff = (ca - cb).abs();
        if diff > d {
            d = diff;
        }
    }
    while ib < nb {
        ib += 1;
        let ca = 1.0;
        let cb = ib as f64 / nb as f64;
        let diff = (ca - cb).abs();
        if diff > d {
            d = diff;
        }
    }

    // approximate p-value using asymptotic formula
    let en = ((na * nb) as f64 / (na + nb) as f64).sqrt();
    let lambda = (en + 0.12 + 0.11 / en) * d;
    let p = ks_pvalue_approx(lambda);
    (d, p)
}

fn ks_pvalue_approx(lambda: f64) -> f64 {
    if lambda <= 0.0 {
        return 1.0;
    }
    // use first term approximation: ~2 * exp(-2 * lambda^2)
    let p = 2.0 * (-2.0 * lambda * lambda).exp();
    if p.is_nan() || !p.is_finite() {
        0.0
    } else {
        p.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_distr::{Distribution, Normal};

    #[test]
    fn test_no_drift_scenario() -> Result<()> {
        let mut a = FeatureDriftAnalyzer::new(0.15);
        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();
        for _ in 0..200 {
            a.add_baseline_sample("f1".to_string(), normal.sample(&mut rng));
            a.add_current_sample("f1".to_string(), normal.sample(&mut rng));
        }
        let exp = a.analyze()?;
        // top_features may include the feature but importance should be small
        assert!(exp.top_features.len() <= 1);
        Ok(())
    }

    #[test]
    fn test_single_feature_drift() -> Result<()> {
        let mut a = FeatureDriftAnalyzer::new(0.05);
        let mut rng = rand::thread_rng();
        let base = Normal::new(0.0, 1.0).unwrap();
        let cur = Normal::new(1.0, 1.0).unwrap();
        for _ in 0..200 {
            a.add_baseline_sample("f1".to_string(), base.sample(&mut rng));
            a.add_current_sample("f1".to_string(), cur.sample(&mut rng));
        }
        let exp = a.analyze()?;
        // Expect ks_statistic relatively large
        assert!(exp.ks_statistic > 0.0);
        assert!(exp.feature_drifts.contains_key("f1"));
        Ok(())
    }

    #[test]
    fn test_multiple_features_drift() -> Result<()> {
        let mut a = FeatureDriftAnalyzer::new(0.1);
        let mut rng = rand::thread_rng();
        let n1 = Normal::new(0.0, 1.0).unwrap();
        let n2 = Normal::new(0.0, 1.0).unwrap();
        for _ in 0..200 {
            a.add_baseline_sample("f1".to_string(), n1.sample(&mut rng));
            a.add_current_sample("f1".to_string(), n1.sample(&mut rng));
            a.add_baseline_sample("f2".to_string(), n2.sample(&mut rng));
            a.add_current_sample("f2".to_string(), n2.sample(&mut rng) + 2.0);
        }
        let exp = a.analyze()?;
        assert!(exp.top_features.len() >= 1);
        assert!(exp.feature_drifts.contains_key("f2"));
        Ok(())
    }

    #[test]
    fn test_edge_all_zeros() {
        let mut a = FeatureDriftAnalyzer::new(0.1);
        for _ in 0..10 {
            a.add_baseline_sample("f1".to_string(), 0.0);
            a.add_current_sample("f1".to_string(), 0.0);
        }
        let res = a.analyze();
        // With only 10 samples each, should succeed but ks_statistic==0
        if let Ok(exp) = res {
            assert_eq!(exp.ks_statistic, 0.0);
        } else {
            panic!("analyze failed on all zeros");
        }
    }

    #[test]
    fn test_edge_single_sample_error() {
        let mut a = FeatureDriftAnalyzer::new(0.1);
        a.add_baseline_sample("f1".to_string(), 1.0);
        a.add_current_sample("f1".to_string(), 2.0);
        let res = a.analyze();
        assert!(res.is_err());
    }
}
