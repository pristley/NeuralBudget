#![allow(clippy::useless_conversion)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;

use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Basic SLO target metadata used by the Rust and Python surfaces.
pub struct SloConfig {
    pub target: f64,
    pub window: String,
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
    UnknownService(String),
    SelfDependency(String),
    CycleDetected,
}

impl std::fmt::Display for CompositeSloError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateService(name) => write!(f, "duplicate service '{name}'"),
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

pub trait JsonYamlExt: Sized + Serialize + DeserializeOwned {
    fn from_json_str(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    fn from_yaml_str(input: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(input)
    }

    fn to_yaml_string(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

impl JsonYamlExt for SloConfig {}
impl JsonYamlExt for ErrorBudget {}
impl JsonYamlExt for MetricPoint {}
impl JsonYamlExt for WebApiRequest {}
impl JsonYamlExt for OutlierFilterConfig {}
impl JsonYamlExt for WebApiSloPolicy {}
impl JsonYamlExt for WebApiSloReport {}
impl JsonYamlExt for HistogramBucket {}
impl JsonYamlExt for HistogramSample {}
impl JsonYamlExt for HttpSlo {}
impl JsonYamlExt for HttpSloEvaluation {}
impl JsonYamlExt for StatefulSample {}
impl JsonYamlExt for StatefulTier {}
impl JsonYamlExt for StatefulPolicyProfile {}
impl JsonYamlExt for StatefulPolicyProfileSet {}
impl JsonYamlExt for StatefulSlo {}
impl JsonYamlExt for StatefulSloEvaluation {}
impl JsonYamlExt for MlSample {}
impl JsonYamlExt for MlSlo {}
impl JsonYamlExt for MlSloEvaluation {}
impl JsonYamlExt for GenAiSample {}
impl JsonYamlExt for GenAiSlo {}
impl JsonYamlExt for GenAiSloEvaluation {}
impl JsonYamlExt for CompositeServiceSlo {}
impl JsonYamlExt for CompositeDependencyEdge {}
impl JsonYamlExt for CompositeSloGraph {}
impl JsonYamlExt for CompositeServiceSloEvaluation {}
impl JsonYamlExt for CompositeSloEvaluation {}
impl JsonYamlExt for TimeWindow {}

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
) -> Result<CompositeSloEvaluation, CompositeSloError> {
    let mut services_by_name: HashMap<String, &CompositeServiceSlo> = HashMap::new();
    for service in &graph.services {
        if services_by_name
            .insert(service.service.clone(), service)
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

        adjacency
            .entry(edge.dependency.clone())
            .or_default()
            .push(edge.dependent.clone());
        incoming
            .entry(edge.dependent.clone())
            .or_default()
            .push((edge.dependency.clone(), edge.failure_penalty.clamp(0.0, 1.0)));

        if let Some(entry) = indegree.get_mut(&edge.dependent) {
            *entry += 1;
        }
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter_map(|(name, degree)| {
            if *degree == 0 {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();
    let mut topological_order: Vec<String> = Vec::with_capacity(services_by_name.len());

    while let Some(service) = queue.pop_front() {
        topological_order.push(service.clone());
        for dependent in adjacency.get(&service).cloned().unwrap_or_default() {
            if let Some(entry) = indegree.get_mut(&dependent) {
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    queue.push_back(dependent);
                }
            }
        }
    }

    if topological_order.len() != services_by_name.len() {
        return Err(CompositeSloError::CycleDetected);
    }

    let mut evaluations_by_service: HashMap<String, CompositeServiceSloEvaluation> = HashMap::new();
    for service_name in &topological_order {
        let service = match services_by_name.get(service_name) {
            Some(node) => *node,
            None => return Err(CompositeSloError::UnknownService(service_name.clone())),
        };

        let local_score = service.local_score.clamp(0.0, 1.0);
        let min_pass_score = service.min_pass_score.clamp(0.0, 1.0);
        let mut effective_score = local_score;
        let mut failed_dependencies = Vec::new();

        for (dependency, penalty) in incoming.get(service_name).cloned().unwrap_or_default() {
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

    let services: Vec<CompositeServiceSloEvaluation> = topological_order
        .iter()
        .filter_map(|name| evaluations_by_service.get(name).cloned())
        .collect();

    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for service in &graph.services {
        if let Some(evaluation) = evaluations_by_service.get(&service.service) {
            let weight = service.impact_weight.max(0.0);
            weighted_sum += evaluation.effective_score * weight;
            total_weight += weight;
        }
    }

    let global_slo = if total_weight > 0.0 {
        (weighted_sum / total_weight).clamp(0.0, 1.0)
    } else if services.is_empty() {
        0.0
    } else {
        (services.iter().map(|entry| entry.effective_score).sum::<f64>() / services.len() as f64)
            .clamp(0.0, 1.0)
    };

    Ok(CompositeSloEvaluation {
        topological_order,
        services,
        global_slo,
        global_pass: global_slo >= graph.global_min_pass_score.clamp(0.0, 1.0),
    })
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
        let locals = PyDict::new_bound(py);
        locals.set_item("reference_text", reference_text)?;
        locals.set_item("generated_text", generated_text)?;
        locals.set_item(
            "model_name",
            model_name.unwrap_or("sentence-transformers/all-MiniLM-L6-v2"),
        )?;

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
