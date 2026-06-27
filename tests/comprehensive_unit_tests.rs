use neuralbudget::{
    ErrorBudget, HttpSloEvaluation, JsonExt, MlSloEvaluation, NeuralBudgetError,
    ParallelMetricBatch, PrometheusExporter, SloConfig, StreamingAggregator,
};
use pyo3::prelude::*;

// ============================================================================
// StreamingAggregator Tests (CRITICAL - Currently Untested)
// ============================================================================

#[test]
fn streaming_aggregator_push_stores_values_in_order() {
    let mut agg = StreamingAggregator::new();

    agg.push(100, 50.0);
    agg.push(200, 60.0);
    agg.push(300, 70.0);

    assert_eq!(agg.len(), 3);
    assert!(!agg.is_empty());
}

#[test]
fn streaming_aggregator_moving_average_computes_correct_window() {
    let mut agg = StreamingAggregator::new();

    // Add values: timestamps 100, 200, 300, 400, 500 with values 10, 20, 30, 40, 50
    for (ts, val) in [
        (100, 10.0),
        (200, 20.0),
        (300, 30.0),
        (400, 40.0),
        (500, 50.0),
    ] {
        agg.push(ts, val);
    }

    // Window: current_ts=500, window_size=200 → [300, 500]
    // Expected values: 30, 40, 50 → average = 40.0
    let avg = agg.get_moving_average(500, 200);
    assert!((avg - 40.0).abs() < 1e-9);
}

#[test]
fn streaming_aggregator_moving_average_empty_buffer_returns_zero() {
    let agg = StreamingAggregator::new();

    let avg = agg.get_moving_average(1000, 500);
    assert_eq!(avg, 0.0);
}

#[test]
fn streaming_aggregator_moving_average_single_value() {
    let mut agg = StreamingAggregator::new();
    agg.push(100, 42.5);

    let avg = agg.get_moving_average(100, 500);
    assert!((avg - 42.5).abs() < 1e-9);
}

#[test]
fn streaming_aggregator_moving_average_excludes_old_values() {
    let mut agg = StreamingAggregator::new();

    agg.push(100, 10.0);
    agg.push(200, 20.0);
    agg.push(300, 30.0);
    agg.push(400, 40.0);

    // Window: current=400, size=150 → only [250, 400] included
    // Expected: 30, 40 → average = 35.0
    let avg = agg.get_moving_average(400, 150);
    assert!((avg - 35.0).abs() < 1e-9);
}

#[test]
fn streaming_aggregator_prune_removes_old_data() {
    let mut agg = StreamingAggregator::new();

    for ts in 0..10 {
        agg.push(ts as i64, (ts as f64) * 10.0);
    }

    assert_eq!(agg.len(), 10);

    // Prune everything before timestamp 5
    agg.prune(5);

    assert_eq!(agg.len(), 5);

    // Verify values still in range
    let avg = agg.get_moving_average(9, 100);
    assert!(avg > 0.0);
}

#[test]
fn streaming_aggregator_monotonic_timestamps() {
    let mut agg = StreamingAggregator::new();

    // Ensure timestamps can be monotonic increasing
    agg.push(1000, 1.0);
    agg.push(2000, 2.0);
    agg.push(3000, 3.0);

    // Window at 3000 with size 1500 should only include 3000 and 2000
    let avg = agg.get_moving_average(3000, 1500);
    assert!((avg - 2.5).abs() < 1e-9);
}

#[test]
fn streaming_aggregator_high_frequency_timestamps() {
    let mut agg = StreamingAggregator::new();

    // Simulate high-frequency data (microseconds)
    for i in 0..1000 {
        agg.push(i as i64, 1.0);
    }

    assert_eq!(agg.len(), 1000);

    // Moving average over all values should be 1.0
    let avg = agg.get_moving_average(999, 10000);
    assert!((avg - 1.0).abs() < 1e-9);
}

#[test]
fn streaming_aggregator_default_initialization() {
    let agg = StreamingAggregator::default();

    assert!(agg.is_empty());
    assert_eq!(agg.len(), 0);
}

#[test]
fn streaming_aggregator_large_value_range() {
    let mut agg = StreamingAggregator::new();

    agg.push(1, 0.001);
    agg.push(2, 1_000_000.0);
    agg.push(3, 50.0);

    let avg = agg.get_moving_average(3, 100);
    assert!(avg > 0.0);
}

