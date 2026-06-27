/// Validate SLO configuration and check for common mistakes

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

struct ValidationIssue {
    level: String, // "error", "warning", "info"
    message: String,
}

/// Run the check command
pub fn run(config_path: &Path, strict: bool) -> Result<()> {
    // Load and parse YAML config
    let config_content = fs::read_to_string(config_path)
        .context(format!("Failed to read config file: {}", config_path.display()))?;

    let config: Value = serde_yaml::from_str(&config_content)
        .context("Failed to parse YAML config")?;

    // Run validation checks
    let issues = validate_config(&config);

    // Print results
    println!("\n✓ SLO Configuration Check\n");
    println!("File: {}\n", config_path.display());

    let mut has_errors = false;
    let mut has_warnings = false;

    for issue in &issues {
        match issue.level.as_str() {
            "error" => {
                println!("  ✗ ERROR: {}", issue.message);
                has_errors = true;
            }
            "warning" => {
                println!("  ⚠ WARNING: {}", issue.message);
                has_warnings = true;
            }
            "info" => {
                println!("  ℹ INFO: {}", issue.message);
            }
            _ => {}
        }
    }

    println!();

    if has_errors {
        println!("❌ Configuration has errors. Please fix them before deploying.");
        return Err(anyhow::anyhow!("Configuration validation failed"));
    }

    if has_warnings && strict {
        println!("⚠️  Configuration has warnings. Use --strict mode. Failing.");
        return Err(anyhow::anyhow!("Configuration validation failed (strict mode)"));
    }

    println!("✅ Configuration is valid!\n");
    Ok(())
}

fn validate_config(config: &Value) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Check for required fields
    if config["service"].is_null() {
        issues.push(ValidationIssue {
            level: "error".to_string(),
            message: "Missing required field: 'service'".to_string(),
        });
    }

    if config["target"].is_null() {
        issues.push(ValidationIssue {
            level: "error".to_string(),
            message: "Missing required field: 'target' (SLO percentage)".to_string(),
        });
    }

    // Check for realistic thresholds
    if let Some(latency) = config["latency_threshold_ms"].as_f64() {
        if latency < 10.0 {
            issues.push(ValidationIssue {
                level: "warning".to_string(),
                message: format!("Latency threshold {}ms is unrealistically low (min: 10ms)", latency),
            });
        }
        if latency > 30000.0 {
            issues.push(ValidationIssue {
                level: "warning".to_string(),
                message: format!("Latency threshold {}ms is unusually high (max: 30000ms)", latency),
            });
        }
    }

    if let Some(target) = config["target"].as_f64() {
        if target < 0.5 || target > 100.0 {
            issues.push(ValidationIssue {
                level: "error".to_string(),
                message: format!("SLO target {}% is out of valid range (0.5-100)", target),
            });
        }
        if target < 90.0 {
            issues.push(ValidationIssue {
                level: "warning".to_string(),
                message: format!("SLO target {}% is quite low; consider 99%+ for production", target),
            });
        }
    }

    // Check for alert configuration
    if config["alerts"].is_null() {
        issues.push(ValidationIssue {
            level: "warning".to_string(),
            message: "No alert configuration found. Recommend setting up multi-window burn rate alerts.".to_string(),
        });
    }

    // Check for window configuration
    if config["window"].is_null() {
        issues.push(ValidationIssue {
            level: "info".to_string(),
            message: "No time window specified; will use default 30-day month".to_string(),
        });
    }

    issues
}

use anyhow;
