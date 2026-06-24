use std::collections::HashMap;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::core::*;

/// Check whether a timestamp is inside the active SLO window.
#[pyfunction]
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

impl<'py> FromPyObject<'py> for HistogramBucket {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyHistogramBucket>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("HistogramBucket"))?;
        Ok(Self {
            upper_bound_ms: extract_required(dict, "upper_bound_ms")?,
            count: extract_required(dict, "count")?,
        })
    }
}

impl<'py> FromPyObject<'py> for HistogramSample {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyHistogramSample>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("HistogramSample"))?;

        let format = parse_histogram_format(&extract_required::<String>(dict, "format")?)?;
        Ok(Self {
            timestamp: extract_required(dict, "timestamp")?,
            success: extract_required(dict, "success")?,
            total: extract_required(dict, "total")?,
            buckets: extract_required(dict, "buckets")?,
            format,
        })
    }
}

impl<'py> FromPyObject<'py> for HttpSlo {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyHttpSlo>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("HttpSlo"))?;
        let latency_threshold_ms = match dict.get_item("latency_threshold_ms")? {
            Some(value) => value.extract::<f64>()?,
            None => match dict.get_item("latency_p99_threshold_ms")? {
                Some(value) => value.extract::<f64>()?,
                None => HttpSlo::default().latency_threshold_ms,
            },
        };
        let latency_percentile = match dict.get_item("latency_percentile")? {
            Some(value) => value.extract::<f64>()?,
            None => HttpSlo::default().latency_percentile,
        };
        let availability_threshold = match dict.get_item("availability_threshold")? {
            Some(value) => value.extract::<f64>()?,
            None => HttpSlo::default().availability_threshold,
        };

        Ok(Self {
            latency_threshold_ms,
            latency_percentile,
            availability_threshold,
        })
    }
}

impl<'py> FromPyObject<'py> for StatefulSample {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyStatefulSample>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("StatefulSample"))?;
        Ok(Self {
            timestamp: extract_required(dict, "timestamp")?,
            replication_lag_ms: extract_required(dict, "replication_lag_ms")?,
            queue_depth: extract_required(dict, "queue_depth")?,
            connection_pool_saturation: extract_required(dict, "connection_pool_saturation")?,
            connection_wait_time_ms: extract_required(dict, "connection_wait_time_ms")?,
        })
    }
}

impl<'py> FromPyObject<'py> for StatefulSlo {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyStatefulSlo>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("StatefulSlo"))?;
        Ok(Self {
            replication_lag_threshold_ms: extract_required(dict, "replication_lag_threshold_ms")?,
            queue_depth_threshold: extract_required(dict, "queue_depth_threshold")?,
            connection_pool_saturation_threshold: extract_required(
                dict,
                "connection_pool_saturation_threshold",
            )?,
            connection_wait_time_threshold_ms: extract_required(
                dict,
                "connection_wait_time_threshold_ms",
            )?,
            connection_wait_penalty_weight: extract_required(
                dict,
                "connection_wait_penalty_weight",
            )?,
            min_pass_score: extract_required(dict, "min_pass_score")?,
        })
    }
}

impl<'py> FromPyObject<'py> for MlSample {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyMlSample>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("MlSample"))?;
        Ok(Self {
            timestamp: extract_required(dict, "timestamp")?,
            inference_latency_ms: extract_required(dict, "inference_latency_ms")?,
            gpu_utilization: extract_required(dict, "gpu_utilization")?,
            feature_drift: extract_required(dict, "feature_drift")?,
            prediction_confidence: extract_required(dict, "prediction_confidence")?,
        })
    }
}

impl<'py> FromPyObject<'py> for MlSlo {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyMlSlo>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("MlSlo"))?;
        Ok(Self {
            max_inference_latency_ms: extract_required(dict, "max_inference_latency_ms")?,
            max_gpu_utilization: extract_required(dict, "max_gpu_utilization")?,
            max_feature_drift: extract_required(dict, "max_feature_drift")?,
            min_prediction_confidence: extract_required(dict, "min_prediction_confidence")?,
            latency_weight: match dict.get_item("latency_weight")? {
                Some(value) => value.extract::<f64>()?,
                None => MlSlo::default().latency_weight,
            },
            drift_weight: match dict.get_item("drift_weight")? {
                Some(value) => value.extract::<f64>()?,
                None => MlSlo::default().drift_weight,
            },
            min_pass_score: match dict.get_item("min_pass_score")? {
                Some(value) => value.extract::<f64>()?,
                None => MlSlo::default().min_pass_score,
            },
        })
    }
}

