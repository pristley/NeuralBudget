#![allow(clippy::useless_conversion)]

use std::collections::HashMap;
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
/// HTTP/gRPC SLO policy: availability + p99 latency objective.
pub struct HttpSlo {
    pub latency_p99_threshold_ms: f64,
    pub availability_threshold: f64,
}

impl Default for HttpSlo {
    fn default() -> Self {
        Self {
            latency_p99_threshold_ms: 200.0,
            availability_threshold: 0.999,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// SLO evaluation output for a single histogram sample.
pub struct HttpSloEvaluation {
    pub timestamp: i64,
    pub availability: f64,
    pub p99_latency_ms: f64,
    pub latency_ok: bool,
    pub availability_ok: bool,
    pub pass: bool,
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
impl JsonYamlExt for TimeWindow {}

fn missing_key(key: &str) -> PyErr {
    PyKeyError::new_err(format!("missing required key '{key}'"))
}

fn invalid_dict_type(type_name: &str) -> PyErr {
    PyTypeError::new_err(format!("expected dict or {type_name} instance"))
}

fn extract_required<'py, T>(dict: &Bound<'py, PyDict>, key: &str) -> PyResult<T>
where
    T: FromPyObject<'py>,
{
    let value = dict.get_item(key)?.ok_or_else(|| missing_key(key))?;
    value.extract::<T>()
}

fn extract_labels(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, String>> {
    match dict.get_item("labels")? {
        Some(labels) => labels.extract::<HashMap<String, String>>(),
        None => Ok(HashMap::new()),
    }
}

fn parse_window_alignment(value: &str) -> PyResult<WindowAlignment> {
    match value {
        "rolling" => Ok(WindowAlignment::Rolling),
        "calendar_aligned" => Ok(WindowAlignment::CalendarAligned),
        _ => Err(PyTypeError::new_err(
            "alignment must be 'rolling' or 'calendar_aligned'",
        )),
    }
}

fn window_alignment_name(alignment: WindowAlignment) -> &'static str {
    match alignment {
        WindowAlignment::Rolling => "rolling",
        WindowAlignment::CalendarAligned => "calendar_aligned",
    }
}

fn median(values: &[f64]) -> Option<f64> {
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

fn mad_outlier_mask(values: &[f64], mad_threshold: f64, min_samples: usize) -> Vec<bool> {
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

fn percentile_from_histogram(
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

impl HttpSlo {
    pub fn evaluate_histogram(&self, sample: &HistogramSample) -> HttpSloEvaluation {
        let availability = calculate_availability(sample.success, sample.total);
        let p99 = percentile_from_histogram(&sample.buckets, sample.format, 0.99)
            .unwrap_or(f64::INFINITY);

        let latency_ok = p99 < self.latency_p99_threshold_ms;
        let availability_ok = availability > self.availability_threshold;

        HttpSloEvaluation {
            timestamp: sample.timestamp,
            availability,
            p99_latency_ms: p99,
            latency_ok,
            availability_ok,
            pass: latency_ok && availability_ok,
        }
    }
}

#[pyfunction]
/// Check whether a timestamp is inside the active SLO window.
pub fn is_timestamp_in_window(timestamp: i64, now: i64, window: TimeWindow) -> bool {
    window.contains(timestamp, now)
}

impl<'py> FromPyObject<'py> for SloConfig {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PySloConfig>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("SloConfig"))?;
        Ok(Self {
            target: extract_required(dict, "target")?,
            window: extract_required(dict, "window")?,
        })
    }
}

impl<'py> FromPyObject<'py> for ErrorBudget {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyErrorBudget>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("ErrorBudget"))?;
        Ok(Self {
            remaining: extract_required(dict, "remaining")?,
            velocity: extract_required(dict, "velocity")?,
        })
    }
}

impl<'py> FromPyObject<'py> for MetricPoint {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyMetricPoint>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("MetricPoint"))?;
        Ok(Self {
            timestamp: extract_required(dict, "timestamp")?,
            value: extract_required(dict, "value")?,
            labels: extract_labels(dict)?,
        })
    }
}