#[test]
fn streaming_aggregator_negative_values() {
    let mut agg = StreamingAggregator::new();

    agg.push(1, -10.0);
    agg.push(2, 20.0);
    agg.push(3, -5.0);

    let avg = agg.get_moving_average(3, 100);
    assert!((avg - 1.666666).abs() < 0.01);
}

#[test]
fn streaming_aggregator_zero_values() {
    let mut agg = StreamingAggregator::new();

    agg.push(1, 0.0);
    agg.push(2, 0.0);
    agg.push(3, 0.0);

    let avg = agg.get_moving_average(3, 100);
    assert_eq!(avg, 0.0);
}

// ============================================================================
// ParallelMetricBatch Tests (Edge Cases & Validation)
// ============================================================================

#[test]
fn parallel_metric_batch_empty_initialization() {
    let batch = ParallelMetricBatch::new(vec![]);

    assert_eq!(batch.node_count, 0);
    assert!(batch.all_pass());
    assert!(batch.get_node("nonexistent").is_none());
}

#[test]
fn parallel_metric_batch_get_node_returns_tuple() {
    let batch = ParallelMetricBatch::new(vec![
        ("metric_1".to_string(), 100.0, 200.0),
        ("metric_2".to_string(), 150.0, 100.0),
    ]);

    let node = batch.get_node("metric_1");
    assert!(node.is_some());

    let (id, value, threshold) = node.unwrap();
    assert_eq!(id, "metric_1");
    assert_eq!(value, 100.0);
    assert_eq!(threshold, 200.0);
}

#[test]
fn parallel_metric_batch_get_node_missing() {
    let batch = ParallelMetricBatch::new(vec![("metric_1".to_string(), 100.0, 200.0)]);

    assert!(batch.get_node("missing_metric").is_none());
}

#[test]
fn parallel_metric_batch_update_node_success() {
    let mut batch = ParallelMetricBatch::new(vec![("metric_1".to_string(), 100.0, 200.0)]);

    assert!(!batch.all_pass()); // 100 < 200, fails

    let updated = batch.update_node("metric_1", 250.0);
    assert!(updated);
    assert!(batch.all_pass()); // 250 >= 200, passes
}

#[test]
fn parallel_metric_batch_update_node_missing() {
    let mut batch = ParallelMetricBatch::new(vec![("metric_1".to_string(), 100.0, 200.0)]);

    let updated = batch.update_node("nonexistent", 250.0);
    assert!(!updated);
}

#[test]
fn parallel_metric_batch_all_pass_true_when_all_exceed_threshold() {
    let batch = ParallelMetricBatch::new(vec![
        ("metric_1".to_string(), 100.0, 50.0),
        ("metric_2".to_string(), 200.0, 100.0),
        ("metric_3".to_string(), 150.0, 125.0),
    ]);

    assert!(batch.all_pass());
}

#[test]
fn parallel_metric_batch_all_pass_false_when_one_fails() {
    let batch = ParallelMetricBatch::new(vec![
        ("metric_1".to_string(), 100.0, 50.0),  // passes
        ("metric_2".to_string(), 80.0, 100.0),  // fails
        ("metric_3".to_string(), 150.0, 125.0), // passes
    ]);

    assert!(!batch.all_pass());
}

#[test]
fn parallel_metric_batch_evaluate_returns_correct_format() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let batch = ParallelMetricBatch::new(vec![
            ("metric_1".to_string(), 150.0, 100.0),
            ("metric_2".to_string(), 50.0, 100.0),
        ]);

        let results = batch.evaluate(py);

        assert_eq!(results.len(), 2);

        // First result: passes (150 >= 100)
        assert_eq!(results[0].0, "metric_1");
        assert_eq!(results[0].1, 150.0);
        assert_eq!(results[0].2, 100.0);
        assert!(results[0].3); // pass flag
        assert!((results[0].4 - 1.0).abs() < 1e-9); // score = 150/100 = 1.5, clamped to 1.0

        // Second result: fails (50 < 100)
        assert_eq!(results[1].0, "metric_2");
        assert_eq!(results[1].1, 50.0);
        assert_eq!(results[1].2, 100.0);
        assert!(!results[1].3); // pass flag
        assert!((results[1].4 - 0.5).abs() < 1e-9); // score = 50/100 = 0.5
    });
}

