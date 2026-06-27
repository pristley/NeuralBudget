/// Generate Prometheus recording and alerting rules from SLO config
///
/// This module implements sophisticated multi-burn-rate alerting as described in:
/// "The Art of SLOs" by Sloth and Google SRE handbook.
///
/// Recording Rules:
/// - Availability SLI (success rate)
/// - Latency SLI (p99 latency)
/// - Error budget remaining
/// - Multi-window burn rates (1h, 6h, 24h, 3d)
///
/// Alerting Rules:
/// - Multi-burn-rate alerts that warn early before budget exhaustion
/// - Contextualized with current burn rate, time to exhaustion, etc.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct BurnRateWindow {
    window: String,
    threshold_percent: f64,
}

#[derive(Debug, Clone)]
struct SloMetrics {
    service_name: String,
    availability_target: f64,
    latency_threshold_ms: f64,
    burn_rate_windows: Vec<BurnRateWindow>,
    job_label: String,
}

/// Run the gen-rules command
pub fn run(config_path: &Path, kubernetes: bool, namespace: &str) -> Result<()> {
    // Load and parse YAML config
    let config_content = fs::read_to_string(config_path)
        .context(format!("Failed to read config file: {}", config_path.display()))?;

    let config: Value = serde_yaml::from_str(&config_content)
        .context("Failed to parse YAML config")?;

    // Extract SLO metrics from config
    let service_name = config["service"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let availability_target = config["availability_threshold"]
        .as_f64()
        .unwrap_or(0.999);

    let latency_threshold_ms = config["latency_threshold_ms"]
        .as_f64()
        .unwrap_or(200.0);

    // Extract job label (default to service name in lowercase with dashes)
    let job_label = config["job_label"]
        .as_str()
        .unwrap_or(&service_name.to_lowercase())
        .to_string();

    // Parse burn rate windows from alerts config
    let mut burn_rate_windows = Vec::new();
    if let Some(alerts) = config["alerts"].as_array() {
        for alert in alerts {
            if let (Some(window), Some(threshold)) = (
                alert["window"].as_str(),
                alert["threshold"].as_f64(),
            ) {
                burn_rate_windows.push(BurnRateWindow {
                    window: window.to_string(),
                    threshold_percent: threshold * 100.0, // Convert to percentage
                });
            }
        }
    }

    // If no burn rate windows configured, use defaults
    if burn_rate_windows.is_empty() {
        burn_rate_windows = vec![
            BurnRateWindow {
                window: "1h".to_string(),
                threshold_percent: 10.0,
            },
            BurnRateWindow {
                window: "6h".to_string(),
                threshold_percent: 5.0,
            },
            BurnRateWindow {
                window: "24h".to_string(),
                threshold_percent: 2.0,
            },
            BurnRateWindow {
                window: "3d".to_string(),
                threshold_percent: 1.0,
            },
        ];
    }

    let metrics = SloMetrics {
        service_name,
        availability_target,
        latency_threshold_ms,
        burn_rate_windows,
        job_label,
    };

    if kubernetes {
        // Output as Kubernetes PrometheusRule CRD
        let rules = generate_kubernetes_rules(&metrics, namespace);
        println!("{}", rules);
    } else {
        // Output as plain Prometheus YAML
        let rules = generate_prometheus_rules(&metrics);
        println!("{}", rules);
    }

    Ok(())
}

fn generate_prometheus_rules(metrics: &SloMetrics) -> String {
    let error_budget_percent = (1.0 - metrics.availability_target) * 100.0;
    let latency_threshold_s = metrics.latency_threshold_ms / 1000.0;

    let mut rules = format!(
        r#"# Generated Prometheus recording and alerting rules for: {}
# Service: {}
# Target Availability: {}% ({})
# Latency Threshold: {}ms (P99)
# Error Budget: {}%
# Generated with NeuralBudget SLO platform

groups:
  - name: "neuralbudget_{}_recording"
    interval: 30s
    rules:
      # Availability SLI: successful requests / total requests
      - record: "neuralbudget:slo:availability"
        expr: |
          100 * sum(rate(http_requests_total{{job="{}",status=~"2.."}}{{1m}})) /
          sum(rate(http_requests_total{{job="{}"}}{{1m}}))

      # Latency SLI: P99 latency in milliseconds
      - record: "neuralbudget:slo:latency_p99_ms"
        expr: |
          histogram_quantile(0.99,
            sum(rate(http_request_duration_seconds_bucket{{job="{}"}}{{5m}})) by (le)
          ) * 1000

      # Error rate: failed requests / total requests
      - record: "neuralbudget:slo:error_rate"
        expr: |
          sum(rate(http_requests_total{{job="{}",status=~"5.."}}{{1m}})) /
          sum(rate(http_requests_total{{job="{}"}}{{1m}}))

      # Error budget remaining (in error budget percentage points)
      - record: "neuralbudget:slo:error_budget_remaining"
        expr: |
          {} - (100 * (neuralbudget:slo:error_rate / (1 - {})))

"#,
        metrics.service_name,
        metrics.service_name,
        (metrics.availability_target * 100.0),
        metrics.availability_target,
        metrics.latency_threshold_ms,
        error_budget_percent,
        metrics.service_name.to_lowercase(),
        metrics.job_label,
        metrics.job_label,
        metrics.job_label,
        metrics.job_label,
        metrics.job_label,
        error_budget_percent,
        metrics.availability_target
    );

    // Add multi-window burn rate recording rules
    rules.push_str("      # Multi-window burn rate indicators\n");

    for window in &metrics.burn_rate_windows {
        let window_prom = convert_window_to_prometheus(&window.window);
        let burn_rate_threshold = metrics.availability_target + (1.0 - metrics.availability_target)
            * (window.threshold_percent / 100.0);

        rules.push_str(&format!(
            "      - record: \"neuralbudget:slo:burn_rate_{}\"\n",
            window.window
        ));
        rules.push_str(&format!(
            "        expr: |\n          sum(rate(http_requests_total{{job=\"{}\",status=~\"5..\"}}{})) / (1 - {})\n\n",
            metrics.job_label,
            window_prom,
            metrics.availability_target
        ));
    }

    // Add alerting rules group
    rules.push_str(&format!(
        "  - name: \"neuralbudget_{}_alerts\"\n",
        metrics.service_name.to_lowercase()
    ));
    rules.push_str("    interval: 1m\n");
    rules.push_str("    rules:\n");

    // Generate multi-burn-rate alerting rules
    for window in &metrics.burn_rate_windows {
        let (duration, for_duration) = get_alert_timing(&window.window);
        let threshold = (1.0 - metrics.availability_target)
            * (window.threshold_percent / 100.0);

        rules.push_str(&format!(
            "      - alert: \"SloErrorBudgetBurnRate{}\"\n",
            format_window_name(&window.window)
        ));
        rules.push_str(&format!(
            "        expr: |\n          neuralbudget:slo:burn_rate_{} > {}\n",
            window.window, threshold
        ));
        rules.push_str(&format!("        for: {}\n", for_duration));
        rules.push_str("        labels:\n");
        rules.push_str("          severity: warning\n");
        rules.push_str("          slo: neuralbudget\n");
        rules.push_str("        annotations:\n");
        rules.push_str(&format!(
            "          summary: \"SLO error budget burning at {{{{ $value | humanizePercentage }}}} rate over {} window\"\n",
            window.window
        ));
        rules.push_str(&format!(
            "          description: \"Service {{{{ $labels.job }}}} is burning error budget at {{{{ $value | humanizePercentage }}}} rate over {} window. Budget may be exhausted within days.\"\n",
            window.window
        ));
        rules.push_str(&format!(
            "          runbook: \"https://neuralbudget.io/runbooks/burn-rate-{}\"\n\n",
            window.window
        ));
    }

    // Add latency alert
    rules.push_str("      - alert: \"SloLatencyExceeded\"\n");
    rules.push_str(&format!(
        "        expr: neuralbudget:slo:latency_p99_ms > {}\n",
        metrics.latency_threshold_ms
    ));
    rules.push_str("        for: 5m\n");
    rules.push_str("        labels:\n");
    rules.push_str("          severity: warning\n");
    rules.push_str("          slo: neuralbudget\n");
    rules.push_str("        annotations:\n");
    rules.push_str(&format!(
        "          summary: \"P99 latency {{{{ $value | humanize }}}}ms exceeds SLO target of {}ms\"\n",
        metrics.latency_threshold_ms
    ));
    rules.push_str("          description: \"Service performance is degraded. P99 latency is above acceptable threshold.\"\n\n");

    // Add error budget exhaustion alert
    rules.push_str("      - alert: \"SloErrorBudgetExhausted\"\n");
    rules.push_str("        expr: neuralbudget:slo:error_budget_remaining <= 0\n");
    rules.push_str("        for: 1m\n");
    rules.push_str("        labels:\n");
    rules.push_str("          severity: critical\n");
    rules.push_str("          slo: neuralbudget\n");
    rules.push_str("        annotations:\n");
    rules.push_str("          summary: \"SLO error budget has been exhausted\"\n");
    rules.push_str("          description: \"Service {{{{ $labels.job }}}} has exhausted its monthly error budget. All requests should be treated as critical.\"\n");

    rules
}

/// Convert SLO time windows (1h, 6h, 24h, 3d) to Prometheus duration format
fn convert_window_to_prometheus(window: &str) -> String {
    match window {
        "1h" => "[1h]".to_string(),
        "6h" => "[6h]".to_string(),
        "24h" => "[24h]".to_string(),
        "3d" => "[72h]".to_string(),
        _ => "[1h]".to_string(),
    }
}

/// Get recommended alert duration and evaluation window for each burn rate window
fn get_alert_timing(window: &str) -> (&'static str, &'static str) {
    match window {
        "1h" => ("1 hour", "1m"), // Fast burn: evaluate every 1m
        "6h" => ("6 hours", "5m"),
        "24h" => ("24 hours", "15m"),
        "3d" => ("3 days", "1h"),
        _ => ("1 hour", "5m"),
    }
}

/// Format window name for alert naming (e.g., "1h" -> "1h", "24h" -> "24h")
fn format_window_name(window: &str) -> String {
    window.replace("d", "d").replace("h", "h")
}

fn generate_kubernetes_rules(metrics: &SloMetrics, namespace: &str) -> String {
    let error_budget_percent = (1.0 - metrics.availability_target) * 100.0;
    let latency_threshold_s = metrics.latency_threshold_ms / 1000.0;

    let mut rules = format!(
        r#"apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: "{}-slo"
  namespace: "{}"
  labels:
    app: neuralbudget
    service: {}
spec:
  groups:
    - name: "neuralbudget_{}_recording"
      interval: 30s
      rules:
        # Availability SLI: successful requests / total requests
        - record: "neuralbudget:slo:availability"
          expr: |
            100 * sum(rate(http_requests_total{{job="{}",status=~"2.."}}{{1m}})) /
            sum(rate(http_requests_total{{job="{}"}}{{1m}}))

        # Latency SLI: P99 latency in milliseconds
        - record: "neuralbudget:slo:latency_p99_ms"
          expr: |
            histogram_quantile(0.99,
              sum(rate(http_request_duration_seconds_bucket{{job="{}"}}{{5m}})) by (le)
            ) * 1000

        # Error rate: failed requests / total requests
        - record: "neuralbudget:slo:error_rate"
          expr: |
            sum(rate(http_requests_total{{job="{}",status=~"5.."}}{{1m}})) /
            sum(rate(http_requests_total{{job="{}"}}{{1m}}))

        # Error budget remaining (in error budget percentage points)
        - record: "neuralbudget:slo:error_budget_remaining"
          expr: |
            {} - (100 * (neuralbudget:slo:error_rate / (1 - {})))

"#,
        metrics.service_name.to_lowercase(),
        namespace,
        metrics.service_name,
        metrics.service_name.to_lowercase(),
        metrics.job_label,
        metrics.job_label,
        metrics.job_label,
        metrics.job_label,
        metrics.job_label,
        error_budget_percent,
        metrics.availability_target
    );

    // Add multi-window burn rate recording rules
    rules.push_str("        # Multi-window burn rate indicators\n");
    for window in &metrics.burn_rate_windows {
        let window_prom = convert_window_to_prometheus(&window.window);
        let burn_rate_threshold = metrics.availability_target + (1.0 - metrics.availability_target)
            * (window.threshold_percent / 100.0);

        rules.push_str(&format!(
            "        - record: \"neuralbudget:slo:burn_rate_{}\"\n",
            window.window
        ));
        rules.push_str(&format!(
            "          expr: |\n            sum(rate(http_requests_total{{job=\"{}\",status=~\"5..\"}}{})) / (1 - {})\n\n",
            metrics.job_label,
            window_prom,
            metrics.availability_target
        ));
    }

    // Add alerting rules group
    rules.push_str(&format!(
        "    - name: \"neuralbudget_{}_alerts\"\n",
        metrics.service_name.to_lowercase()
    ));
    rules.push_str("      interval: 1m\n");
    rules.push_str("      rules:\n");

    // Generate multi-burn-rate alerting rules
    for window in &metrics.burn_rate_windows {
        let (duration, for_duration) = get_alert_timing(&window.window);
        let threshold = (1.0 - metrics.availability_target)
            * (window.threshold_percent / 100.0);

        rules.push_str(&format!(
            "        - alert: \"SloErrorBudgetBurnRate{}\"\n",
            format_window_name(&window.window)
        ));
        rules.push_str(&format!(
            "          expr: |\n            neuralbudget:slo:burn_rate_{} > {}\n",
            window.window, threshold
        ));
        rules.push_str(&format!("          for: {}\n", for_duration));
        rules.push_str("          labels:\n");
        rules.push_str("            severity: warning\n");
        rules.push_str("            slo: neuralbudget\n");
        rules.push_str(&format!("            service: {}\n", &metrics.service_name));
        rules.push_str("          annotations:\n");
        rules.push_str(&format!(
            "            summary: \"SLO error budget burning at high rate over {} window\"\n",
            window.window
        ));
        rules.push_str(&format!(
            "            description: \"Service {{{{ $labels.job }}}} has error rate of {{{{ $value | humanizePercentage }}}} over {} window, consuming error budget at {{{{ $value }}}}x rate. Budget exhaustion in ~{{{{ div 1 $value | humanizeDuration }}}}.\"\n",
            window.window
        ));
        rules.push_str(&format!(
            "            dashboard: \"https://neuralbudget.io/dashboard?service={{{{ $labels.job }}}}\"\n\n"
        ));
    }

    // Add latency alert
    rules.push_str("        - alert: \"SloLatencyExceeded\"\n");
    rules.push_str(&format!(
        "          expr: neuralbudget:slo:latency_p99_ms > {}\n",
        metrics.latency_threshold_ms
    ));
    rules.push_str("          for: 5m\n");
    rules.push_str("          labels:\n");
    rules.push_str("            severity: warning\n");
    rules.push_str("            slo: neuralbudget\n");
    rules.push_str("          annotations:\n");
    rules.push_str(&format!(
        "            summary: \"P99 latency exceeds SLO target of {}ms\"\n",
        metrics.latency_threshold_ms
    ));
    rules.push_str(&format!("            description: \"Service {{{{ $labels.job }}}} P99 latency is {{{{ $value | humanize }}}}ms (target: {}ms). Performance is degraded.\"\n", metrics.latency_threshold_ms));
    rules.push_str("            dashboard: \"https://neuralbudget.io/dashboard?service={{{{ $labels.job }}}}\"\n\n");

    // Add error budget exhaustion alert
    rules.push_str("        - alert: \"SloErrorBudgetExhausted\"\n");
    rules.push_str("          expr: neuralbudget:slo:error_budget_remaining <= 0\n");
    rules.push_str("          for: 1m\n");
    rules.push_str("          labels:\n");
    rules.push_str("            severity: critical\n");
    rules.push_str("            slo: neuralbudget\n");
    rules.push_str("          annotations:\n");
    rules.push_str("            summary: \"SLO error budget has been exhausted\"\n");
    rules.push_str("            description: \"Service {{{{ $labels.job }}}} has exhausted its monthly error budget ({{{{ $value | humanizePercentage }}}} remaining). All requests are now at risk of violating SLO.\"\n");
    rules.push_str("            dashboard: \"https://neuralbudget.io/dashboard?service={{{{ $labels.job }}}}\"\n");

    rules
}
