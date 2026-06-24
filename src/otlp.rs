use std::collections::HashMap;

use serde::de::Error as DeError;
use serde::Deserialize;

use crate::core::{HistogramBucket, HistogramFormat, HistogramSample, MetricPoint};

#[derive(Debug)]
pub enum OtlpIngestError {
    Json(serde_json::Error),
    MetricNotFound(String),
    UnsupportedMetricType(String),
    InvalidHistogramBuckets(String),
    TimestampOutOfRange(u64),
}

impl std::fmt::Display for OtlpIngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => write!(f, "invalid OTLP JSON payload: {err}"),
            Self::MetricNotFound(name) => write!(f, "OTLP metric '{name}' was not found"),
            Self::UnsupportedMetricType(name) => {
                write!(
                    f,
                    "OTLP metric '{name}' is not a supported type for this operation"
                )
            }
            Self::InvalidHistogramBuckets(name) => write!(
                f,
                "OTLP histogram metric '{name}' has invalid bucketCounts/explicitBounds shape"
            ),
            Self::TimestampOutOfRange(value) => {
                write!(f, "OTLP timestamp '{value}' is outside i64 range")
            }
        }
    }
}

impl std::error::Error for OtlpIngestError {}

impl From<serde_json::Error> for OtlpIngestError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpExportRequest {
    #[serde(default)]
    resource_metrics: Vec<OtlpResourceMetrics>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpResourceMetrics {
    #[serde(default)]
    scope_metrics: Vec<OtlpScopeMetrics>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpScopeMetrics {
    #[serde(default)]
    metrics: Vec<OtlpMetric>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpMetric {
    name: String,
    #[serde(default)]
    gauge: Option<OtlpNumberData>,
    #[serde(default)]
    sum: Option<OtlpNumberData>,
    #[serde(default)]
    histogram: Option<OtlpHistogramData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpNumberData {
    #[serde(default)]
    data_points: Vec<OtlpNumberDataPoint>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpNumberDataPoint {
    #[serde(deserialize_with = "de_u64_from_str_or_num")]
    time_unix_nano: u64,
    #[serde(default)]
    as_double: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_i64_from_str_or_num")]
    as_int: Option<i64>,
    #[serde(default)]
    attributes: Vec<OtlpKeyValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpHistogramData {
    #[serde(default)]
    data_points: Vec<OtlpHistogramDataPoint>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpHistogramDataPoint {
    #[serde(deserialize_with = "de_u64_from_str_or_num")]
    time_unix_nano: u64,
    #[serde(deserialize_with = "de_u64_from_str_or_num")]
    count: u64,
    #[serde(deserialize_with = "de_vec_u64_from_str_or_num")]
    bucket_counts: Vec<u64>,
    #[serde(default)]
    explicit_bounds: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct OtlpKeyValue {
    key: String,
    value: OtlpAnyValue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpAnyValue {
    #[serde(default)]
    string_value: Option<String>,
    #[serde(default)]
    bool_value: Option<bool>,
    #[serde(default, deserialize_with = "de_opt_i64_from_str_or_num")]
    int_value: Option<i64>,
    #[serde(default)]
    double_value: Option<f64>,
}

impl OtlpAnyValue {
    fn as_label_value(&self) -> String {
        if let Some(value) = &self.string_value {
            return value.clone();
        }
        if let Some(value) = self.bool_value {
            return value.to_string();
        }
        if let Some(value) = self.int_value {
            return value.to_string();
        }
        if let Some(value) = self.double_value {
            return value.to_string();
        }
        String::new()
    }
}

fn de_u64_from_str_or_num<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum U64Like {
        Number(u64),
        String(String),
    }

    match U64Like::deserialize(deserializer)? {
        U64Like::Number(value) => Ok(value),
        U64Like::String(value) => value.parse::<u64>().map_err(D::Error::custom),
    }
}

fn de_opt_i64_from_str_or_num<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum I64Like {
        Number(i64),
        String(String),
    }

    let parsed = Option::<I64Like>::deserialize(deserializer)?;
    match parsed {
        None => Ok(None),
        Some(I64Like::Number(value)) => Ok(Some(value)),
        Some(I64Like::String(value)) => value.parse::<i64>().map(Some).map_err(D::Error::custom),
    }
}

fn de_vec_u64_from_str_or_num<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum U64Like {
        Number(u64),
        String(String),
    }

    let raw = Vec::<U64Like>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|value| match value {
            U64Like::Number(number) => Ok(number),
            U64Like::String(number) => number.parse::<u64>().map_err(D::Error::custom),
        })
        .collect()
}

fn nanos_to_seconds(timestamp_nanos: u64) -> Result<i64, OtlpIngestError> {
    let seconds = timestamp_nanos / 1_000_000_000;
    i64::try_from(seconds).map_err(|_| OtlpIngestError::TimestampOutOfRange(timestamp_nanos))
}

fn flatten_metrics(payload: OtlpExportRequest) -> Vec<OtlpMetric> {
    payload
        .resource_metrics
        .into_iter()
        .flat_map(|resource| resource.scope_metrics.into_iter())
        .flat_map(|scope| scope.metrics.into_iter())
        .collect()
}

fn metric_attributes(attributes: &[OtlpKeyValue]) -> HashMap<String, String> {
    attributes
        .iter()
        .map(|attr| (attr.key.clone(), attr.value.as_label_value()))
        .collect()
}

pub fn ingest_otlp_histogram_json(
    payload_json: &str,
    metric_name: &str,
) -> Result<Vec<HistogramSample>, OtlpIngestError> {
    let payload: OtlpExportRequest = serde_json::from_str(payload_json)?;
    let metrics = flatten_metrics(payload);

    let metric = metrics
        .into_iter()
        .find(|metric| metric.name == metric_name)
        .ok_or_else(|| OtlpIngestError::MetricNotFound(metric_name.to_string()))?;

    let histogram = metric
        .histogram
        .ok_or_else(|| OtlpIngestError::UnsupportedMetricType(metric_name.to_string()))?;

    let mut samples = Vec::new();
    for point in histogram.data_points {
        if point.bucket_counts.len() != point.explicit_bounds.len() + 1 {
            return Err(OtlpIngestError::InvalidHistogramBuckets(
                metric_name.to_string(),
            ));
        }

        let mut buckets = Vec::with_capacity(point.bucket_counts.len());
        for (idx, count) in point.bucket_counts.iter().enumerate() {
            let upper_bound_ms = if idx < point.explicit_bounds.len() {
                point.explicit_bounds[idx]
            } else {
                f64::INFINITY
            };

            buckets.push(HistogramBucket {
                upper_bound_ms,
                count: *count,
            });
        }

        samples.push(HistogramSample {
            timestamp: nanos_to_seconds(point.time_unix_nano)?,
            success: point.count,
            total: point.count,
            buckets,
            format: HistogramFormat::OpenTelemetryDelta,
        });
    }

    Ok(samples)
}

pub fn ingest_otlp_numeric_json(
    payload_json: &str,
    metric_name: &str,
) -> Result<Vec<MetricPoint>, OtlpIngestError> {
    let payload: OtlpExportRequest = serde_json::from_str(payload_json)?;
    let metrics = flatten_metrics(payload);

    let metric = metrics
        .into_iter()
        .find(|metric| metric.name == metric_name)
        .ok_or_else(|| OtlpIngestError::MetricNotFound(metric_name.to_string()))?;

    let points = if let Some(gauge) = metric.gauge {
        gauge.data_points
    } else if let Some(sum) = metric.sum {
        sum.data_points
    } else {
        return Err(OtlpIngestError::UnsupportedMetricType(
            metric_name.to_string(),
        ));
    };

    points
        .into_iter()
        .map(|point| {
            let value = if let Some(as_double) = point.as_double {
                as_double
            } else if let Some(as_int) = point.as_int {
                as_int as f64
            } else {
                0.0
            };

            Ok(MetricPoint {
                timestamp: nanos_to_seconds(point.time_unix_nano)?,
                value,
                labels: metric_attributes(&point.attributes),
            })
        })
        .collect()
}
