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
}
