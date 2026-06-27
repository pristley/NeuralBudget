/// Comprehensive tests for OpenSLO support
///
/// Tests cover:
/// - OpenSLO parsing from real examples
/// - Round-trip conversion (OpenSLO → NeuralBudget → OpenSLO)
/// - Multi-objective handling
/// - Service name extraction
/// - Format validation
#[cfg(test)]
mod openslo_tests {
    use neuralbudget::{
        parse_openslo_service, parse_openslo_yaml, to_openslo_json, to_openslo_object,
        to_openslo_yaml, HttpSlo,
    };

    /// OpenSLO example from CNCF repository (simplified)
    const EXAMPLE_OPENSLO_BASIC: &str = r#"
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

    /// OpenSLO example with multiple objectives
    const EXAMPLE_OPENSLO_MULTI: &str = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: payment-api-slo
  namespace: payments
  labels:
    service: payment-api
    team: backend
spec:
  service: payment-api
  description: Payment processing SLO
  objectives:
    - ratio_metrics:
        total:
          prometheus:
            query: rate(payment_requests_total[5m])
        good:
          prometheus:
            query: rate(payment_requests_success[5m])
      target: 0.9999
      window: rolling_1d
      description: Payment availability SLO
    - threshold_metrics:
        threshold: 200
        prometheus:
          query: histogram_quantile(0.99, rate(payment_duration_ms_bucket[5m]))
      target: 0.99
      window: rolling_1d
      description: Payment latency SLO (P99)
"#;

    #[test]
    fn test_parse_basic_openslo() {
        let result = parse_openslo_yaml(EXAMPLE_OPENSLO_BASIC);
        assert!(result.is_ok());

        let slo = result.unwrap();
        assert!((slo.availability_threshold - 0.999).abs() < 1e-9);
        assert!(slo.latency_threshold_ms > 0.0); // Default value
    }