#[test]
fn parallel_metric_batch_evaluate_empty_batch() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let batch = ParallelMetricBatch::new(vec![]);

        let results = batch.evaluate(py);
        assert_eq!(results.len(), 0);
    });
}

#[test]
fn parallel_metric_batch_score_calculation_with_zero_threshold() {
    let batch = ParallelMetricBatch::new(vec![("metric_1".to_string(), 100.0, 0.0)]);

    // Score should be 1.0 when threshold is 0 (to avoid division by zero)
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let results = batch.evaluate(py);
        assert!((results[0].4 - 1.0).abs() < 1e-9);
    });
}

// ============================================================================
// PrometheusExporter Tests
// ============================================================================

#[test]
fn prometheus_exporter_default_namespace() {
    let exporter = PrometheusExporter::new();
    let output = exporter.render();

    // Empty exporter should produce empty output or just comments
    assert_eq!(output.trim(), "");
}

#[test]
fn prometheus_exporter_observe_http_slo_renders_metrics() {
    let mut exporter = PrometheusExporter::new();

    let evaluation = HttpSloEvaluation {
        timestamp: 1000,
        pass: true,
        latency_ok: true,
        availability_ok: true,
        availability: 0.999,
        percentile_latency_ms: 150.0,
        evaluated_percentile: 99.0,
    };

    exporter.observe_http_slo("api_service", &evaluation);

    let output = exporter.render();
    assert!(output.contains("neuralbudget_http_pass"));
    assert!(output.contains("api_service"));
    assert!(!output.is_empty());
}

#[test]
fn prometheus_exporter_observe_ml_slo_renders_metrics() {
    let mut exporter = PrometheusExporter::new();

    let evaluation = MlSloEvaluation {
        timestamp: 2000,
        pass: true,
        hybrid_score: 0.95,
        system_score: 0.90,
        drift_score: 0.99,
        latency_weight: 0.5,
        drift_weight: 0.5,
        inference_latency_score: 0.92,
        gpu_utilization_score: 0.88,
        latency_score: 0.90,
        feature_drift_score: 0.97,
        prediction_confidence_score: 0.95,
    };

    exporter.observe_ml_slo("model_service", &evaluation);

    let output = exporter.render();
    assert!(output.contains("neuralbudget_ml_pass"));
    assert!(output.contains("model_service"));
}

#[test]
fn prometheus_exporter_set_static_label() {
    let mut exporter = PrometheusExporter::with_namespace("custom");
    exporter.set_static_label("env", "production");

    let evaluation = HttpSloEvaluation {
        timestamp: 1000,
        pass: true,
        latency_ok: true,
        availability_ok: true,
        availability: 0.999,
        percentile_latency_ms: 150.0,
        evaluated_percentile: 99.0,
    };

    exporter.observe_http_slo("api_service", &evaluation);

    let output = exporter.render();
    assert!(output.contains("env=\"production\""));
}

