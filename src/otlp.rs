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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nanos_to_seconds_valid() {
        assert_eq!(nanos_to_seconds(1_000_000_000).unwrap(), 1);
        assert_eq!(nanos_to_seconds(0).unwrap(), 0);
        assert_eq!(nanos_to_seconds(1_500_000_000).unwrap(), 1);
        assert_eq!(nanos_to_seconds(2_500_000_000).unwrap(), 2);
    }

    #[test]
    fn test_nanos_to_seconds_large_value() {
        // Test a large valid value close to max range
        let nanos = 9_000_000_000_000_000_000u64; // 9 exaseconds
        assert_eq!(nanos_to_seconds(nanos).unwrap(), 9_000_000_000);
    }

    #[test]
    fn test_nanos_to_seconds_max_valid() {
        // u64::MAX / 1_000_000_000 is within i64 range, so should succeed
        let nanos = u64::MAX;
        assert!(nanos_to_seconds(nanos).is_ok());
    }

    #[test]
    fn test_otlp_any_value_string() {
        let value = OtlpAnyValue {
            string_value: Some("test".to_string()),
            bool_value: None,
            int_value: None,
            double_value: None,
        };
        assert_eq!(value.as_label_value(), "test");
    }

    #[test]
    fn test_otlp_any_value_bool_true() {
        let value = OtlpAnyValue {
            string_value: None,
            bool_value: Some(true),
            int_value: None,
            double_value: None,
        };
        assert_eq!(value.as_label_value(), "true");
    }

    #[test]
    fn test_otlp_any_value_bool_false() {
        let value = OtlpAnyValue {
            string_value: None,
            bool_value: Some(false),
            int_value: None,
            double_value: None,
        };
        assert_eq!(value.as_label_value(), "false");
    }

    #[test]
    fn test_otlp_any_value_int() {
        let value = OtlpAnyValue {
            string_value: None,
            bool_value: None,
            int_value: Some(42),
            double_value: None,
        };
        assert_eq!(value.as_label_value(), "42");
    }

    #[test]
    fn test_otlp_any_value_double() {
        let value = OtlpAnyValue {
            string_value: None,
            bool_value: None,
            int_value: None,
            double_value: Some(3.14),
        };
        assert_eq!(value.as_label_value(), "3.14");
    }

    #[test]
    fn test_otlp_any_value_priority() {
        // string_value takes priority over others
        let value = OtlpAnyValue {
            string_value: Some("string".to_string()),
            bool_value: Some(true),
            int_value: Some(42),
            double_value: Some(3.14),
        };
        assert_eq!(value.as_label_value(), "string");
    }

    #[test]
    fn test_otlp_any_value_empty() {
        let value = OtlpAnyValue {
            string_value: None,
            bool_value: None,
            int_value: None,
            double_value: None,
        };
        assert_eq!(value.as_label_value(), "");
    }

    #[test]
    fn test_ingest_otlp_numeric_gauge_json() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "request.latency",
                        "gauge": {
                            "dataPoints": [{
                                "timeUnixNano": "1000000000",
                                "asDouble": 42.5,
                                "attributes": [
                                    {"key": "service", "value": {"stringValue": "api"}}
                                ]
                            }]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_numeric_json(json, "request.latency").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, 42.5);
        assert_eq!(result[0].timestamp, 1);
        assert_eq!(result[0].labels.get("service").map(|s| s.as_str()), Some("api"));
    }

    #[test]
    fn test_ingest_otlp_numeric_sum_json() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "request.count",
                        "sum": {
                            "dataPoints": [{
                                "timeUnixNano": "2000000000",
                                "asInt": "100"
                            }]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_numeric_json(json, "request.count").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, 100.0);
        assert_eq!(result[0].timestamp, 2);
    }

    #[test]
    fn test_ingest_otlp_numeric_missing_value() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "request.latency",
                        "gauge": {
                            "dataPoints": [{
                                "timeUnixNano": "1000000000"
                            }]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_numeric_json(json, "request.latency").unwrap();
        assert_eq!(result[0].value, 0.0);
    }

    #[test]
    fn test_ingest_otlp_numeric_metric_not_found() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "request.latency",
                        "gauge": {"dataPoints": []}
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_numeric_json(json, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            OtlpIngestError::MetricNotFound(name) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected MetricNotFound error"),
        }
    }

    #[test]
    fn test_ingest_otlp_numeric_unsupported_type() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "request.latency",
                        "histogram": {
                            "dataPoints": []
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_numeric_json(json, "request.latency");
        assert!(result.is_err());
        match result.unwrap_err() {
            OtlpIngestError::UnsupportedMetricType(name) => assert_eq!(name, "request.latency"),
            _ => panic!("Expected UnsupportedMetricType error"),
        }
    }

    #[test]
    fn test_ingest_otlp_histogram_json() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "http.request.duration",
                        "histogram": {
                            "dataPoints": [{
                                "timeUnixNano": "1000000000",
                                "count": "100",
                                "bucketCounts": ["10", "20", "30", "40"],
                                "explicitBounds": [100.0, 250.0, 500.0]
                            }]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_histogram_json(json, "http.request.duration").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].timestamp, 1);
        assert_eq!(result[0].buckets.len(), 4);
        assert_eq!(result[0].buckets[0].upper_bound_ms, 100.0);
        assert_eq!(result[0].buckets[0].count, 10);
        assert_eq!(result[0].buckets[3].upper_bound_ms, f64::INFINITY);
        assert_eq!(result[0].format, HistogramFormat::OpenTelemetryDelta);
    }

    #[test]
    fn test_ingest_otlp_histogram_metric_not_found() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "http.request.duration",
                        "histogram": {"dataPoints": []}
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_histogram_json(json, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_ingest_otlp_histogram_unsupported_type() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "http.request.duration",
                        "gauge": {
                            "dataPoints": [{"timeUnixNano": "1000000000", "asDouble": 42.0}]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_histogram_json(json, "http.request.duration");
        assert!(result.is_err());
    }

    #[test]
    fn test_ingest_otlp_histogram_invalid_buckets() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "http.request.duration",
                        "histogram": {
                            "dataPoints": [{
                                "timeUnixNano": "1000000000",
                                "count": "100",
                                "bucketCounts": ["10", "20"],
                                "explicitBounds": [100.0, 250.0, 500.0]
                            }]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_histogram_json(json, "http.request.duration");
        assert!(result.is_err());
        match result.unwrap_err() {
            OtlpIngestError::InvalidHistogramBuckets(name) => assert_eq!(name, "http.request.duration"),
            _ => panic!("Expected InvalidHistogramBuckets error"),
        }
    }

    #[test]
    fn test_ingest_otlp_json_invalid_json() {
        let result = ingest_otlp_numeric_json("not valid json", "metric");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OtlpIngestError::Json(_)));
    }

    #[test]
    fn test_flatten_metrics_nested() {
        let payload = OtlpExportRequest {
            resource_metrics: vec![
                OtlpResourceMetrics {
                    scope_metrics: vec![
                        OtlpScopeMetrics {
                            metrics: vec![
                                OtlpMetric {
                                    name: "metric1".to_string(),
                                    gauge: None,
                                    sum: None,
                                    histogram: None,
                                },
                                OtlpMetric {
                                    name: "metric2".to_string(),
                                    gauge: None,
                                    sum: None,
                                    histogram: None,
                                },
                            ],
                        },
                        OtlpScopeMetrics {
                            metrics: vec![OtlpMetric {
                                name: "metric3".to_string(),
                                gauge: None,
                                sum: None,
                                histogram: None,
                            }],
                        },
                    ],
                },
                OtlpResourceMetrics {
                    scope_metrics: vec![OtlpScopeMetrics {
                        metrics: vec![OtlpMetric {
                            name: "metric4".to_string(),
                            gauge: None,
                            sum: None,
                            histogram: None,
                        }],
                    }],
                },
            ],
        };

        let flattened = flatten_metrics(payload);
        assert_eq!(flattened.len(), 4);
        assert_eq!(flattened[0].name, "metric1");
        assert_eq!(flattened[1].name, "metric2");
        assert_eq!(flattened[2].name, "metric3");
        assert_eq!(flattened[3].name, "metric4");
    }

    #[test]
    fn test_metric_attributes() {
        let attrs = vec![
            OtlpKeyValue {
                key: "service".to_string(),
                value: OtlpAnyValue {
                    string_value: Some("api".to_string()),
                    bool_value: None,
                    int_value: None,
                    double_value: None,
                },
            },
            OtlpKeyValue {
                key: "region".to_string(),
                value: OtlpAnyValue {
                    string_value: Some("us-west".to_string()),
                    bool_value: None,
                    int_value: None,
                    double_value: None,
                },
            },
        ];

        let result = metric_attributes(&attrs);
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("service").map(|s| s.as_str()), Some("api"));
        assert_eq!(result.get("region").map(|s| s.as_str()), Some("us-west"));
    }

    #[test]
    fn test_otlp_ingest_error_display() {
        assert_eq!(
            OtlpIngestError::MetricNotFound("test".to_string()).to_string(),
            "OTLP metric 'test' was not found"
        );
        assert_eq!(
            OtlpIngestError::UnsupportedMetricType("histogram".to_string()).to_string(),
            "OTLP metric 'histogram' is not a supported type for this operation"
        );
        assert_eq!(
            OtlpIngestError::InvalidHistogramBuckets("metric".to_string()).to_string(),
            "OTLP histogram metric 'metric' has invalid bucketCounts/explicitBounds shape"
        );
        assert_eq!(
            OtlpIngestError::TimestampOutOfRange(999).to_string(),
            "OTLP timestamp '999' is outside i64 range"
        );
    }

    #[test]
    fn test_ingest_otlp_numeric_multiple_datapoints() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "request.latency",
                        "gauge": {
                            "dataPoints": [
                                {"timeUnixNano": "1000000000", "asDouble": 10.5},
                                {"timeUnixNano": "2000000000", "asDouble": 20.5},
                                {"timeUnixNano": "3000000000", "asDouble": 30.5}
                            ]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_numeric_json(json, "request.latency").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].value, 10.5);
        assert_eq!(result[1].value, 20.5);
        assert_eq!(result[2].value, 30.5);
    }

    #[test]
    fn test_ingest_otlp_histogram_multiple_samples() {
        let json = r#"{
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [{
                        "name": "http.duration",
                        "histogram": {
                            "dataPoints": [
                                {
                                    "timeUnixNano": "1000000000",
                                    "count": "10",
                                    "bucketCounts": ["1", "2", "3", "4"],
                                    "explicitBounds": [100.0, 250.0, 500.0]
                                },
                                {
                                    "timeUnixNano": "2000000000",
                                    "count": "20",
                                    "bucketCounts": ["5", "6", "7", "2"],
                                    "explicitBounds": [100.0, 250.0, 500.0]
                                }
                            ]
                        }
                    }]
                }]
            }]
        }"#;

        let result = ingest_otlp_histogram_json(json, "http.duration").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].timestamp, 1);
        assert_eq!(result[1].timestamp, 2);
    }
}
