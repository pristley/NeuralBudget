//! OpenSLO (CNCF-aligned standard) support for NeuralBudget
//!
//! This module provides bidirectional conversion between OpenSLO and NeuralBudget SLO definitions.
//! OpenSLO is a vendor-neutral format that removes lock-in and enables portability across tools.
//!
//! # Example
//!
//! ```ignore
//! use neuralbudget::openslo::{parse_openslo_yaml, to_openslo_yaml};
//!
//! let openslo_yaml = r#"
//! apiVersion: openslo/v1
//! kind: SLO
//! metadata:
//!   name: api-gateway-slo
//! spec:
//!   service: api-gateway
//!   objectives:
//!     - ratioMetrics:
//!         total:
//!           metricSource:
//!             prometheus:
//!               query: rate(http_requests_total[5m])
//!         good:
//!           metricSource:
//!             prometheus:
//!               query: rate(http_requests_total{status=~"2.."}{5m])
//!       target: 0.999
//!       window: rolling_1d
//! "#;
//!
//! let slo_config = parse_openslo_yaml(openslo_yaml)?;
//! let converted_back = to_openslo_yaml(&slo_config)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{HttpSlo, NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenSLO API version (constant)
const OPENSLO_API_VERSION: &str = "openslo/v1";
/// OpenSLO Kind for SLO objects
const OPENSLO_KIND: &str = "SLO";

/// OpenSLO metadata for SLO object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenSloMetadata {
    /// Name of the SLO
    pub name: String,
    /// Namespace (optional, defaults to "default")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Labels for organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
}

/// OpenSLO metric source specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricSource {
    /// Prometheus metric query
    Prometheus { prometheus: PrometheusMetricSource },
    /// Datadog metric query
    Datadog { datadog: DatadogMetricSource },
    /// CloudWatch metric query
    CloudWatch { cloudwatch: CloudWatchMetricSource },
}

/// Prometheus-specific metric source
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrometheusMetricSource {
    /// PromQL query
    pub query: String,
}

/// Datadog-specific metric source (placeholder for future support)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatadogMetricSource {
    /// Datadog metric query
    pub query: String,
}

/// CloudWatch-specific metric source (placeholder for future support)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudWatchMetricSource {
    /// CloudWatch metric name
    pub metric_name: String,
    /// CloudWatch namespace
    pub namespace: String,
}

/// Ratio metrics definition (good/total ratio)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioMetrics {
    /// Numerator: good events
    pub good: RatioMetricPart,
    /// Denominator: total events
    pub total: RatioMetricPart,
}

/// Single part of a ratio metric
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioMetricPart {
    /// Metric source configuration
    #[serde(flatten)]
    pub metric_source: MetricSource,
}

