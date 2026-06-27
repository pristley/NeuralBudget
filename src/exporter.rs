use std::collections::BTreeMap;

use crate::core::{
    CompositeSloEvaluation, ErrorBudget, GenAiSloEvaluation, HttpSloEvaluation, MlSloEvaluation,
    StatefulSloEvaluation,
};

const METRIC_TYPE_GAUGE: &str = "gauge";

fn bool_as_f64(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

fn sanitize_metric_component(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if idx == 0 && ch.is_ascii_digit() {
                output.push('_');
            }
            output.push(ch);
        } else {
            output.push('_');
        }
    }

    if output.is_empty() {
        "neuralbudget".to_string()
    } else {
        output
    }
}

fn escape_label_value(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

fn format_help(help: &str) -> String {
    help.replace('\\', "\\\\").replace('\n', "\\n")
}

#[derive(Debug, Clone)]
struct MetricFamily {
    help: String,
    metric_type: &'static str,
    samples: BTreeMap<Vec<(String, String)>, f64>,
}

impl MetricFamily {
    fn new(help: String, metric_type: &'static str) -> Self {
        Self {
            help,
            metric_type,
            samples: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
/// Native Prometheus exporter for NeuralBudget SLO evaluation outputs.
///
/// This exporter stores the latest gauge values for each `(metric, labels)` pair and
/// renders Prometheus text exposition format (`0.0.4`) via [`render`].
pub struct PrometheusExporter {
    namespace: String,
    static_labels: BTreeMap<String, String>,
    families: BTreeMap<String, MetricFamily>,
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusExporter {
    /// Create an exporter with default namespace `neuralbudget`.
    pub fn new() -> Self {
        Self {
            namespace: "neuralbudget".to_string(),
            static_labels: BTreeMap::new(),
            families: BTreeMap::new(),
        }
    }

    /// Create an exporter using an explicit metric namespace prefix.
    pub fn with_namespace(namespace: impl Into<String>) -> Self {
        let mut exporter = Self::new();
        exporter.namespace = sanitize_metric_component(&namespace.into());
        exporter
    }

    /// Set or replace a static label applied to all exported metrics.
    pub fn set_static_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.static_labels.insert(key.into(), value.into());
    }

    /// Remove all currently stored metric samples while keeping namespace and static labels.
    pub fn clear(&mut self) {
        self.families.clear();
    }

    /// Record HTTP SLO evaluation metrics.
    pub fn observe_http_slo(&mut self, service: &str, evaluation: &HttpSloEvaluation) {
        self.observe_mode_timestamp("http", service, evaluation.timestamp);
        self.observe_mode_bool(
            "http",
            service,
            "pass",
            evaluation.pass,
            "HTTP SLO pass flag",
        );
        self.observe_mode_bool(
            "http",
            service,
            "latency_ok",
            evaluation.latency_ok,
            "HTTP SLO latency objective pass flag",
        );
        self.observe_mode_bool(
            "http",
            service,
            "availability_ok",
            evaluation.availability_ok,
            "HTTP SLO availability objective pass flag",
        );
        self.observe_mode_gauge(
            "http",
            service,
            "availability",
            evaluation.availability,
            "HTTP SLO availability ratio",
        );
        self.observe_mode_gauge(
            "http",
            service,
            "percentile_latency_ms",
            evaluation.percentile_latency_ms,
            "HTTP SLO evaluated latency percentile in milliseconds",
        );
        self.observe_mode_gauge(
            "http",
            service,
            "evaluated_percentile",
            evaluation.evaluated_percentile,
            "HTTP SLO configured percentile used for latency evaluation",
        );
    }

    /// Record ML SLO evaluation metrics.
    pub fn observe_ml_slo(&mut self, service: &str, evaluation: &MlSloEvaluation) {
        self.observe_mode_timestamp("ml", service, evaluation.timestamp);
        self.observe_mode_bool("ml", service, "pass", evaluation.pass, "ML SLO pass flag");
        self.observe_mode_gauge(
            "ml",
            service,
            "hybrid_score",
            evaluation.hybrid_score,
            "ML SLO hybrid score",
        );
        self.observe_mode_gauge(
            "ml",
            service,
            "system_score",
            evaluation.system_score,
            "ML SLO system score",
        );
        self.observe_mode_gauge(
            "ml",
            service,
            "drift_score",
            evaluation.drift_score,
            "ML SLO drift score",
        );
        self.observe_mode_gauge(
            "ml",
            service,
            "latency_weight",
            evaluation.latency_weight,
            "ML SLO latency score weight",
        );
        self.observe_mode_gauge(
            "ml",
            service,
            "drift_weight",
            evaluation.drift_weight,
            "ML SLO drift score weight",
        );
    }

    /// Record Stateful SLO evaluation metrics.
    pub fn observe_stateful_slo(&mut self, service: &str, evaluation: &StatefulSloEvaluation) {
        self.observe_mode_timestamp("stateful", service, evaluation.timestamp);
        self.observe_mode_bool(
            "stateful",
            service,
            "pass",
            evaluation.pass,
            "Stateful SLO pass flag",
        );
        self.observe_mode_gauge(
            "stateful",
            service,
            "score",
            evaluation.score,
            "Stateful SLO score",
        );
        self.observe_mode_bool(
            "stateful",
            service,
            "replication_lag_ok",
            evaluation.replication_lag_ok,
            "Stateful SLO replication lag objective pass flag",
        );
        self.observe_mode_bool(
            "stateful",
            service,
            "queue_depth_ok",
            evaluation.queue_depth_ok,
            "Stateful SLO queue depth objective pass flag",
        );
        self.observe_mode_bool(
            "stateful",
            service,
            "connection_pool_ok",
            evaluation.connection_pool_ok,
            "Stateful SLO connection pool objective pass flag",
        );
        self.observe_mode_bool(
            "stateful",
            service,
            "connection_wait_penalized",
            evaluation.connection_wait_penalized,
            "Stateful SLO connection wait penalty applied flag",
        );
    }

    /// Record GenAI SLO evaluation metrics.
    pub fn observe_genai_slo(&mut self, service: &str, evaluation: &GenAiSloEvaluation) {
        self.observe_mode_timestamp("genai", service, evaluation.timestamp);
        self.observe_mode_bool(
            "genai",
            service,
            "pass",
            evaluation.pass,
            "GenAI SLO pass flag",
        );
        self.observe_mode_gauge(
            "genai",
            service,
            "tokens_per_second",
            evaluation.tokens_per_second,
            "GenAI SLO tokens per second",
        );
        self.observe_mode_gauge(
            "genai",
            service,
            "time_to_first_token_ms",
            evaluation.time_to_first_token_ms,
            "GenAI SLO time to first token in milliseconds",
        );
        self.observe_mode_gauge(
            "genai",
            service,
            "semantic_similarity",
            evaluation.semantic_similarity,
            "GenAI SLO semantic similarity score",
        );
        self.observe_mode_bool(
            "genai",
            service,
            "tokens_per_second_ok",
            evaluation.tokens_per_second_ok,
            "GenAI SLO tokens per second objective pass flag",
        );
        self.observe_mode_bool(
            "genai",
            service,
            "time_to_first_token_ok",
            evaluation.time_to_first_token_ok,
            "GenAI SLO time to first token objective pass flag",
        );
        self.observe_mode_bool(
            "genai",
            service,
            "semantic_similarity_ok",
            evaluation.semantic_similarity_ok,
            "GenAI SLO semantic similarity objective pass flag",
        );
    }

    /// Record Composite SLO evaluation metrics.
    pub fn observe_composite_slo(&mut self, graph: &str, evaluation: &CompositeSloEvaluation) {
        self.observe_gauge(
            "composite_global_slo",
            "Composite SLO global weighted score",
            evaluation.global_slo,
            vec![("graph", graph.to_string())],
        );
        self.observe_gauge(
            "composite_global_pass",
            "Composite SLO global pass flag",
            bool_as_f64(evaluation.global_pass),
            vec![("graph", graph.to_string())],
        );
        for service_eval in &evaluation.services {
            self.observe_gauge(
                "composite_service_effective_score",
                "Composite SLO per-service effective score after dependency adjustments",
                service_eval.effective_score,
                vec![
                    ("graph", graph.to_string()),
                    ("service", service_eval.service.clone()),
                ],
            );
            self.observe_gauge(
                "composite_service_local_score",
                "Composite SLO per-service local score before dependency adjustments",
                service_eval.local_score,
                vec![
                    ("graph", graph.to_string()),
                    ("service", service_eval.service.clone()),
                ],
            );
            self.observe_gauge(
                "composite_service_pass",
                "Composite SLO per-service pass flag",
                bool_as_f64(service_eval.pass),
                vec![
                    ("graph", graph.to_string()),
                    ("service", service_eval.service.clone()),
                ],
            );
            self.observe_gauge(
                "composite_service_dependency_adjusted",
                "Composite SLO per-service dependency adjustment applied flag",
                bool_as_f64(service_eval.dependency_adjusted),
                vec![
                    ("graph", graph.to_string()),
                    ("service", service_eval.service.clone()),
                ],
            );
        }
    }

    /// Record error budget metrics.
    pub fn observe_error_budget(&mut self, service: &str, budget: &ErrorBudget) {
        self.observe_mode_gauge(
            "error_budget",
            service,
            "remaining",
            budget.remaining,
            "Error budget remaining fraction",
        );
        self.observe_mode_gauge(
            "error_budget",
            service,
            "velocity",
            budget.velocity,
            "Error budget burn velocity",
        );
    }

    /// Render all observed metric samples in Prometheus text exposition format.
    pub fn render(&self) -> String {
        let mut lines = Vec::new();

        for (metric_name, family) in &self.families {
            lines.push(format!(
                "# HELP {metric_name} {}",
                format_help(&family.help)
            ));
            lines.push(format!("# TYPE {metric_name} {}", family.metric_type));

            for (labels, value) in &family.samples {
                if labels.is_empty() {
                    lines.push(format!("{metric_name} {value}"));
                } else {
                    let encoded = labels
                        .iter()
                        .map(|(k, v)| format!("{k}=\"{}\"", escape_label_value(v)))
                        .collect::<Vec<_>>()
                        .join(",");
                    lines.push(format!("{metric_name}{{{encoded}}} {value}"));
                }
            }
        }

        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        }
    }

    fn observe_mode_timestamp(&mut self, mode: &str, service: &str, timestamp: i64) {
        self.observe_mode_gauge(
            mode,
            service,
            "timestamp_seconds",
            timestamp as f64,
            "Latest observed timestamp in seconds",
        );
    }

    fn observe_mode_bool(
        &mut self,
        mode: &str,
        service: &str,
        suffix: &str,
        value: bool,
        help: &str,
    ) {
        self.observe_mode_gauge(mode, service, suffix, bool_as_f64(value), help);
    }

    fn observe_mode_gauge(
        &mut self,
        mode: &str,
        service: &str,
        suffix: &str,
        value: f64,
        help: &str,
    ) {
        self.observe_gauge(
            &format!("{mode}_{suffix}"),
            help,
            value,
            vec![("mode", mode.to_string()), ("service", service.to_string())],
        );
    }

    fn observe_gauge(
        &mut self,
        metric_suffix: &str,
        help: &str,
        value: f64,
        mut labels: Vec<(&str, String)>,
    ) {
        let metric_name = format!(
            "{}_{}",
            sanitize_metric_component(&self.namespace),
            sanitize_metric_component(metric_suffix)
        );

        let mut merged: BTreeMap<String, String> = self.static_labels.clone();
        for (key, value) in labels.drain(..) {
            merged.insert(key.to_string(), value);
        }

        let final_labels = merged.into_iter().collect::<Vec<_>>();

        let family = self
            .families
            .entry(metric_name)
            .or_insert_with(|| MetricFamily::new(help.to_string(), METRIC_TYPE_GAUGE));
        family.samples.insert(final_labels, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_default_creation() {
        let exporter = PrometheusExporter::new();
        assert_eq!(exporter.namespace, "neuralbudget");
        assert!(exporter.static_labels.is_empty());
        assert!(exporter.families.is_empty());
    }

    #[test]
    fn test_exporter_default_trait() {
        let exporter = PrometheusExporter::default();
        assert_eq!(exporter.namespace, "neuralbudget");
    }

    #[test]
    fn test_exporter_custom_namespace() {
        let exporter = PrometheusExporter::with_namespace("my_service");
        assert_eq!(exporter.namespace, "my_service");
        assert!(exporter.families.is_empty());
    }

    #[test]
    fn test_sanitize_metric_component_alphanumeric() {
        assert_eq!(sanitize_metric_component("metric_name"), "metric_name");
        assert_eq!(sanitize_metric_component("http_slo"), "http_slo");
        assert_eq!(sanitize_metric_component("HTTP_SLO"), "HTTP_SLO");
    }

    #[test]
    fn test_sanitize_metric_component_special_chars() {
        assert_eq!(sanitize_metric_component("metric-name"), "metric_name");
        assert_eq!(sanitize_metric_component("metric.name"), "metric_name");
        assert_eq!(sanitize_metric_component("metric name"), "metric_name");
        assert_eq!(sanitize_metric_component("metric@name#2"), "metric_name_2");
    }

    #[test]
    fn test_sanitize_metric_component_leading_digit() {
        assert_eq!(sanitize_metric_component("2metric"), "_2metric");
        assert_eq!(sanitize_metric_component("9slo"), "_9slo");
    }

    #[test]
    fn test_sanitize_metric_component_empty() {
        // Special chars are converted to underscores, not defaulting to neuralbudget unless truly empty
        assert_eq!(sanitize_metric_component("@#$%"), "____");
        assert_eq!(sanitize_metric_component("!!!"), "___");
        assert_eq!(sanitize_metric_component(""), "neuralbudget");
    }

    #[test]
    fn test_escape_label_value_simple() {
        assert_eq!(escape_label_value("simple"), "simple");
        assert_eq!(escape_label_value("service_name"), "service_name");
    }

    #[test]
    fn test_escape_label_value_special_chars() {
        assert_eq!(escape_label_value("path\\to\\file"), "path\\\\to\\\\file");
        assert_eq!(escape_label_value("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_label_value("quoted\"text"), "quoted\\\"text");
    }

    #[test]
    fn test_escape_label_value_combined() {
        assert_eq!(
            escape_label_value("error: \"failed\"\npath\\to\\log"),
            "error: \\\"failed\\\"\\npath\\\\to\\\\log"
        );
    }

    #[test]
    fn test_bool_as_f64() {
        assert_eq!(bool_as_f64(true), 1.0);
        assert_eq!(bool_as_f64(false), 0.0);
    }

    #[test]
    fn test_set_static_label() {
        let mut exporter = PrometheusExporter::new();
        exporter.set_static_label("env", "prod");
        assert_eq!(
            exporter.static_labels.get("env").map(|s| s.as_str()),
            Some("prod")
        );
    }

    #[test]
    fn test_set_multiple_static_labels() {
        let mut exporter = PrometheusExporter::new();
        exporter.set_static_label("env", "prod");
        exporter.set_static_label("region", "us-west");
        exporter.set_static_label("team", "backend");

        assert_eq!(exporter.static_labels.len(), 3);
        assert_eq!(
            exporter.static_labels.get("env").map(|s| s.as_str()),
            Some("prod")
        );
        assert_eq!(
            exporter.static_labels.get("region").map(|s| s.as_str()),
            Some("us-west")
        );
        assert_eq!(
            exporter.static_labels.get("team").map(|s| s.as_str()),
            Some("backend")
        );
    }

    #[test]
    fn test_clear_metrics() {
        let mut exporter = PrometheusExporter::new();
        exporter.set_static_label("env", "test");

        // Simulate adding some metrics (use observe_gauge internally)
        exporter.observe_gauge(
            "test_metric",
            "Test metric",
            42.0,
            vec![("service", "test".to_string())],
        );

        assert!(!exporter.families.is_empty());

        exporter.clear();
        assert!(exporter.families.is_empty());
        // Static labels should persist
        assert_eq!(exporter.static_labels.len(), 1);
    }

    #[test]
    fn test_exporter_namespace_sanitization() {
        let exporter = PrometheusExporter::with_namespace("my-service@2024");
        assert_eq!(exporter.namespace, "my_service_2024");
    }

    #[test]
    fn test_format_help_simple() {
        assert_eq!(format_help("Simple help text"), "Simple help text");
    }

    #[test]
    fn test_format_help_with_escapes() {
        assert_eq!(
            format_help("Path: C:\\Users\\John"),
            "Path: C:\\\\Users\\\\John"
        );
        assert_eq!(format_help("Line1\nLine2"), "Line1\\nLine2");
        assert_eq!(format_help("C:\\path\nNext"), "C:\\\\path\\nNext");
    }

    #[test]
    fn test_metric_family_creation() {
        let family = MetricFamily::new("Test metric".to_string(), METRIC_TYPE_GAUGE);
        assert_eq!(family.help, "Test metric");
        assert_eq!(family.metric_type, METRIC_TYPE_GAUGE);
        assert!(family.samples.is_empty());
    }

    #[test]
    fn test_observe_gauge_creates_metric() {
        let mut exporter = PrometheusExporter::new();
        exporter.observe_gauge(
            "test_metric",
            "Test gauge",
            42.5,
            vec![("service", "api".to_string())],
        );

        assert_eq!(exporter.families.len(), 1);
        assert!(exporter.families.contains_key("neuralbudget_test_metric"));
    }

    #[test]
    fn test_observe_gauge_with_static_labels() {
        let mut exporter = PrometheusExporter::new();
        exporter.set_static_label("env", "prod");
        exporter.observe_gauge("test", "Test", 10.0, vec![("service", "api".to_string())]);

        let family = exporter.families.get("neuralbudget_test").unwrap();
        let labels = family.samples.keys().next().unwrap();

        // Should include both static and dynamic labels
        let label_map: std::collections::HashMap<_, _> = labels.iter().cloned().collect();
        assert_eq!(label_map.get("env").map(|s| s.as_str()), Some("prod"));
        assert_eq!(label_map.get("service").map(|s| s.as_str()), Some("api"));
    }

    #[test]
    fn test_observe_gauge_overwrites_same_labels() {
        let mut exporter = PrometheusExporter::new();
        exporter.observe_gauge("metric", "Help", 10.0, vec![("service", "api".to_string())]);
        exporter.observe_gauge("metric", "Help", 20.0, vec![("service", "api".to_string())]);

        let family = exporter.families.get("neuralbudget_metric").unwrap();
        // Should have only one sample (overwritten)
        assert_eq!(family.samples.len(), 1);
        assert_eq!(*family.samples.values().next().unwrap(), 20.0);
    }
}
