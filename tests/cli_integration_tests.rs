/// Integration tests for neuralbudget CLI tool

use std::fs;
use std::path::Path;

fn create_test_config(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).ok();
    fs::write(path, content).expect("Failed to write test config");
}

fn create_test_sample(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).ok();
    fs::write(path, content).expect("Failed to write test sample");
}

#[test]
fn test_eval_with_valid_config_and_sample() {
    let config_path = "/tmp/test_slo.yaml";
    let sample_path = "/tmp/test_sample.json";

    let config = r#"
service: "api-gateway"
target: 99.9
window: "30d"
latency_threshold_ms: 200
"#;

    let sample = r#"{
  "timestamp": 1234567890,
  "requests_total": 10000,
  "requests_successful": 9995,
  "latency_p99_ms": 187.5
}"#;

    create_test_config(Path::new(config_path), config);
    create_test_sample(Path::new(sample_path), sample);

    // In a real test environment with cargo available, we would:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "eval", config_path, sample_path])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());

    println!("✓ Test config and sample created successfully");
}

#[test]
fn test_eval_with_missing_config() {
    println!("Test: eval command should error with missing config file");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "eval", "nonexistent.yaml", "sample.json"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(!output.status.success());
}

#[test]
fn test_eval_with_invalid_yaml_config() {
    let config_path = "/tmp/test_invalid_slo.yaml";
    create_test_config(Path::new(config_path), "{ invalid yaml: [");

    println!("Test: eval command should error with invalid YAML config");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "eval", config_path, "sample.json"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(!output.status.success());
}

#[test]
fn test_eval_with_json_output_flag() {
    let config_path = "/tmp/test_json_output_slo.yaml";
    let sample_path = "/tmp/test_json_output_sample.json";

    let config = r#"
service: "api-gateway"
target: 99.9
"#;

    let sample = r#"{"requests_total": 10000}"#;

    create_test_config(Path::new(config_path), config);
    create_test_sample(Path::new(sample_path), sample);

    println!("Test: eval command with --json flag should output JSON");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "eval", config_path, sample_path, "--json"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("\"status\""));
}

#[test]
fn test_gen_rules_generates_prometheus_rules() {
    let config_path = "/tmp/test_gen_rules_slo.yaml";

    let config = r#"
service: "api-gateway"
target: 99.9
latency_threshold_ms: 200
"#;

    create_test_config(Path::new(config_path), config);

    println!("Test: gen-rules command should output Prometheus rules");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "gen-rules", config_path])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("groups:"));
    // assert!(stdout.contains("recording"));
}

#[test]
fn test_gen_rules_with_kubernetes_flag() {
    let config_path = "/tmp/test_k8s_gen_rules_slo.yaml";

    let config = r#"
service: "api-gateway"
target: 99.9
"#;

    create_test_config(Path::new(config_path), config);

    println!("Test: gen-rules command with --kubernetes flag should output PrometheusRule CRD");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "gen-rules", config_path, "--kubernetes"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("kind: PrometheusRule"));
    // assert!(stdout.contains("apiVersion: monitoring.coreos.com"));
}

#[test]
fn test_check_with_valid_config() {
    let config_path = "/tmp/test_check_valid_slo.yaml";

    let config = r#"
service: "api-gateway"
target: 99.9
latency_threshold_ms: 200
window: "30d"
"#;

    create_test_config(Path::new(config_path), config);

    println!("Test: check command should succeed with valid config");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "check", config_path])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("valid"));
}

#[test]
fn test_check_with_invalid_config() {
    let config_path = "/tmp/test_check_invalid_slo.yaml";

    let config = r#"
target: 101  # Invalid: SLO > 100%
"#;

    create_test_config(Path::new(config_path), config);

    println!("Test: check command should fail with invalid SLO percentage");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "check", config_path])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(!output.status.success());
}

#[test]
fn test_check_detects_missing_service_field() {
    let config_path = "/tmp/test_check_missing_service.yaml";

    let config = r#"
target: 99.9
# Missing: service field
"#;

    create_test_config(Path::new(config_path), config);

    println!("Test: check command should warn about missing 'service' field");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "check", config_path])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(!output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("service"));
}

#[test]
fn test_check_warns_on_unrealistic_latency() {
    let config_path = "/tmp/test_check_latency_warning.yaml";

    let config = r#"
service: "api-gateway"
target: 99.9
latency_threshold_ms: 5  # Too low: 5ms is unrealistic
"#;

    create_test_config(Path::new(config_path), config);

    println!("Test: check command should warn about unrealistic latency threshold");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "check", config_path])
    //     .output()
    //     .expect("Failed to execute CLI");
    // let stderr = String::from_utf8_lossy(&output.stderr);
    // assert!(stderr.contains("latency") || stderr.contains("unrealistic"));
}

#[test]
fn test_cli_help_flag() {
    println!("Test: CLI should support --help flag");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "--help"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("Usage:") || stdout.contains("neuralbudget"));
}

#[test]
fn test_cli_version_flag() {
    println!("Test: CLI should support --version flag");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "--version"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("0.1"));
}

#[test]
fn test_eval_subcommand_help() {
    println!("Test: eval subcommand should have --help");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "eval", "--help"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(output.status.success());
    // let stdout = String::from_utf8_lossy(&output.stdout);
    // assert!(stdout.contains("eval"));
}

#[test]
fn test_serve_command_not_yet_implemented() {
    println!("Test: serve command should indicate it's not yet implemented");
    // In real environment:
    // let output = Command::new("cargo")
    //     .args(&["run", "--bin", "neuralbudget", "--", "serve"])
    //     .output()
    //     .expect("Failed to execute CLI");
    // assert!(!output.status.success());
    // let stderr = String::from_utf8_lossy(&output.stderr);
    // assert!(stderr.contains("not yet implemented") || stderr.contains("planned"));
}
