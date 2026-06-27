/// Evaluate an SLO against a metric sample
use anyhow::{anyhow, Context, Result};
use neuralbudget::{calculate_availability, calculate_error_budget};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Run the eval command
pub fn run(
    config_path: &Path,
    sample_path: &Path,
    json_output: bool,
    _verbose: bool,
) -> Result<()> {
    // Load and parse YAML config
    let config_content = fs::read_to_string(config_path).context(format!(
        "Failed to read config file: {}",
        config_path.display()
    ))?;

    let config: Value =
        serde_yaml::from_str(&config_content).context("Failed to parse YAML config")?;

    // Load and parse JSON sample
    let sample_content = fs::read_to_string(sample_path).context(format!(
        "Failed to read sample file: {}",
        sample_path.display()
    ))?;

    let sample: Value =
        serde_json::from_str(&sample_content).context("Failed to parse JSON sample")?;

    // Detect SLO type and evaluate
    let result = evaluate_slo(&config, &sample)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Human-readable output
        print_human_result(&result);
    }

    // Exit with appropriate code
    let passed = result["status"].as_str() == Some("pass");
    if !passed {
        std::process::exit(1);
    }

    Ok(())
}

/// Evaluate an SLO based on config type
fn evaluate_slo(config: &Value, sample: &Value) -> Result<Value> {
    // Try to detect SLO type from config structure
    if config["agent_slo"].is_object() {
        return evaluate_agent_slo(config, sample);
    }

    if config["genai_slo"].is_object() {
        return evaluate_genai_slo(config, sample);
    }

    if config["ml_slo"].is_object() {
        return evaluate_ml_slo(config, sample);
    }

    if config["stateful_slo"].is_object() {
        return evaluate_stateful_slo(config, sample);
    }

    // Default to HTTP SLO evaluation
    evaluate_http_slo(config, sample)
}

/// Evaluate an HTTP/Web API SLO
fn evaluate_http_slo(config: &Value, sample: &Value) -> Result<Value> {
    // Extract configuration
    let availability_threshold = config["availability_threshold"]
        .as_f64()
        .or_else(|| {
            let target = config["target"].as_f64()?;
            // Convert percentage to decimal if needed
            if target > 1.0 {
                Some(target / 100.0)
            } else {
                Some(target)
            }
        })
        .unwrap_or(0.999);

    let latency_threshold_ms = config["latency_threshold_ms"].as_f64().unwrap_or(200.0);

    // Parse sample data
    let total_requests = sample["requests"]["total"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid requests.total in sample"))?;

    let successful_requests = sample["requests"]["successful"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid requests.successful in sample"))?;

    let p99_ms = sample["latency"]["p99_ms"]
        .as_f64()
        .ok_or_else(|| anyhow!("Missing or invalid latency.p99_ms in sample"))?;

    // Calculate actual metrics
    let availability = calculate_availability(successful_requests, total_requests);
    let timestamp = sample["timestamp"].as_i64().unwrap_or(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    );

    // Evaluate SLO criteria (using strict > for availability, <= for latency)
    let availability_ok = availability > availability_threshold;
    let latency_ok = p99_ms <= latency_threshold_ms;
    let passed = availability_ok && latency_ok;

    // Calculate error budget
    let window_seconds = 86400 * 30; // 30 days default
    let error_budget_remaining = calculate_error_budget(availability_threshold, window_seconds);

    Ok(json!({
        "status": if passed { "pass" } else { "fail" },
        "timestamp": timestamp,
        "availability": {
            "actual": availability,
            "target": availability_threshold,
            "passed": availability_ok,
        },
        "latency_p99_ms": {
            "actual": p99_ms,
            "threshold": latency_threshold_ms,
            "passed": latency_ok,
        },
        "requests": {
            "total": total_requests,
            "successful": successful_requests,
            "failed": total_requests - successful_requests,
        },
        "error_budget_remaining_seconds": error_budget_remaining,
    }))
}

/// Placeholder implementations for other SLO types
fn evaluate_agent_slo(_config: &Value, _sample: &Value) -> Result<Value> {
    Err(anyhow!(
        "Agent SLO evaluation not yet implemented in CLI. Use Python SDK for agent-slo evaluation."
    ))
}

fn evaluate_genai_slo(_config: &Value, _sample: &Value) -> Result<Value> {
    Err(anyhow!(
        "GenAI SLO evaluation not yet implemented in CLI. Use Python SDK for genai-slo evaluation."
    ))
}

fn evaluate_ml_slo(_config: &Value, _sample: &Value) -> Result<Value> {
    Err(anyhow!(
        "ML SLO evaluation not yet implemented in CLI. Use Python SDK for ml-slo evaluation."
    ))
}

fn evaluate_stateful_slo(_config: &Value, _sample: &Value) -> Result<Value> {
    Err(anyhow!(
        "Stateful SLO evaluation not yet implemented in CLI. Use Python SDK for stateful-slo evaluation."
    ))
}

/// Print human-readable evaluation result
fn print_human_result(result: &Value) {
    let status = result["status"].as_str().unwrap_or("unknown");
    let symbol = if status == "pass" { "✓" } else { "✗" };
    let status_str = if status == "pass" { "PASS" } else { "FAIL" };

    println!();
    println!("{} SLO {}", symbol, status_str);
    println!();

    if let Some(availability) = result["availability"].as_object() {
        let actual = availability["actual"].as_f64().unwrap_or(0.0);
        let target = availability["target"].as_f64().unwrap_or(0.0);
        let symbol = if availability["passed"].as_bool().unwrap_or(false) {
            "✓"
        } else {
            "✗"
        };
        println!(
            "  {} Availability:  {:.3}% (target: {:.3}%)",
            symbol,
            actual * 100.0,
            target * 100.0
        );
    }

    if let Some(latency) = result["latency_p99_ms"].as_object() {
        let actual = latency["actual"].as_f64().unwrap_or(0.0);
        let threshold = latency["threshold"].as_f64().unwrap_or(0.0);
        let symbol = if latency["passed"].as_bool().unwrap_or(false) {
            "✓"
        } else {
            "✗"
        };
        println!(
            "  {} P99 Latency:   {:.1}ms (threshold: {:.1}ms)",
            symbol, actual, threshold
        );
    }

    if let Some(requests) = result["requests"].as_object() {
        let total = requests["total"].as_u64().unwrap_or(0);
        let successful = requests["successful"].as_u64().unwrap_or(0);
        println!("  • Requests:     {}/{} successful", successful, total);
    }

    if let Some(error_budget) = result["error_budget_remaining_seconds"].as_f64() {
        println!("  • Error Budget:  {:.1} seconds remaining", error_budget);
    }

    println!();
}
