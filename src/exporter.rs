use std::collections::BTreeMap;

use crate::core::{
    CompositeServiceSloEvaluation, CompositeSloEvaluation, ErrorBudget, GenAiSloEvaluation,
    HttpSloEvaluation, MlSloEvaluation, StatefulSloEvaluation, WebApiSloReport,
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
