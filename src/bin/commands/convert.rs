/// Convert between SLO formats (NeuralBudget ↔ OpenSLO)

use anyhow::{anyhow, Result};
use neuralbudget::openslo::to_openslo_yaml;
use neuralbudget::HttpSlo;
use std::fs;
use std::path::Path;

/// Supported format types
#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    /// NeuralBudget format (simplified YAML with HttpSlo fields)
    NeuralBudget,
    /// OpenSLO format (CNCF standard)
    OpenSlo,
}

impl Format {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "neuralbudget" | "nb" => Ok(Self::NeuralBudget),
            "openslo" | "slo" => Ok(Self::OpenSlo),
            _ => Err(anyhow!("Unknown format: {}. Expected 'neuralbudget' or 'openslo'", s)),
        }
    }
}

/// Convert between SLO formats
///
/// # Arguments
///
/// * `input_path` - Path to input file
/// * `from_format` - Source format
/// * `to_format` - Target format
///
/// # Returns
///
/// Converted SLO as string
pub fn run(
    input_path: &Path,
    from_format: Format,
    to_format: Format,
    service_name: &str,
    slo_name: &str,
) -> Result<String> {
    // Read input file
    let input_content = fs::read_to_string(input_path)
        .map_err(|e| anyhow!("Failed to read input file: {}", e))?;

    // Convert based on format combination
    match (from_format, to_format) {
        (Format::OpenSlo, Format::NeuralBudget) => {
            convert_openslo_to_neuralbudget(&input_content)
        }
        (Format::NeuralBudget, Format::OpenSlo) => {
            convert_neuralbudget_to_openslo(&input_content, service_name, slo_name)
        }
        (fmt1, fmt2) if fmt1 == fmt2 => {
            Err(anyhow!("Source and target formats are the same"))
        }
        _ => Err(anyhow!("Unsupported format conversion")),
    }
}

/// Convert OpenSLO to NeuralBudget format
fn convert_openslo_to_neuralbudget(input: &str) -> Result<String> {
    let slo = neuralbudget::openslo::parse_openslo_yaml(input)
        .map_err(|e| anyhow!("Failed to parse OpenSLO: {}", e))?;

    // Convert to simple YAML format
    let output = format!(
        "# Converted from OpenSLO format by NeuralBudget\n\
         # Original format: OpenSLO\n\
         # Converted to: NeuralBudget\n\
         \n\
         availability_threshold: {}\n\
         latency_threshold_ms: {}\n\
         latency_percentile: {}\n",
        slo.availability_threshold, slo.latency_threshold_ms, slo.latency_percentile
    );

    Ok(output)
}

/// Convert NeuralBudget to OpenSLO format
fn convert_neuralbudget_to_openslo(
    input: &str,
    service_name: &str,
    slo_name: &str,
) -> Result<String> {
    // Parse simple YAML format
    let slo_data: serde_yaml::Value = serde_yaml::from_str(input)
        .map_err(|e| anyhow!("Failed to parse NeuralBudget format: {}", e))?;

    let slo = HttpSlo {
        availability_threshold: slo_data["availability_threshold"]
            .as_f64()
            .ok_or_else(|| anyhow!("Missing or invalid availability_threshold"))?,
        latency_threshold_ms: slo_data["latency_threshold_ms"]
            .as_f64()
            .unwrap_or(200.0),
        latency_percentile: slo_data["latency_percentile"].as_f64().unwrap_or(0.99),
    };

    to_openslo_yaml(&slo, service_name, slo_name)
        .map_err(|e| anyhow!("Failed to convert to OpenSLO: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_parsing() {
        assert_eq!(Format::from_str("openslo").unwrap(), Format::OpenSlo);
        assert_eq!(Format::from_str("slo").unwrap(), Format::OpenSlo);
        assert_eq!(Format::from_str("neuralbudget").unwrap(), Format::NeuralBudget);
        assert_eq!(Format::from_str("nb").unwrap(), Format::NeuralBudget);
    }

    #[test]
    fn test_openslo_to_neuralbudget() {
        let openslo_yaml = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: test-slo
spec:
  service: test-service
  objectives:
    - ratio_metrics:
        total:
          prometheus:
            query: rate(requests_total[5m])
        good:
          prometheus:
            query: rate(requests_success[5m])
      target: 0.9999
      window: rolling_1d
"#;

        let result = convert_openslo_to_neuralbudget(openslo_yaml);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("0.9999"));
        assert!(output.contains("availability_threshold"));
    }
}
