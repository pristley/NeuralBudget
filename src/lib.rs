use std::collections::HashMap;

use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SloConfig {
    pub target: f64,
    pub window: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorBudget {
    pub remaining: f64,
    pub velocity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: i64,
    pub value: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
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

impl<'py> FromPyObject<'py> for SloConfig {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PySloConfig>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj.downcast::<PyDict>().map_err(|_| invalid_dict_type("SloConfig"))?;
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

        let dict = obj.downcast::<PyDict>().map_err(|_| invalid_dict_type("ErrorBudget"))?;
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

        let dict = obj.downcast::<PyDict>().map_err(|_| invalid_dict_type("MetricPoint"))?;
        Ok(Self {
            timestamp: extract_required(dict, "timestamp")?,
            value: extract_required(dict, "value")?,
            labels: extract_labels(dict)?,
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

#[pymodule]
fn neuralbudget(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = py;
    module.add_class::<PySloConfig>()?;
    module.add_class::<PyErrorBudget>()?;
    module.add_class::<PyMetricPoint>()?;
    module.add_function(wrap_pyfunction!(coerce_slo_config, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_error_budget, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_metric_point, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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