#[test]
fn prometheus_exporter_render_format_is_valid() {
    let mut exporter = PrometheusExporter::new();

    let evaluation = HttpSloEvaluation {
        timestamp: 1000,
        pass: true,
        latency_ok: true,
        availability_ok: true,
        availability: 0.999,
        percentile_latency_ms: 150.0,
        evaluated_percentile: 99.0,
    };

    exporter.observe_http_slo("test_service", &evaluation);

    let output = exporter.render();

    // Valid Prometheus format should contain HELP, TYPE, and metric lines
    assert!(output.contains("# HELP"));
    assert!(output.contains("# TYPE"));
    assert!(output.contains("neuralbudget_"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn neuralbudget_error_display_config_error() {
    let err = NeuralBudgetError::ConfigError("invalid config".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Configuration error"));
    assert!(msg.contains("invalid config"));
}

#[test]
fn neuralbudget_error_display_format_error() {
    let err = NeuralBudgetError::FormatError("malformed JSON".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Format conversion error"));
}

#[test]
fn neuralbudget_error_display_evaluation_error() {
    let err = NeuralBudgetError::EvaluationError("division by zero".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Evaluation error"));
}

#[test]
fn neuralbudget_error_display_composite_error() {
    let err = NeuralBudgetError::CompositeError("cyclic dependency".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Composite DAG error"));
}

#[test]
fn neuralbudget_error_display_schema_version_error() {
    let err = NeuralBudgetError::SchemaVersionError {
        found: 2,
        supported: "1".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Unsupported schema version 2"));
    assert!(msg.contains("supported: 1"));
}

#[test]
fn neuralbudget_error_from_serde_json_error() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json")
        .err()
        .unwrap();

    let err = NeuralBudgetError::from(json_err);

    match err {
        NeuralBudgetError::FormatError(msg) => {
            assert!(msg.contains("JSON error"));
        }
        _ => panic!("Expected FormatError"),
    }
}

// ============================================================================
// Serialization Tests (JSON only, post-YAGNI cleanup)
// ============================================================================

#[test]
fn slo_config_json_serialization_round_trip() {
    let config = SloConfig {
        target: 99.9,
        window: "7d".to_string(),
    };

    let json = config
        .to_json_string()
        .expect("serialization should succeed");
    let deserialized = SloConfig::from_json_str(&json).expect("deserialization should succeed");

    assert_eq!(config, deserialized);
}

#[test]
fn error_budget_json_serialization_round_trip() {
    let budget = ErrorBudget {
        remaining: 42.5,
        velocity: 1.75,
    };

    let json = budget
        .to_json_string()
        .expect("serialization should succeed");
    let deserialized = ErrorBudget::from_json_str(&json).expect("deserialization should succeed");

    assert_eq!(budget, deserialized);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn slo_config_minimum_values() {
    let config = SloConfig {
        target: 0.0,
        window: "1m".to_string(),
    };

    let json = config
        .to_json_string()
        .expect("serialization should succeed");
    let deserialized = SloConfig::from_json_str(&json).expect("deserialization should succeed");

    assert_eq!(deserialized.target, 0.0);
}

#[test]
fn slo_config_maximum_values() {
    let config = SloConfig {
        target: 100.0,
        window: "365d".to_string(),
    };

    let json = config
        .to_json_string()
        .expect("serialization should succeed");
    let deserialized = SloConfig::from_json_str(&json).expect("deserialization should succeed");

    assert_eq!(deserialized.target, 100.0);
}

#[test]
fn error_budget_zero_velocity() {
    let budget = ErrorBudget {
        remaining: 100.0,
        velocity: 0.0,
    };

    let json = budget
        .to_json_string()
        .expect("serialization should succeed");
    let deserialized = ErrorBudget::from_json_str(&json).expect("deserialization should succeed");

    assert_eq!(deserialized.velocity, 0.0);
}

#[test]
fn parallel_metric_batch_single_node() {
    let batch = ParallelMetricBatch::new(vec![("single".to_string(), 100.0, 100.0)]);

    assert_eq!(batch.node_count, 1);
    assert!(batch.all_pass()); // 100 >= 100 is true
}

#[test]
fn parallel_metric_batch_large_number_of_metrics() {
    let metrics: Vec<_> = (0..1000)
        .map(|i| (format!("metric_{}", i), 100.0, 50.0))
        .collect();

    let batch = ParallelMetricBatch::new(metrics);

    assert_eq!(batch.node_count, 1000);
    assert!(batch.all_pass()); // all have 100 >= 50
}

#[test]
fn prometheus_exporter_multiple_observations() {
    let mut exporter = PrometheusExporter::new();

    let eval1 = HttpSloEvaluation {
        timestamp: 1000,
        pass: true,
        latency_ok: true,
        availability_ok: true,
        availability: 0.999,
        percentile_latency_ms: 150.0,
        evaluated_percentile: 99.0,
    };

    let eval2 = HttpSloEvaluation {
        timestamp: 2000,
        pass: false,
        latency_ok: false,
        availability_ok: true,
        availability: 0.998,
        percentile_latency_ms: 250.0,
        evaluated_percentile: 99.0,
    };

    exporter.observe_http_slo("service_a", &eval1);
    exporter.observe_http_slo("service_b", &eval2);

    let output = exporter.render();
    assert!(output.contains("service_a"));
    assert!(output.contains("service_b"));
}