impl<'py> FromPyObject<'py> for GenAiSample {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyGenAiSample>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("GenAiSample"))?;
        Ok(Self {
            timestamp: extract_required(dict, "timestamp")?,
            tokens_generated: extract_required(dict, "tokens_generated")?,
            generation_duration_ms: extract_required(dict, "generation_duration_ms")?,
            time_to_first_token_ms: extract_required(dict, "time_to_first_token_ms")?,
            reference_text: extract_required(dict, "reference_text")?,
            generated_text: extract_required(dict, "generated_text")?,
        })
    }
}

impl<'py> FromPyObject<'py> for GenAiSlo {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(wrapper) = obj.extract::<PyRef<'py, PyGenAiSlo>>() {
            return Ok(wrapper.inner.clone());
        }

        let dict = obj
            .downcast::<PyDict>()
            .map_err(|_| invalid_dict_type("GenAiSlo"))?;
        Ok(Self {
            min_tokens_per_second: extract_required(dict, "min_tokens_per_second")?,
            max_time_to_first_token_ms: extract_required(dict, "max_time_to_first_token_ms")?,
            min_semantic_similarity: extract_required(dict, "min_semantic_similarity")?,
            semantic_model_name: match dict.get_item("semantic_model_name")? {
                Some(value) => value.extract::<String>()?,
                None => GenAiSlo::default().semantic_model_name,
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

#[pyclass(name = "HistogramBucket")]
#[derive(Clone)]
pub struct PyHistogramBucket {
    inner: HistogramBucket,
}

#[pymethods]
impl PyHistogramBucket {
    #[new]
    fn new(upper_bound_ms: f64, count: u64) -> Self {
        Self {
            inner: HistogramBucket {
                upper_bound_ms,
                count,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<HistogramBucket>()?,
        })
    }

    #[getter]
    fn upper_bound_ms(&self) -> f64 {
        self.inner.upper_bound_ms
    }

    #[getter]
    fn count(&self) -> u64 {
        self.inner.count
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("upper_bound_ms", self.inner.upper_bound_ms)?;
        dict.set_item("count", self.inner.count)?;
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

#[pyclass(name = "HistogramSample")]
#[derive(Clone)]
pub struct PyHistogramSample {
    inner: HistogramSample,
}

#[pymethods]
impl PyHistogramSample {
    #[new]
    #[pyo3(signature = (timestamp, success, total, buckets, format="prometheus_cumulative"))]
    fn new(
        timestamp: i64,
        success: u64,
        total: u64,
        buckets: Vec<HistogramBucket>,
        format: &str,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: HistogramSample {
                timestamp,
                success,
                total,
                buckets,
                format: parse_histogram_format(format)?,
            },
        })
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<HistogramSample>()?,
        })
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn success(&self) -> u64 {
        self.inner.success
    }

    #[getter]
    fn total(&self) -> u64 {
        self.inner.total
    }

    #[getter]
    fn buckets(&self) -> Vec<PyHistogramBucket> {
        self.inner.buckets.iter().cloned().map(Into::into).collect()
    }

    #[getter]
    fn format(&self) -> String {
        histogram_format_name(self.inner.format).to_string()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        let buckets: Vec<Bound<'_, PyDict>> = self
            .buckets()
            .iter()
            .map(|bucket| bucket.to_dict(py))
            .collect::<PyResult<Vec<_>>>()?;

        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("success", self.inner.success)?;
        dict.set_item("total", self.inner.total)?;
        dict.set_item("buckets", buckets)?;
        dict.set_item("format", self.format())?;
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

#[pyclass(name = "HttpSloEvaluation")]
#[derive(Clone)]
pub struct PyHttpSloEvaluation {
    inner: HttpSloEvaluation,
}

#[pymethods]
impl PyHttpSloEvaluation {
    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn availability(&self) -> f64 {
        self.inner.availability
    }

    #[getter]
    fn evaluated_percentile(&self) -> f64 {
        self.inner.evaluated_percentile
    }

    #[getter]
    fn percentile_latency_ms(&self) -> f64 {
        self.inner.percentile_latency_ms
    }

    // Backward-compatible alias for older clients that still read p99_latency_ms.
    #[getter]
    fn p99_latency_ms(&self) -> f64 {
        self.inner.percentile_latency_ms
    }

    #[getter]
    fn latency_ok(&self) -> bool {
        self.inner.latency_ok
    }

    #[getter]
    fn availability_ok(&self) -> bool {
        self.inner.availability_ok
    }

    #[getter]
    fn pass(&self) -> bool {
        self.inner.pass
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("availability", self.inner.availability)?;
        dict.set_item("evaluated_percentile", self.inner.evaluated_percentile)?;
        dict.set_item("percentile_latency_ms", self.inner.percentile_latency_ms)?;
        dict.set_item("p99_latency_ms", self.inner.percentile_latency_ms)?;
        dict.set_item("latency_ok", self.inner.latency_ok)?;
        dict.set_item("availability_ok", self.inner.availability_ok)?;
        dict.set_item("pass", self.inner.pass)?;
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

#[pyclass(name = "HttpSlo")]
#[derive(Clone)]
pub struct PyHttpSlo {
    inner: HttpSlo,
}

#[pymethods]
impl PyHttpSlo {
    #[new]
    #[pyo3(signature = (latency_threshold_ms=200.0, latency_percentile=0.99, availability_threshold=0.999))]
    fn new(
        latency_threshold_ms: f64,
        latency_percentile: f64,
        availability_threshold: f64,
    ) -> Self {
        Self {
            inner: HttpSlo {
                latency_threshold_ms,
                latency_percentile,
                availability_threshold,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<HttpSlo>()?,
        })
    }

    #[getter]
    fn latency_threshold_ms(&self) -> f64 {
        self.inner.latency_threshold_ms
    }

    // Backward-compatible alias for older clients.
    #[getter]
    fn latency_p99_threshold_ms(&self) -> f64 {
        self.inner.latency_threshold_ms
    }

    #[getter]
    fn latency_percentile(&self) -> f64 {
        self.inner.latency_percentile
    }

    #[getter]
    fn availability_threshold(&self) -> f64 {
        self.inner.availability_threshold
    }

    fn evaluate_histogram(&self, sample: HistogramSample) -> PyHttpSloEvaluation {
        self.inner.evaluate_histogram(&sample).into()
    }

    fn evaluate_stream(&self, samples: Vec<HistogramSample>) -> Vec<PyHttpSloEvaluation> {
        HttpSloIterator::new(self.inner.clone(), samples.into_iter())
            .map(Into::into)
            .collect()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("latency_threshold_ms", self.inner.latency_threshold_ms)?;
        dict.set_item("latency_p99_threshold_ms", self.inner.latency_threshold_ms)?;
        dict.set_item("latency_percentile", self.inner.latency_percentile)?;
        dict.set_item("availability_threshold", self.inner.availability_threshold)?;
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

#[pyclass(name = "StatefulSample")]
#[derive(Clone)]
pub struct PyStatefulSample {
    inner: StatefulSample,
}

#[pymethods]
impl PyStatefulSample {
    #[new]
    fn new(
        timestamp: i64,
        replication_lag_ms: f64,
        queue_depth: u64,
        connection_pool_saturation: f64,
        connection_wait_time_ms: f64,
    ) -> Self {
        Self {
            inner: StatefulSample {
                timestamp,
                replication_lag_ms,
                queue_depth,
                connection_pool_saturation,
                connection_wait_time_ms,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<StatefulSample>()?,
        })
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn replication_lag_ms(&self) -> f64 {
        self.inner.replication_lag_ms
    }

    #[getter]
    fn queue_depth(&self) -> u64 {
        self.inner.queue_depth
    }

    #[getter]
    fn connection_pool_saturation(&self) -> f64 {
        self.inner.connection_pool_saturation
    }

    #[getter]
    fn connection_wait_time_ms(&self) -> f64 {
        self.inner.connection_wait_time_ms
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("replication_lag_ms", self.inner.replication_lag_ms)?;
        dict.set_item("queue_depth", self.inner.queue_depth)?;
        dict.set_item(
            "connection_pool_saturation",
            self.inner.connection_pool_saturation,
        )?;
        dict.set_item(
            "connection_wait_time_ms",
            self.inner.connection_wait_time_ms,
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

#[pyclass(name = "StatefulSloEvaluation")]
#[derive(Clone)]
pub struct PyStatefulSloEvaluation {
    inner: StatefulSloEvaluation,
}

#[pymethods]
impl PyStatefulSloEvaluation {
    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn replication_lag_ok(&self) -> bool {
        self.inner.replication_lag_ok
    }

    #[getter]
    fn queue_depth_ok(&self) -> bool {
        self.inner.queue_depth_ok
    }

    #[getter]
    fn connection_pool_ok(&self) -> bool {
        self.inner.connection_pool_ok
    }

    #[getter]
    fn connection_wait_penalized(&self) -> bool {
        self.inner.connection_wait_penalized
    }

    #[getter]
    fn score(&self) -> f64 {
        self.inner.score
    }

    #[getter]
    fn pass(&self) -> bool {
        self.inner.pass
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("replication_lag_ok", self.inner.replication_lag_ok)?;
        dict.set_item("queue_depth_ok", self.inner.queue_depth_ok)?;
        dict.set_item("connection_pool_ok", self.inner.connection_pool_ok)?;
        dict.set_item(
            "connection_wait_penalized",
            self.inner.connection_wait_penalized,
        )?;
        dict.set_item("score", self.inner.score)?;
        dict.set_item("pass", self.inner.pass)?;
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

#[pyclass(name = "StatefulSlo")]
#[derive(Clone)]
pub struct PyStatefulSlo {
    inner: StatefulSlo,
}

#[pymethods]
impl PyStatefulSlo {
    #[new]
    #[pyo3(signature = (
        replication_lag_threshold_ms=250.0,
        queue_depth_threshold=1000,
        connection_pool_saturation_threshold=0.8,
        connection_wait_time_threshold_ms=20.0,
        connection_wait_penalty_weight=0.2,
        min_pass_score=0.9
    ))]
    fn new(
        replication_lag_threshold_ms: f64,
        queue_depth_threshold: u64,
        connection_pool_saturation_threshold: f64,
        connection_wait_time_threshold_ms: f64,
        connection_wait_penalty_weight: f64,
        min_pass_score: f64,
    ) -> Self {
        Self {
            inner: StatefulSlo {
                replication_lag_threshold_ms,
                queue_depth_threshold,
                connection_pool_saturation_threshold,
                connection_wait_time_threshold_ms,
                connection_wait_penalty_weight,
                min_pass_score,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<StatefulSlo>()?,
        })
    }

    #[getter]
    fn replication_lag_threshold_ms(&self) -> f64 {
        self.inner.replication_lag_threshold_ms
    }

    #[getter]
    fn queue_depth_threshold(&self) -> u64 {
        self.inner.queue_depth_threshold
    }

    #[getter]
    fn connection_pool_saturation_threshold(&self) -> f64 {
        self.inner.connection_pool_saturation_threshold
    }

    #[getter]
    fn connection_wait_time_threshold_ms(&self) -> f64 {
        self.inner.connection_wait_time_threshold_ms
    }

    #[getter]
    fn connection_wait_penalty_weight(&self) -> f64 {
        self.inner.connection_wait_penalty_weight
    }

    #[getter]
    fn min_pass_score(&self) -> f64 {
        self.inner.min_pass_score
    }

    fn evaluate_sample(&self, sample: StatefulSample) -> PyStatefulSloEvaluation {
        self.inner.evaluate_sample(&sample).into()
    }

    fn evaluate_stream(&self, samples: Vec<StatefulSample>) -> Vec<PyStatefulSloEvaluation> {
        StatefulSloIterator::new(self.inner.clone(), samples.into_iter())
            .map(Into::into)
            .collect()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item(
            "replication_lag_threshold_ms",
            self.inner.replication_lag_threshold_ms,
        )?;
        dict.set_item("queue_depth_threshold", self.inner.queue_depth_threshold)?;
        dict.set_item(
            "connection_pool_saturation_threshold",
            self.inner.connection_pool_saturation_threshold,
        )?;
        dict.set_item(
            "connection_wait_time_threshold_ms",
            self.inner.connection_wait_time_threshold_ms,
        )?;
        dict.set_item(
            "connection_wait_penalty_weight",
            self.inner.connection_wait_penalty_weight,
        )?;
        dict.set_item("min_pass_score", self.inner.min_pass_score)?;
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

#[pyclass(name = "MlSample")]
#[derive(Clone)]
pub struct PyMlSample {
    inner: MlSample,
}

#[pymethods]
impl PyMlSample {
    #[new]
    fn new(
        timestamp: i64,
        inference_latency_ms: f64,
        gpu_utilization: f64,
        feature_drift: f64,
        prediction_confidence: f64,
    ) -> Self {
        Self {
            inner: MlSample {
                timestamp,
                inference_latency_ms,
                gpu_utilization,
                feature_drift,
                prediction_confidence,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<MlSample>()?,
        })
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn inference_latency_ms(&self) -> f64 {
        self.inner.inference_latency_ms
    }

    #[getter]
    fn gpu_utilization(&self) -> f64 {
        self.inner.gpu_utilization
    }

    #[getter]
    fn feature_drift(&self) -> f64 {
        self.inner.feature_drift
    }

    #[getter]
    fn prediction_confidence(&self) -> f64 {
        self.inner.prediction_confidence
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("inference_latency_ms", self.inner.inference_latency_ms)?;
        dict.set_item("gpu_utilization", self.inner.gpu_utilization)?;
        dict.set_item("feature_drift", self.inner.feature_drift)?;
        dict.set_item("prediction_confidence", self.inner.prediction_confidence)?;
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

#[pyclass(name = "MlSloEvaluation")]
#[derive(Clone)]
pub struct PyMlSloEvaluation {
    inner: MlSloEvaluation,
}

#[pymethods]
impl PyMlSloEvaluation {
    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn inference_latency_score(&self) -> f64 {
        self.inner.inference_latency_score
    }

    #[getter]
    fn gpu_utilization_score(&self) -> f64 {
        self.inner.gpu_utilization_score
    }

    #[getter]
    fn system_score(&self) -> f64 {
        self.inner.system_score
    }

    #[getter]
    fn latency_score(&self) -> f64 {
        self.inner.latency_score
    }

    #[getter]
    fn feature_drift_score(&self) -> f64 {
        self.inner.feature_drift_score
    }

    #[getter]
    fn prediction_confidence_score(&self) -> f64 {
        self.inner.prediction_confidence_score
    }

    #[getter]
    fn drift_score(&self) -> f64 {
        self.inner.drift_score
    }

    #[getter]
    fn latency_weight(&self) -> f64 {
        self.inner.latency_weight
    }

    #[getter]
    fn drift_weight(&self) -> f64 {
        self.inner.drift_weight
    }

    #[getter]
    fn hybrid_score(&self) -> f64 {
        self.inner.hybrid_score
    }

    #[getter]
    fn pass(&self) -> bool {
        self.inner.pass
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item(
            "inference_latency_score",
            self.inner.inference_latency_score,
        )?;
        dict.set_item("gpu_utilization_score", self.inner.gpu_utilization_score)?;
        dict.set_item("system_score", self.inner.system_score)?;
        dict.set_item("latency_score", self.inner.latency_score)?;
        dict.set_item("feature_drift_score", self.inner.feature_drift_score)?;
        dict.set_item(
            "prediction_confidence_score",
            self.inner.prediction_confidence_score,
        )?;
        dict.set_item("drift_score", self.inner.drift_score)?;
        dict.set_item("latency_weight", self.inner.latency_weight)?;
        dict.set_item("drift_weight", self.inner.drift_weight)?;
        dict.set_item("hybrid_score", self.inner.hybrid_score)?;
        dict.set_item("pass", self.inner.pass)?;
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

#[pyclass(name = "MlSlo")]
#[derive(Clone)]
pub struct PyMlSlo {
    inner: MlSlo,
}

#[pymethods]
impl PyMlSlo {
    #[new]
    #[pyo3(signature = (
        max_inference_latency_ms=200.0,
        max_gpu_utilization=0.85,
        max_feature_drift=0.2,
        min_prediction_confidence=0.8,
        latency_weight=0.6,
        drift_weight=0.4,
        min_pass_score=0.9
    ))]
    fn new(
        max_inference_latency_ms: f64,
        max_gpu_utilization: f64,
        max_feature_drift: f64,
        min_prediction_confidence: f64,
        latency_weight: f64,
        drift_weight: f64,
        min_pass_score: f64,
    ) -> Self {
        Self {
            inner: MlSlo {
                max_inference_latency_ms,
                max_gpu_utilization,
                max_feature_drift,
                min_prediction_confidence,
                latency_weight,
                drift_weight,
                min_pass_score,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<MlSlo>()?,
        })
    }

    #[getter]
    fn max_inference_latency_ms(&self) -> f64 {
        self.inner.max_inference_latency_ms
    }

    #[getter]
    fn max_gpu_utilization(&self) -> f64 {
        self.inner.max_gpu_utilization
    }

    #[getter]
    fn max_feature_drift(&self) -> f64 {
        self.inner.max_feature_drift
    }

    #[getter]
    fn min_prediction_confidence(&self) -> f64 {
        self.inner.min_prediction_confidence
    }

    #[getter]
    fn latency_weight(&self) -> f64 {
        self.inner.latency_weight
    }

    #[getter]
    fn drift_weight(&self) -> f64 {
        self.inner.drift_weight
    }

    #[getter]
    fn min_pass_score(&self) -> f64 {
        self.inner.min_pass_score
    }

    fn evaluate_sample(&self, sample: MlSample) -> PyMlSloEvaluation {
        self.inner.evaluate_sample(&sample).into()
    }

    fn evaluate_stream(&self, samples: Vec<MlSample>) -> Vec<PyMlSloEvaluation> {
        MlSloIterator::new(self.inner.clone(), samples.into_iter())
            .map(Into::into)
            .collect()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item(
            "max_inference_latency_ms",
            self.inner.max_inference_latency_ms,
        )?;
        dict.set_item("max_gpu_utilization", self.inner.max_gpu_utilization)?;
        dict.set_item("max_feature_drift", self.inner.max_feature_drift)?;
        dict.set_item(
            "min_prediction_confidence",
            self.inner.min_prediction_confidence,
        )?;
        dict.set_item("latency_weight", self.inner.latency_weight)?;
        dict.set_item("drift_weight", self.inner.drift_weight)?;
        dict.set_item("min_pass_score", self.inner.min_pass_score)?;
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

#[pyclass(name = "GenAiSample")]
#[derive(Clone)]
pub struct PyGenAiSample {
    inner: GenAiSample,
}

#[pymethods]
impl PyGenAiSample {
    #[new]
    fn new(
        timestamp: i64,
        tokens_generated: u64,
        generation_duration_ms: f64,
        time_to_first_token_ms: f64,
        reference_text: String,
        generated_text: String,
    ) -> Self {
        Self {
            inner: GenAiSample {
                timestamp,
                tokens_generated,
                generation_duration_ms,
                time_to_first_token_ms,
                reference_text,
                generated_text,
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<GenAiSample>()?,
        })
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn tokens_generated(&self) -> u64 {
        self.inner.tokens_generated
    }

    #[getter]
    fn generation_duration_ms(&self) -> f64 {
        self.inner.generation_duration_ms
    }

    #[getter]
    fn time_to_first_token_ms(&self) -> f64 {
        self.inner.time_to_first_token_ms
    }

    #[getter]
    fn reference_text(&self) -> String {
        self.inner.reference_text.clone()
    }

    #[getter]
    fn generated_text(&self) -> String {
        self.inner.generated_text.clone()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("tokens_generated", self.inner.tokens_generated)?;
        dict.set_item("generation_duration_ms", self.inner.generation_duration_ms)?;
        dict.set_item("time_to_first_token_ms", self.inner.time_to_first_token_ms)?;
        dict.set_item("reference_text", self.inner.reference_text.clone())?;
        dict.set_item("generated_text", self.inner.generated_text.clone())?;
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

#[pyclass(name = "GenAiSloEvaluation")]
#[derive(Clone)]
pub struct PyGenAiSloEvaluation {
    inner: GenAiSloEvaluation,
}

#[pymethods]
impl PyGenAiSloEvaluation {
    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn tokens_per_second(&self) -> f64 {
        self.inner.tokens_per_second
    }

    #[getter]
    fn time_to_first_token_ms(&self) -> f64 {
        self.inner.time_to_first_token_ms
    }

    #[getter]
    fn semantic_similarity(&self) -> f64 {
        self.inner.semantic_similarity
    }

    #[getter]
    fn tokens_per_second_ok(&self) -> bool {
        self.inner.tokens_per_second_ok
    }

    #[getter]
    fn time_to_first_token_ok(&self) -> bool {
        self.inner.time_to_first_token_ok
    }

    #[getter]
    fn semantic_similarity_ok(&self) -> bool {
        self.inner.semantic_similarity_ok
    }

    #[getter]
    fn pass(&self) -> bool {
        self.inner.pass
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", self.inner.timestamp)?;
        dict.set_item("tokens_per_second", self.inner.tokens_per_second)?;
        dict.set_item("time_to_first_token_ms", self.inner.time_to_first_token_ms)?;
        dict.set_item("semantic_similarity", self.inner.semantic_similarity)?;
        dict.set_item("tokens_per_second_ok", self.inner.tokens_per_second_ok)?;
        dict.set_item("time_to_first_token_ok", self.inner.time_to_first_token_ok)?;
        dict.set_item("semantic_similarity_ok", self.inner.semantic_similarity_ok)?;
        dict.set_item("pass", self.inner.pass)?;
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

#[pyclass(name = "GenAiSlo")]
#[derive(Clone)]
pub struct PyGenAiSlo {
    inner: GenAiSlo,
}

#[pymethods]
impl PyGenAiSlo {
    #[new]
    #[pyo3(signature = (
        min_tokens_per_second=20.0,
        max_time_to_first_token_ms=1200.0,
        min_semantic_similarity=0.7,
        semantic_model_name="sentence-transformers/all-MiniLM-L6-v2"
    ))]
    fn new(
        min_tokens_per_second: f64,
        max_time_to_first_token_ms: f64,
        min_semantic_similarity: f64,
        semantic_model_name: &str,
    ) -> Self {
        Self {
            inner: GenAiSlo {
                min_tokens_per_second,
                max_time_to_first_token_ms,
                min_semantic_similarity,
                semantic_model_name: semantic_model_name.to_string(),
            },
        }
    }

    #[staticmethod]
    fn from_dict(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: data.extract::<GenAiSlo>()?,
        })
    }

    #[getter]
    fn min_tokens_per_second(&self) -> f64 {
        self.inner.min_tokens_per_second
    }

    #[getter]
    fn max_time_to_first_token_ms(&self) -> f64 {
        self.inner.max_time_to_first_token_ms
    }

    #[getter]
    fn min_semantic_similarity(&self) -> f64 {
        self.inner.min_semantic_similarity
    }

    #[getter]
    fn semantic_model_name(&self) -> String {
        self.inner.semantic_model_name.clone()
    }

    fn evaluate_sample(&self, sample: GenAiSample) -> PyGenAiSloEvaluation {
        self.inner.evaluate_sample(&sample).into()
    }

    fn evaluate_stream(&self, samples: Vec<GenAiSample>) -> Vec<PyGenAiSloEvaluation> {
        GenAiSloIterator::new(self.inner.clone(), samples.into_iter())
            .map(Into::into)
            .collect()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("min_tokens_per_second", self.inner.min_tokens_per_second)?;
        dict.set_item(
            "max_time_to_first_token_ms",
            self.inner.max_time_to_first_token_ms,
        )?;
        dict.set_item(
            "min_semantic_similarity",
            self.inner.min_semantic_similarity,
        )?;
        dict.set_item(
            "semantic_model_name",
            self.inner.semantic_model_name.clone(),
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

impl From<HistogramBucket> for PyHistogramBucket {
    fn from(inner: HistogramBucket) -> Self {
        Self { inner }
    }
}

impl From<HistogramSample> for PyHistogramSample {
    fn from(inner: HistogramSample) -> Self {
        Self { inner }
    }
}

impl From<HttpSloEvaluation> for PyHttpSloEvaluation {
    fn from(inner: HttpSloEvaluation) -> Self {
        Self { inner }
    }
}

impl From<HttpSlo> for PyHttpSlo {
    fn from(inner: HttpSlo) -> Self {
        Self { inner }
    }
}

impl From<StatefulSample> for PyStatefulSample {
    fn from(inner: StatefulSample) -> Self {
        Self { inner }
    }
}

impl From<StatefulSloEvaluation> for PyStatefulSloEvaluation {
    fn from(inner: StatefulSloEvaluation) -> Self {
        Self { inner }
    }
}

impl From<StatefulSlo> for PyStatefulSlo {
    fn from(inner: StatefulSlo) -> Self {
        Self { inner }
    }
}

impl From<MlSample> for PyMlSample {
    fn from(inner: MlSample) -> Self {
        Self { inner }
    }
}

impl From<MlSloEvaluation> for PyMlSloEvaluation {
    fn from(inner: MlSloEvaluation) -> Self {
        Self { inner }
    }
}

impl From<MlSlo> for PyMlSlo {
    fn from(inner: MlSlo) -> Self {
        Self { inner }
    }
}

impl From<GenAiSample> for PyGenAiSample {
    fn from(inner: GenAiSample) -> Self {
        Self { inner }
    }
}

impl From<GenAiSloEvaluation> for PyGenAiSloEvaluation {
    fn from(inner: GenAiSloEvaluation) -> Self {
        Self { inner }
    }
}

impl From<GenAiSlo> for PyGenAiSlo {
    fn from(inner: GenAiSlo) -> Self {
        Self { inner }
    }
}

#[pyfunction]
pub fn coerce_slo_config(config: SloConfig) -> PySloConfig {
    config.into()
}

#[pyfunction]
pub fn coerce_error_budget(error_budget: ErrorBudget) -> PyErrorBudget {
    error_budget.into()
}

#[pyfunction]
pub fn coerce_metric_point(metric_point: MetricPoint) -> PyMetricPoint {
    metric_point.into()
}

#[pyfunction]
pub fn coerce_time_window(time_window: TimeWindow) -> PyTimeWindow {
    time_window.into()
}

#[pyfunction]
pub fn coerce_histogram_bucket(bucket: HistogramBucket) -> PyHistogramBucket {
    bucket.into()
}

#[pyfunction]
pub fn coerce_histogram_sample(sample: HistogramSample) -> PyHistogramSample {
    sample.into()
}

#[pyfunction]
pub fn coerce_http_slo(slo: HttpSlo) -> PyHttpSlo {
    slo.into()
}

#[pyfunction]
pub fn coerce_stateful_sample(sample: StatefulSample) -> PyStatefulSample {
    sample.into()
}

#[pyfunction]
pub fn coerce_stateful_slo(slo: StatefulSlo) -> PyStatefulSlo {
    slo.into()
}

#[pyfunction]
pub fn coerce_ml_sample(sample: MlSample) -> PyMlSample {
    sample.into()
}

#[pyfunction]
pub fn coerce_ml_slo(slo: MlSlo) -> PyMlSlo {
    slo.into()
}

#[pyfunction]
pub fn coerce_genai_sample(sample: GenAiSample) -> PyGenAiSample {
    sample.into()
}

#[pyfunction]
pub fn coerce_genai_slo(slo: GenAiSlo) -> PyGenAiSlo {
    slo.into()
}

#[pyfunction]
pub fn evaluate_http_slo_histogram(sample: HistogramSample, slo: HttpSlo) -> PyHttpSloEvaluation {
    slo.evaluate_histogram(&sample).into()
}

#[pyfunction]
pub fn evaluate_http_slo_histogram_stream(
    samples: Vec<HistogramSample>,
    slo: HttpSlo,
) -> Vec<PyHttpSloEvaluation> {
    HttpSloIterator::new(slo, samples.into_iter())
        .map(Into::into)
        .collect()
}

#[pyfunction]
pub fn evaluate_stateful_slo(sample: StatefulSample, slo: StatefulSlo) -> PyStatefulSloEvaluation {
    slo.evaluate_sample(&sample).into()
}

#[pyfunction]
pub fn evaluate_stateful_slo_stream(
    samples: Vec<StatefulSample>,
    slo: StatefulSlo,
) -> Vec<PyStatefulSloEvaluation> {
    StatefulSloIterator::new(slo, samples.into_iter())
        .map(Into::into)
        .collect()
}

#[pyfunction]
pub fn evaluate_ml_slo(sample: MlSample, slo: MlSlo) -> PyMlSloEvaluation {
    slo.evaluate_sample(&sample).into()
}

#[pyfunction]
pub fn evaluate_ml_slo_stream(samples: Vec<MlSample>, slo: MlSlo) -> Vec<PyMlSloEvaluation> {
    MlSloIterator::new(slo, samples.into_iter())
        .map(Into::into)
        .collect()
}

#[pyfunction]
pub fn evaluate_genai_slo(sample: GenAiSample, slo: GenAiSlo) -> PyGenAiSloEvaluation {
    slo.evaluate_sample(&sample).into()
}

#[pyfunction]
pub fn evaluate_genai_slo_stream(
    samples: Vec<GenAiSample>,
    slo: GenAiSlo,
) -> Vec<PyGenAiSloEvaluation> {
    GenAiSloIterator::new(slo, samples.into_iter())
        .map(Into::into)
        .collect()
}

#[pymodule]
fn neuralbudget(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = py;
    module.add_class::<PySloConfig>()?;
    module.add_class::<PyErrorBudget>()?;
    module.add_class::<PyMetricPoint>()?;
    module.add_class::<PyTimeWindow>()?;
    module.add_class::<PyHistogramBucket>()?;
    module.add_class::<PyHistogramSample>()?;
    module.add_class::<PyHttpSlo>()?;
    module.add_class::<PyHttpSloEvaluation>()?;
    module.add_class::<PyStatefulSample>()?;
    module.add_class::<PyStatefulSlo>()?;
    module.add_class::<PyStatefulSloEvaluation>()?;
    module.add_class::<PyMlSample>()?;
    module.add_class::<PyMlSlo>()?;
    module.add_class::<PyMlSloEvaluation>()?;
    module.add_class::<PyGenAiSample>()?;
    module.add_class::<PyGenAiSlo>()?;
    module.add_class::<PyGenAiSloEvaluation>()?;
    module.add_function(wrap_pyfunction!(calculate_availability, module)?)?;
    module.add_function(wrap_pyfunction!(calculate_error_budget, module)?)?;
    module.add_function(wrap_pyfunction!(calculate_burn_rate, module)?)?;
    module.add_function(wrap_pyfunction!(semantic_similarity_placeholder, module)?)?;
    module.add_function(wrap_pyfunction!(is_timestamp_in_window, module)?)?;
    module.add_function(wrap_pyfunction!(evaluate_http_slo_histogram, module)?)?;
    module.add_function(wrap_pyfunction!(
        evaluate_http_slo_histogram_stream,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(evaluate_stateful_slo, module)?)?;
    module.add_function(wrap_pyfunction!(evaluate_stateful_slo_stream, module)?)?;
    module.add_function(wrap_pyfunction!(evaluate_ml_slo, module)?)?;
    module.add_function(wrap_pyfunction!(evaluate_ml_slo_stream, module)?)?;
    module.add_function(wrap_pyfunction!(evaluate_genai_slo, module)?)?;
    module.add_function(wrap_pyfunction!(evaluate_genai_slo_stream, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_slo_config, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_error_budget, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_metric_point, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_time_window, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_histogram_bucket, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_histogram_sample, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_http_slo, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_stateful_sample, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_stateful_slo, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_ml_sample, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_ml_slo, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_genai_sample, module)?)?;
    module.add_function(wrap_pyfunction!(coerce_genai_slo, module)?)?;
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