/// OpenSLO objective (single SLI target)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenSloObjective {
    /// Ratio metrics definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratio_metrics: Option<RatioMetrics>,
    /// Threshold metrics definition (for latency, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_metrics: Option<ThresholdMetrics>,
    /// Target value (0.0-1.0 for availability, etc.)
    pub target: f64,
    /// Time window (e.g., "rolling_1d", "rolling_1h", "calendar_month")
    pub window: String,
    /// Description of this objective
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Threshold metrics definition (for latency, error rate, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThresholdMetrics {
    /// Threshold value (e.g., latency in milliseconds)
    pub threshold: f64,
    /// Metric to evaluate
    #[serde(flatten)]
    pub metric_source: MetricSource,
}

/// OpenSLO SLO spec
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenSloSpec {
    /// Service name
    pub service: String,
    /// List of SLO objectives
    pub objectives: Vec<OpenSloObjective>,
    /// Description of the SLO
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OpenSLO SLO object (top-level structure)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenSloObject {
    /// API version (always "openslo/v1")
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    /// Kind (always "SLO")
    pub kind: String,
    /// Metadata
    pub metadata: OpenSloMetadata,
    /// Specification
    pub spec: OpenSloSpec,
}

impl Default for OpenSloObject {
    fn default() -> Self {
        Self {
            api_version: OPENSLO_API_VERSION.to_string(),
            kind: OPENSLO_KIND.to_string(),
            metadata: OpenSloMetadata {
                name: "default-slo".to_string(),
                namespace: None,
                labels: None,
            },
            spec: OpenSloSpec {
                service: "default-service".to_string(),
                objectives: vec![],
                description: None,
            },
        }
    }
}

/// Parse OpenSLO YAML/JSON and convert to NeuralBudget HttpSlo
///
/// # Arguments
///
/// * `input` - YAML or JSON string in OpenSLO format
///
/// # Returns
///
/// HttpSlo configuration extracted from first objective
///
/// # Errors
///
/// - Returns `FormatError` if input is invalid YAML/JSON
/// - Returns `ConfigError` if required fields are missing or invalid
pub fn parse_openslo_yaml(input: &str) -> Result<HttpSlo> {
    // Try to parse as YAML/JSON
    let openslo: OpenSloObject = serde_yaml::from_str(input)
        .map_err(|e| NeuralBudgetError::FormatError(format!("Failed to parse OpenSLO: {e}")))?;

    // Extract first objective as primary SLO
    let objective = openslo.spec.objectives.first().ok_or_else(|| {
        NeuralBudgetError::ConfigError("No objectives found in OpenSLO".to_string())
    })?;

    // Build HttpSlo from objective
    let mut slo = HttpSlo {
        availability_threshold: objective.target,
        ..Default::default()
    };

    // If there are multiple objectives, try to extract latency from second one
    if openslo.spec.objectives.len() > 1 {
        if let Some(ref threshold_metrics) = &openslo.spec.objectives[1].threshold_metrics {
            slo.latency_threshold_ms = threshold_metrics.threshold;
        }
    }

    Ok(slo)
}

/// Parse OpenSLO YAML/JSON and extract service name
pub fn parse_openslo_service(input: &str) -> Result<String> {
    let openslo: OpenSloObject = serde_yaml::from_str(input)
        .map_err(|e| NeuralBudgetError::FormatError(format!("Failed to parse OpenSLO: {e}")))?;

    Ok(openslo.spec.service)
}

/// Convert NeuralBudget HttpSlo to OpenSLO YAML
///
/// # Arguments
///
/// * `slo` - HttpSlo configuration
/// * `service_name` - Name of the service
/// * `slo_name` - Name of the SLO object
///
/// # Returns
///
/// OpenSLO YAML string
///
/// # Errors
///
/// - Returns `FormatError` if serialization fails
pub fn to_openslo_yaml(slo: &HttpSlo, service_name: &str, slo_name: &str) -> Result<String> {
    let openslo = to_openslo_object(slo, service_name, slo_name)?;

    serde_yaml::to_string(&openslo)
        .map_err(|e| NeuralBudgetError::FormatError(format!("Failed to serialize to OpenSLO: {e}")))
}

/// Convert NeuralBudget HttpSlo to OpenSLO JSON
pub fn to_openslo_json(slo: &HttpSlo, service_name: &str, slo_name: &str) -> Result<String> {
    let openslo = to_openslo_object(slo, service_name, slo_name)?;

    serde_json::to_string_pretty(&openslo)
        .map_err(|e| NeuralBudgetError::FormatError(format!("Failed to serialize to OpenSLO: {e}")))
}

/// Convert HttpSlo to OpenSloObject
pub fn to_openslo_object(
    slo: &HttpSlo,
    service_name: &str,
    slo_name: &str,
) -> Result<OpenSloObject> {
    let mut objectives = vec![];

    // Availability objective
    objectives.push(OpenSloObjective {
        ratio_metrics: Some(RatioMetrics {
            good: RatioMetricPart {
                metric_source: MetricSource::Prometheus {
                    prometheus: PrometheusMetricSource {
                        query: format!(
                            "rate(http_requests_total{{status=~\"2..\",service=\"{service_name}\"}}[5m])"
                        ),
                    },
                },
            },
            total: RatioMetricPart {
                metric_source: MetricSource::Prometheus {
                    prometheus: PrometheusMetricSource {
                        query: format!(
                            "rate(http_requests_total{{service=\"{service_name}\"}}[5m])"
                        ),
                    },
                },
            },
        }),
        threshold_metrics: None,
        target: slo.availability_threshold,
        window: "rolling_1d".to_string(),
        description: Some(format!("Availability SLO for {service_name}")),
    });

    // Latency objective
    objectives.push(OpenSloObjective {
        ratio_metrics: None,
        threshold_metrics: Some(ThresholdMetrics {
            threshold: slo.latency_threshold_ms,
            metric_source: MetricSource::Prometheus {
                prometheus: PrometheusMetricSource {
                    query: format!(
                        "histogram_quantile({}, rate(http_request_duration_seconds_bucket{{service=\"{service_name}\"}}[5m]))",
                        slo.latency_percentile
                    ),
                },
            },
        }),
        target: 0.99, // P99 latency target
        window: "rolling_1d".to_string(),
        description: Some(format!(
            "P99 Latency SLO for {service_name} ({}ms)",
            slo.latency_threshold_ms
        )),
    });

    Ok(OpenSloObject {
        api_version: OPENSLO_API_VERSION.to_string(),
        kind: OPENSLO_KIND.to_string(),
        metadata: OpenSloMetadata {
            name: slo_name.to_string(),
            namespace: Some("default".to_string()),
            labels: Some(
                [
                    ("service".to_string(), service_name.to_string()),
                    ("generated-by".to_string(), "neuralbudget".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        },
        spec: OpenSloSpec {
            service: service_name.to_string(),
            objectives,
            description: Some(format!("NeuralBudget SLO: {service_name}")),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openslo_parsing() {
        let yaml = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: api-gateway-slo
  namespace: platform
spec:
  service: api-gateway
  objectives:
    - ratio_metrics:
        total:
          prometheus:
            query: rate(http_requests_total[5m])
        good:
          prometheus:
            query: rate(http_requests_total{status=~"2.."}[5m])
      target: 0.999
      window: rolling_1d
"#;

        let result = parse_openslo_yaml(yaml);
        assert!(result.is_ok());
        let slo = result.unwrap();
        assert!((slo.availability_threshold - 0.999).abs() < 1e-9);
    }

    #[test]
    fn test_service_extraction() {
        let yaml = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: test-slo
spec:
  service: my-service
  objectives:
    - ratio_metrics:
        total:
          prometheus:
            query: rate(requests_total[5m])
        good:
          prometheus:
            query: rate(requests_success[5m])
      target: 0.999
      window: rolling_1d
"#;

        let service = parse_openslo_service(yaml).unwrap();
        assert_eq!(service, "my-service");
    }

    #[test]
    fn test_to_openslo_yaml() {
        let slo = HttpSlo {
            availability_threshold: 0.999,
            latency_threshold_ms: 200.0,
            latency_percentile: 0.99,
        };

        let result = to_openslo_yaml(&slo, "payment-api", "payment-api-slo");
        assert!(result.is_ok());

        let yaml_str = result.unwrap();
        assert!(yaml_str.contains("openslo/v1"));
        assert!(yaml_str.contains("payment-api"));
        assert!(yaml_str.contains("0.999"));
    }

    #[test]
    fn test_round_trip_conversion() {
        // Create original OpenSLO
        let original_yaml = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: round-trip-test
spec:
  service: test-service
  objectives:
    - ratio_metrics:
        total:
          prometheus:
            query: rate(http_requests_total[5m])
        good:
          prometheus:
            query: rate(http_requests_total{status=~"2.."}[5m])
      target: 0.9999
      window: rolling_1d
"#;

        // Parse to NeuralBudget
        let parsed = parse_openslo_yaml(original_yaml).unwrap();
        assert!((parsed.availability_threshold - 0.9999).abs() < 1e-9);

        // Convert back to OpenSLO
        let converted = to_openslo_yaml(&parsed, "test-service", "round-trip-test").unwrap();

        // Parse converted version
        let reparsed = parse_openslo_yaml(&converted).unwrap();

        // Should match
        assert!((parsed.availability_threshold - reparsed.availability_threshold).abs() < 1e-9);
    }
}
