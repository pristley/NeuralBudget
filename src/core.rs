#![allow(clippy::useless_conversion)]

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::convert::TryFrom;

use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::de::{DeserializeOwned, Error as DeError};
use serde::{Deserialize, Serialize};

/// Comprehensive error type for NeuralBudget library operations.
///
/// This enum covers all error conditions that may occur during:
/// - Configuration deserialization
/// - SLO evaluation
/// - Composite DAG processing
/// - Format conversions (OTLP, Prometheus)
#[derive(Debug, Clone)]
pub enum NeuralBudgetError {
    /// Configuration deserialization or validation failed
    ConfigError(String),
    /// Composite dependency DAG operation failed
    CompositeError(String),
    /// Format conversion (OTLP, Prometheus, etc.) failed
    FormatError(String),
    /// SLO evaluation logic encountered an invariant violation
    EvaluationError(String),
    /// Schema version is unsupported
    SchemaVersionError { found: u32, supported: String },
}

impl std::fmt::Display for NeuralBudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::CompositeError(msg) => write!(f, "Composite DAG error: {msg}"),
            Self::FormatError(msg) => write!(f, "Format conversion error: {msg}"),
            Self::EvaluationError(msg) => write!(f, "Evaluation error: {msg}"),
            Self::SchemaVersionError { found, supported } => {
                write!(
                    f,
                    "Unsupported schema version {found}; supported: {supported}"
                )
            }
        }
    }
}

impl std::error::Error for NeuralBudgetError {}

impl From<serde_json::Error> for NeuralBudgetError {
    fn from(err: serde_json::Error) -> Self {
        Self::FormatError(format!("JSON error: {err}"))
    }
}

/// Result type alias using NeuralBudgetError as the error type.
pub type Result<T> = std::result::Result<T, NeuralBudgetError>;

const SLO_CONFIG_SCHEMA_VERSION: u32 = 1;

fn deserialize_slo_config_schema_version<'de, D>(
    deserializer: D,
) -> std::result::Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = Option::<u32>::deserialize(deserializer)?.unwrap_or(SLO_CONFIG_SCHEMA_VERSION);
    if version != SLO_CONFIG_SCHEMA_VERSION {
        return Err(D::Error::custom(format!(
            "unsupported schema_version {version}; supported schema_version is {SLO_CONFIG_SCHEMA_VERSION}"
        )));
    }
    Ok(version)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SloConfigSchemaV1 {
    #[serde(default, deserialize_with = "deserialize_slo_config_schema_version")]
    schema_version: u32,
    target: f64,
    window: String,
}

impl From<SloConfigSchemaV1> for SloConfig {
    fn from(value: SloConfigSchemaV1) -> Self {
        Self {
            target: value.target,
            window: value.window,
        }
    }
}

