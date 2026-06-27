/// Burn-rate forecasting and multi-window alert rules following Google's SRE workbook approach.
///
/// This module implements:
/// - Multi-window burn rate calculations (5m, 30m, 1h, 6h)
/// - Alert severity levels based on combined burn rates
/// - Time-to-error-exhaustion (TTEE) forecasting
/// - Budget consumption projections
///
/// Reference: Google SRE Workbook - Alerting on SLOs
/// https://sre.google/books/site-reliability-engineering-workbook/

use serde::{Deserialize, Serialize};
use crate::core::{MetricPoint, Result as NeuralBudgetResult};

/// Burn rate severity levels used in multi-window alert rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnRateSeverity {
    /// No alert (burn rate normal)
    Ok,
    /// Slow burn: long-term budget exhaustion (6+ days)
    SlowBurn,
    /// Medium burn: medium-term budget exhaustion (1-6 days)
    MediumBurn,
    /// Fast burn: rapid budget exhaustion (hours to 1 day)
    FastBurn,
    /// Critical burn: immediate budget exhaustion (minutes to hours)
    CriticalBurn,
}

impl BurnRateSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::SlowBurn => "slow_burn",
            Self::MediumBurn => "medium_burn",
            Self::FastBurn => "fast_burn",
            Self::CriticalBurn => "critical_burn",
        }
    }
}

/// Multi-window burn rate sample with burns at different time scales.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiWindowBurnRate {
    /// Timestamp (Unix seconds)
    pub timestamp: i64,
    /// 5-minute burn rate
    pub burn_rate_5m: f64,
    /// 30-minute burn rate
    pub burn_rate_30m: f64,
    /// 1-hour burn rate
    pub burn_rate_1h: f64,
    /// 6-hour burn rate
    pub burn_rate_6h: f64,
}

impl MultiWindowBurnRate {
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Burn rate alert rule configuration following SRE workbook patterns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BurnRateAlertRule {
    /// Name of the alert rule (e.g., "fast-burn-1hr")
    pub name: String,
    /// Burn rate threshold for short window (e.g., 5m window)
    pub short_window_threshold: f64,
    /// Burn rate threshold for long window (e.g., 1h window)
    pub long_window_threshold: f64,
    /// Duration of short window in seconds (e.g., 300 for 5m)
    pub short_window_seconds: u64,
    /// Duration of long window in seconds (e.g., 3600 for 1h)
    pub long_window_seconds: u64,
    /// Severity level for this rule
    pub severity: BurnRateSeverity,
}

impl BurnRateAlertRule {
    /// Create a fast-burn rule: 10x burn rate over 5m + 6x burn rate over 1h.
    pub fn fast_burn() -> Self {
        Self {
            name: "fast-burn-1hr".to_string(),
            short_window_threshold: 10.0,
            long_window_threshold: 6.0,
            short_window_seconds: 300,
            long_window_seconds: 3_600,
            severity: BurnRateSeverity::FastBurn,
        }
    }

    /// Create a medium-burn rule: 3x burn rate over 30m + 1x burn rate over 6h.
    pub fn medium_burn() -> Self {
        Self {
            name: "medium-burn-6hr".to_string(),
            short_window_threshold: 3.0,
            long_window_threshold: 1.0,
            short_window_seconds: 1_800,
            long_window_seconds: 21_600,
            severity: BurnRateSeverity::MediumBurn,
        }
    }

    /// Create a slow-burn rule: 1x burn rate over 1h + 0.1x burn rate over 30 days.
    pub fn slow_burn() -> Self {
        Self {
            name: "slow-burn-30day".to_string(),
            short_window_threshold: 1.0,
            long_window_threshold: 0.1,
            short_window_seconds: 3_600,
            long_window_seconds: 2_592_000, // 30 days
            severity: BurnRateSeverity::SlowBurn,
        }
    }

    /// Create all standard SRE workbook alert rules.
    pub fn standard_rules() -> Vec<Self> {
        vec![
            Self::fast_burn(),
            Self::medium_burn(),
            Self::slow_burn(),
        ]
    }
}

/// Burn rate alert evaluation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BurnRateAlertResult {
    /// The rule that was evaluated
    pub rule: BurnRateAlertRule,
    /// Whether the alert should fire
    pub triggered: bool,
    /// Short window burn rate
    pub short_burn_rate: f64,
    /// Long window burn rate
    pub long_burn_rate: f64,
    /// Additional context message
    pub message: String,
}

