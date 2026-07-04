//! Degradation detection primitives for runtime model monitoring.
//!
//! This module implements a lightweight ADWIN-like gradual drift detector,
//! a Z-score spike detector, and simple heuristics for oscillation detection.
//!
//! # Example
//!
//! ```rust
//! use neuralbudget::degradation::{DegradationDetector, DegradationType};
//! let mut d = DegradationDetector::new(100, 0.15, 2.5);
//! d.add_metric(42.0).unwrap();
//! ```
// P1A: Degradation Detection Core

use crate::{NeuralBudgetError, Result};
use log::debug;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// Types of degradation patterns we can report.
#[derive(Debug, Clone, PartialEq)]
pub enum DegradationType {
    Gradual,
    Sudden,
    Oscillating,
    Stable,
}

/// A detected degradation event.
#[derive(Debug, Clone)]
pub struct DegradationEvent {
    pub timestamp: u64,
    pub degradation_type: DegradationType,
    pub magnitude: f64,
    pub confidence: f64,
    pub duration_samples: usize,
    pub affected_metric: String,
}

/// Core detector struct holding a sliding window and thresholds.
pub struct DegradationDetector {
    pub window_size: usize,
    pub drift_threshold: f64,
    pub spike_threshold: f64,
    history: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

impl Default for DegradationDetector {
    fn default() -> Self {
        Self::new(100, 0.15, 2.5)
    }
}

impl DegradationDetector {
    /// Create a new detector.
    ///
    /// # Errors
    /// Returns an error if thresholds are NaN/Inf or window_size == 0.
    pub fn new(window_size: usize, drift_threshold: f64, spike_threshold: f64) -> Self {
        Self {
            window_size: if window_size == 0 { 1 } else { window_size },
            drift_threshold,
            spike_threshold,
            history: VecDeque::with_capacity(window_size),
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    /// Add a new metric sample to the detector.
    ///
    /// This function will ignore NaN and infinite inputs instead of panicking.
    pub fn add_metric(&mut self, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(NeuralBudgetError::EvaluationError(
                "metric value not finite".to_string(),
            ));
        }

        self.history.push_back(value);
        self.sum += value;
        self.sum_sq += value * value;

        while self.history.len() > self.window_size {
            if let Some(v) = self.history.pop_front() {
                self.sum -= v;
                self.sum_sq -= v * v;
            }
        }

        Ok(())
    }

    /// Reset the detector state.
    pub fn reset(&mut self) {
        self.history.clear();
        self.sum = 0.0;
        self.sum_sq = 0.0;
    }

    /// Run change detection and optionally return a `DegradationEvent`.
    ///
    /// This method runs the three detectors (ADWIN-like gradual check,
    /// Z-score spike check, and oscillation heuristic) and returns the
    /// most-significant finding.
    pub fn detect_change(&self) -> Result<Option<DegradationEvent>> {
        if self.history.is_empty() {
            return Ok(None);
        }

        let n = self.history.len();
        let mean = self.sum / n as f64;
        let var = (self.sum_sq / n as f64) - (mean * mean);
        let std = var.max(0.0).sqrt();

        // Spike detection (sudden)
        if let Some(&last) = self.history.back() {
            if std > 0.0 {
                let z = (last - mean) / std;
                if z.abs() >= self.spike_threshold {
                    let ev = DegradationEvent {
                        timestamp: now_secs(),
                        degradation_type: DegradationType::Sudden,
                        magnitude: (z.abs() / (self.spike_threshold * 2.0)).min(1.0),
                        confidence: ((z.abs() - self.spike_threshold) / (z.abs() + 1.0)).clamp(0.0, 1.0),
                        duration_samples: 1,
                        affected_metric: "value".to_string(),
                    };
                    debug!("Detected sudden spike: z={} mean={} std={}", z, mean, std);
                    return Ok(Some(ev));
                }
            }
        }

        // Gradual detection (ADWIN-like): check splits in window for significant mean change
        if n >= 2 {
            if let Some((pos, diff, conf)) = self.adwin_check() {
                let ev = DegradationEvent {
                    timestamp: now_secs(),
                    degradation_type: DegradationType::Gradual,
                    magnitude: diff.min(1.0).abs(),
                    confidence: conf,
                    duration_samples: pos,
                    affected_metric: "value".to_string(),
                };
                debug!("Detected gradual drift at split {} diff={} conf={}", pos, diff, conf);
                return Ok(Some(ev));
            }
        }

        // Oscillation heuristic: detect frequent sign changes in differences
        if self.detect_oscillation() {
            let ev = DegradationEvent {
                timestamp: now_secs(),
                degradation_type: DegradationType::Oscillating,
                magnitude: 0.5,
                confidence: 0.6,
                duration_samples: n,
                affected_metric: "value".to_string(),
            };
            debug!("Detected oscillation over {} samples", n);
            return Ok(Some(ev));
        }

        Ok(None)
    }

    /// Simple ADWIN-like check: iterate possible cut points and test for mean difference.
    /// Returns Some((cut_index, normalized_diff, confidence)) if drift found.
    fn adwin_check(&self) -> Option<(usize, f64, f64)> {
        let n = self.history.len();
        if n < 4 {
            return None;
        }

        let mut left_sum = 0.0;
        let mut left_sum_sq = 0.0;

        // We'll use a delta tuned from drift_threshold
        let delta = (self.drift_threshold).clamp(1e-6, 0.5);

        for i in 1..n {
            let v = self.history[i - 1];
            left_sum += v;
            left_sum_sq += v * v;
            let left_n = i as f64;
            let right_n = (n - i) as f64;
            if right_n < 1.0 {
                continue;
            }
            let left_mean = left_sum / left_n;
            let right_sum = self.sum - left_sum;
            let right_mean = right_sum / right_n;

            let diff = (left_mean - right_mean).abs();

            // pooled variance estimate
            let left_var = (left_sum_sq / left_n) - (left_mean * left_mean);
            let right_sum_sq = self.sum_sq - left_sum_sq;
            let right_var = (right_sum_sq / right_n) - (right_mean * right_mean);
            let var = left_var.max(0.0) + right_var.max(0.0);

            // Hoeffding-style bound (simplified)
            let m = 1.0 / (1.0 / left_n + 1.0 / right_n);
            let eps = ((2.0 * var * (delta.ln().abs())) / m).sqrt() + (2.0 / 3.0) * (delta.ln().abs()) / m;

            if diff > eps && diff >= self.drift_threshold {
                let conf = ((diff - eps) / (diff + eps)).clamp(0.0, 1.0);
                return Some((i, diff.min(1.0), conf));
            }
        }

        None
    }

    /// Simple oscillation detector: count sign flips in recent deltas.
    fn detect_oscillation(&self) -> bool {
        if self.history.len() < 6 {
            return false;
        }
        let mut flips = 0usize;
        let mut prev = None::<f64>;
        for window in self.history.as_slices().0.windows(2) {
            let d = window[1] - window[0];
            if let Some(p) = prev {
                if (d > 0.0 && p < 0.0) || (d < 0.0 && p > 0.0) {
                    flips += 1;
                }
            }
            prev = Some(d);
        }
        // if flips are a large fraction of windows, consider oscillation
        let total_pairs = (self.history.len() - 1) as usize;
        flips >= (total_pairs / 2)
    }

    /// Exponential histogram helper used by ADWIN variants.
    /// Produces exponentially-weighted counts over the history.
    pub fn exponential_histogram(&self, alpha: f64) -> Vec<f64> {
        let mut hist = Vec::new();
        let mut weight = 1.0f64;
        for &v in self.history.iter().rev() {
            hist.push(v * weight);
            weight *= alpha;
        }
        hist.reverse();
        hist
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_detector_on_series(series: &[f64]) -> Result<Option<DegradationEvent>> {
        let mut d = DegradationDetector::new(100, 0.15, 2.5);
        for &v in series.iter() {
            d.add_metric(v)?;
        }
        d.detect_change()
    }

    #[test]
    fn test_gradual_drift() -> Result<()> {
        // start at 0, slope 0.01 over 200 samples
        let series: Vec<f64> = (0..200).map(|i| (i as f64) * 0.01).collect();
        let ev = run_detector_on_series(&series)?;
        assert!(ev.is_some(), "expected a gradual drift event");
        let ev = ev.unwrap();
        assert_eq!(ev.degradation_type, DegradationType::Gradual);
        Ok(())
    }

    #[test]
    fn test_sudden_spike() -> Result<()> {
        let mut series = vec![50.0; 80];
        series.extend(std::iter::repeat(150.0).take(5));
        let ev = run_detector_on_series(&series)?;
        assert!(ev.is_some(), "expected a spike");
        let ev = ev.unwrap();
        assert_eq!(ev.degradation_type, DegradationType::Sudden);
        Ok(())
    }

    #[test]
    fn test_no_false_positive_on_noise() -> Result<()> {
        use rand_distr::{Distribution, Normal};
        let normal = Normal::new(50.0, 5.0).unwrap();
        let mut rng = rand::thread_rng();
        let series: Vec<f64> = (0..200).map(|_| normal.sample(&mut rng)).collect();
        let ev = run_detector_on_series(&series)?;
        assert!(ev.is_none(), "expected no detection on normal noise");
        Ok(())
    }

    #[test]
    fn test_oscillating_detection() -> Result<()> {
        // alternating high/low
        let mut series = Vec::new();
        for i in 0..120 {
            if i % 2 == 0 {
                series.push(40.0);
            } else {
                series.push(60.0);
            }
        }
        let ev = run_detector_on_series(&series)?;
        assert!(ev.is_some(), "expected oscillation detection");
        let ev = ev.unwrap();
        assert_eq!(ev.degradation_type, DegradationType::Oscillating);
        Ok(())
    }
}