impl<'py> FromPyObject<'py> for TimeWindow {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyTimeWindow>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("TimeWindow"))?;

        let alignment = parse_window_alignment(&extract_required::<String>(dict, "alignment")?)?;
        Ok(Self {
            alignment,
            window_seconds: extract_required(dict, "window_seconds")?,
            timezone_offset_seconds: match dict.get_item("timezone_offset_seconds")? {
                Some(value) => value.extract::<i32>()?,
                None => 0,
            },
        })
    }
}

#[pyclass(name = "SloConfig")]
#[derive(Clone)]
pub struct PySloConfig {
    inner: SloConfig,
}

#[pymethods]
impl PySloConfig {
    #[new]
    fn new(target: f64, window: String) -> Self {
        Self {
            inner: SloConfig { target, window },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<SloConfig>()?,
        })
    }

    #[getter]
    fn target(&self) -> f64 {
        self.inner.target
    }

    #[getter]
    fn window(&self) -> String {
        self.inner.window.clone()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", self.inner.target)?;
        dict.set_item("window", self.inner.window.clone())?;
        Ok(dict)
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }

    fn to_yaml(&self) -> PyResult<String> {
        self.inner
            .to_yaml_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }
}

#[pyclass(name = "ErrorBudget")]
#[derive(Clone)]
pub struct PyErrorBudget {
    inner: ErrorBudget,
}

#[pymethods]
impl PyErrorBudget {
    #[new]
    fn new(remaining: f64, velocity: f64) -> Self {
        Self {
            inner: ErrorBudget {
                remaining,
                velocity,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<ErrorBudget>()?,
        })
    }

    #[getter]
    fn remaining(&self) -> f64 {
        self.inner.remaining
    }

    #[getter]
    fn velocity(&self) -> f64 {
        self.inner.velocity
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("remaining", self.inner.remaining)?;
        dict.set_item("velocity", self.inner.velocity)?;
        Ok(dict)
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }

    fn to_yaml(&self) -> PyResult<String> {
        self.inner
            .to_yaml_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }
}

#[pyclass(name = "MetricPoint")]
#[derive(Clone)]
pub struct PyMetricPoint {
    inner: MetricPoint,
}

#[pymethods]
impl PyMetricPoint {
    #[new]
    #[pyo3(signature = (timestamp, value, labels=None))]
    fn new(timestamp: i64, value: f64, labels: Option<HashMap<String, String>>) -> Self {
        Self {
            inner: MetricPoint {
                timestamp,
                value,
                labels: labels.unwrap_or_default(),
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<MetricPoint>()?,
        })
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn value(&self) -> f64 {
        self.inner.value
    }

    #[getter]
    fn labels(&self) -> HashMap<String, String> {
        self.inner.labels.clone()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("value", self.inner.value)?;
        dict.set_item("labels", self.inner.labels.clone())?;
        Ok(dict)
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }

    fn to_yaml(&self) -> PyResult<String> {
        self.inner
            .to_yaml_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }
}

#[pyclass(name = "TimeWindow")]
#[derive(Clone)]
pub struct PyTimeWindow {
    inner: TimeWindow,
}

#[pymethods]
impl PyTimeWindow {
    #[new]
    #[pyo3(signature = (window_seconds, alignment="rolling", timezone_offset_seconds=0))]
    fn new(window_seconds: u64, alignment: &str, timezone_offset_seconds: i32) -> PyResult<Self> {
        Ok(Self {
            inner: TimeWindow {
                alignment: parse_window_alignment(alignment)?,
                window_seconds,
                timezone_offset_seconds,
            },
        })
    }

    #[staticmethod]
    fn rolling(window_seconds: u64) -> Self {
        Self {
            inner: TimeWindow::rolling(window_seconds),
        }
    }