/// Time-to-error-exhaustion (TTEE) forecast.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetExhaustionForecast {
    /// Current timestamp
    pub timestamp: i64,
    /// Current burn rate (e.g., from 1h window)
    pub current_burn_rate: f64,
    /// Remaining error budget in seconds
    pub remaining_budget_seconds: f64,
    /// Estimated time until budget exhaustion in seconds
    pub time_to_exhaustion_seconds: f64,
    /// Estimated exhaustion timestamp (Unix seconds)
    pub projected_exhaustion_timestamp: i64,
    /// Whether budget is projected to be exhausted
    pub will_exhaust: bool,
}

impl BudgetExhaustionForecast {
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Calculate burn rate for a specific window in seconds.
pub fn calculate_burn_rate_for_window(
    metric_stream: &[MetricPoint],
    window_seconds: u64,
) -> f64 {
    if metric_stream.is_empty() || window_seconds == 0 {
        return 0.0;
    }

    let window_secs = match i64::try_from(window_seconds) {
        Ok(value) if value > 0 => value,
        _ => return 0.0,
    };

    let now = match metric_stream.iter().map(|point| point.timestamp).max() {
        Some(value) => value,
        None => return 0.0,
    };

    let window_start = match now.checked_sub(window_secs) {
        Some(value) => value,
        None => return 0.0,
    };

    let consumed_seconds = metric_stream
        .iter()
        .filter(|point| point.timestamp > window_start && point.timestamp <= now)
        .filter(|point| point.value > 0.0)
        .count() as f64;

    consumed_seconds / window_secs as f64
}

/// Calculate multi-window burn rates.
pub fn calculate_multi_window_burn_rate(
    metric_stream: &[MetricPoint],
) -> MultiWindowBurnRate {
    let now = metric_stream
        .iter()
        .map(|point| point.timestamp)
        .max()
        .unwrap_or(0);

    MultiWindowBurnRate {
        timestamp: now,
        burn_rate_5m: calculate_burn_rate_for_window(metric_stream, 300),
        burn_rate_30m: calculate_burn_rate_for_window(metric_stream, 1_800),
        burn_rate_1h: calculate_burn_rate_for_window(metric_stream, 3_600),
        burn_rate_6h: calculate_burn_rate_for_window(metric_stream, 21_600),
    }
}

/// Evaluate if a burn rate alert rule should fire.
pub fn evaluate_burn_rate_alert(
    rule: &BurnRateAlertRule,
    short_burn: f64,
    long_burn: f64,
) -> BurnRateAlertResult {
    let triggered = short_burn >= rule.short_window_threshold
        && long_burn >= rule.long_window_threshold;

    let message = if triggered {
        format!(
            "Alert {} triggered: short_burn={:.2}x (threshold: {:.2}x), long_burn={:.2}x (threshold: {:.2}x)",
            rule.name, short_burn, rule.short_window_threshold, long_burn, rule.long_window_threshold
        )
    } else {
        format!(
            "Alert {} not triggered: short_burn={:.2}x (threshold: {:.2}x), long_burn={:.2}x (threshold: {:.2}x)",
            rule.name, short_burn, rule.short_window_threshold, long_burn, rule.long_window_threshold
        )
    };

    BurnRateAlertResult {
        rule: rule.clone(),
        triggered,
        short_burn_rate: short_burn,
        long_burn_rate: long_burn,
        message,
    }
}

/// Forecast when error budget will be exhausted.
pub fn forecast_budget_exhaustion(
    current_burn_rate: f64,
    remaining_budget_seconds: f64,
    now: i64,
) -> BudgetExhaustionForecast {
    let will_exhaust = current_burn_rate > 0.0 && remaining_budget_seconds > 0.0;
    
    let time_to_exhaustion_seconds = if will_exhaust && current_burn_rate > 0.0 {
        remaining_budget_seconds / current_burn_rate
    } else {
        f64::INFINITY
    };

    let projected_exhaustion_timestamp = if will_exhaust && time_to_exhaustion_seconds.is_finite() {
        now + time_to_exhaustion_seconds as i64
    } else {
        i64::MAX
    };

    BudgetExhaustionForecast {
        timestamp: now,
        current_burn_rate,
        remaining_budget_seconds,
        time_to_exhaustion_seconds,
        projected_exhaustion_timestamp,
        will_exhaust,
    }
}

/// Evaluate multi-window burn rate alerts based on SRE workbook rules.
pub fn evaluate_multi_window_alerts(
    multi_window: &MultiWindowBurnRate,
    rules: &[BurnRateAlertRule],
) -> Vec<BurnRateAlertResult> {
    let mut results = Vec::new();

    for rule in rules {
        let (short_burn, long_burn) = match rule.short_window_seconds {
            300 => (multi_window.burn_rate_5m, multi_window.burn_rate_1h),
            1_800 => (multi_window.burn_rate_30m, multi_window.burn_rate_6h),
            3_600 => (multi_window.burn_rate_1h, multi_window.burn_rate_6h),
            _ => continue, // Skip unknown windows
        };

        results.push(evaluate_burn_rate_alert(rule, short_burn, long_burn));
    }

    results
}

/// Determine overall severity from a list of alert results.
pub fn determine_overall_severity(alerts: &[BurnRateAlertResult]) -> BurnRateSeverity {
    let triggered_alerts: Vec<_> = alerts.iter().filter(|a| a.triggered).collect();

    if triggered_alerts.is_empty() {
        return BurnRateSeverity::Ok;
    }

    // Return the highest severity
    for alert in &triggered_alerts {
        if alert.rule.severity == BurnRateSeverity::CriticalBurn {
            return BurnRateSeverity::CriticalBurn;
        }
    }
    for alert in &triggered_alerts {
        if alert.rule.severity == BurnRateSeverity::FastBurn {
            return BurnRateSeverity::FastBurn;
        }
    }
    for alert in &triggered_alerts {
        if alert.rule.severity == BurnRateSeverity::MediumBurn {
            return BurnRateSeverity::MediumBurn;
        }
    }

    BurnRateSeverity::SlowBurn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_burn_rate_for_window() {
        let stream = vec![
            MetricPoint { timestamp: 1, value: 1.0, labels: Default::default() },
            MetricPoint { timestamp: 2, value: 0.0, labels: Default::default() },
            MetricPoint { timestamp: 3, value: 1.0, labels: Default::default() },
            MetricPoint { timestamp: 4, value: 1.0, labels: Default::default() },
            MetricPoint { timestamp: 5, value: 0.0, labels: Default::default() },
        ];

        // Over 5 second window: 3 bad seconds out of 5
        let burn = calculate_burn_rate_for_window(&stream, 5);
        assert_eq!(burn, 0.6);
    }

    #[test]
    fn test_multi_window_burn_rate() {
        let stream: Vec<MetricPoint> = (1..=3600)
            .map(|i| MetricPoint {
                timestamp: i as i64,
                value: if i % 10 < 5 { 1.0 } else { 0.0 },
                labels: Default::default(),
            })
            .collect();

        let multi = calculate_multi_window_burn_rate(&stream);
        assert!(multi.burn_rate_5m > 0.0);
        assert!(multi.burn_rate_30m > 0.0);
        assert!(multi.burn_rate_1h > 0.0);
    }

    #[test]
    fn test_burn_rate_alert_rule_fast_burn() {
        let rule = BurnRateAlertRule::fast_burn();
        assert_eq!(rule.severity, BurnRateSeverity::FastBurn);
        assert_eq!(rule.short_window_threshold, 10.0);
        assert_eq!(rule.long_window_threshold, 6.0);
    }

    #[test]
    fn test_evaluate_burn_rate_alert() {
        let rule = BurnRateAlertRule::fast_burn();
        
        // Alert should trigger
        let result = evaluate_burn_rate_alert(&rule, 10.0, 6.0);
        assert!(result.triggered);

        // Alert should not trigger (short burn too low)
        let result = evaluate_burn_rate_alert(&rule, 5.0, 6.0);
        assert!(!result.triggered);

        // Alert should not trigger (long burn too low)
        let result = evaluate_burn_rate_alert(&rule, 10.0, 5.0);
        assert!(!result.triggered);
    }

    #[test]
    fn test_forecast_budget_exhaustion() {
        let forecast = forecast_budget_exhaustion(2.0, 3600.0, 1000);
        assert!(forecast.will_exhaust);
        assert_eq!(forecast.time_to_exhaustion_seconds, 1800.0);
        assert_eq!(forecast.projected_exhaustion_timestamp, 3800);
    }

    #[test]
    fn test_determine_overall_severity() {
        let alerts = vec![
            BurnRateAlertResult {
                rule: BurnRateAlertRule::medium_burn(),
                triggered: true,
                short_burn_rate: 1.0,
                long_burn_rate: 0.5,
                message: "test".to_string(),
            },
        ];
        let severity = determine_overall_severity(&alerts);
        assert_eq!(severity, BurnRateSeverity::MediumBurn);
    }
}