impl From<&SloConfig> for SloConfigSchemaV1 {
    fn from(value: &SloConfig) -> Self {
        Self {
            schema_version: SLO_CONFIG_SCHEMA_VERSION,
            target: value.target,
            window: value.window.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Basic SLO target metadata used by the Rust and Python surfaces.
pub struct SloConfig {
    pub target: f64,
    pub window: String,
}

impl SloConfig {
    pub fn from_json_str(input: &str) -> std::result::Result<Self, serde_json::Error> {
        let schema: SloConfigSchemaV1 = serde_json::from_str(input)?;
        Ok(schema.into())
    }

    pub fn to_json_string(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(&SloConfigSchemaV1::from(self))
    }

    pub fn from_yaml_str(input: &str) -> std::result::Result<Self, serde_yaml::Error> {
        let schema: SloConfigSchemaV1 = serde_yaml::from_str(input)?;
        Ok(schema.into())
    }

    pub fn to_yaml_string(&self) -> std::result::Result<String, serde_yaml::Error> {
        serde_yaml::to_string(&SloConfigSchemaV1::from(self))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Remaining error budget and burn velocity for an SLO objective.
pub struct ErrorBudget {
    pub remaining: f64,
    pub velocity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Timestamped metric sample with optional labels.
pub struct MetricPoint {
    pub timestamp: i64,
    pub value: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Timestamped web API request sample.
pub struct WebApiRequest {
    pub timestamp: i64,
    pub latency_ms: f64,
    pub status_code: u16,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Outlier filtering controls for latency-sensitive SLO calculations.
pub struct OutlierFilterConfig {
    pub enabled: bool,
    pub mad_threshold: f64,
    pub min_samples: usize,
}

impl Default for OutlierFilterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mad_threshold: 3.5,
            min_samples: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// SLO policy for a generic web API.
pub struct WebApiSloPolicy {
    /// Availability target in decimal form, e.g. `0.999`.
    pub availability_target: f64,
    /// Latency objective threshold in milliseconds.
    pub latency_threshold_ms: f64,
    /// Evaluation window in seconds.
    pub time_window_seconds: u64,
    /// Optional outlier filtering strategy.
    #[serde(default)]
    pub outlier_filter: OutlierFilterConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Computed SLO report for a web API evaluation window.
pub struct WebApiSloReport {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub availability: f64,
    pub latency_evaluated_requests: u64,
    pub latency_compliant_requests: u64,
    pub latency_sli: f64,
    pub filtered_outliers: u64,
    pub error_budget_seconds: f64,
    pub burn_rate_5m: f64,
    pub burn_rate_1h: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Histogram bucket where `count` is either cumulative or delta depending on format.
pub struct HistogramBucket {
    pub upper_bound_ms: f64,
    pub count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Histogram wire format for latency inputs.
pub enum HistogramFormat {
    /// Prometheus histogram buckets where counts are cumulative.
    PrometheusCumulative,
    /// OpenTelemetry explicit buckets where counts are per-bucket deltas.
    OpenTelemetryDelta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One timestamped SLO sample assembled from telemetry.
pub struct HistogramSample {
    pub timestamp: i64,
    pub success: u64,
    pub total: u64,
    pub buckets: Vec<HistogramBucket>,
    pub format: HistogramFormat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// HTTP/gRPC SLO policy: availability + percentile latency objective.
pub struct HttpSlo {
    pub latency_threshold_ms: f64,
    pub latency_percentile: f64,
    pub availability_threshold: f64,
}

impl Default for HttpSlo {
    fn default() -> Self {
        Self {
            latency_threshold_ms: 200.0,
            latency_percentile: 0.99,
            availability_threshold: 0.999,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// SLO evaluation output for a single histogram sample.
pub struct HttpSloEvaluation {
    pub timestamp: i64,
    pub availability: f64,
    pub evaluated_percentile: f64,
    pub percentile_latency_ms: f64,
    pub latency_ok: bool,
    pub availability_ok: bool,
    pub pass: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// Alert severity levels for burn-rate alerts.
pub enum AlertSeverity {
    /// Low priority - informational only (no paging)
    Info,
    /// Medium priority - requires investigation within hours
    Warning,
    /// High priority - immediate response required
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One multi-burn-rate alert window in a Google SRE-based alerting policy.
///
/// For a 99.9% SLO (0.001 error budget):
/// - 1h window @ 10x burn → error_rate > 0.009 for 1m → immediate
/// - 6h window @ 2x burn → error_rate > 0.002 for 15m → urgent
/// - 24h window @ 0.5x burn → error_rate > 0.0005 for 1h → trend
/// - 3d window @ 1x burn → error_rate > 0.001 for 3h → exhaustion
pub struct BurnRateWindow {
    /// Duration of window: "1h", "6h", "24h", "3d", etc.
    pub duration: String,
    /// Multiplier on burn rate threshold (10 = "burning at 10x rate")
    pub burn_rate: f64,
    /// Duration to evaluate before firing (e.g., "1m", "15m", "1h")
    pub for_duration: String,
    /// Alert severity for this window
    pub severity: AlertSeverity,
}

impl BurnRateWindow {
    /// Create a burn rate window with explicit parameters.
    pub fn new(
        duration: impl Into<String>,
        burn_rate: f64,
        for_duration: impl Into<String>,
        severity: AlertSeverity,
    ) -> Self {
        Self {
            duration: duration.into(),
            burn_rate,
            for_duration: for_duration.into(),
            severity,
        }
    }

    /// Convert burn rate multiplier to actual error rate threshold.
    ///
    /// Given availability_target (e.g., 0.999):
    /// - allowed_error = 1 - availability_target = 0.001
    /// - threshold = allowed_error * burn_rate_multiplier
    pub fn calculate_error_threshold(&self, availability_target: f64) -> f64 {
        let allowed_error = 1.0 - availability_target;
        allowed_error * self.burn_rate
    }

    /// Duration in seconds for use in calculations.
    pub fn duration_seconds(&self) -> Result<u64> {
        match self.duration.as_str() {
            "1h" => Ok(3600),
            "6h" => Ok(21600),
            "24h" => Ok(86400),
            "2d" => Ok(172800),
            "3d" => Ok(259200),
            "7d" => Ok(604800),
            _ => Err(NeuralBudgetError::ConfigError(format!(
                "Unknown duration: {}",
                self.duration
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Multi-window burn-rate alerting configuration (Google SRE pattern).
///
/// Recommended defaults for production:
/// - 1h window @ 10x burn, 1min duration (catch fast burns immediately)
/// - 6h window @ 2x burn, 15min duration (detect sustained degradation)
/// - 24h window @ 0.5x burn, 1h duration (track slow trends)
/// - 3d window @ 1x burn, 3h duration (predict budget exhaustion)
pub struct MultiWindowAlertConfig {
    /// Burn rate windows, typically 3-4 of them
    pub windows: Vec<BurnRateWindow>,
}

impl MultiWindowAlertConfig {
    /// Create an empty configuration.
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    /// Add a burn rate window to the configuration.
    pub fn with_window(mut self, window: BurnRateWindow) -> Self {
        self.windows.push(window);
        self
    }

    /// Default 4-window configuration following Google SRE recommendations.
    ///
    /// For 99.9% SLO (0.001 error budget, ~8.6 minutes per month):
    /// - 1h window: alert if burning >10% of budget/hour for 1 minute
    /// - 6h window: alert if burning >2% of budget/hour for 15 minutes
    /// - 24h window: alert if burning >0.5% of budget/hour for 1 hour
    /// - 3d window: alert if burning >1% of budget/hour for 3 hours
    pub fn default_four_window() -> Self {
        Self {
            windows: vec![
                BurnRateWindow::new("1h", 10.0, "1m", AlertSeverity::Critical),
                BurnRateWindow::new("6h", 2.0, "15m", AlertSeverity::Warning),
                BurnRateWindow::new("24h", 0.5, "1h", AlertSeverity::Info),
                BurnRateWindow::new("3d", 1.0, "3h", AlertSeverity::Warning),
            ],
        }
    }

    /// Validate that windows don't have conflicting thresholds.
    pub fn validate(&self) -> Result<()> {
        if self.windows.is_empty() {
            return Err(NeuralBudgetError::ConfigError(
                "No burn rate windows configured".to_string(),
            ));
        }

        // Check that burn rates are in descending order (faster windows have higher thresholds)
        for i in 0..self.windows.len().saturating_sub(1) {
            if self.windows[i].burn_rate < self.windows[i + 1].burn_rate {
                return Err(NeuralBudgetError::ConfigError(format!(
                    "Burn rate windows must be in descending order by burn_rate: {} < {}",
                    self.windows[i].burn_rate, self.windows[i + 1].burn_rate
                )));
            }
        }

        Ok(())
    }
}

impl Default for MultiWindowAlertConfig {
    fn default() -> Self {
        Self::default_four_window()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One timestamped stateful-system sample for DB/queue health.
pub struct StatefulSample {
    pub timestamp: i64,
    pub replication_lag_ms: f64,
    pub queue_depth: u64,
    /// Saturation ratio in `[0.0, 1.0]`.
    pub connection_pool_saturation: f64,
    pub connection_wait_time_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Tier selector for stateful policy profiles.
pub enum StatefulTier {
    Database,
    Queue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Weighted stateful SLO profile for a specific database or queue tier.
pub struct StatefulPolicyProfile {
    pub name: String,
    pub tier: StatefulTier,
    pub replication_lag_weight: f64,
    pub queue_depth_weight: f64,
    pub connection_pool_weight: f64,
    pub connection_wait_penalty_weight: f64,
    pub min_pass_score: f64,
}

impl StatefulPolicyProfile {
    /// Balanced profile for database tiers that prioritize replication lag.
    pub fn database() -> Self {
        Self {
            name: "database_primary".to_string(),
            tier: StatefulTier::Database,
            replication_lag_weight: 3.0,
            queue_depth_weight: 1.0,
            connection_pool_weight: 2.0,
            connection_wait_penalty_weight: 1.5,
            min_pass_score: 0.88,
        }
    }

    /// Balanced profile for queue tiers that prioritize queue depth.
    pub fn queue() -> Self {
        Self {
            name: "queue_hot_path".to_string(),
            tier: StatefulTier::Queue,
            replication_lag_weight: 0.75,
            queue_depth_weight: 3.0,
            connection_pool_weight: 1.5,
            connection_wait_penalty_weight: 2.0,
            min_pass_score: 0.9,
        }
    }

    fn total_weight(&self) -> f64 {
        self.replication_lag_weight.max(0.0)
            + self.queue_depth_weight.max(0.0)
            + self.connection_pool_weight.max(0.0)
            + self.connection_wait_penalty_weight.max(0.0)
    }
}

impl Default for StatefulPolicyProfile {
    fn default() -> Self {
        Self::database()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Tier-specific profile set for database and queue workloads.
pub struct StatefulPolicyProfileSet {
    pub database: StatefulPolicyProfile,
    pub queue: StatefulPolicyProfile,
}

impl StatefulPolicyProfileSet {
    pub fn profile_for_tier(&self, tier: StatefulTier) -> &StatefulPolicyProfile {
        match tier {
            StatefulTier::Database => &self.database,
            StatefulTier::Queue => &self.queue,
        }
    }
}

impl Default for StatefulPolicyProfileSet {
    fn default() -> Self {
        Self {
            database: StatefulPolicyProfile::database(),
            queue: StatefulPolicyProfile::queue(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// SLO policy for stateful systems such as databases and queues.
pub struct StatefulSlo {
    pub replication_lag_threshold_ms: f64,
    pub queue_depth_threshold: u64,
    pub connection_pool_saturation_threshold: f64,
    pub connection_wait_time_threshold_ms: f64,
    /// Penalty applied when wait time exceeds threshold.
    pub connection_wait_penalty_weight: f64,
    /// Minimum score required to pass after penalties are applied.
    pub min_pass_score: f64,
}

impl Default for StatefulSlo {
    fn default() -> Self {
        Self {
            replication_lag_threshold_ms: 250.0,
            queue_depth_threshold: 1_000,
            connection_pool_saturation_threshold: 0.8,
            connection_wait_time_threshold_ms: 20.0,
            connection_wait_penalty_weight: 0.2,
            min_pass_score: 0.9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Stateful SLO evaluation output per sample.
pub struct StatefulSloEvaluation {
    pub timestamp: i64,
    pub replication_lag_ok: bool,
    pub queue_depth_ok: bool,
    pub connection_pool_ok: bool,
    pub connection_wait_penalized: bool,
    pub score: f64,
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One timestamped ML-serving sample combining system and data quality signals.
pub struct MlSample {
    pub timestamp: i64,
    pub inference_latency_ms: f64,
    /// Saturation ratio in `[0.0, 1.0]`.
    pub gpu_utilization: f64,
    /// Feature drift distance where lower is better.
    pub feature_drift: f64,
    /// Model confidence in `[0.0, 1.0]` where higher is better.
    pub prediction_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Hybrid MLOps SLO policy that blends serving latency/system and drift/data signals.
pub struct MlSlo {
    pub max_inference_latency_ms: f64,
    pub max_gpu_utilization: f64,
    pub max_feature_drift: f64,
    pub min_prediction_confidence: f64,
    /// Weight applied to `latency_score` (`system_score`) in the hybrid formula.
    pub latency_weight: f64,
    /// Weight applied to `drift_score` (`data_score`) in the hybrid formula.
    pub drift_weight: f64,
    /// Minimum hybrid score required to pass.
    pub min_pass_score: f64,
}

impl Default for MlSlo {
    fn default() -> Self {
        Self {
            max_inference_latency_ms: 200.0,
            max_gpu_utilization: 0.85,
            max_feature_drift: 0.2,
            min_prediction_confidence: 0.8,
            latency_weight: 0.6,
            drift_weight: 0.4,
            min_pass_score: 0.9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// ML-serving SLO evaluation output per sample.
pub struct MlSloEvaluation {
    pub timestamp: i64,
    pub inference_latency_score: f64,
    pub gpu_utilization_score: f64,
    pub system_score: f64,
    /// Alias for `system_score` used by the published hybrid formula.
    pub latency_score: f64,
    pub feature_drift_score: f64,
    pub prediction_confidence_score: f64,
    pub drift_score: f64,
    pub latency_weight: f64,
    pub drift_weight: f64,
    pub hybrid_score: f64,
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One timestamped GenAI request/response sample for qualitative SLO evaluation.
pub struct GenAiSample {
    pub timestamp: i64,
    pub tokens_generated: u64,
    pub generation_duration_ms: f64,
    pub time_to_first_token_ms: f64,
    pub reference_text: String,
    pub generated_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// GenAI SLO policy combining speed and output quality checks.
pub struct GenAiSlo {
    pub min_tokens_per_second: f64,
    pub max_time_to_first_token_ms: f64,
    pub min_semantic_similarity: f64,
    pub semantic_model_name: String,
}

impl Default for GenAiSlo {
    fn default() -> Self {
        Self {
            min_tokens_per_second: 20.0,
            max_time_to_first_token_ms: 1_200.0,
            min_semantic_similarity: 0.7,
            semantic_model_name: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// GenAI SLO evaluation output for one generation sample.
pub struct GenAiSloEvaluation {
    pub timestamp: i64,
    pub tokens_per_second: f64,
    pub time_to_first_token_ms: f64,
    pub semantic_similarity: f64,
    pub tokens_per_second_ok: bool,
    pub time_to_first_token_ok: bool,
    pub semantic_similarity_ok: bool,
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Quality evaluator configuration for GenAI SLOs (reference-free evaluation).
pub enum QualityEvaluator {
    /// LLM-as-Judge evaluator with caching support
    LlmJudge {
        model: String,
        provider: String,
        cache_config: Option<CacheConfigSpec>,
        dimensions: Vec<QualityDimensionSpec>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Cache configuration for quality evaluators.
pub struct CacheConfigSpec {
    pub redis_url: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Single quality dimension specification for evaluation.
pub struct QualityDimensionSpec {
    pub name: String,
    pub prompt: String,
    pub weight: f64,
    pub threshold: f64,
    pub cost_per_call_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// GenAI SLO configuration with quality evaluator.
pub struct GenAiSloConfig {
    pub quality_evaluator: QualityEvaluator,
    pub sample: GenAiQualitySample,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Sample for GenAI quality evaluation.
pub struct GenAiQualitySample {
    pub timestamp: i64,
    pub query: String,
    pub response: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// GenAI quality evaluation result with detailed scoring.
pub struct GenAiQualityEvaluation {
    pub timestamp: i64,
    pub cache_key: String,
    pub from_cache: bool,
    pub dimension_scores: Vec<DimensionScoreResult>,
    pub weighted_score: f64,
    pub pass: bool,
    pub total_cost_usd: f64,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Individual dimension score result.
pub struct DimensionScoreResult {
    pub name: String,
    pub score: f64,
    pub reasoning: Option<String>,
    pub pass: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Method for extracting claims from LLM response text
pub enum HallucinationExtractionMethod {
    /// Use simple rule-based extraction (split by sentences)
    RuleBased,
    /// Use LLM to extract claims
    LlmBased,
    /// Dependency parsing (future)
    DependencyParsing,
}

impl Default for HallucinationExtractionMethod {
    fn default() -> Self {
        Self::RuleBased
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Method for scoring claim-document similarity
pub enum HallucinationScoringMethod {
    /// Token overlap similarity
    TokenOverlap,
    /// Embedding-based similarity
    EmbeddingSimilarity,
    /// TF-IDF similarity
    TfIdf,
    /// Entailment (RTE) scoring
    Entailment,
}

impl Default for HallucinationScoringMethod {
    fn default() -> Self {
        Self::TokenOverlap
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Hallucination detection configuration for GenAI SLOs
pub struct HallucinationDetectionConfig {
    /// Enable hallucination detection
    pub enabled: bool,
    /// Method for extracting claims
    pub extraction_method: HallucinationExtractionMethod,
    /// Method for scoring claim groundedness
    pub scoring_method: HallucinationScoringMethod,
    /// Minimum threshold for claim to be considered grounded [0.0-1.0]
    pub groundedness_threshold: f64,
    /// Minimum overall groundedness score required [0.0-1.0]
    pub min_groundedness_score: f64,
}

impl Default for HallucinationDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            extraction_method: HallucinationExtractionMethod::RuleBased,
            scoring_method: HallucinationScoringMethod::TokenOverlap,
            groundedness_threshold: 0.5,
            min_groundedness_score: 0.75,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// GenAI quality evaluation result including hallucination detection
pub struct GenAiQualityWithHallucinationEvaluation {
    pub timestamp: i64,
    pub cache_key: String,
    pub from_cache: bool,
    pub dimension_scores: Vec<DimensionScoreResult>,
    pub weighted_score: f64,
    pub groundedness_score: Option<f64>,
    pub hallucination_rate: Option<f64>,
    pub pass: bool,
    pub total_cost_usd: f64,
    pub total_tokens: usize,
}

// ============================================================================
// Cost-Based SLO Types (GenAI Token Usage and Cost Budgets)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Cost budget configuration for GenAI SLOs
pub struct CostBudgetConfig {
    /// Cost per 1000 input tokens in USD
    pub input_cost_per_1k: f64,
    /// Cost per 1000 output tokens in USD
    pub output_cost_per_1k: f64,
    /// Maximum allowed cost per request in USD
    pub max_per_request: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Cost SLO configuration for tracking and limiting token costs
pub struct GenAiCostSloConfig {
    /// Enable cost-based SLO evaluation
    pub enabled: bool,
    /// Cost budget configuration
    pub budget: CostBudgetConfig,
    /// Cost acceptability threshold (0.0-1.0, where 1.0 = free, 0.0 = at budget limit)
    pub cost_threshold: f64,
    /// Optional monthly cost limit in USD
    pub monthly_limit: Option<f64>,
    /// Weight of cost in hybrid scoring (combined with quality)
    pub cost_weight: Option<f64>,
}

impl Default for GenAiCostSloConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            budget: CostBudgetConfig {
                input_cost_per_1k: 0.00015,   // GPT-4 Mini default
                output_cost_per_1k: 0.0006,
                max_per_request: 0.015,
            },
            cost_threshold: 0.95,
            monthly_limit: None,
            cost_weight: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Cost evaluation result for a GenAI request
pub struct GenAiCostEvaluation {
    /// Input token cost in USD
    pub input_cost: f64,
    /// Output token cost in USD
    pub output_cost: f64,
    /// Total cost in USD
    pub total_cost: f64,
    /// Whether cost is within per-request budget
    pub within_budget: bool,
    /// Cost score (0.0-1.0)
    pub cost_score: f64,
    /// Whether cost passes SLO
    pub pass: bool,
}

// ============================================================================
// Agent SLO Configuration Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Trajectory metrics configuration for agent SLO evaluation
pub struct TrajectoryMetricConfig {
    /// Maximum number of steps agent should take
    pub max_steps: u32,
    /// Minimum acceptable tool call success rate (0.0-1.0)
    pub tool_success_threshold: f64,
    /// Maximum times the same action can repeat
    pub max_repeated_actions: u32,
    /// Minimum acceptable overall success rate (0.0-1.0)
    pub success_threshold: f64,
}

impl Default for TrajectoryMetricConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            tool_success_threshold: 0.95,
            max_repeated_actions: 2,
            success_threshold: 0.90,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Agent SLO configuration for tracking agent execution reliability
pub struct AgentSloConfig {
    /// Enable agent SLO evaluation
    pub enabled: bool,
    /// Trajectory metrics thresholds
    pub trajectory_metrics: TrajectoryMetricConfig,
    /// Enable loop detection
    pub loop_detection_enabled: bool,
}

impl Default for AgentSloConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            trajectory_metrics: TrajectoryMetricConfig::default(),
            loop_detection_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Agent evaluation result
pub struct AgentEvaluationResult {
    /// Number of steps taken
    pub steps_taken: u32,
    /// Tool call success rate (0.0-1.0)
    pub tool_success_rate: f64,
    /// Whether loop was detected
    pub loop_detected: bool,
    /// Whether final status was success
    pub success: bool,
    /// Overall SLO pass/fail
    pub pass: bool,
}

// ============================================================================
// TTFT (Time to First Token) SLO Configuration Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Configuration for TTFT (Time to First Token) SLO evaluation
pub struct TtftSloConfig {
    /// Enable TTFT SLO evaluation
    pub enabled: bool,
    /// Maximum acceptable TTFT in milliseconds (e.g., 500ms)
    pub ttft_threshold_ms: f64,
    /// Percentile for TTFT threshold (e.g., 0.99 for P99)
    pub ttft_percentile: f64,
    /// Maximum acceptable inter-token latency in milliseconds
    pub inter_token_latency_threshold_ms: f64,
    /// Percentile for inter-token latency (e.g., 0.95 for P95)
    pub inter_token_percentile: f64,
}

impl Default for TtftSloConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttft_threshold_ms: 500.0,
            ttft_percentile: 0.99,
            inter_token_latency_threshold_ms: 50.0,
            inter_token_percentile: 0.95,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// TTFT evaluation result for configuration compatibility
pub struct TtftEvaluationResult {
    /// Whether TTFT meets threshold
    pub ttft_pass: bool,
    /// Actual TTFT in milliseconds
    pub ttft_ms: f64,
    /// Whether inter-token latency meets threshold
    pub inter_token_pass: bool,
    /// Actual inter-token latency in milliseconds
    pub inter_token_latency_ms: f64,
    /// Overall SLO pass/fail
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Weights for unified GenAI SLO evaluation combining all quality dimensions
pub struct CompositeGenAiWeights {
    /// Weight for throughput (TPS) — serving speed (0.0-1.0)
    #[serde(default = "default_throughput_weight")]
    pub throughput_weight: f64,
    /// Weight for TTFT (Time to First Token) — perceived responsiveness (0.0-1.0)
    #[serde(default = "default_ttft_weight")]
    pub ttft_weight: f64,
    /// Weight for quality (semantic similarity or LLM judge) (0.0-1.0)
    #[serde(default = "default_quality_weight")]
    pub quality_weight: f64,
    /// Weight for groundedness (hallucination rate) — factual accuracy (0.0-1.0)
    #[serde(default = "default_groundedness_weight")]
    pub groundedness_weight: f64,
    /// Weight for cost (token budget) — cost efficiency (0.0-1.0)
    #[serde(default = "default_cost_weight")]
    pub cost_weight: f64,
    /// Weight for retrieval quality (recall@k, MRR for RAG) (0.0-1.0)
    #[serde(default = "default_retrieval_weight")]
    pub retrieval_weight: f64,
    /// Weight for success rate (request completion) (0.0-1.0)
    #[serde(default = "default_success_rate_weight")]
    pub success_rate_weight: f64,
    /// Minimum target score for overall SLO pass (0.0-1.0)
    #[serde(default = "default_min_target")]
    pub min_target_score: f64,
}

// Default weight functions
fn default_throughput_weight() -> f64 { 0.15 }
fn default_ttft_weight() -> f64 { 0.15 }
fn default_quality_weight() -> f64 { 0.30 }
fn default_groundedness_weight() -> f64 { 0.15 }
fn default_cost_weight() -> f64 { 0.10 }
fn default_retrieval_weight() -> f64 { 0.10 }
fn default_success_rate_weight() -> f64 { 0.05 }
fn default_min_target() -> f64 { 0.85 }

impl Default for CompositeGenAiWeights {
    fn default() -> Self {
        Self {
            throughput_weight: 0.15,
            ttft_weight: 0.15,
            quality_weight: 0.30,
            groundedness_weight: 0.15,
            cost_weight: 0.10,
            retrieval_weight: 0.10,
            success_rate_weight: 0.05,
            min_target_score: 0.85,
        }
    }
}

impl CompositeGenAiWeights {
    /// Validate that weights sum to approximately 1.0 (allowing small floating-point error)
    pub fn validate(&self) -> crate::Result<()> {
        let sum = self.throughput_weight
            + self.ttft_weight
            + self.quality_weight
            + self.groundedness_weight
            + self.cost_weight
            + self.retrieval_weight
            + self.success_rate_weight;
        
        // Allow 1% tolerance for floating-point math
        if (sum - 1.0).abs() > 0.01 {
            return Err(NeuralBudgetError::ConfigError(format!(
                "Composite GenAI weights must sum to 1.0 (got {:.4})",
                sum
            )));
        }
        
        if self.min_target_score < 0.0 || self.min_target_score > 1.0 {
            return Err(NeuralBudgetError::ConfigError(
                "min_target_score must be between 0.0 and 1.0".to_string(),
            ));
        }
        
        Ok(())
    }
}

// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Composite SLO policy for one service node in a dependency graph.
pub struct CompositeServiceSlo {
    pub service: String,
    /// Local service SLO score in `[0.0, 1.0]` before dependency adjustments.
    pub local_score: f64,
    /// Minimum effective score required for this service to pass.
    pub min_pass_score: f64,
    /// Relative impact weight for System Global SLO aggregation.
    pub impact_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Directed edge where `dependent` consumes `dependency`.
pub struct CompositeDependencyEdge {
    /// Upstream dependency service name.
    pub dependency: String,
    /// Downstream dependent service name.
    pub dependent: String,
    /// Penalty in `[0.0, 1.0]` applied to dependent if dependency fails.
    pub failure_penalty: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Composite SLO graph input for DAG traversal and global score computation.
pub struct CompositeSloGraph {
    pub services: Vec<CompositeServiceSlo>,
    pub dependencies: Vec<CompositeDependencyEdge>,
    /// Minimum system score required for the graph to pass globally.
    pub global_min_pass_score: f64,
}

impl Default for CompositeSloGraph {
    fn default() -> Self {
        Self {
            services: Vec::new(),
            dependencies: Vec::new(),
            global_min_pass_score: 0.9,
        }
    }
}

impl CompositeSloGraph {
    /// Evaluate this dependency graph and compute per-service and global SLO outcomes.
    pub fn evaluate(&self) -> std::result::Result<CompositeSloEvaluation, CompositeSloError> {
        evaluate_composite_slo(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Composite SLO evaluation result for one service node.
pub struct CompositeServiceSloEvaluation {
    pub service: String,
    pub local_score: f64,
    pub effective_score: f64,
    pub min_pass_score: f64,
    /// True when at least one failed upstream dependency altered this node.
    pub dependency_adjusted: bool,
    /// Upstream services that failed and impacted this service.
    pub failed_dependencies: Vec<String>,
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Full DAG run output including node evaluations and global SLO.
pub struct CompositeSloEvaluation {
    pub topological_order: Vec<String>,
    pub services: Vec<CompositeServiceSloEvaluation>,
    pub global_slo: f64,
    pub global_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Error conditions returned when composite graph evaluation is invalid.
pub enum CompositeSloError {
    DuplicateService(String),
    DuplicateDependencyEdge {
        dependency: String,
        dependent: String,
    },
    UnknownService(String),
    SelfDependency(String),
    CycleDetected,
}

impl std::fmt::Display for CompositeSloError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateService(name) => write!(f, "duplicate service '{name}'"),
            Self::DuplicateDependencyEdge {
                dependency,
                dependent,
            } => {
                write!(
                    f,
                    "duplicate dependency edge '{dependency}' -> '{dependent}'"
                )
            }
            Self::UnknownService(name) => write!(f, "unknown service '{name}' in dependency graph"),
            Self::SelfDependency(name) => write!(f, "service '{name}' cannot depend on itself"),
            Self::CycleDetected => write!(f, "composite dependency graph contains a cycle"),
        }
    }
}

impl std::error::Error for CompositeSloError {}

pub struct GenAiSloIterator<I>
where
    I: Iterator<Item = GenAiSample>,
{
    slo: GenAiSlo,
    source: I,
}

impl<I> GenAiSloIterator<I>
where
    I: Iterator<Item = GenAiSample>,
{
    pub fn new(slo: GenAiSlo, source: I) -> Self {
        Self { slo, source }
    }
}

impl<I> Iterator for GenAiSloIterator<I>
where
    I: Iterator<Item = GenAiSample>,
{
    type Item = GenAiSloEvaluation;

    fn next(&mut self) -> Option<Self::Item> {
        self.source
            .next()
            .map(|sample| self.slo.evaluate_sample(&sample))
    }
}

pub struct MlSloIterator<I>
where
    I: Iterator<Item = MlSample>,
{
    slo: MlSlo,
    source: I,
}

impl<I> MlSloIterator<I>
where
    I: Iterator<Item = MlSample>,
{
    pub fn new(slo: MlSlo, source: I) -> Self {
        Self { slo, source }
    }
}

impl<I> Iterator for MlSloIterator<I>
where
    I: Iterator<Item = MlSample>,
{
    type Item = MlSloEvaluation;

    fn next(&mut self) -> Option<Self::Item> {
        self.source
            .next()
            .map(|sample| self.slo.evaluate_sample(&sample))
    }
}

pub struct StatefulSloIterator<I>
where
    I: Iterator<Item = StatefulSample>,
{
    slo: StatefulSlo,
    source: I,
}

impl<I> StatefulSloIterator<I>
where
    I: Iterator<Item = StatefulSample>,
{
    pub fn new(slo: StatefulSlo, source: I) -> Self {
        Self { slo, source }
    }
}

impl<I> Iterator for StatefulSloIterator<I>
where
    I: Iterator<Item = StatefulSample>,
{
    type Item = StatefulSloEvaluation;

    fn next(&mut self) -> Option<Self::Item> {
        self.source
            .next()
            .map(|sample| self.slo.evaluate_sample(&sample))
    }
}

pub struct HttpSloIterator<I>
where
    I: Iterator<Item = HistogramSample>,
{
    slo: HttpSlo,
    source: I,
}

impl<I> HttpSloIterator<I>
where
    I: Iterator<Item = HistogramSample>,
{
    pub fn new(slo: HttpSlo, source: I) -> Self {
        Self { slo, source }
    }
}

impl<I> Iterator for HttpSloIterator<I>
where
    I: Iterator<Item = HistogramSample>,
{
    type Item = HttpSloEvaluation;

    fn next(&mut self) -> Option<Self::Item> {
        self.source
            .next()
            .map(|sample| self.slo.evaluate_histogram(&sample))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Supported SLO window styles.
pub enum WindowAlignment {
    /// A fixed-duration window anchored to the current evaluation time.
    Rolling,
    /// A window that snaps to calendar boundaries in a specific timezone.
    CalendarAligned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A concrete SLO time window definition with optional timezone offset.
pub struct TimeWindow {
    /// Whether the window is rolling or calendar aligned.
    pub alignment: WindowAlignment,
    /// Window size in seconds.
    pub window_seconds: u64,
    /// Timezone offset in seconds used for calendar-aligned windows.
    #[serde(default)]
    pub timezone_offset_seconds: i32,
}

impl TimeWindow {
    /// Build a rolling window with UTC semantics.
    pub fn rolling(window_seconds: u64) -> Self {
        Self {
            alignment: WindowAlignment::Rolling,
            window_seconds,
            timezone_offset_seconds: 0,
        }
    }

    /// Build a calendar-aligned window using an explicit timezone offset.
    pub fn calendar_aligned(window_seconds: u64, timezone_offset_seconds: i32) -> Self {
        Self {
            alignment: WindowAlignment::CalendarAligned,
            window_seconds,
            timezone_offset_seconds,
        }
    }

    /// Return `true` when `timestamp` belongs to the active window ending at `now`.
    pub fn contains(&self, timestamp: i64, now: i64) -> bool {
        let window_seconds = match i64::try_from(self.window_seconds) {
            Ok(value) if value > 0 => value,
            _ => return false,
        };

        match self.alignment {
            WindowAlignment::Rolling => {
                // Rolling windows compare directly against the current evaluation time.
                let start = match now.checked_sub(window_seconds) {
                    Some(value) => value,
                    None => return false,
                };

                timestamp >= start && timestamp <= now
            }
            WindowAlignment::CalendarAligned => {
                // Shift both timestamps into the local timezone before snapping to boundaries.
                let offset = i64::from(self.timezone_offset_seconds);
                let local_now = match now.checked_add(offset) {
                    Some(value) => value,
                    None => return false,
                };
                let local_timestamp = match timestamp.checked_add(offset) {
                    Some(value) => value,
                    None => return false,
                };

                let window_start_local = local_now.div_euclid(window_seconds) * window_seconds;
                let window_end_local = match window_start_local.checked_add(window_seconds) {
                    Some(value) => value,
                    None => return false,
                };

                // Calendar-aligned windows are start-inclusive and end-exclusive.
                local_timestamp >= window_start_local && local_timestamp < window_end_local
            }
        }
    }
}

pub trait JsonExt: Sized + Serialize + DeserializeOwned {
    fn from_json_str(input: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    fn to_json_string(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

pub trait YamlExt: Sized + Serialize + DeserializeOwned {
    fn from_yaml_str(input: &str) -> std::result::Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(input)
    }

    fn to_yaml_string(&self) -> std::result::Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

impl JsonExt for SloConfig {}
impl JsonExt for ErrorBudget {}
impl JsonExt for MetricPoint {}
impl JsonExt for WebApiRequest {}
impl JsonExt for OutlierFilterConfig {}
impl JsonExt for WebApiSloPolicy {}
impl JsonExt for WebApiSloReport {}
impl JsonExt for HistogramBucket {}
impl JsonExt for HistogramSample {}
impl JsonExt for HttpSlo {}
impl JsonExt for HttpSloEvaluation {}
impl JsonExt for StatefulSample {}
impl JsonExt for StatefulTier {}
impl JsonExt for StatefulPolicyProfile {}
impl JsonExt for StatefulPolicyProfileSet {}
impl JsonExt for StatefulSlo {}
impl JsonExt for StatefulSloEvaluation {}
impl JsonExt for MlSample {}
impl JsonExt for MlSlo {}
impl JsonExt for MlSloEvaluation {}
impl JsonExt for GenAiSample {}
impl JsonExt for GenAiSlo {}
impl JsonExt for GenAiSloEvaluation {}
impl JsonExt for CompositeServiceSlo {}
impl JsonExt for CompositeDependencyEdge {}
impl JsonExt for CompositeSloGraph {}
impl JsonExt for CompositeServiceSloEvaluation {}
impl JsonExt for CompositeSloEvaluation {}
impl JsonExt for TimeWindow {}

impl YamlExt for SloConfig {}
impl YamlExt for ErrorBudget {}
impl YamlExt for MetricPoint {}
impl YamlExt for WebApiRequest {}
impl YamlExt for OutlierFilterConfig {}
impl YamlExt for WebApiSloPolicy {}
impl YamlExt for WebApiSloReport {}
impl YamlExt for HistogramBucket {}
impl YamlExt for HistogramSample {}
impl YamlExt for HttpSlo {}
impl YamlExt for HttpSloEvaluation {}
impl YamlExt for StatefulSample {}
impl YamlExt for StatefulTier {}
impl YamlExt for StatefulPolicyProfile {}
impl YamlExt for StatefulPolicyProfileSet {}
impl YamlExt for StatefulSlo {}
impl YamlExt for StatefulSloEvaluation {}
impl YamlExt for MlSample {}
impl YamlExt for MlSlo {}
impl YamlExt for MlSloEvaluation {}
impl YamlExt for GenAiSample {}
impl YamlExt for GenAiSlo {}
impl YamlExt for GenAiSloEvaluation {}
impl YamlExt for CompositeServiceSlo {}
impl YamlExt for CompositeDependencyEdge {}
impl YamlExt for CompositeSloGraph {}
impl YamlExt for CompositeServiceSloEvaluation {}
impl YamlExt for CompositeSloEvaluation {}
impl YamlExt for TimeWindow {}

pub(crate) fn missing_key(key: &str) -> PyErr {
    PyKeyError::new_err(format!("missing required key '{key}'"))
}

pub(crate) fn invalid_dict_type(type_name: &str) -> PyErr {
    PyTypeError::new_err(format!("expected dict or {type_name} instance"))
}

pub(crate) fn extract_required<'py, T>(dict: &Bound<'py, PyDict>, key: &str) -> PyResult<T>
where
    T: FromPyObject<'py>,
{
    let value = dict.get_item(key)?.ok_or_else(|| missing_key(key))?;
    value.extract::<T>()
}

pub(crate) fn extract_labels(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, String>> {
    match dict.get_item("labels")? {
        Some(labels) => labels.extract::<HashMap<String, String>>(),
        None => Ok(HashMap::new()),
    }
}

pub(crate) fn parse_window_alignment(value: &str) -> PyResult<WindowAlignment> {
    match value {
        "rolling" => Ok(WindowAlignment::Rolling),
        "calendar_aligned" => Ok(WindowAlignment::CalendarAligned),
        _ => Err(PyTypeError::new_err(
            "alignment must be 'rolling' or 'calendar_aligned'",
        )),
    }
}

pub(crate) fn window_alignment_name(alignment: WindowAlignment) -> &'static str {
    match alignment {
        WindowAlignment::Rolling => "rolling",
        WindowAlignment::CalendarAligned => "calendar_aligned",
    }
}

pub(crate) fn parse_histogram_format(value: &str) -> PyResult<HistogramFormat> {
    match value {
        "prometheus_cumulative" => Ok(HistogramFormat::PrometheusCumulative),
        "open_telemetry_delta" => Ok(HistogramFormat::OpenTelemetryDelta),
        _ => Err(PyTypeError::new_err(
            "format must be 'prometheus_cumulative' or 'open_telemetry_delta'",
        )),
    }
}

pub(crate) fn histogram_format_name(format: HistogramFormat) -> &'static str {
    match format {
        HistogramFormat::PrometheusCumulative => "prometheus_cumulative",
        HistogramFormat::OpenTelemetryDelta => "open_telemetry_delta",
    }
}

pub(crate) fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

/// Compute Median Absolute Deviation (MAD) for a value series.
pub fn calculate_mad(values: &[f64]) -> Option<f64> {
    let center = median(values)?;
    let deviations: Vec<f64> = values.iter().map(|value| (value - center).abs()).collect();
    median(&deviations)
}

pub(crate) fn mad_outlier_mask(
    values: &[f64],
    mad_threshold: f64,
    min_samples: usize,
) -> Vec<bool> {
    if values.len() < min_samples || mad_threshold <= 0.0 {
        return vec![false; values.len()];
    }

    let center = match median(values) {
        Some(value) => value,
        None => return vec![false; values.len()],
    };
    let mad = match calculate_mad(values) {
        Some(value) if value > 0.0 => value,
        _ => return vec![false; values.len()],
    };

    values
        .iter()
        .map(|value| {
            let modified_z = 0.6745 * (value - center) / mad;
            modified_z.abs() > mad_threshold
        })
        .collect()
}

pub(crate) fn percentile_from_histogram(
    buckets: &[HistogramBucket],
    format: HistogramFormat,
    percentile: f64,
) -> Option<f64> {
    if buckets.is_empty() || percentile <= 0.0 {
        return None;
    }

    let mut sorted = buckets.to_vec();
    sorted.sort_by(|a, b| a.upper_bound_ms.total_cmp(&b.upper_bound_ms));

    let mut cumulative: Vec<(f64, f64)> = Vec::with_capacity(sorted.len());
    match format {
        HistogramFormat::PrometheusCumulative => {
            for bucket in &sorted {
                cumulative.push((bucket.upper_bound_ms, bucket.count as f64));
            }
        }
        HistogramFormat::OpenTelemetryDelta => {
            let mut running = 0.0;
            for bucket in &sorted {
                running += bucket.count as f64;
                cumulative.push((bucket.upper_bound_ms, running));
            }
        }
    }

    let total = cumulative.last().map(|(_, count)| *count).unwrap_or(0.0);
    if total <= 0.0 {
        return None;
    }

    let target = percentile.clamp(0.0, 1.0) * total;
    let mut prev_upper = 0.0;
    let mut prev_cumulative = 0.0;

    for (upper, cumulative_count) in cumulative {
        if cumulative_count >= target {
            let bucket_count = cumulative_count - prev_cumulative;
            if bucket_count <= 0.0 {
                return Some(upper);
            }

            let fraction = ((target - prev_cumulative) / bucket_count).clamp(0.0, 1.0);
            if upper.is_infinite() {
                return Some(prev_upper);
            }
            return Some(prev_upper + (upper - prev_upper) * fraction);
        }
        prev_upper = upper;
        prev_cumulative = cumulative_count;
    }

    Some(prev_upper)
}

/// Filter metric outliers using Median Absolute Deviation.
pub fn filter_statistical_outliers(
    metric_stream: &[MetricPoint],
    mad_threshold: f64,
    min_samples: usize,
) -> Vec<MetricPoint> {
    let values: Vec<f64> = metric_stream.iter().map(|point| point.value).collect();
    let outlier_mask = mad_outlier_mask(&values, mad_threshold, min_samples);
    metric_stream
        .iter()
        .zip(outlier_mask)
        .filter_map(|(point, is_outlier)| {
            if is_outlier {
                None
            } else {
                Some(point.clone())
            }
        })
        .collect()
}

#[pyfunction]
/// Compute classic availability as `success / total`.
pub fn calculate_availability(success: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        success as f64 / total as f64
    }
}

#[pyfunction]
/// Calculate the remaining error budget in seconds for a target and window.
pub fn calculate_error_budget(slo_target: f64, time_window_seconds: u64) -> f64 {
    let bounded_target = slo_target.clamp(0.0, 1.0);
    (1.0 - bounded_target) * time_window_seconds as f64
}

#[pyfunction]
/// Calculate burn rate from a metric stream where values above `0.0` consume budget.
pub fn calculate_burn_rate(metric_stream: Vec<MetricPoint>, window_secs: u64) -> f64 {
    if metric_stream.is_empty() || window_secs == 0 {
        return 0.0;
    }

    let window_secs = match i64::try_from(window_secs) {
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

    // Treat the metric stream as one sample per second and count budget-consuming samples.
    let consumed_seconds = metric_stream
        .iter()
        .filter(|point| point.timestamp > window_start && point.timestamp <= now)
        .filter(|point| point.value > 0.0)
        .count() as f64;

    consumed_seconds / window_secs as f64
}

/// Compute a complete SLO report for a generic web API request stream.
pub fn calculate_web_api_slo(
    requests: &[WebApiRequest],
    policy: &WebApiSloPolicy,
    now: i64,
) -> WebApiSloReport {
    let window_secs = match i64::try_from(policy.time_window_seconds) {
        Ok(value) if value > 0 => value,
        _ => {
            return WebApiSloReport {
                total_requests: 0,
                successful_requests: 0,
                availability: 0.0,
                latency_evaluated_requests: 0,
                latency_compliant_requests: 0,
                latency_sli: 0.0,
                filtered_outliers: 0,
                error_budget_seconds: 0.0,
                burn_rate_5m: 0.0,
                burn_rate_1h: 0.0,
            }
        }
    };

    let window_start = match now.checked_sub(window_secs) {
        Some(value) => value,
        None => now,
    };

    let in_window: Vec<&WebApiRequest> = requests
        .iter()
        .filter(|request| request.timestamp > window_start && request.timestamp <= now)
        .collect();

    let total_requests = in_window.len() as u64;
    let successful_requests = in_window
        .iter()
        .filter(|request| request.status_code < 500)
        .count() as u64;
    let availability = calculate_availability(successful_requests, total_requests);

    let latency_values: Vec<f64> = in_window.iter().map(|request| request.latency_ms).collect();
    let outlier_mask = if policy.outlier_filter.enabled {
        mad_outlier_mask(
            &latency_values,
            policy.outlier_filter.mad_threshold,
            policy.outlier_filter.min_samples,
        )
    } else {
        vec![false; latency_values.len()]
    };

    let filtered_outliers = outlier_mask
        .iter()
        .filter(|is_outlier| **is_outlier)
        .count() as u64;
    let latency_evaluated_requests = (outlier_mask.len() as u64).saturating_sub(filtered_outliers);
    let latency_compliant_requests = in_window
        .iter()
        .zip(outlier_mask.iter())
        .filter(|(_, is_outlier)| !**is_outlier)
        .filter(|(request, _)| request.latency_ms <= policy.latency_threshold_ms)
        .count() as u64;
    let latency_sli =
        calculate_availability(latency_compliant_requests, latency_evaluated_requests);

    let burn_stream: Vec<MetricPoint> = in_window
        .iter()
        .map(|request| MetricPoint {
            timestamp: request.timestamp,
            value: if request.status_code >= 500 { 1.0 } else { 0.0 },
            labels: HashMap::new(),
        })
        .collect();

    WebApiSloReport {
        total_requests,
        successful_requests,
        availability,
        latency_evaluated_requests,
        latency_compliant_requests,
        latency_sli,
        filtered_outliers,
        error_budget_seconds: calculate_error_budget(
            policy.availability_target,
            policy.time_window_seconds,
        ),
        burn_rate_5m: calculate_burn_rate(burn_stream.clone(), 300),
        burn_rate_1h: calculate_burn_rate(burn_stream, 3_600),
    }
}

/// Evaluate a Composite SLO dependency DAG and compute the System Global SLO.
pub fn evaluate_composite_slo(
    graph: &CompositeSloGraph,
) -> std::result::Result<CompositeSloEvaluation, CompositeSloError> {
    let graph_index = build_composite_graph_index(graph)?;
    let topological_order = composite_topological_order(&graph_index)?;
    let evaluations_by_service = evaluate_composite_services(&topological_order, &graph_index)?;

    let services: Vec<CompositeServiceSloEvaluation> = topological_order
        .iter()
        .filter_map(|name| evaluations_by_service.get(name).cloned())
        .collect();

    let global_slo = compute_composite_global_slo(&services, &graph_index.services_by_name);

    Ok(CompositeSloEvaluation {
        topological_order,
        services,
        global_slo,
        global_pass: global_slo >= graph.global_min_pass_score.clamp(0.0, 1.0),
    })
}

struct CompositeGraphIndex {
    services_by_name: HashMap<String, CompositeServiceSlo>,
    indegree: HashMap<String, usize>,
    adjacency: HashMap<String, Vec<String>>,
    incoming: HashMap<String, Vec<(String, f64)>>,
}

fn build_composite_graph_index(
    graph: &CompositeSloGraph,
) -> std::result::Result<CompositeGraphIndex, CompositeSloError> {
    let mut services_by_name: HashMap<String, CompositeServiceSlo> = HashMap::new();
    for service in &graph.services {
        if services_by_name
            .insert(service.service.clone(), service.clone())
            .is_some()
        {
            return Err(CompositeSloError::DuplicateService(service.service.clone()));
        }
    }

    let mut indegree: HashMap<String, usize> = services_by_name
        .keys()
        .map(|name| (name.clone(), 0_usize))
        .collect();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();

    for edge in &graph.dependencies {
        if edge.dependency == edge.dependent {
            return Err(CompositeSloError::SelfDependency(edge.dependency.clone()));
        }

        if !services_by_name.contains_key(&edge.dependency) {
            return Err(CompositeSloError::UnknownService(edge.dependency.clone()));
        }
        if !services_by_name.contains_key(&edge.dependent) {
            return Err(CompositeSloError::UnknownService(edge.dependent.clone()));
        }

        let edge_key = (edge.dependency.clone(), edge.dependent.clone());
        if !seen_edges.insert(edge_key.clone()) {
            return Err(CompositeSloError::DuplicateDependencyEdge {
                dependency: edge_key.0,
                dependent: edge_key.1,
            });
        }

        adjacency
            .entry(edge.dependency.clone())
            .or_default()
            .push(edge.dependent.clone());
        incoming.entry(edge.dependent.clone()).or_default().push((
            edge.dependency.clone(),
            edge.failure_penalty.clamp(0.0, 1.0),
        ));

        if let Some(entry) = indegree.get_mut(&edge.dependent) {
            *entry += 1;
        }
    }

    // Keep downstream and upstream dependency iteration deterministic.
    for dependents in adjacency.values_mut() {
        dependents.sort();
    }
    for dependencies in incoming.values_mut() {
        dependencies.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Ok(CompositeGraphIndex {
        services_by_name,
        indegree,
        adjacency,
        incoming,
    })
}

fn composite_topological_order(
    graph_index: &CompositeGraphIndex,
) -> std::result::Result<Vec<String>, CompositeSloError> {
    let mut indegree = graph_index.indegree.clone();
    let mut ready: BinaryHeap<Reverse<String>> = BinaryHeap::new();

    for (name, degree) in &indegree {
        if *degree == 0 {
            ready.push(Reverse(name.clone()));
        }
    }

    let mut topological_order: Vec<String> = Vec::with_capacity(graph_index.services_by_name.len());
    while let Some(Reverse(service)) = ready.pop() {
        topological_order.push(service.clone());
        for dependent in graph_index
            .adjacency
            .get(&service)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(entry) = indegree.get_mut(&dependent) {
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    ready.push(Reverse(dependent));
                }
            }
        }
    }

    if topological_order.len() != graph_index.services_by_name.len() {
        return Err(CompositeSloError::CycleDetected);
    }

    Ok(topological_order)
}

fn evaluate_composite_services(
    topological_order: &[String],
    graph_index: &CompositeGraphIndex,
) -> std::result::Result<HashMap<String, CompositeServiceSloEvaluation>, CompositeSloError> {
    let mut evaluations_by_service: HashMap<String, CompositeServiceSloEvaluation> = HashMap::new();

    for service_name in topological_order {
        let service = match graph_index.services_by_name.get(service_name) {
            Some(node) => node,
            None => return Err(CompositeSloError::UnknownService(service_name.clone())),
        };

        let local_score = service.local_score.clamp(0.0, 1.0);
        let min_pass_score = service.min_pass_score.clamp(0.0, 1.0);
        let mut effective_score = local_score;
        let mut failed_dependencies = Vec::new();

        for (dependency, penalty) in graph_index
            .incoming
            .get(service_name)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(dependency_eval) = evaluations_by_service.get(&dependency) {
                if !dependency_eval.pass {
                    failed_dependencies.push(dependency);
                    effective_score *= 1.0 - penalty;
                }
            }
        }

        effective_score = effective_score.clamp(0.0, 1.0);
        let pass = effective_score >= min_pass_score;

        evaluations_by_service.insert(
            service_name.clone(),
            CompositeServiceSloEvaluation {
                service: service_name.clone(),
                local_score,
                effective_score,
                min_pass_score,
                dependency_adjusted: !failed_dependencies.is_empty(),
                failed_dependencies,
                pass,
            },
        );
    }

    Ok(evaluations_by_service)
}

fn compute_composite_global_slo(
    services: &[CompositeServiceSloEvaluation],
    services_by_name: &HashMap<String, CompositeServiceSlo>,
) -> f64 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for service in services {
        if let Some(policy) = services_by_name.get(&service.service) {
            let weight = policy.impact_weight.max(0.0);
            weighted_sum += service.effective_score * weight;
            total_weight += weight;
        }
    }

    if total_weight > 0.0 {
        (weighted_sum / total_weight).clamp(0.0, 1.0)
    } else if services.is_empty() {
        0.0
    } else {
        (services
            .iter()
            .map(|entry| entry.effective_score)
            .sum::<f64>()
            / services.len() as f64)
            .clamp(0.0, 1.0)
    }
}

impl HttpSlo {
    pub fn evaluate_histogram(&self, sample: &HistogramSample) -> HttpSloEvaluation {
        let availability = calculate_availability(sample.success, sample.total);
        let percentile = self.latency_percentile.clamp(0.0, 1.0);
        let percentile_latency =
            percentile_from_histogram(&sample.buckets, sample.format, percentile)
                .unwrap_or(f64::INFINITY);

        let latency_ok = percentile_latency < self.latency_threshold_ms;
        let availability_ok = availability > self.availability_threshold;

        HttpSloEvaluation {
            timestamp: sample.timestamp,
            availability,
            evaluated_percentile: percentile,
            percentile_latency_ms: percentile_latency,
            latency_ok,
            availability_ok,
            pass: latency_ok && availability_ok,
        }
    }

    /// Calculate error budget as a percentage of the allowed error.
    ///
    /// Returns value in [0, 100] representing % of monthly error budget remaining.
    /// For 99.9% SLO with 0.1% error budget, returns 0-100.
    pub fn calculate_error_budget_percent(&self, current_error_rate: f64) -> f64 {
        let allowed_error_rate = 1.0 - self.availability_threshold;
        if allowed_error_rate <= 0.0 {
            return 100.0; // 100% SLO: no budget
        }

        let remaining = (allowed_error_rate - current_error_rate) / allowed_error_rate;
        (remaining * 100.0).clamp(0.0, 100.0)
    }

    /// Calculate burn rate given an error rate and time window.
    ///
    /// Burn rate indicates how many times faster the error budget is being consumed
    /// compared to a uniform rate over the entire SLO window.
    ///
    /// Example: For 99.9% SLO over 30 days:
    /// - Allowed error = 0.001 (0.1%)
    /// - Daily allowed = 0.001 / 30 = 0.0000333 per day
    /// - If current error rate over 1h = 0.009:
    ///   - Hourly allowed = 0.001 / (30*24) = 0.00000139
    ///   - Burn rate = 0.009 / 0.00000139 ≈ 6475 (very high!)
    pub fn calculate_burn_rate(
        &self,
        error_rate_in_window: f64,
        window_seconds: u64,
        slo_window_seconds: u64,
    ) -> f64 {
        if slo_window_seconds == 0 {
            return 0.0;
        }

        let allowed_error = 1.0 - self.availability_threshold;
        if allowed_error <= 0.0 {
            return 0.0; // 100% SLO: no budget, no burn
        }

        let allowed_error_in_window = allowed_error * (window_seconds as f64 / slo_window_seconds as f64);
        if allowed_error_in_window <= 0.0 {
            return 0.0;
        }

        (error_rate_in_window / allowed_error_in_window).max(0.0)
    }

    /// Check if an error rate violates a burn rate threshold.
    ///
    /// Returns true if burn_rate exceeds threshold for the given window.
    pub fn check_burn_rate_violation(
        &self,
        error_rate: f64,
        window: &BurnRateWindow,
        slo_window_seconds: u64,
    ) -> Result<bool> {
        let window_seconds = window.duration_seconds()?;
        let _burn_rate = self.calculate_burn_rate(error_rate, window_seconds, slo_window_seconds);
        let threshold = window.calculate_error_threshold(self.availability_threshold);
        Ok(error_rate > threshold)
    }
}

impl StatefulSlo {
    pub fn evaluate_sample(&self, sample: &StatefulSample) -> StatefulSloEvaluation {
        let replication_lag_ok = sample.replication_lag_ms <= self.replication_lag_threshold_ms;
        let queue_depth_ok = sample.queue_depth <= self.queue_depth_threshold;
        let connection_pool_ok =
            sample.connection_pool_saturation <= self.connection_pool_saturation_threshold;
        let connection_wait_penalized =
            sample.connection_wait_time_ms > self.connection_wait_time_threshold_ms;

        let mut score = 1.0;
        if !replication_lag_ok {
            score -= 0.25;
        }
        if !queue_depth_ok {
            score -= 0.25;
        }
        if !connection_pool_ok {
            score -= 0.25;
        }

        if connection_wait_penalized {
            let threshold = self.connection_wait_time_threshold_ms.max(1e-9);
            let excess_ratio =
                ((sample.connection_wait_time_ms - threshold) / threshold).clamp(0.0, 1.0);
            score -= self.connection_wait_penalty_weight.max(0.0) * excess_ratio;
        }

        let score = score.clamp(0.0, 1.0);
        let pass = replication_lag_ok
            && queue_depth_ok
            && connection_pool_ok
            && score >= self.min_pass_score.clamp(0.0, 1.0);

        StatefulSloEvaluation {
            timestamp: sample.timestamp,
            replication_lag_ok,
            queue_depth_ok,
            connection_pool_ok,
            connection_wait_penalized,
            score,
            pass,
        }
    }

    pub fn evaluate_sample_with_profile(
        &self,
        sample: &StatefulSample,
        profile: &StatefulPolicyProfile,
    ) -> StatefulSloEvaluation {
        let replication_lag_ok = sample.replication_lag_ms <= self.replication_lag_threshold_ms;
        let queue_depth_ok = sample.queue_depth <= self.queue_depth_threshold;
        let connection_pool_ok =
            sample.connection_pool_saturation <= self.connection_pool_saturation_threshold;
        let connection_wait_penalized =
            sample.connection_wait_time_ms > self.connection_wait_time_threshold_ms;

        let mut weighted_penalty = 0.0;
        if !replication_lag_ok {
            weighted_penalty += profile.replication_lag_weight.max(0.0);
        }
        if !queue_depth_ok {
            weighted_penalty += profile.queue_depth_weight.max(0.0);
        }
        if !connection_pool_ok {
            weighted_penalty += profile.connection_pool_weight.max(0.0);
        }

        if connection_wait_penalized {
            let threshold = self.connection_wait_time_threshold_ms.max(1e-9);
            let excess_ratio =
                ((sample.connection_wait_time_ms - threshold) / threshold).clamp(0.0, 1.0);
            weighted_penalty += profile.connection_wait_penalty_weight.max(0.0) * excess_ratio;
        }

        let total_weight = profile.total_weight().max(1e-9);
        let score = (1.0 - weighted_penalty / total_weight).clamp(0.0, 1.0);
        let pass = replication_lag_ok
            && queue_depth_ok
            && connection_pool_ok
            && score >= profile.min_pass_score.clamp(0.0, 1.0);

        StatefulSloEvaluation {
            timestamp: sample.timestamp,
            replication_lag_ok,
            queue_depth_ok,
            connection_pool_ok,
            connection_wait_penalized,
            score,
            pass,
        }
    }

    pub fn evaluate_sample_for_tier(
        &self,
        sample: &StatefulSample,
        tier: StatefulTier,
        profiles: &StatefulPolicyProfileSet,
    ) -> StatefulSloEvaluation {
        self.evaluate_sample_with_profile(sample, profiles.profile_for_tier(tier))
    }
}

impl MlSlo {
    fn normalized_weights(&self) -> (f64, f64) {
        let latency = self.latency_weight.max(0.0);
        let drift = self.drift_weight.max(0.0);
        let sum = latency + drift;

        if sum <= 0.0 {
            return (0.6, 0.4);
        }

        (latency / sum, drift / sum)
    }

    fn score_below_threshold(sample_value: f64, max_allowed: f64) -> f64 {
        if !sample_value.is_finite() || max_allowed <= 0.0 {
            return 0.0;
        }

        if sample_value <= 0.0 {
            return 1.0;
        }

        (max_allowed / sample_value).clamp(0.0, 1.0)
    }

    fn score_above_threshold(sample_value: f64, min_allowed: f64) -> f64 {
        if !sample_value.is_finite() || min_allowed <= 0.0 {
            return 0.0;
        }

        if sample_value >= min_allowed {
            return 1.0;
        }

        (sample_value / min_allowed).clamp(0.0, 1.0)
    }

    pub fn evaluate_sample(&self, sample: &MlSample) -> MlSloEvaluation {
        let inference_latency_score =
            Self::score_below_threshold(sample.inference_latency_ms, self.max_inference_latency_ms);
        let gpu_utilization_score =
            Self::score_below_threshold(sample.gpu_utilization, self.max_gpu_utilization);
        let feature_drift_score =
            if !sample.feature_drift.is_finite() || self.max_feature_drift <= 0.0 {
                0.0
            } else {
                (1.0 - (sample.feature_drift / self.max_feature_drift)).clamp(0.0, 1.0)
            };
        let prediction_confidence_score = Self::score_above_threshold(
            sample.prediction_confidence,
            self.min_prediction_confidence,
        );

        let system_score =
            ((inference_latency_score + gpu_utilization_score) / 2.0).clamp(0.0, 1.0);
        let latency_score = system_score;
        let drift_score =
            ((feature_drift_score + prediction_confidence_score) / 2.0).clamp(0.0, 1.0);

        let (latency_weight, drift_weight) = self.normalized_weights();
        let hybrid_score =
            (latency_score * latency_weight + drift_score * drift_weight).clamp(0.0, 1.0);
        let pass = hybrid_score >= self.min_pass_score.clamp(0.0, 1.0);

        MlSloEvaluation {
            timestamp: sample.timestamp,
            inference_latency_score,
            gpu_utilization_score,
            system_score,
            latency_score,
            feature_drift_score,
            prediction_confidence_score,
            drift_score,
            latency_weight,
            drift_weight,
            hybrid_score,
            pass,
        }
    }
}

fn lexical_similarity_fallback(reference_text: &str, generated_text: &str) -> f64 {
    let tokenize = |input: &str| -> HashSet<String> {
        input
            .to_lowercase()
            .split_whitespace()
            .map(|token| {
                token
                    .chars()
                    .filter(|ch| ch.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|token| !token.is_empty())
            .collect::<HashSet<_>>()
    };

    let reference_tokens = tokenize(reference_text);
    let generated_tokens = tokenize(generated_text);

    if reference_tokens.is_empty() && generated_tokens.is_empty() {
        return 1.0;
    }

    let intersection = reference_tokens.intersection(&generated_tokens).count() as f64;
    let union = reference_tokens.union(&generated_tokens).count() as f64;
    if union <= 0.0 {
        0.0
    } else {
        (intersection / union).clamp(0.0, 1.0)
    }
}

#[pyfunction]
#[pyo3(signature = (reference_text, generated_text, model_name=None))]
/// Placeholder semantic similarity score using sentence-transformers via PyO3.
///
/// When sentence-transformers is unavailable, this falls back to lexical overlap.
pub fn semantic_similarity_placeholder(
    reference_text: &str,
    generated_text: &str,
    model_name: Option<&str>,
) -> f64 {
    let fallback = lexical_similarity_fallback(reference_text, generated_text);

    if unsafe { pyo3::ffi::Py_IsInitialized() == 0 } {
        return fallback;
    }

    let score = Python::with_gil(|py| -> PyResult<f64> {
        let locals = PyDict::new(py);
        locals.set_item("reference_text", reference_text)?;
        locals.set_item("generated_text", generated_text)?;
        locals.set_item(
            "model_name",
            model_name.unwrap_or("sentence-transformers/all-MiniLM-L6-v2"),
        )?;

        #[allow(deprecated)]
        py.run_bound(
            r#"
from sentence_transformers import SentenceTransformer

_model = SentenceTransformer(model_name)
_embeddings = _model.encode([reference_text, generated_text], normalize_embeddings=True)
_ref = [float(v) for v in _embeddings[0]]
_cand = [float(v) for v in _embeddings[1]]
_ref_norm = sum(v * v for v in _ref) ** 0.5
_cand_norm = sum(v * v for v in _cand) ** 0.5
if _ref_norm <= 0.0 or _cand_norm <= 0.0:
    score = 0.0
else:
    score = float(sum(a * b for a, b in zip(_ref, _cand)) / (_ref_norm * _cand_norm))
"#,
            None,
            Some(&locals),
        )?;

        extract_required::<f64>(&locals, "score")
    })
    .unwrap_or(fallback);

    score.clamp(0.0, 1.0)
}

impl GenAiSlo {
    pub fn evaluate_sample(&self, sample: &GenAiSample) -> GenAiSloEvaluation {
        let tokens_per_second = if sample.generation_duration_ms > 0.0 {
            sample.tokens_generated as f64 / (sample.generation_duration_ms / 1_000.0)
        } else {
            0.0
        };
        let semantic_similarity = semantic_similarity_placeholder(
            &sample.reference_text,
            &sample.generated_text,
            Some(&self.semantic_model_name),
        );

        let tokens_per_second_ok = tokens_per_second >= self.min_tokens_per_second.max(0.0);
        let time_to_first_token_ok =
            sample.time_to_first_token_ms <= self.max_time_to_first_token_ms.max(0.0);
        let semantic_similarity_ok =
            semantic_similarity >= self.min_semantic_similarity.clamp(0.0, 1.0);
        let pass = tokens_per_second_ok && time_to_first_token_ok && semantic_similarity_ok;

        GenAiSloEvaluation {
            timestamp: sample.timestamp,
            tokens_per_second,
            time_to_first_token_ms: sample.time_to_first_token_ms,
            semantic_similarity,
            tokens_per_second_ok,
            time_to_first_token_ok,
            semantic_similarity_ok,
            pass,
        }
    }
}