    #[staticmethod]
    fn calendar_aligned(window_seconds: u64, timezone_offset_seconds: i32) -> Self {
        Self {
            inner: TimeWindow::calendar_aligned(window_seconds, timezone_offset_seconds),
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<TimeWindow>()?,
        })
    }

    #[getter]
    fn alignment(&self) -> String {
        window_alignment_name(self.inner.alignment).to_string()
    }

    #[getter]
    fn window_seconds(&self) -> u64 {
        self.inner.window_seconds
    }

    #[getter]
    fn timezone_offset_seconds(&self) -> i32 {
        self.inner.timezone_offset_seconds
    }

    fn contains(&self, timestamp: i64, now: i64) -> bool {
        self.inner.contains(timestamp, now)
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("alignment", self.alignment())?;
        dict.set_item("window_seconds", self.inner.window_seconds)?;
        dict.set_item(
            "timezone_offset_seconds",
            self.inner.timezone_offset_seconds,
        )?;
        Ok(dict)
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }

    fn to_yaml(&self) -> PyResult<String> {
        self.inner
            .to_yaml_string()
            .map_err(|err| PyTypeError::new_err(err.to_string()))
    }
}

impl From<SloConfig> for PySloConfig {
    fn from(inner: SloConfig) -> Self {
        Self { inner }
    }
}

impl From<ErrorBudget> for PyErrorBudget {
    fn from(inner: ErrorBudget) -> Self {
        Self { inner }
    }
}

impl From<MetricPoint> for PyMetricPoint {
    fn from(inner: MetricPoint) -> Self {
        Self { inner }
    }
}

impl From<TimeWindow> for PyTimeWindow {
    fn from(inner: TimeWindow) -> Self {
        Self { inner }
    }
}

#[pyfunction]
fn coerce_slo_config(config: SloConfig) -> PySloConfig {
    config.into()
}

#[pyfunction]
fn coerce_error_budget(error_budget: ErrorBudget) -> PyErrorBudget {
    error_budget.into()
}

#[pyfunction]
fn coerce_metric_point(metric_point: MetricPoint) -> PyMetricPoint {
    metric_point.into()
}

#[pyfunction]
fn coerce_time_window(time_window: TimeWindow) -> PyTimeWindow {
    time_window.into()
}

