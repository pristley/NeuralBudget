/// Generate Prometheus recording and alerting rules from SLO config

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Run the gen-rules command
pub fn run(config_path: &Path, kubernetes: bool, namespace: &str) -> Result<()> {
    // Load and parse YAML config
    let config_content = fs::read_to_string(config_path)
        .context(format!("Failed to read config file: {}", config_path.display()))?;

    let config: Value = serde_yaml::from_str(&config_content)
        .context("Failed to parse YAML config")?;

    // Extract service name from config
    let service_name = config["service"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    if kubernetes {
        // Output as Kubernetes PrometheusRule CRD
        let rules = generate_kubernetes_rules(&service_name, namespace);
        println!("{}", rules);
    } else {
        // Output as plain Prometheus YAML
        let rules = generate_prometheus_rules(&service_name);
        println!("{}", rules);
    }

    Ok(())
}

fn generate_prometheus_rules(service_name: &str) -> String {
    format!(
        r#"# Generated Prometheus recording and alerting rules for: {}

groups:
  - name: "{}_slo"
    interval: 30s
    rules:
      # Recording rules for availability
      - record: "slo:availability:1m"
        expr: 'count(up{{job="{}"}}) / count(up) * 100'

      # Recording rules for latency (p99)
      - record: "slo:latency_p99:1m"
        expr: 'histogram_quantile(0.99, rate(request_duration_seconds_bucket{{job="{}"}}[1m]))'

      # Multi-window burn rate (1h window)
      - record: "slo:burn_rate:1h"
        expr: 'rate(errors_total{{job="{}"}}[1h])'

      # Multi-window burn rate (6h window)
      - record: "slo:burn_rate:6h"
        expr: 'rate(errors_total{{job="{}"}}[6h])'

      # Multi-window burn rate (24h window)
      - record: "slo:burn_rate:24h"
        expr: 'rate(errors_total{{job="{}"}}[24h])'

      # Multi-window burn rate (3d window)
      - record: "slo:burn_rate:3d"
        expr: 'rate(errors_total{{job="{}"}}[72h])'

alerts:
  - alert: "SLOAvailabilityWarning"
    expr: 'slo:availability:1m < 99'
    for: 5m
    annotations:
      summary: "SLO availability warning for {{}}"
      
  - alert: "SLOAvailabilityCritical"
    expr: 'slo:availability:1m < 95'
    for: 1m
    annotations:
      summary: "SLO availability critical for {{}}"

  - alert: "SLOLatencyWarning"
    expr: 'slo:latency_p99:1m > 250'
    for: 5m
    annotations:
      summary: "SLO latency warning for {{}}"

  - alert: "SLOBurnRateWarning"
    expr: 'slo:burn_rate:1h > 0.1 or slo:burn_rate:6h > 0.05'
    for: 5m
    annotations:
      summary: "High SLO burn rate for {{}}"
"#,
        service_name, service_name, service_name, service_name, service_name, service_name, service_name
    )
}

fn generate_kubernetes_rules(service_name: &str, namespace: &str) -> String {
    format!(
        r#"apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: "{}-slo"
  namespace: {}
spec:
  groups:
    - name: "{}_slo"
      interval: 30s
      rules:
        # Recording rules for availability
        - record: "slo:availability:1m"
          expr: 'count(up{{job="{}"}}) / count(up) * 100'

        # Recording rules for latency (p99)
        - record: "slo:latency_p99:1m"
          expr: 'histogram_quantile(0.99, rate(request_duration_seconds_bucket{{job="{}"}}[1m]))'

        # Multi-window burn rate
        - record: "slo:burn_rate:1h"
          expr: 'rate(errors_total{{job="{}"}}[1h])'

        - record: "slo:burn_rate:6h"
          expr: 'rate(errors_total{{job="{}"}}[6h])'

        - record: "slo:burn_rate:24h"
          expr: 'rate(errors_total{{job="{}"}}[24h])'

        - record: "slo:burn_rate:3d"
          expr: 'rate(errors_total{{job="{}"}}[72h])'

    - name: "{}_slo_alerts"
      interval: 30s
      rules:
        - alert: "SLOAvailabilityWarning"
          expr: 'slo:availability:1m < 99'
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: "SLO availability warning for {{}}"
            
        - alert: "SLOAvailabilityCritical"
          expr: 'slo:availability:1m < 95'
          for: 1m
          labels:
            severity: critical
          annotations:
            summary: "SLO availability critical for {{}}"

        - alert: "SLOLatencyWarning"
          expr: 'slo:latency_p99:1m > 250'
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: "SLO latency warning for {{}}"

        - alert: "SLOBurnRateWarning"
          expr: 'slo:burn_rate:1h > 0.1 or slo:burn_rate:6h > 0.05'
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: "High SLO burn rate for {{}}"
"#,
        service_name, namespace, service_name, service_name, service_name, service_name, service_name, service_name, service_name
    )
}