    #[test]
    fn test_extract_service_name() {
        let result = parse_openslo_service(EXAMPLE_OPENSLO_BASIC);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "api-gateway");
    }

    #[test]
    fn test_parse_multi_objective_openslo() {
        let result = parse_openslo_yaml(EXAMPLE_OPENSLO_MULTI);
        assert!(result.is_ok());

        let slo = result.unwrap();
        assert!((slo.availability_threshold - 0.9999).abs() < 1e-9);
        // Should extract latency threshold from second objective
        assert!((slo.latency_threshold_ms - 200.0).abs() < 1e-9);
    }

    #[test]
    fn test_to_openslo_yaml_format() {
        let slo = HttpSlo {
            availability_threshold: 0.999,
            latency_threshold_ms: 200.0,
            latency_percentile: 0.99,
        };

        let result = to_openslo_yaml(&slo, "test-service", "test-slo");
        assert!(result.is_ok());

        let yaml = result.unwrap();
        assert!(yaml.contains("openslo/v1"));
        assert!(yaml.contains("test-slo"));
        assert!(yaml.contains("test-service"));
        assert!(yaml.contains("0.999"));
        assert!(yaml.contains("200"));
    }

    #[test]
    fn test_to_openslo_json_format() {
        let slo = HttpSlo {
            availability_threshold: 0.9999,
            latency_threshold_ms: 150.0,
            latency_percentile: 0.95,
        };

        let result = to_openslo_json(&slo, "payment-api", "payment-slo");
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("openslo/v1"));
        assert!(json.contains("payment-api"));
        assert!(json.contains("0.9999"));
    }

    #[test]
    fn test_round_trip_conversion() {
        // Start with OpenSLO
        let original_yaml = EXAMPLE_OPENSLO_BASIC;

        // Parse to NeuralBudget
        let parsed = parse_openslo_yaml(original_yaml).unwrap();
        let service = parse_openslo_service(original_yaml).unwrap();

        // Convert back to OpenSLO
        let converted = to_openslo_yaml(&parsed, &service, "api-gateway-slo").unwrap();

        // Parse converted version
        let reparsed = parse_openslo_yaml(&converted).unwrap();

        // Should match original (approximately)
        assert!(
            (parsed.availability_threshold - reparsed.availability_threshold).abs() < 1e-9,
            "Availability threshold changed during round-trip"
        );
    }

    #[test]
    fn test_invalid_openslo_missing_objectives() {
        let invalid_yaml = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: invalid-slo
spec:
  service: test-service
  objectives: []
"#;

        let result = parse_openslo_yaml(invalid_yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_openslo_bad_yaml() {
        let bad_yaml = r#"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: [invalid yaml structure here
  invalid indentation below
spec:
"#;

        let result = parse_openslo_yaml(bad_yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_openslo_metadata_preservation() {
        let result = to_openslo_yaml(&HttpSlo::default(), "my-service", "my-slo-name");
        assert!(result.is_ok());

        let yaml = result.unwrap();
        assert!(yaml.contains("my-slo-name"));
        assert!(yaml.contains("my-service"));
        assert!(yaml.contains("default")); // namespace
        assert!(yaml.contains("generated-by: neuralbudget")); // labels
    }

    #[test]
    fn test_different_availability_targets() {
        let test_cases = vec![
            0.95,   // 95% - lenient
            0.99,   // 99% - standard
            0.999,  // 99.9% - typical
            0.9999, // 99.99% - strict
        ];

        for target in test_cases {
            let slo = HttpSlo {
                availability_threshold: target,
                latency_threshold_ms: 200.0,
                latency_percentile: 0.99,
            };

            let yaml = to_openslo_yaml(&slo, "svc", "slo").unwrap();
            let reparsed = parse_openslo_yaml(&yaml).unwrap();

            assert!(
                (reparsed.availability_threshold - target).abs() < 1e-9,
                "Target {} not preserved in round-trip",
                target
            );
        }
    }

    #[test]
    fn test_latency_threshold_variations() {
        let test_cases = vec![
            100.0,  // Fast service
            200.0,  // Standard latency
            500.0,  // Slower service
            2000.0, // Batch processing
        ];

        for latency in test_cases {
            let slo = HttpSlo {
                availability_threshold: 0.999,
                latency_threshold_ms: latency,
                latency_percentile: 0.99,
            };

            let yaml = to_openslo_yaml(&slo, "svc", "slo").unwrap();
            let reparsed = parse_openslo_yaml(&yaml).unwrap();

            // Latency should be preserved (when specified)
            if latency != 200.0 {
                // Custom latency
                assert!(
                    (reparsed.latency_threshold_ms - latency).abs() < 1e-9,
                    "Latency {} not preserved",
                    latency
                );
            }
        }
    }

    #[test]
    fn test_openslo_object_creation() {
        let slo = HttpSlo {
            availability_threshold: 0.99,
            latency_threshold_ms: 300.0,
            latency_percentile: 0.95,
        };

        let openslo = to_openslo_object(&slo, "test-svc", "test-slo");
        assert!(openslo.is_ok());

        let obj = openslo.unwrap();
        assert_eq!(obj.api_version, "openslo/v1");
        assert_eq!(obj.kind, "SLO");
        assert_eq!(obj.metadata.name, "test-slo");
        assert_eq!(obj.spec.service, "test-svc");
        assert!(!obj.spec.objectives.is_empty());
    }

    #[test]
    fn test_openslo_real_world_scenario() {
        // Simulating migration from another tool
        let openslo_from_nobl9 = r###"
apiVersion: openslo/v1
kind: SLO
metadata:
  name: checkout-slo
  namespace: e-commerce
  labels:
    team: payments
    team_slack: "#payments"
spec:
  service: checkout-service
  description: Checkout service availability and performance SLO
  objectives:
    - ratio_metrics:
        total:
          prometheus:
            query: rate(checkout_requests_total{service="checkout"}[5m])
        good:
          prometheus:
            query: rate(checkout_requests_success{service="checkout"}[5m])
      target: 0.995
      window: rolling_30d
      description: Checkout availability
    - threshold_metrics:
        threshold: 500
        prometheus:
          query: histogram_quantile(0.99, rate(checkout_duration_ms_bucket{service="checkout"}[5m]))
      target: 0.99
      window: rolling_30d
      description: Checkout latency (P99 < 500ms)
"###;

        // Parse as OpenSLO
        let parsed = parse_openslo_yaml(openslo_from_nobl9);
        assert!(parsed.is_ok());

        // Extract service name
        let service = parse_openslo_service(openslo_from_nobl9);
        assert!(service.is_ok());
        assert_eq!(service.unwrap(), "checkout-service");

        // Verify parsed SLO
        let slo = parsed.unwrap();
        assert!((slo.availability_threshold - 0.995).abs() < 1e-9);
        assert!((slo.latency_threshold_ms - 500.0).abs() < 1e-9);
    }
}