#[pymodule]
fn neuralbudget(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = py;
    module.add_class::<PySloConfig>()?;
    module.add_class::<PyErrorBudget>()?;
    module.add_class::<PyMetricPoint>()?;
    module.add_class::<PyTimeWindow>()?;
    module.add_function(wrap_pyfunction!(calculate_availability, module)?)?;
    module.add_function(wrap_pyfunction!(calculate_error_budget, module)?)?;
    module.add_function(wrap_pyfunction!(calculate_burn_rate, module)?)?;
    module.add_function(wrap_pyfunction!(is_timestamp_in_window, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_slo_config, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_error_budget, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_metric_point, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_time_window, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(timestamp: i64, latency_ms: f64, status_code: u16) -> WebApiRequest {
        WebApiRequest {
            timestamp,
            latency_ms,
            status_code,
            labels: HashMap::new(),
        }
    }

    #[test]
    fn calculate_availability_matches_pure_python_ratio() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let success = 47_u64;
            let total = 50_u64;

            let expected: f64 = {
                let builtins = py.import_bound("builtins").unwrap();
                let eval_fn = builtins.getattr("eval").unwrap();
                let globals = PyDict::new_bound(py);
                globals.set_item("__builtins__", &builtins).unwrap();
                let locals = PyDict::new_bound(py);
                locals.set_item("success", success).unwrap();
                locals.set_item("total", total).unwrap();

                eval_fn
                    .call1(("success / total", &globals, &locals))
                    .unwrap()
                    .extract()
                    .unwrap()
            };

            assert_eq!(calculate_availability(success, total), expected);
        });
    }

    #[test]
    fn calculate_error_budget_scales_with_window() {
        let budget = calculate_error_budget(0.99, 3_600);

        assert!((budget - 36.0).abs() < 1e-9);
    }

    #[test]
    fn burn_rate_rises_with_more_recent_consumption() {
        let stream: Vec<MetricPoint> = (0..3_600)
            .map(|timestamp| MetricPoint {
                timestamp,
                value: if timestamp >= 3_300 { 1.0 } else { 0.0 },
                labels: HashMap::new(),
            })
            .collect();

        let five_minute = calculate_burn_rate(stream.clone(), 300);
        let one_hour = calculate_burn_rate(stream, 3_600);

        assert_eq!(five_minute, 1.0);
        assert_eq!(one_hour, 300.0 / 3_600.0);
    }

    #[test]
    fn mad_identifies_large_latency_spike() {
        let values = vec![100.0, 101.0, 99.0, 102.0, 500.0];
        let mad = calculate_mad(&values).unwrap();
        let mask = mad_outlier_mask(&values, 3.5, 3);

        assert!(mad > 0.0);
        assert_eq!(mask, vec![false, false, false, false, true]);
    }

    #[test]
    fn filter_statistical_outliers_removes_single_spike() {
        let stream = vec![
            MetricPoint {
                timestamp: 1,
                value: 100.0,
                labels: HashMap::new(),
            },
            MetricPoint {
                timestamp: 2,
                value: 101.0,
                labels: HashMap::new(),
            },
            MetricPoint {
                timestamp: 3,
                value: 500.0,
                labels: HashMap::new(),
            },
        ];

        let filtered = filter_statistical_outliers(&stream, 3.5, 3);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|point| point.value < 200.0));
    }

    #[test]
    fn web_api_slo_filters_latency_outlier_when_enabled() {
        let requests = vec![
            make_request(1, 120.0, 200),
            make_request(2, 130.0, 200),
            make_request(3, 110.0, 200),
            make_request(4, 4_000.0, 200),
            make_request(5, 115.0, 500),
        ];

        let policy = WebApiSloPolicy {
            availability_target: 0.99,
            latency_threshold_ms: 250.0,
            time_window_seconds: 10,
            outlier_filter: OutlierFilterConfig {
                enabled: true,
                mad_threshold: 3.5,
                min_samples: 3,
            },
        };

        let report = calculate_web_api_slo(&requests, &policy, 6);

        assert_eq!(report.total_requests, 5);
        assert_eq!(report.successful_requests, 4);
        assert!((report.availability - 0.8).abs() < 1e-9);
        assert_eq!(report.filtered_outliers, 1);
        assert_eq!(report.latency_evaluated_requests, 4);
        assert_eq!(report.latency_compliant_requests, 4);
        assert!((report.latency_sli - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rolling_window_detects_recent_timestamps() {
        let window = TimeWindow::rolling(3_600);
        let now = 1_700_000_000_i64;

        assert!(window.contains(now - 3_600, now));
        assert!(window.contains(now - 1, now));
        assert!(!window.contains(now - 3_601, now));
        assert!(!window.contains(now + 1, now));
    }

    #[test]
    fn calendar_aligned_window_uses_timezone_offset() {
        let window = TimeWindow::calendar_aligned(86_400, 18_000);
        let now = 90_000_i64;

        assert!(window.contains(69_000, now));
        assert!(window.contains(104_999, now));
        assert!(window.contains(68_400, now));
        assert!(!window.contains(68_399, now));
        assert!(!window.contains(154_800, now));
    }

    #[test]
    fn python_window_function_matches_rust_logic() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let module = PyModule::new_bound(py, "neuralbudget_test").unwrap();
            module
                .add_function(wrap_pyfunction!(is_timestamp_in_window, &module).unwrap())
                .unwrap();

            let window = TimeWindow::calendar_aligned(86_400, 18_000);
            let py_window = PyTimeWindow::from(window.clone());
            let actual: bool = module
                .getattr("is_timestamp_in_window")
                .unwrap()
                .call1((69_000_i64, 90_000_i64, py_window))
                .unwrap()
                .extract()
                .unwrap();

            assert!(actual);
            assert!(window.contains(69_000, 90_000));
        });
    }

    #[test]
    fn slo_config_round_trips_through_json_and_yaml() {
        let config = SloConfig {
            target: 99.9,
            window: "30d".to_string(),
        };

        let json = config.to_json_string().unwrap();
        let yaml = config.to_yaml_string().unwrap();

        assert_eq!(SloConfig::from_json_str(&json).unwrap(), config);
        assert_eq!(SloConfig::from_yaml_str(&yaml).unwrap(), config);
    }

    #[test]
    fn metric_point_defaults_labels_when_absent() {
        let yaml = "timestamp: 1719220000\nvalue: 0.999\n";
        let point = MetricPoint::from_yaml_str(yaml).unwrap();

        assert!(point.labels.is_empty());
    }

    #[test]
    fn http_slo_iterator_passes_for_prometheus_histogram() {
        let slo = HttpSlo::default();
        let sample = HistogramSample {
            timestamp: 1,
            success: 10_000,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 9_700,
                },
                HistogramBucket {
                    upper_bound_ms: 150.0,
                    count: 9_900,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 9_970,
                },
                HistogramBucket {
                    upper_bound_ms: 300.0,
                    count: 10_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        };

        let mut iter = HttpSloIterator::new(slo, vec![sample].into_iter());
        let result = iter.next().unwrap();

        assert!(result.p99_latency_ms < 200.0);
        assert!(result.latency_ok);
        assert!(result.availability_ok);
        assert!(result.pass);
        assert!(iter.next().is_none());
    }

    #[test]
    fn http_slo_iterator_fails_when_p99_or_availability_miss() {
        let slo = HttpSlo::default();
        let samples = vec![
            HistogramSample {
                timestamp: 1,
                success: 10_000,
                total: 10_000,
                buckets: vec![
                    HistogramBucket {
                        upper_bound_ms: 100.0,
                        count: 9_500,
                    },
                    HistogramBucket {
                        upper_bound_ms: 200.0,
                        count: 9_600,
                    },
                    HistogramBucket {
                        upper_bound_ms: 500.0,
                        count: 10_000,
                    },
                ],
                format: HistogramFormat::PrometheusCumulative,
            },
            HistogramSample {
                timestamp: 2,
                success: 998,
                total: 1_000,
                buckets: vec![
                    HistogramBucket {
                        upper_bound_ms: 100.0,
                        count: 970,
                    },
                    HistogramBucket {
                        upper_bound_ms: 150.0,
                        count: 995,
                    },
                    HistogramBucket {
                        upper_bound_ms: 200.0,
                        count: 1_000,
                    },
                ],
                format: HistogramFormat::PrometheusCumulative,
            },
        ];

        let results: Vec<HttpSloEvaluation> =
            HttpSloIterator::new(slo, samples.into_iter()).collect();
        assert_eq!(results.len(), 2);

        assert!(!results[0].latency_ok);
        assert!(results[0].availability_ok);
        assert!(!results[0].pass);

        assert!(results[1].latency_ok);
        assert!(!results[1].availability_ok);
        assert!(!results[1].pass);
    }

    #[test]
    fn http_slo_iterator_supports_opentelemetry_delta_buckets() {
        let slo = HttpSlo::default();
        let sample = HistogramSample {
            timestamp: 1,
            success: 9_995,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 9_700,
                },
                HistogramBucket {
                    upper_bound_ms: 150.0,
                    count: 200,
                },
                HistogramBucket {
                    upper_bound_ms: 180.0,
                    count: 70,
                },
                HistogramBucket {
                    upper_bound_ms: 220.0,
                    count: 30,
                },
            ],
            format: HistogramFormat::OpenTelemetryDelta,
        };

        let result = HttpSloIterator::new(slo, vec![sample].into_iter())
            .next()
            .unwrap();

        assert!(result.p99_latency_ms < 200.0);
        assert!(result.pass);
    }
}
