use std::collections::HashMap;

use proptest::prelude::*;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use super::*;

fn make_request(timestamp: i64, latency_ms: f64, status_code: u16) -> WebApiRequest {
    WebApiRequest {
        timestamp,
        latency_ms,
        status_code,
        labels: HashMap::new(),
    }
}

#[test]
fn calculate_availability_matches_pure_python_ratio() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let success = 47_u64;
        let total = 50_u64;

        let expected: f64 = {
            let builtins = py.import_bound("builtins").unwrap();
            let eval_fn = builtins.getattr("eval").unwrap();
            let globals = PyDict::new_bound(py);
            globals.set_item("__builtins__", &builtins).unwrap();
            let locals = PyDict::new_bound(py);
            locals.set_item("success", success).unwrap();
            locals.set_item("total", total).unwrap();

            eval_fn
                .call1(("success / total", &globals, &locals))
                .unwrap()
                .extract()
                .unwrap()
        };

        assert_eq!(calculate_availability(success, total), expected);
    });
}

#[test]
fn calculate_error_budget_scales_with_window() {
    let budget = calculate_error_budget(0.99, 3_600);

    assert!((budget - 36.0).abs() < 1e-9);
}

#[test]
fn burn_rate_rises_with_more_recent_consumption() {
    let stream: Vec<MetricPoint> = (0..3_600)
        .map(|timestamp| MetricPoint {
            timestamp,
            value: if timestamp >= 3_300 { 1.0 } else { 0.0 },
            labels: HashMap::new(),
        })
        .collect();

    let five_minute = calculate_burn_rate(stream.clone(), 300);
    let one_hour = calculate_burn_rate(stream, 3_600);

    assert_eq!(five_minute, 1.0);
    assert_eq!(one_hour, 300.0 / 3_600.0);
}

#[test]
fn mad_identifies_large_latency_spike() {
    let values = vec![100.0, 101.0, 99.0, 102.0, 500.0];
    let mad = calculate_mad(&values).unwrap();
    let mask = mad_outlier_mask(&values, 3.5, 3);

    assert!(mad > 0.0);
    assert_eq!(mask, vec![false, false, false, false, true]);
}

#[test]
fn filter_statistical_outliers_removes_single_spike() {
    let stream = vec![
        MetricPoint {
            timestamp: 1,
            value: 100.0,
            labels: HashMap::new(),
        },
        MetricPoint {
            timestamp: 2,
            value: 101.0,
            labels: HashMap::new(),
        },
        MetricPoint {
            timestamp: 3,
            value: 500.0,
            labels: HashMap::new(),
        },
    ];

    let filtered = filter_statistical_outliers(&stream, 3.5, 3);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|point| point.value < 200.0));
}

#[test]
fn web_api_slo_filters_latency_outlier_when_enabled() {
    let requests = vec![
        make_request(1, 120.0, 200),
        make_request(2, 130.0, 200),
        make_request(3, 110.0, 200),
        make_request(4, 4_000.0, 200),
        make_request(5, 115.0, 500),
    ];

    let policy = WebApiSloPolicy {
        availability_target: 0.99,
        latency_threshold_ms: 250.0,
        time_window_seconds: 10,
        outlier_filter: OutlierFilterConfig {
            enabled: true,
            mad_threshold: 3.5,
            min_samples: 3,
        },
    };

    let report = calculate_web_api_slo(&requests, &policy, 6);

    assert_eq!(report.total_requests, 5);
    assert_eq!(report.successful_requests, 4);
    assert!((report.availability - 0.8).abs() < 1e-9);
    assert_eq!(report.filtered_outliers, 1);
    assert_eq!(report.latency_evaluated_requests, 4);
    assert_eq!(report.latency_compliant_requests, 4);
    assert!((report.latency_sli - 1.0).abs() < 1e-9);
}

#[test]
fn rolling_window_detects_recent_timestamps() {
    let window = TimeWindow::rolling(3_600);
    let now = 1_700_000_000_i64;

    assert!(window.contains(now - 3_600, now));
    assert!(window.contains(now - 1, now));
    assert!(!window.contains(now - 3_601, now));
    assert!(!window.contains(now + 1, now));
}

#[test]
fn calendar_aligned_window_uses_timezone_offset() {
    let window = TimeWindow::calendar_aligned(86_400, 18_000);
    let now = 90_000_i64;

    assert!(window.contains(69_000, now));
    assert!(window.contains(104_999, now));
    assert!(window.contains(68_400, now));
    assert!(!window.contains(68_399, now));
    assert!(!window.contains(154_800, now));
}

#[test]
fn python_window_function_matches_rust_logic() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "neuralbudget_test").unwrap();
        module
            .add_function(wrap_pyfunction!(is_timestamp_in_window, &module).unwrap())
            .unwrap();

        let window = TimeWindow::calendar_aligned(86_400, 18_000);
        let py_window = PyTimeWindow::from(window.clone());
        let actual: bool = module
            .getattr("is_timestamp_in_window")
            .unwrap()
            .call1((69_000_i64, 90_000_i64, py_window))
            .unwrap()
            .extract()
            .unwrap();

        assert!(actual);
        assert!(window.contains(69_000, 90_000));
    });
}

#[test]
fn python_http_slo_histogram_wrapper_evaluates_sample() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "neuralbudget_http_slo_test").unwrap();
        module
            .add_function(wrap_pyfunction!(evaluate_http_slo_histogram, &module).unwrap())
            .unwrap();

        let sample = PyDict::new_bound(py);
        sample.set_item("timestamp", 1_i64).unwrap();
        sample.set_item("success", 9_995_u64).unwrap();
        sample.set_item("total", 10_000_u64).unwrap();
        let buckets = PyList::empty_bound(py);
        for (upper_bound_ms, count) in [(100.0, 9_700_u64), (150.0, 200_u64), (220.0, 100_u64)] {
            let bucket = PyDict::new_bound(py);
            bucket.set_item("upper_bound_ms", upper_bound_ms).unwrap();
            bucket.set_item("count", count).unwrap();
            buckets.append(bucket).unwrap();
        }
        sample.set_item("buckets", buckets).unwrap();
        sample.set_item("format", "open_telemetry_delta").unwrap();

        let slo = PyDict::new_bound(py);
        slo.set_item("latency_threshold_ms", 200.0).unwrap();
        slo.set_item("latency_percentile", 0.99).unwrap();
        slo.set_item("availability_threshold", 0.999).unwrap();

        let result = module
            .getattr("evaluate_http_slo_histogram")
            .unwrap()
            .call1((&sample, &slo))
            .unwrap();
        let pass: bool = result.getattr("pass").unwrap().extract().unwrap();

        assert!(pass);
    });
}

#[test]
fn python_stateful_slo_wrapper_penalizes_wait_time() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "neuralbudget_stateful_slo_test").unwrap();
        module
            .add_function(wrap_pyfunction!(evaluate_stateful_slo, &module).unwrap())
            .unwrap();

        let sample = PyDict::new_bound(py);
        sample.set_item("timestamp", 2_i64).unwrap();
        sample.set_item("replication_lag_ms", 200.0).unwrap();
        sample.set_item("queue_depth", 800_u64).unwrap();
        sample.set_item("connection_pool_saturation", 0.75).unwrap();
        sample.set_item("connection_wait_time_ms", 60.0).unwrap();

        let slo = PyDict::new_bound(py);
        slo.set_item("replication_lag_threshold_ms", 250.0).unwrap();
        slo.set_item("queue_depth_threshold", 1_000_u64).unwrap();
        slo.set_item("connection_pool_saturation_threshold", 0.8)
            .unwrap();
        slo.set_item("connection_wait_time_threshold_ms", 20.0)
            .unwrap();
        slo.set_item("connection_wait_penalty_weight", 0.3).unwrap();
        slo.set_item("min_pass_score", 0.85).unwrap();

        let result = module
            .getattr("evaluate_stateful_slo")
            .unwrap()
            .call1((&sample, &slo))
            .unwrap();
        let pass: bool = result.getattr("pass").unwrap().extract().unwrap();
        let penalized: bool = result
            .getattr("connection_wait_penalized")
            .unwrap()
            .extract()
            .unwrap();

        assert!(penalized);
        assert!(!pass);
    });
}

#[test]
fn slo_config_round_trips_through_json_and_yaml() {
    let config = SloConfig {
        target: 99.9,
        window: "30d".to_string(),
    };

    let json = config.to_json_string().unwrap();
    let yaml = config.to_yaml_string().unwrap();

    assert_eq!(SloConfig::from_json_str(&json).unwrap(), config);
    assert_eq!(SloConfig::from_yaml_str(&yaml).unwrap(), config);
}

#[test]
fn slo_config_rejects_unsupported_schema_versions() {
    let bad_json = r#"{"schema_version":2,"target":99.9,"window":"30d"}"#;
    let bad_yaml = "schema_version: 2\ntarget: 99.9\nwindow: 30d\n";

    let json_err = SloConfig::from_json_str(bad_json).unwrap_err().to_string();
    let yaml_err = SloConfig::from_yaml_str(bad_yaml).unwrap_err().to_string();

    assert!(json_err.contains("unsupported schema_version"));
    assert!(yaml_err.contains("unsupported schema_version"));
}

#[test]
fn slo_config_rejects_unknown_keys() {
    let bad_json = r#"{"target":99.9,"window":"30d","unexpected":true}"#;
    let bad_yaml = "target: 99.9\nwindow: 30d\nunexpected: true\n";

    assert!(SloConfig::from_json_str(bad_json).is_err());
    assert!(SloConfig::from_yaml_str(bad_yaml).is_err());
}

#[test]
fn slo_config_defaults_schema_version_when_omitted() {
    let json = r#"{"target":99.9,"window":"30d"}"#;
    let yaml = "target: 99.9\nwindow: 30d\n";
    let expected = SloConfig {
        target: 99.9,
        window: "30d".to_string(),
    };

    assert_eq!(SloConfig::from_json_str(json).unwrap(), expected);
    assert_eq!(SloConfig::from_yaml_str(yaml).unwrap(), expected);
}

#[test]
fn metric_point_defaults_labels_when_absent() {
    let yaml = "timestamp: 1719220000\nvalue: 0.999\n";
    let point = MetricPoint::from_yaml_str(yaml).unwrap();

    assert!(point.labels.is_empty());
}

#[test]
fn http_slo_iterator_passes_for_prometheus_histogram() {
    let slo = HttpSlo::default();
    let sample = HistogramSample {
        timestamp: 1,
        success: 10_000,
        total: 10_000,
        buckets: vec![
            HistogramBucket {
                upper_bound_ms: 100.0,
                count: 9_700,
            },
            HistogramBucket {
                upper_bound_ms: 150.0,
                count: 9_900,
            },
            HistogramBucket {
                upper_bound_ms: 200.0,
                count: 9_970,
            },
            HistogramBucket {
                upper_bound_ms: 300.0,
                count: 10_000,
            },
        ],
        format: HistogramFormat::PrometheusCumulative,
    };

    let mut iter = HttpSloIterator::new(slo, vec![sample].into_iter());
    let result = iter.next().unwrap();

    assert_eq!(result.evaluated_percentile, 0.99);
    assert!(result.percentile_latency_ms < 200.0);
    assert!(result.latency_ok);
    assert!(result.availability_ok);
    assert!(result.pass);
    assert!(iter.next().is_none());
}

#[test]
fn http_slo_iterator_fails_when_p99_or_availability_miss() {
    let slo = HttpSlo::default();
    let samples = vec![
        HistogramSample {
            timestamp: 1,
            success: 10_000,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 9_500,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 9_600,
                },
                HistogramBucket {
                    upper_bound_ms: 500.0,
                    count: 10_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        },
        HistogramSample {
            timestamp: 2,
            success: 998,
            total: 1_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 970,
                },
                HistogramBucket {
                    upper_bound_ms: 150.0,
                    count: 995,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 1_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        },
    ];

    let results: Vec<HttpSloEvaluation> = HttpSloIterator::new(slo, samples.into_iter()).collect();
    assert_eq!(results.len(), 2);

    assert!(!results[0].latency_ok);
    assert!(results[0].availability_ok);
    assert!(!results[0].pass);

    assert!(results[1].latency_ok);
    assert!(!results[1].availability_ok);
    assert!(!results[1].pass);
}

#[test]
fn http_slo_iterator_supports_opentelemetry_delta_buckets() {
    let slo = HttpSlo::default();
    let sample = HistogramSample {
        timestamp: 1,
        success: 9_995,
        total: 10_000,
        buckets: vec![
            HistogramBucket {
                upper_bound_ms: 100.0,
                count: 9_700,
            },
            HistogramBucket {
                upper_bound_ms: 150.0,
                count: 200,
            },
            HistogramBucket {
                upper_bound_ms: 180.0,
                count: 70,
            },
            HistogramBucket {
                upper_bound_ms: 220.0,
                count: 30,
            },
        ],
        format: HistogramFormat::OpenTelemetryDelta,
    };

    let result = HttpSloIterator::new(slo, vec![sample].into_iter())
        .next()
        .unwrap();

    assert_eq!(result.evaluated_percentile, 0.99);
    assert!(result.percentile_latency_ms < 200.0);
    assert!(result.pass);
}

#[test]
fn http_slo_supports_custom_percentile_policy() {
    let slo = HttpSlo {
        latency_threshold_ms: 150.0,
        latency_percentile: 0.95,
        availability_threshold: 0.999,
    };
    let sample = HistogramSample {
        timestamp: 1,
        success: 9_999,
        total: 10_000,
        buckets: vec![
            HistogramBucket {
                upper_bound_ms: 100.0,
                count: 9_000,
            },
            HistogramBucket {
                upper_bound_ms: 140.0,
                count: 9_600,
            },
            HistogramBucket {
                upper_bound_ms: 200.0,
                count: 10_000,
            },
        ],
        format: HistogramFormat::PrometheusCumulative,
    };

    let result = slo.evaluate_histogram(&sample);
    assert_eq!(result.evaluated_percentile, 0.95);
    assert!(result.percentile_latency_ms <= 150.0);
    assert!(result.pass);
}

#[test]
fn stateful_slo_passes_when_all_signals_within_thresholds() {
    let slo = StatefulSlo::default();
    let sample = StatefulSample {
        timestamp: 1,
        replication_lag_ms: 120.0,
        queue_depth: 300,
        connection_pool_saturation: 0.6,
        connection_wait_time_ms: 10.0,
    };

    let evaluation = slo.evaluate_sample(&sample);
    assert!(evaluation.replication_lag_ok);
    assert!(evaluation.queue_depth_ok);
    assert!(evaluation.connection_pool_ok);
    assert!(!evaluation.connection_wait_penalized);
    assert!((evaluation.score - 1.0).abs() < 1e-9);
    assert!(evaluation.pass);
}

#[test]
fn stateful_slo_penalizes_excessive_connection_wait_time() {
    let slo = StatefulSlo {
        connection_wait_penalty_weight: 0.3,
        min_pass_score: 0.85,
        ..StatefulSlo::default()
    };
    let sample = StatefulSample {
        timestamp: 1,
        replication_lag_ms: 150.0,
        queue_depth: 200,
        connection_pool_saturation: 0.5,
        connection_wait_time_ms: 60.0,
    };

    let evaluation = slo.evaluate_sample(&sample);
    assert!(evaluation.replication_lag_ok);
    assert!(evaluation.queue_depth_ok);
    assert!(evaluation.connection_pool_ok);
    assert!(evaluation.connection_wait_penalized);
    assert!((evaluation.score - 0.7).abs() < 1e-9);
    assert!(!evaluation.pass);
}

#[test]
fn stateful_slo_iterator_handles_mixed_stream() {
    let slo = StatefulSlo::default();
    let samples = vec![
        StatefulSample {
            timestamp: 1,
            replication_lag_ms: 200.0,
            queue_depth: 900,
            connection_pool_saturation: 0.7,
            connection_wait_time_ms: 15.0,
        },
        StatefulSample {
            timestamp: 2,
            replication_lag_ms: 300.0,
            queue_depth: 1_200,
            connection_pool_saturation: 0.9,
            connection_wait_time_ms: 40.0,
        },
    ];

    let results: Vec<StatefulSloEvaluation> =
        StatefulSloIterator::new(slo, samples.into_iter()).collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].pass);
    assert!(!results[1].pass);
}

#[test]
fn pyo3_wrapper_surface_round_trip_methods_work() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let cfg = PySloConfig::new(99.9, "30d".to_string());
        let _ = cfg.target();
        let _ = cfg.window();
        let cfg_dict = cfg.to_dict(py).unwrap();
        let _ = cfg.to_json().unwrap();
        let _ = cfg.to_yaml().unwrap();
        let _ = PySloConfig::from_dict(&cfg_dict).unwrap();

        let budget = PyErrorBudget::new(0.5, 1.2);
        let _ = budget.remaining();
        let _ = budget.velocity();
        let budget_dict = budget.to_dict(py).unwrap();
        let _ = budget.to_json().unwrap();
        let _ = budget.to_yaml().unwrap();
        let _ = PyErrorBudget::from_dict(&budget_dict).unwrap();

        let point = PyMetricPoint::new(1, 0.9, None);
        let _ = point.timestamp();
        let _ = point.value();
        let _ = point.labels();
        let point_dict = point.to_dict(py).unwrap();
        let _ = point.to_json().unwrap();
        let _ = point.to_yaml().unwrap();
        let _ = PyMetricPoint::from_dict(&point_dict).unwrap();

        let window = PyTimeWindow::new(3_600, "rolling", 0).unwrap();
        let _ = window.alignment();
        let _ = window.window_seconds();
        let _ = window.timezone_offset_seconds();
        let _ = window.contains(1, 2);
        let window_dict = window.to_dict(py).unwrap();
        let _ = window.to_json().unwrap();
        let _ = window.to_yaml().unwrap();
        let _ = PyTimeWindow::from_dict(&window_dict).unwrap();

        let bucket = PyHistogramBucket::new(100.0, 10);
        let _ = bucket.upper_bound_ms();
        let _ = bucket.count();
        let bucket_dict = bucket.to_dict(py).unwrap();
        let _ = bucket.to_json().unwrap();
        let _ = bucket.to_yaml().unwrap();
        let _ = PyHistogramBucket::from_dict(&bucket_dict).unwrap();

        let sample = PyHistogramSample::new(
            1,
            100,
            100,
            vec![HistogramBucket {
                upper_bound_ms: 100.0,
                count: 100,
            }],
            "prometheus_cumulative",
        )
        .unwrap();
        let _ = sample.timestamp();
        let _ = sample.success();
        let _ = sample.total();
        let _ = sample.buckets();
        let _ = sample.format();
        let sample_dict = sample.to_dict(py).unwrap();
        let _ = sample.to_json().unwrap();
        let _ = sample.to_yaml().unwrap();
        let _ = PyHistogramSample::from_dict(&sample_dict).unwrap();

        let http = PyHttpSlo::new(200.0, 0.99, 0.999);
        let _ = http.latency_threshold_ms();
        let _ = http.latency_p99_threshold_ms();
        let _ = http.latency_percentile();
        let _ = http.availability_threshold();
        let http_dict = http.to_dict(py).unwrap();
        let _ = http.to_json().unwrap();
        let _ = http.to_yaml().unwrap();
        let _ = PyHttpSlo::from_dict(&http_dict).unwrap();
        let eval = http.evaluate_histogram(sample.inner.clone());
        let _ = eval.timestamp();
        let _ = eval.availability();
        let _ = eval.evaluated_percentile();
        let _ = eval.percentile_latency_ms();
        let _ = eval.p99_latency_ms();
        let _ = eval.latency_ok();
        let _ = eval.availability_ok();
        let _ = eval.pass();
        let _ = eval.to_dict(py).unwrap();
        let _ = eval.to_json().unwrap();
        let _ = eval.to_yaml().unwrap();
        let stream_eval = http.evaluate_stream(vec![sample.inner.clone()]);
        assert_eq!(stream_eval.len(), 1);

        let stateful_sample = PyStatefulSample::new(1, 100.0, 10, 0.2, 5.0);
        let _ = stateful_sample.timestamp();
        let _ = stateful_sample.replication_lag_ms();
        let _ = stateful_sample.queue_depth();
        let _ = stateful_sample.connection_pool_saturation();
        let _ = stateful_sample.connection_wait_time_ms();
        let stateful_sample_dict = stateful_sample.to_dict(py).unwrap();
        let _ = stateful_sample.to_json().unwrap();
        let _ = stateful_sample.to_yaml().unwrap();
        let _ = PyStatefulSample::from_dict(&stateful_sample_dict).unwrap();

        let stateful = PyStatefulSlo::new(250.0, 1_000, 0.8, 20.0, 0.2, 0.9);
        let _ = stateful.replication_lag_threshold_ms();
        let _ = stateful.queue_depth_threshold();
        let _ = stateful.connection_pool_saturation_threshold();
        let _ = stateful.connection_wait_time_threshold_ms();
        let _ = stateful.connection_wait_penalty_weight();
        let _ = stateful.min_pass_score();
        let stateful_dict = stateful.to_dict(py).unwrap();
        let _ = stateful.to_json().unwrap();
        let _ = stateful.to_yaml().unwrap();
        let _ = PyStatefulSlo::from_dict(&stateful_dict).unwrap();
        let state_eval = stateful.evaluate_sample(stateful_sample.inner.clone());
        let _ = state_eval.timestamp();
        let _ = state_eval.replication_lag_ok();
        let _ = state_eval.queue_depth_ok();
        let _ = state_eval.connection_pool_ok();
        let _ = state_eval.connection_wait_penalized();
        let _ = state_eval.score();
        let _ = state_eval.pass();
        let _ = state_eval.to_dict(py).unwrap();
        let _ = state_eval.to_json().unwrap();
        let _ = state_eval.to_yaml().unwrap();
        let state_stream = stateful.evaluate_stream(vec![stateful_sample.inner.clone()]);
        assert_eq!(state_stream.len(), 1);

        let ml_sample = PyMlSample::new(3, 180.0, 0.7, 0.09, 0.92);
        let _ = ml_sample.timestamp();
        let _ = ml_sample.inference_latency_ms();
        let _ = ml_sample.gpu_utilization();
        let _ = ml_sample.feature_drift();
        let _ = ml_sample.prediction_confidence();
        let ml_sample_dict = ml_sample.to_dict(py).unwrap();
        let _ = ml_sample.to_json().unwrap();
        let _ = ml_sample.to_yaml().unwrap();
        let _ = PyMlSample::from_dict(&ml_sample_dict).unwrap();

        let ml = PyMlSlo::new(200.0, 0.85, 0.2, 0.8, 0.6, 0.4, 0.9);
        let _ = ml.max_inference_latency_ms();
        let _ = ml.max_gpu_utilization();
        let _ = ml.max_feature_drift();
        let _ = ml.min_prediction_confidence();
        let _ = ml.latency_weight();
        let _ = ml.drift_weight();
        let _ = ml.min_pass_score();
        let ml_dict = ml.to_dict(py).unwrap();
        let _ = ml.to_json().unwrap();
        let _ = ml.to_yaml().unwrap();
        let _ = PyMlSlo::from_dict(&ml_dict).unwrap();
        let ml_eval = ml.evaluate_sample(ml_sample.inner.clone());
        let _ = ml_eval.timestamp();
        let _ = ml_eval.inference_latency_score();
        let _ = ml_eval.gpu_utilization_score();
        let _ = ml_eval.system_score();
        let _ = ml_eval.latency_score();
        let _ = ml_eval.feature_drift_score();
        let _ = ml_eval.prediction_confidence_score();
        let _ = ml_eval.drift_score();
        let _ = ml_eval.latency_weight();
        let _ = ml_eval.drift_weight();
        let _ = ml_eval.hybrid_score();
        let _ = ml_eval.pass();
        let _ = ml_eval.to_dict(py).unwrap();
        let _ = ml_eval.to_json().unwrap();
        let _ = ml_eval.to_yaml().unwrap();
        let ml_stream = ml.evaluate_stream(vec![ml_sample.inner.clone()]);
        assert_eq!(ml_stream.len(), 1);

        let genai_sample = PyGenAiSample::new(
            7,
            200,
            5_000.0,
            700.0,
            "hello world".to_string(),
            "hello world".to_string(),
        );
        let _ = genai_sample.timestamp();
        let _ = genai_sample.tokens_generated();
        let _ = genai_sample.generation_duration_ms();
        let _ = genai_sample.time_to_first_token_ms();
        let _ = genai_sample.reference_text();
        let _ = genai_sample.generated_text();
        let genai_sample_dict = genai_sample.to_dict(py).unwrap();
        let _ = genai_sample.to_json().unwrap();
        let _ = genai_sample.to_yaml().unwrap();
        let _ = PyGenAiSample::from_dict(&genai_sample_dict).unwrap();

        let genai = PyGenAiSlo::new(20.0, 1_200.0, 0.7, "sentence-transformers/all-MiniLM-L6-v2");
        let _ = genai.min_tokens_per_second();
        let _ = genai.max_time_to_first_token_ms();
        let _ = genai.min_semantic_similarity();
        let _ = genai.semantic_model_name();
        let genai_dict = genai.to_dict(py).unwrap();
        let _ = genai.to_json().unwrap();
        let _ = genai.to_yaml().unwrap();
        let _ = PyGenAiSlo::from_dict(&genai_dict).unwrap();
        let genai_eval = genai.evaluate_sample(genai_sample.inner.clone());
        let _ = genai_eval.timestamp();
        let _ = genai_eval.tokens_per_second();
        let _ = genai_eval.time_to_first_token_ms();
        let _ = genai_eval.semantic_similarity();
        let _ = genai_eval.tokens_per_second_ok();
        let _ = genai_eval.time_to_first_token_ok();
        let _ = genai_eval.semantic_similarity_ok();
        let _ = genai_eval.pass();
        let _ = genai_eval.to_dict(py).unwrap();
        let _ = genai_eval.to_json().unwrap();
        let _ = genai_eval.to_yaml().unwrap();
        let genai_stream = genai.evaluate_stream(vec![genai_sample.inner.clone()]);
        assert_eq!(genai_stream.len(), 1);

        let composite_service = PyCompositeServiceSlo::new("svc_a".to_string(), 0.95, 0.9, 2.0);
        let _ = composite_service.service();
        let _ = composite_service.local_score();
        let _ = composite_service.min_pass_score();
        let _ = composite_service.impact_weight();
        let composite_service_dict = composite_service.to_dict(py).unwrap();
        let _ = composite_service.to_json().unwrap();
        let _ = composite_service.to_yaml().unwrap();
        let _ = PyCompositeServiceSlo::from_dict(&composite_service_dict).unwrap();

        let composite_edge =
            PyCompositeDependencyEdge::new("svc_a".to_string(), "svc_b".to_string(), 0.2);
        let _ = composite_edge.dependency();
        let _ = composite_edge.dependent();
        let _ = composite_edge.failure_penalty();
        let composite_edge_dict = composite_edge.to_dict(py).unwrap();
        let _ = composite_edge.to_json().unwrap();
        let _ = composite_edge.to_yaml().unwrap();
        let _ = PyCompositeDependencyEdge::from_dict(&composite_edge_dict).unwrap();

        let composite_graph = PyCompositeSloGraph::new(
            vec![
                CompositeServiceSlo {
                    service: "svc_a".to_string(),
                    local_score: 0.95,
                    min_pass_score: 0.9,
                    impact_weight: 2.0,
                },
                CompositeServiceSlo {
                    service: "svc_b".to_string(),
                    local_score: 0.98,
                    min_pass_score: 0.9,
                    impact_weight: 1.0,
                },
            ],
            vec![CompositeDependencyEdge {
                dependency: "svc_a".to_string(),
                dependent: "svc_b".to_string(),
                failure_penalty: 0.1,
            }],
            0.9,
        );
        let _ = composite_graph.services();
        let _ = composite_graph.dependencies();
        let _ = composite_graph.global_min_pass_score();
        let composite_graph_dict = composite_graph.to_dict(py).unwrap();
        let _ = composite_graph.to_json().unwrap();
        let _ = composite_graph.to_yaml().unwrap();
        let _ = PyCompositeSloGraph::from_dict(&composite_graph_dict).unwrap();

        let composite_eval = composite_graph.evaluate().unwrap();
        let _ = composite_eval.topological_order();
        let _ = composite_eval.services();
        let _ = composite_eval.global_slo();
        let _ = composite_eval.global_pass();
        let _ = composite_eval.to_dict(py).unwrap();
        let _ = composite_eval.to_json().unwrap();
        let _ = composite_eval.to_yaml().unwrap();

        let _ = coerce_slo_config(cfg.inner.clone());
        let _ = coerce_error_budget(budget.inner.clone());
        let _ = coerce_metric_point(point.inner.clone());
        let _ = coerce_time_window(window.inner.clone());
        let _ = coerce_histogram_bucket(bucket.inner.clone());
        let _ = coerce_histogram_sample(sample.inner.clone());
        let _ = coerce_http_slo(http.inner.clone());
        let _ = coerce_stateful_sample(stateful_sample.inner.clone());
        let _ = coerce_stateful_slo(stateful.inner.clone());
        let _ = coerce_ml_sample(ml_sample.inner.clone());
        let _ = coerce_ml_slo(ml.inner.clone());
        let _ = coerce_genai_sample(genai_sample.inner.clone());
        let _ = coerce_genai_slo(genai.inner.clone());
        let _ = coerce_composite_service_slo(composite_service.inner.clone());
        let _ = coerce_composite_dependency_edge(composite_edge.inner.clone());
        let _ = coerce_composite_slo_graph(composite_graph.inner.clone());

        let _ = evaluate_http_slo_histogram(sample.inner.clone(), http.inner.clone());
        let _ = evaluate_http_slo_histogram_stream(vec![sample.inner.clone()], http.inner.clone());
        let _ = evaluate_stateful_slo(stateful_sample.inner.clone(), stateful.inner.clone());
        let _ = evaluate_stateful_slo_stream(
            vec![stateful_sample.inner.clone()],
            stateful.inner.clone(),
        );
        let _ = evaluate_ml_slo(ml_sample.inner.clone(), ml.inner.clone());
        let _ = evaluate_ml_slo_stream(vec![ml_sample.inner.clone()], ml.inner.clone());
        let _ = evaluate_genai_slo(genai_sample.inner.clone(), genai.inner.clone());
        let _ = evaluate_genai_slo_stream(vec![genai_sample.inner.clone()], genai.inner.clone());
        let _ = evaluate_composite_slo_graph(composite_graph.inner.clone()).unwrap();
        let _ = semantic_similarity_placeholder("hello world", "hello world", None);
    });
}

#[test]
fn default_configs_and_error_paths_are_exercised() {
    let outlier = OutlierFilterConfig::default();
    assert!(!outlier.enabled);
    assert_eq!(outlier.mad_threshold, 3.5);
    assert_eq!(outlier.min_samples, 10);

    assert!(parse_window_alignment("invalid").is_err());
    assert!(parse_histogram_format("invalid").is_err());
    assert_eq!(
        histogram_format_name(HistogramFormat::PrometheusCumulative),
        "prometheus_cumulative"
    );
    assert_eq!(window_alignment_name(WindowAlignment::Rolling), "rolling");

    assert_eq!(calculate_mad(&[]), None);
    assert_eq!(filter_statistical_outliers(&[], 3.5, 3).len(), 0);

    let sample = HistogramSample {
        timestamp: 1,
        success: 1,
        total: 1,
        buckets: vec![],
        format: HistogramFormat::PrometheusCumulative,
    };
    let result = HttpSlo::default().evaluate_histogram(&sample);
    assert!(!result.latency_ok);
}

#[test]
fn edge_cases_cover_helper_branches() {
    assert_eq!(calculate_availability(0, 0), 0.0);
    assert_eq!(calculate_error_budget(-0.5, 10), 10.0);
    assert_eq!(calculate_error_budget(2.0, 10), 0.0);
    assert_eq!(calculate_burn_rate(Vec::new(), 0), 0.0);
    assert_eq!(calculate_burn_rate(Vec::new(), 10), 0.0);
    assert!(!TimeWindow::rolling(0).contains(1, 2));
    assert!(!TimeWindow::calendar_aligned(0, 0).contains(1, 2));
    assert_eq!(
        window_alignment_name(WindowAlignment::CalendarAligned),
        "calendar_aligned"
    );
    assert_eq!(
        histogram_format_name(HistogramFormat::OpenTelemetryDelta),
        "open_telemetry_delta"
    );
    assert!(parse_window_alignment("rolling").is_ok());
    assert!(parse_window_alignment("calendar_aligned").is_ok());
    assert!(parse_histogram_format("prometheus_cumulative").is_ok());
    assert!(parse_histogram_format("open_telemetry_delta").is_ok());

    let empty_histogram = HistogramSample {
        timestamp: 1,
        success: 0,
        total: 0,
        buckets: vec![],
        format: HistogramFormat::PrometheusCumulative,
    };
    let empty_eval = HttpSlo::default().evaluate_histogram(&empty_histogram);
    assert!(!empty_eval.pass);

    let inf_histogram = HistogramSample {
        timestamp: 1,
        success: 100,
        total: 100,
        buckets: vec![
            HistogramBucket {
                upper_bound_ms: 10.0,
                count: 100,
            },
            HistogramBucket {
                upper_bound_ms: f64::INFINITY,
                count: 100,
            },
        ],
        format: HistogramFormat::PrometheusCumulative,
    };
    let inf_eval = HttpSlo::default().evaluate_histogram(&inf_histogram);
    assert!(inf_eval.percentile_latency_ms.is_finite());

    let weird_stateful = StatefulSample {
        timestamp: 1,
        replication_lag_ms: 1_000.0,
        queue_depth: 10_000,
        connection_pool_saturation: 1.0,
        connection_wait_time_ms: 1_000.0,
    };
    let weird_eval = StatefulSlo {
        connection_wait_penalty_weight: 1.0,
        min_pass_score: 0.99,
        ..StatefulSlo::default()
    }
    .evaluate_sample(&weird_stateful);
    assert!(!weird_eval.pass);

    let http_zero = WebApiSloPolicy {
        availability_target: 1.2,
        latency_threshold_ms: 0.0,
        time_window_seconds: 0,
        outlier_filter: OutlierFilterConfig::default(),
    };
    let report = calculate_web_api_slo(&[], &http_zero, 0);
    assert_eq!(report.total_requests, 0);
}

#[test]
fn weighted_stateful_policy_profiles_shift_tier_scores() {
    let profiles = StatefulPolicyProfileSet::default();
    assert_eq!(profiles.database.name, "database_primary");
    assert_eq!(profiles.queue.name, "queue_hot_path");

    let sample = StatefulSample {
        timestamp: 8,
        replication_lag_ms: 120.0,
        queue_depth: 700,
        connection_pool_saturation: 0.7,
        connection_wait_time_ms: 30.0,
    };
    let slo = StatefulSlo::default();

    let database_eval = slo.evaluate_sample_for_tier(&sample, StatefulTier::Database, &profiles);
    let queue_eval = slo.evaluate_sample_for_tier(&sample, StatefulTier::Queue, &profiles);

    assert!(database_eval.pass);
    assert!(!queue_eval.pass);
    assert!(database_eval.score > queue_eval.score);

    let profile_round_trip =
        StatefulPolicyProfileSet::from_json_str(&profiles.to_json_string().unwrap()).unwrap();
    assert_eq!(profile_round_trip, profiles);
}

#[test]
fn ml_slo_formula_matches_weighted_hybrid_definition() {
    let slo = MlSlo::default();
    let sample = MlSample {
        timestamp: 30,
        inference_latency_ms: 210.0,
        gpu_utilization: 0.9,
        feature_drift: 0.1,
        prediction_confidence: 0.9,
    };

    let evaluation = slo.evaluate_sample(&sample);

    let expected_latency_score =
        (evaluation.inference_latency_score + evaluation.gpu_utilization_score) / 2.0;
    let expected_drift_score =
        (evaluation.feature_drift_score + evaluation.prediction_confidence_score) / 2.0;
    let expected_hybrid = expected_latency_score * 0.6 + expected_drift_score * 0.4;

    assert!((evaluation.latency_score - expected_latency_score).abs() < 1e-9);
    assert!((evaluation.drift_score - expected_drift_score).abs() < 1e-9);
    assert!((evaluation.hybrid_score - expected_hybrid).abs() < 1e-9);
}

#[test]
fn ml_slo_rebalances_non_normalized_weights() {
    let slo = MlSlo {
        latency_weight: 12.0,
        drift_weight: 8.0,
        ..MlSlo::default()
    };
    let sample = MlSample {
        timestamp: 31,
        inference_latency_ms: 190.0,
        gpu_utilization: 0.8,
        feature_drift: 0.08,
        prediction_confidence: 0.9,
    };

    let evaluation = slo.evaluate_sample(&sample);
    assert!((evaluation.latency_weight - 0.6).abs() < 1e-9);
    assert!((evaluation.drift_weight - 0.4).abs() < 1e-9);
}

#[test]
fn ml_slo_fallback_weights_apply_when_config_is_invalid() {
    let slo = MlSlo {
        latency_weight: -1.0,
        drift_weight: -4.0,
        ..MlSlo::default()
    };
    let sample = MlSample {
        timestamp: 32,
        inference_latency_ms: 180.0,
        gpu_utilization: 0.7,
        feature_drift: 0.06,
        prediction_confidence: 0.95,
    };

    let evaluation = slo.evaluate_sample(&sample);
    assert!((evaluation.latency_weight - 0.6).abs() < 1e-9);
    assert!((evaluation.drift_weight - 0.4).abs() < 1e-9);
    assert!(evaluation.pass);
}

#[test]
fn genai_slo_tracks_tps_ttft_and_semantic_similarity() {
    let slo = GenAiSlo::default();
    let sample = GenAiSample {
        timestamp: 55,
        tokens_generated: 240,
        generation_duration_ms: 6_000.0,
        time_to_first_token_ms: 800.0,
        reference_text: "the cat sat on the mat".to_string(),
        generated_text: "the cat sat on the mat".to_string(),
    };

    let evaluation = slo.evaluate_sample(&sample);
    assert!((evaluation.tokens_per_second - 40.0).abs() < 1e-9);
    assert!(evaluation.tokens_per_second_ok);
    assert!(evaluation.time_to_first_token_ok);
    assert!(evaluation.semantic_similarity_ok);
    assert!(evaluation.semantic_similarity >= 0.95);
    assert!(evaluation.pass);
}

#[test]
fn semantic_similarity_placeholder_is_ordered_for_simple_inputs() {
    let same = semantic_similarity_placeholder("hello world", "hello world", None);
    let different = semantic_similarity_placeholder("hello world", "quantum banana", None);

    assert!((0.0..=1.0).contains(&same));
    assert!((0.0..=1.0).contains(&different));
    assert!(same >= different);
}

#[test]
fn python_module_registration_exports_expected_symbols() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "neuralbudget_module_init_test").unwrap();
        neuralbudget(py, &module).unwrap();

        for name in [
            "SloConfig",
            "ErrorBudget",
            "MetricPoint",
            "TimeWindow",
            "HistogramBucket",
            "HistogramSample",
            "HttpSlo",
            "HttpSloEvaluation",
            "StatefulSample",
            "StatefulSlo",
            "StatefulSloEvaluation",
            "MlSample",
            "MlSlo",
            "MlSloEvaluation",
            "GenAiSample",
            "GenAiSlo",
            "GenAiSloEvaluation",
            "CompositeServiceSlo",
            "CompositeDependencyEdge",
            "CompositeServiceSloEvaluation",
            "CompositeSloEvaluation",
            "CompositeSloGraph",
            "calculate_availability",
            "calculate_error_budget",
            "calculate_burn_rate",
            "semantic_similarity_placeholder",
            "is_timestamp_in_window",
            "evaluate_http_slo_histogram",
            "evaluate_http_slo_histogram_stream",
            "evaluate_stateful_slo",
            "evaluate_stateful_slo_stream",
            "evaluate_ml_slo",
            "evaluate_ml_slo_stream",
            "evaluate_genai_slo",
            "evaluate_genai_slo_stream",
            "evaluate_composite_slo_graph",
            "coerce_slo_config",
            "coerce_error_budget",
            "coerce_metric_point",
            "coerce_time_window",
            "coerce_histogram_bucket",
            "coerce_histogram_sample",
            "coerce_http_slo",
            "coerce_stateful_sample",
            "coerce_stateful_slo",
            "coerce_ml_sample",
            "coerce_ml_slo",
            "coerce_genai_sample",
            "coerce_genai_slo",
            "coerce_composite_service_slo",
            "coerce_composite_dependency_edge",
            "coerce_composite_slo_graph",
        ] {
            assert!(module.getattr(name).is_ok(), "missing symbol: {name}");
        }
    });
}

#[test]
fn wrapper_extract_fast_paths_and_dict_fallbacks_work() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let cfg_obj = Py::new(py, PySloConfig::new(99.95, "7d".to_string())).unwrap();
        let cfg: SloConfig = cfg_obj.bind(py).extract().unwrap();
        assert_eq!(cfg.target, 99.95);

        let budget_obj = Py::new(py, PyErrorBudget::new(0.75, 1.1)).unwrap();
        let budget: ErrorBudget = budget_obj.bind(py).extract().unwrap();
        assert_eq!(budget.remaining, 0.75);

        let point_obj = Py::new(py, PyMetricPoint::new(10, 0.42, None)).unwrap();
        let point: MetricPoint = point_obj.bind(py).extract().unwrap();
        assert_eq!(point.timestamp, 10);

        let rolling_obj = Py::new(py, PyTimeWindow::rolling(300)).unwrap();
        let rolling: TimeWindow = rolling_obj.bind(py).extract().unwrap();
        assert_eq!(rolling.alignment, WindowAlignment::Rolling);

        let aligned_obj = Py::new(py, PyTimeWindow::calendar_aligned(86_400, 18_000)).unwrap();
        let aligned: TimeWindow = aligned_obj.bind(py).extract().unwrap();
        assert_eq!(aligned.alignment, WindowAlignment::CalendarAligned);

        let bucket_obj = Py::new(py, PyHistogramBucket::new(250.0, 10)).unwrap();
        let bucket: HistogramBucket = bucket_obj.bind(py).extract().unwrap();
        assert_eq!(bucket.upper_bound_ms, 250.0);

        let sample_obj = Py::new(
            py,
            PyHistogramSample::new(1, 100, 100, vec![bucket.clone()], "prometheus_cumulative")
                .unwrap(),
        )
        .unwrap();
        let sample: HistogramSample = sample_obj.bind(py).extract().unwrap();
        assert_eq!(sample.total, 100);

        let http_obj = Py::new(py, PyHttpSlo::new(200.0, 0.99, 0.999)).unwrap();
        let http: HttpSlo = http_obj.bind(py).extract().unwrap();
        assert_eq!(http.latency_percentile, 0.99);

        let state_sample_obj = Py::new(py, PyStatefulSample::new(1, 100.0, 20, 0.4, 5.0)).unwrap();
        let state_sample: StatefulSample = state_sample_obj.bind(py).extract().unwrap();
        assert_eq!(state_sample.queue_depth, 20);

        let state_slo_obj =
            Py::new(py, PyStatefulSlo::new(250.0, 1_000, 0.8, 20.0, 0.2, 0.9)).unwrap();
        let state_slo: StatefulSlo = state_slo_obj.bind(py).extract().unwrap();
        assert_eq!(state_slo.queue_depth_threshold, 1_000);

        let ml_sample_obj = Py::new(py, PyMlSample::new(3, 150.0, 0.6, 0.08, 0.95)).unwrap();
        let ml_sample: MlSample = ml_sample_obj.bind(py).extract().unwrap();
        assert_eq!(ml_sample.timestamp, 3);

        let ml_slo_obj = Py::new(py, PyMlSlo::new(200.0, 0.85, 0.2, 0.8, 0.6, 0.4, 0.9)).unwrap();
        let ml_slo: MlSlo = ml_slo_obj.bind(py).extract().unwrap();
        assert_eq!(ml_slo.max_feature_drift, 0.2);

        let genai_sample_obj = Py::new(
            py,
            PyGenAiSample::new(
                7,
                256,
                8_000.0,
                900.0,
                "the cat sat on the mat".to_string(),
                "the cat sat on the mat".to_string(),
            ),
        )
        .unwrap();
        let genai_sample: GenAiSample = genai_sample_obj.bind(py).extract().unwrap();
        assert_eq!(genai_sample.tokens_generated, 256);

        let genai_slo_obj = Py::new(
            py,
            PyGenAiSlo::new(20.0, 1_200.0, 0.7, "sentence-transformers/all-MiniLM-L6-v2"),
        )
        .unwrap();
        let genai_slo: GenAiSlo = genai_slo_obj.bind(py).extract().unwrap();
        assert_eq!(genai_slo.min_tokens_per_second, 20.0);

        let dict = PyDict::new_bound(py);
        dict.set_item("latency_p99_threshold_ms", 175.0).unwrap();
        let fallback_http: HttpSlo = dict.extract().unwrap();
        assert_eq!(fallback_http.latency_threshold_ms, 175.0);
        assert_eq!(
            fallback_http.latency_percentile,
            HttpSlo::default().latency_percentile
        );
        assert_eq!(
            fallback_http.availability_threshold,
            HttpSlo::default().availability_threshold
        );

        let empty = PyDict::new_bound(py);
        assert!(extract_labels(&empty).unwrap().is_empty());

        let err = extract_required::<i64>(&empty, "missing").unwrap_err();
        assert!(err.to_string().contains("missing required key 'missing'"));

        let py_int = 123_i64.into_py(py);
        let bad_extract = py_int.bind(py).extract::<SloConfig>();
        assert!(bad_extract.is_err());
        assert!(bad_extract
            .unwrap_err()
            .to_string()
            .contains("expected dict or SloConfig instance"));
    });
}

#[test]
fn core_branch_edges_are_exercised_for_maximum_coverage() {
    let default_profile = StatefulPolicyProfile::default();
    assert_eq!(default_profile.name, StatefulPolicyProfile::database().name);

    let no_center = mad_outlier_mask(&[], 3.5, 0);
    assert!(no_center.is_empty());

    let zero_mad = mad_outlier_mask(&[1.0, 1.0, 1.0], 3.5, 3);
    assert_eq!(zero_mad, vec![false, false, false]);

    let inf_percentile = percentile_from_histogram(
        &[
            HistogramBucket {
                upper_bound_ms: 100.0,
                count: 50,
            },
            HistogramBucket {
                upper_bound_ms: f64::INFINITY,
                count: 100,
            },
        ],
        HistogramFormat::PrometheusCumulative,
        0.99,
    )
    .unwrap();
    assert_eq!(inf_percentile, 100.0);

    let nan_percentile = percentile_from_histogram(
        &[
            HistogramBucket {
                upper_bound_ms: 100.0,
                count: 5,
            },
            HistogramBucket {
                upper_bound_ms: 200.0,
                count: 10,
            },
        ],
        HistogramFormat::PrometheusCumulative,
        f64::NAN,
    )
    .unwrap();
    assert_eq!(nan_percentile, 200.0);

    let huge_window_burn = calculate_burn_rate(
        vec![MetricPoint {
            timestamp: 1,
            value: 1.0,
            labels: HashMap::new(),
        }],
        u64::MAX,
    );
    assert_eq!(huge_window_burn, 0.0);

    let underflow_burn = calculate_burn_rate(
        vec![MetricPoint {
            timestamp: i64::MIN,
            value: 1.0,
            labels: HashMap::new(),
        }],
        i64::MAX as u64,
    );
    assert_eq!(underflow_burn, 0.0);

    let overflow_window_report = calculate_web_api_slo(
        &[WebApiRequest {
            timestamp: i64::MIN,
            latency_ms: 10.0,
            status_code: 200,
            labels: HashMap::new(),
        }],
        &WebApiSloPolicy {
            availability_target: 0.99,
            latency_threshold_ms: 50.0,
            time_window_seconds: i64::MAX as u64,
            outlier_filter: OutlierFilterConfig::default(),
        },
        i64::MIN,
    );
    assert_eq!(overflow_window_report.total_requests, 0);

    let weighted_fail = StatefulSlo::default().evaluate_sample_with_profile(
        &StatefulSample {
            timestamp: 99,
            replication_lag_ms: 1_000.0,
            queue_depth: 10_000,
            connection_pool_saturation: 1.0,
            connection_wait_time_ms: 100.0,
        },
        &StatefulPolicyProfile {
            name: "strict".to_string(),
            tier: StatefulTier::Database,
            replication_lag_weight: 1.0,
            queue_depth_weight: 1.0,
            connection_pool_weight: 1.0,
            connection_wait_penalty_weight: 1.0,
            min_pass_score: 0.99,
        },
    );
    assert!(!weighted_fail.pass);

    let invalid_thresholds = MlSlo {
        max_inference_latency_ms: 0.0,
        max_gpu_utilization: 0.0,
        max_feature_drift: 0.0,
        min_prediction_confidence: 0.0,
        ..MlSlo::default()
    };
    let invalid_eval = invalid_thresholds.evaluate_sample(&MlSample {
        timestamp: 40,
        inference_latency_ms: 100.0,
        gpu_utilization: 0.5,
        feature_drift: 0.1,
        prediction_confidence: 0.9,
    });
    assert_eq!(invalid_eval.inference_latency_score, 0.0);
    assert_eq!(invalid_eval.gpu_utilization_score, 0.0);
    assert_eq!(invalid_eval.feature_drift_score, 0.0);
    assert_eq!(invalid_eval.prediction_confidence_score, 0.0);
    assert!(!invalid_eval.pass);
}

#[test]
fn time_window_overflow_paths_return_false() {
    let rolling = TimeWindow::rolling(1);
    assert!(!rolling.contains(0, i64::MIN));

    let calendar_now_overflow = TimeWindow::calendar_aligned(86_400, i32::MAX);
    assert!(!calendar_now_overflow.contains(0, i64::MAX));

    let calendar_timestamp_overflow = TimeWindow::calendar_aligned(86_400, i32::MAX);
    assert!(!calendar_timestamp_overflow.contains(i64::MAX, 0));

    let huge_window = TimeWindow::calendar_aligned(u64::MAX, 0);
    assert!(!huge_window.contains(0, i64::MAX));
}

#[test]
fn histogram_percentile_handles_zero_totals_and_zero_bucket_counts() {
    let empty_total = HistogramSample {
        timestamp: 1,
        success: 0,
        total: 0,
        buckets: vec![HistogramBucket {
            upper_bound_ms: 100.0,
            count: 0,
        }],
        format: HistogramFormat::PrometheusCumulative,
    };
    let slo = HttpSlo {
        latency_threshold_ms: 200.0,
        latency_percentile: 0.99,
        availability_threshold: 0.9,
    };
    let eval = slo.evaluate_histogram(&empty_total);
    assert!(eval.percentile_latency_ms.is_infinite());

    // For percentile=0.0 the first cumulative bucket can match target with zero bucket count.
    let zero_bucket_count = HistogramSample {
        timestamp: 2,
        success: 10,
        total: 10,
        buckets: vec![
            HistogramBucket {
                upper_bound_ms: 100.0,
                count: 0,
            },
            HistogramBucket {
                upper_bound_ms: 200.0,
                count: 10,
            },
        ],
        format: HistogramFormat::PrometheusCumulative,
    };
    let eval_zero = HttpSlo {
        latency_threshold_ms: 500.0,
        latency_percentile: 0.0,
        availability_threshold: 0.0,
    }
    .evaluate_histogram(&zero_bucket_count);
    assert!(eval_zero.percentile_latency_ms.is_infinite());
}

#[test]
fn web_api_slo_zero_window_returns_zeroed_report() {
    let report = calculate_web_api_slo(
        &[WebApiRequest {
            timestamp: 1,
            latency_ms: 10.0,
            status_code: 200,
            labels: HashMap::new(),
        }],
        &WebApiSloPolicy {
            availability_target: 0.99,
            latency_threshold_ms: 100.0,
            time_window_seconds: 0,
            outlier_filter: OutlierFilterConfig::default(),
        },
        1,
    );

    assert_eq!(report.total_requests, 0);
    assert_eq!(report.successful_requests, 0);
    assert_eq!(report.availability, 0.0);
    assert_eq!(report.latency_sli, 0.0);
}

#[test]
fn composite_global_score_fallback_paths_are_exercised() {
    // Empty graph exercises empty-service fallback in global score calculation.
    let empty = evaluate_composite_slo(&CompositeSloGraph::default()).unwrap();
    assert_eq!(empty.global_slo, 0.0);

    // All non-positive impact weights exercise average-score fallback branch.
    let negative_weights = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "a".to_string(),
                local_score: 0.5,
                min_pass_score: 0.0,
                impact_weight: -1.0,
            },
            CompositeServiceSlo {
                service: "b".to_string(),
                local_score: 0.9,
                min_pass_score: 0.0,
                impact_weight: 0.0,
            },
        ],
        dependencies: vec![],
        global_min_pass_score: 0.0,
    };
    let eval = evaluate_composite_slo(&negative_weights).unwrap();
    assert!((eval.global_slo - 0.7).abs() < 1e-12);
}

#[test]
fn composite_error_display_variants_are_exercised() {
    let messages = vec![
        CompositeSloError::DuplicateService("svc".to_string()).to_string(),
        CompositeSloError::DuplicateDependencyEdge {
            dependency: "a".to_string(),
            dependent: "b".to_string(),
        }
        .to_string(),
        CompositeSloError::UnknownService("ghost".to_string()).to_string(),
        CompositeSloError::SelfDependency("svc".to_string()).to_string(),
        CompositeSloError::CycleDetected.to_string(),
    ];

    assert!(messages[0].contains("duplicate service"));
    assert!(messages[1].contains("duplicate dependency edge"));
    assert!(messages[2].contains("unknown service"));
    assert!(messages[3].contains("cannot depend on itself"));
    assert!(messages[4].contains("contains a cycle"));
}

#[test]
fn semantic_similarity_additional_branches_are_exercised() {
    assert_eq!(semantic_similarity_placeholder("", "", None), 1.0);

    pyo3::prepare_freethreaded_python();
    let with_model = semantic_similarity_placeholder("alpha", "beta", Some("missing-model"));
    assert!((0.0..=1.0).contains(&with_model));

    let genai = GenAiSlo::default();
    let eval = genai.evaluate_sample(&GenAiSample {
        timestamp: 1,
        tokens_generated: 100,
        generation_duration_ms: 0.0,
        time_to_first_token_ms: 100.0,
        reference_text: "alpha".to_string(),
        generated_text: "alpha".to_string(),
    });
    assert_eq!(eval.tokens_per_second, 0.0);
}

#[test]
fn pyo3_wrapper_coverage_sweep_for_remaining_branches() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let cfg = PySloConfig::new(99.9, "30d".to_string());
        assert_eq!(cfg.target(), 99.9);
        assert_eq!(cfg.window(), "30d".to_string());
        let cfg_dict = cfg.to_dict(py).unwrap();
        let cfg2 = PySloConfig::from_dict(&cfg_dict).unwrap();
        assert_eq!(cfg2.window(), "30d".to_string());
        assert!(cfg.to_json().unwrap().contains("target"));
        assert!(cfg.to_yaml().unwrap().contains("window"));

        let budget = PyErrorBudget::new(0.8, 1.2);
        assert_eq!(budget.remaining(), 0.8);
        assert_eq!(budget.velocity(), 1.2);
        let budget_dict = budget.to_dict(py).unwrap();
        assert_eq!(PyErrorBudget::from_dict(&budget_dict).unwrap().remaining(), 0.8);
        assert!(budget.to_json().unwrap().contains("remaining"));
        assert!(budget.to_yaml().unwrap().contains("velocity"));

        let point = PyMetricPoint::new(10, 2.5, Some(HashMap::from([(String::from("k"), String::from("v"))])));
        assert_eq!(point.timestamp(), 10);
        assert_eq!(point.value(), 2.5);
        assert_eq!(point.labels().get("k").unwrap(), "v");
        let point_dict = point.to_dict(py).unwrap();
        assert_eq!(PyMetricPoint::from_dict(&point_dict).unwrap().timestamp(), 10);
        assert!(point.to_json().unwrap().contains("timestamp"));
        assert!(point.to_yaml().unwrap().contains("value"));

        let tw = PyTimeWindow::new(300, "rolling", 0).unwrap();
        assert_eq!(tw.alignment(), "rolling");
        assert_eq!(tw.window_seconds(), 300);
        assert_eq!(tw.timezone_offset_seconds(), 0);
        assert!(tw.contains(100, 120));
        let tw_dict = tw.to_dict(py).unwrap();
        let tw2 = PyTimeWindow::from_dict(&tw_dict).unwrap();
        assert_eq!(tw2.alignment(), "rolling");
        assert!(tw.to_json().unwrap().contains("window_seconds"));
        assert!(tw.to_yaml().unwrap().contains("alignment"));

        // Dict fallback for missing timezone offset.
        let tw_no_tz = PyDict::new_bound(py);
        tw_no_tz.set_item("alignment", "calendar_aligned").unwrap();
        tw_no_tz.set_item("window_seconds", 600_u64).unwrap();
        let parsed_tw: TimeWindow = tw_no_tz.extract().unwrap();
        assert_eq!(parsed_tw.timezone_offset_seconds, 0);

        let bucket = PyHistogramBucket::new(250.0, 7);
        assert_eq!(bucket.upper_bound_ms(), 250.0);
        assert_eq!(bucket.count(), 7);
        let bucket_dict = bucket.to_dict(py).unwrap();
        let bucket2 = PyHistogramBucket::from_dict(&bucket_dict).unwrap();
        assert_eq!(bucket2.count(), 7);
        assert!(bucket.to_json().unwrap().contains("count"));
        assert!(bucket.to_yaml().unwrap().contains("upper_bound_ms"));

        let hs = PyHistogramSample::new(1, 95, 100, vec![HistogramBucket { upper_bound_ms: 250.0, count: 95 }], "prometheus_cumulative").unwrap();
        assert_eq!(hs.timestamp(), 1);
        assert_eq!(hs.success(), 95);
        assert_eq!(hs.total(), 100);
        assert_eq!(hs.buckets().len(), 1);
        assert_eq!(hs.format(), "prometheus_cumulative");
        let hs_dict = hs.to_dict(py).unwrap();
        assert_eq!(PyHistogramSample::from_dict(&hs_dict).unwrap().total(), 100);
        assert!(hs.to_json().unwrap().contains("buckets"));
        assert!(hs.to_yaml().unwrap().contains("format"));

        let http_default_dict = PyDict::new_bound(py);
        let http_defaults: HttpSlo = http_default_dict.extract().unwrap();
        assert_eq!(http_defaults.latency_threshold_ms, HttpSlo::default().latency_threshold_ms);
        assert_eq!(http_defaults.latency_percentile, HttpSlo::default().latency_percentile);
        assert_eq!(http_defaults.availability_threshold, HttpSlo::default().availability_threshold);

        let http = PyHttpSlo::new(200.0, 0.99, 0.999);
        assert_eq!(http.latency_p99_threshold_ms(), 200.0);
        let http_eval = http.evaluate_histogram(HistogramSample {
            timestamp: 1,
            success: 99,
            total: 100,
            buckets: vec![
                HistogramBucket { upper_bound_ms: 100.0, count: 98 },
                HistogramBucket { upper_bound_ms: 200.0, count: 100 },
            ],
            format: HistogramFormat::PrometheusCumulative,
        });
        assert_eq!(http_eval.timestamp(), 1);
        assert_eq!(http_eval.p99_latency_ms(), http_eval.percentile_latency_ms());
        assert!(http_eval.to_dict(py).unwrap().contains("pass").unwrap());
        assert!(http_eval.to_json().unwrap().contains("availability"));
        assert!(http_eval.to_yaml().unwrap().contains("latency_ok"));
        let _ = http.evaluate_stream(vec![HistogramSample {
            timestamp: 2,
            success: 100,
            total: 100,
            buckets: vec![HistogramBucket { upper_bound_ms: 100.0, count: 100 }],
            format: HistogramFormat::PrometheusCumulative,
        }]);
        let _ = http.to_dict(py).unwrap();
        assert!(http.to_json().unwrap().contains("latency_percentile"));
        assert!(http.to_yaml().unwrap().contains("availability_threshold"));

        let state_sample = PyStatefulSample::new(1, 50.0, 10, 0.5, 10.0);
        assert_eq!(state_sample.connection_pool_saturation(), 0.5);
        let state_sample_dict = state_sample.to_dict(py).unwrap();
        assert_eq!(PyStatefulSample::from_dict(&state_sample_dict).unwrap().queue_depth(), 10);
        assert!(state_sample.to_json().unwrap().contains("queue_depth"));
        assert!(state_sample.to_yaml().unwrap().contains("replication_lag_ms"));

        let state_slo = PyStatefulSlo::new(200.0, 100, 0.8, 20.0, 0.2, 0.9);
        let state_eval = state_slo.evaluate_sample(StatefulSample {
            timestamp: 1,
            replication_lag_ms: 100.0,
            queue_depth: 20,
            connection_pool_saturation: 0.6,
            connection_wait_time_ms: 10.0,
        });
        assert!(state_eval.replication_lag_ok());
        assert!(state_eval.to_dict(py).unwrap().contains("connection_wait_penalized").unwrap());
        assert!(state_eval.to_json().unwrap().contains("score"));
        assert!(state_eval.to_yaml().unwrap().contains("pass"));
        let _ = state_slo.evaluate_stream(vec![StatefulSample {
            timestamp: 2,
            replication_lag_ms: 90.0,
            queue_depth: 10,
            connection_pool_saturation: 0.4,
            connection_wait_time_ms: 5.0,
        }]);
        let _ = state_slo.to_dict(py).unwrap();
        assert!(state_slo.to_json().unwrap().contains("queue_depth_threshold"));
        assert!(state_slo.to_yaml().unwrap().contains("min_pass_score"));

        let ml_default_dict = PyDict::new_bound(py);
        ml_default_dict.set_item("max_inference_latency_ms", 300.0).unwrap();
        ml_default_dict.set_item("max_gpu_utilization", 0.9).unwrap();
        ml_default_dict.set_item("max_feature_drift", 0.3).unwrap();
        ml_default_dict.set_item("min_prediction_confidence", 0.7).unwrap();
        let ml_defaults: MlSlo = ml_default_dict.extract().unwrap();
        assert_eq!(ml_defaults.latency_weight, MlSlo::default().latency_weight);
        assert_eq!(ml_defaults.drift_weight, MlSlo::default().drift_weight);
        assert_eq!(ml_defaults.min_pass_score, MlSlo::default().min_pass_score);

        let ml_sample = PyMlSample::new(1, 100.0, 0.7, 0.1, 0.9);
        assert_eq!(ml_sample.inference_latency_ms(), 100.0);
        let ml_sample_dict = ml_sample.to_dict(py).unwrap();
        assert_eq!(PyMlSample::from_dict(&ml_sample_dict).unwrap().timestamp(), 1);
        assert!(ml_sample.to_json().unwrap().contains("feature_drift"));
        assert!(ml_sample.to_yaml().unwrap().contains("prediction_confidence"));

        let ml_slo = PyMlSlo::new(200.0, 0.85, 0.2, 0.8, 0.6, 0.4, 0.9);
        let ml_eval = ml_slo.evaluate_sample(MlSample {
            timestamp: 1,
            inference_latency_ms: 120.0,
            gpu_utilization: 0.6,
            feature_drift: 0.1,
            prediction_confidence: 0.9,
        });
        assert!(ml_eval.system_score() > 0.0);
        assert!(ml_eval.to_dict(py).unwrap().contains("prediction_confidence_score").unwrap());
        assert!(ml_eval.to_json().unwrap().contains("hybrid_score"));
        assert!(ml_eval.to_yaml().unwrap().contains("latency_weight"));
        let _ = ml_slo.evaluate_stream(vec![MlSample {
            timestamp: 2,
            inference_latency_ms: 150.0,
            gpu_utilization: 0.7,
            feature_drift: 0.15,
            prediction_confidence: 0.92,
        }]);
        let _ = ml_slo.to_dict(py).unwrap();
        assert!(ml_slo.to_json().unwrap().contains("max_gpu_utilization"));
        assert!(ml_slo.to_yaml().unwrap().contains("min_prediction_confidence"));

        let genai_default_dict = PyDict::new_bound(py);
        genai_default_dict.set_item("min_tokens_per_second", 10.0).unwrap();
        genai_default_dict.set_item("max_time_to_first_token_ms", 1500.0).unwrap();
        genai_default_dict.set_item("min_semantic_similarity", 0.6).unwrap();
        let genai_defaults: GenAiSlo = genai_default_dict.extract().unwrap();
        assert_eq!(
            genai_defaults.semantic_model_name,
            GenAiSlo::default().semantic_model_name
        );

        let genai_sample = PyGenAiSample::new(
            1,
            200,
            8_000.0,
            900.0,
            "hello world".to_string(),
            "hello world".to_string(),
        );
        assert_eq!(genai_sample.tokens_generated(), 200);
        let genai_sample_dict = genai_sample.to_dict(py).unwrap();
        assert_eq!(PyGenAiSample::from_dict(&genai_sample_dict).unwrap().timestamp(), 1);
        assert!(genai_sample.to_json().unwrap().contains("reference_text"));
        assert!(genai_sample.to_yaml().unwrap().contains("generated_text"));

        let genai_slo = PyGenAiSlo::new(20.0, 1200.0, 0.7, "sentence-transformers/all-MiniLM-L6-v2");
        let genai_eval = genai_slo.evaluate_sample(GenAiSample {
            timestamp: 1,
            tokens_generated: 200,
            generation_duration_ms: 8_000.0,
            time_to_first_token_ms: 900.0,
            reference_text: "hello world".to_string(),
            generated_text: "hello world".to_string(),
        });
        assert!(genai_eval.semantic_similarity_ok());
        assert!(genai_eval.to_dict(py).unwrap().contains("tokens_per_second_ok").unwrap());
        assert!(genai_eval.to_json().unwrap().contains("semantic_similarity"));
        assert!(genai_eval.to_yaml().unwrap().contains("time_to_first_token_ok"));
        let _ = genai_slo.evaluate_stream(vec![GenAiSample {
            timestamp: 2,
            tokens_generated: 250,
            generation_duration_ms: 9_000.0,
            time_to_first_token_ms: 850.0,
            reference_text: "a b c".to_string(),
            generated_text: "a b c".to_string(),
        }]);
        let _ = genai_slo.to_dict(py).unwrap();
        assert!(genai_slo.to_json().unwrap().contains("min_semantic_similarity"));
        assert!(genai_slo.to_yaml().unwrap().contains("semantic_model_name"));

        let composite_service = PyCompositeServiceSlo::new("api".to_string(), 0.95, 0.9, 1.0);
        assert_eq!(composite_service.service(), "api");
        let cs_dict = composite_service.to_dict(py).unwrap();
        assert_eq!(PyCompositeServiceSlo::from_dict(&cs_dict).unwrap().impact_weight(), 1.0);
        assert!(composite_service.to_json().unwrap().contains("local_score"));
        assert!(composite_service.to_yaml().unwrap().contains("min_pass_score"));

        let composite_edge = PyCompositeDependencyEdge::new("db".to_string(), "api".to_string(), 0.3);
        assert_eq!(composite_edge.dependent(), "api");
        let ce_dict = composite_edge.to_dict(py).unwrap();
        assert_eq!(PyCompositeDependencyEdge::from_dict(&ce_dict).unwrap().failure_penalty(), 0.3);
        assert!(composite_edge.to_json().unwrap().contains("dependency"));
        assert!(composite_edge.to_yaml().unwrap().contains("failure_penalty"));

        let graph = PyCompositeSloGraph::new(
            vec![CompositeServiceSlo {
                service: "api".to_string(),
                local_score: 0.95,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
            CompositeServiceSlo {
                service: "db".to_string(),
                local_score: 0.99,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            }],
            vec![CompositeDependencyEdge {
                dependency: "db".to_string(),
                dependent: "api".to_string(),
                failure_penalty: 0.2,
            }],
            0.9,
        );
        let graph_dict = graph.to_dict(py).unwrap();
        let graph_from = PyCompositeSloGraph::from_dict(&graph_dict).unwrap();
        assert_eq!(graph_from.global_min_pass_score(), 0.9);
        assert!(graph.to_json().unwrap().contains("services"));
        assert!(graph.to_yaml().unwrap().contains("dependencies"));

        // Dict fallback for missing global_min_pass_score.
        let graph_default_dict = PyDict::new_bound(py);
        graph_default_dict
            .set_item("services", Vec::<i32>::new())
            .unwrap();
        graph_default_dict
            .set_item("dependencies", Vec::<i32>::new())
            .unwrap();
        let graph_default: CompositeSloGraph = graph_default_dict.extract().unwrap();
        assert_eq!(
            graph_default.global_min_pass_score,
            CompositeSloGraph::default().global_min_pass_score
        );

        let composite_eval = evaluate_composite_slo_graph(graph.inner.clone()).unwrap();
        let _ = composite_eval.topological_order();
        let services = composite_eval.services();
        if let Some(first) = services.first() {
            let _ = first.service();
            let _ = first.local_score();
            let _ = first.effective_score();
            let _ = first.min_pass_score();
            let _ = first.dependency_adjusted();
            let _ = first.failed_dependencies();
            let _ = first.pass();
            let _ = first.to_dict(py).unwrap();
            assert!(first.to_json().unwrap().contains("effective_score"));
            assert!(first.to_yaml().unwrap().contains("dependency_adjusted"));
        }
        let _ = composite_eval.to_dict(py).unwrap();
        assert!(composite_eval.to_json().unwrap().contains("global_slo"));
        assert!(composite_eval.to_yaml().unwrap().contains("global_pass"));

        // Wrapper extraction fast paths for composite wrappers.
        let service_obj = Py::new(py, composite_service.clone()).unwrap();
        let _: CompositeServiceSlo = service_obj.bind(py).extract().unwrap();
        let edge_obj = Py::new(py, composite_edge.clone()).unwrap();
        let _: CompositeDependencyEdge = edge_obj.bind(py).extract().unwrap();
        let graph_obj = Py::new(py, graph.clone()).unwrap();
        let _: CompositeSloGraph = graph_obj.bind(py).extract().unwrap();
    });
}

#[test]
fn pyo3_python_call_paths_cover_wrapper_regions() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "neuralbudget_pycoverage").unwrap();
        neuralbudget(py, &module).unwrap();

        let locals = PyDict::new_bound(py);
        locals.set_item("nb", &module).unwrap();

        py.run_bound(
            r#"
cfg = nb.SloConfig(99.9, "7d")
cfg.to_dict(); cfg.to_json(); cfg.to_yaml()
nb.coerce_slo_config(cfg)

eb = nb.ErrorBudget(0.8, 1.2)
eb.to_dict(); eb.to_json(); eb.to_yaml()
nb.coerce_error_budget(eb)

mp = nb.MetricPoint(1, 0.5, {"k": "v"})
mp.to_dict(); mp.to_json(); mp.to_yaml()
nb.coerce_metric_point(mp)

tw = nb.TimeWindow(300, "rolling", 0)
tw.contains(1, 2); tw.to_dict(); tw.to_json(); tw.to_yaml()
nb.coerce_time_window(tw)

hb = nb.HistogramBucket(100.0, 10)
hb.to_dict(); hb.to_json(); hb.to_yaml()
nb.coerce_histogram_bucket(hb)

hs = nb.HistogramSample(1, 95, 100, [hb], "prometheus_cumulative")
hs.to_dict(); hs.to_json(); hs.to_yaml()
nb.coerce_histogram_sample(hs)

http = nb.HttpSlo(200.0, 0.99, 0.999)
http_eval = http.evaluate_histogram(hs)
http_eval.to_dict(); http_eval.to_json(); http_eval.to_yaml()
http.evaluate_stream([hs])
http.to_dict(); http.to_json(); http.to_yaml()
nb.evaluate_http_slo_histogram(hs, http)
nb.evaluate_http_slo_histogram_stream([hs], http)
nb.coerce_http_slo(http)

ss = nb.StatefulSample(1, 100.0, 10, 0.5, 10.0)
ss.to_dict(); ss.to_json(); ss.to_yaml()
nb.coerce_stateful_sample(ss)

sso = nb.StatefulSlo(250.0, 1000, 0.8, 20.0, 0.2, 0.9)
s_eval = sso.evaluate_sample(ss)
s_eval.to_dict(); s_eval.to_json(); s_eval.to_yaml()
sso.evaluate_stream([ss])
sso.to_dict(); sso.to_json(); sso.to_yaml()
nb.evaluate_stateful_slo(ss, sso)
nb.evaluate_stateful_slo_stream([ss], sso)
nb.coerce_stateful_slo(sso)

mls = nb.MlSample(1, 120.0, 0.7, 0.1, 0.9)
mls.to_dict(); mls.to_json(); mls.to_yaml()
nb.coerce_ml_sample(mls)

mlo = nb.MlSlo(200.0, 0.85, 0.2, 0.8, 0.6, 0.4, 0.9)
ml_eval = mlo.evaluate_sample(mls)
ml_eval.to_dict(); ml_eval.to_json(); ml_eval.to_yaml()
mlo.evaluate_stream([mls])
mlo.to_dict(); mlo.to_json(); mlo.to_yaml()
nb.evaluate_ml_slo(mls, mlo)
nb.evaluate_ml_slo_stream([mls], mlo)
nb.coerce_ml_slo(mlo)

gs = nb.GenAiSample(1, 200, 8000.0, 900.0, "hello world", "hello world")
gs.to_dict(); gs.to_json(); gs.to_yaml()
nb.coerce_genai_sample(gs)

go = nb.GenAiSlo(20.0, 1200.0, 0.7, "sentence-transformers/all-MiniLM-L6-v2")
g_eval = go.evaluate_sample(gs)
g_eval.to_dict(); g_eval.to_json(); g_eval.to_yaml()
go.evaluate_stream([gs])
go.to_dict(); go.to_json(); go.to_yaml()
nb.evaluate_genai_slo(gs, go)
nb.evaluate_genai_slo_stream([gs], go)
nb.coerce_genai_slo(go)

svc_api = nb.CompositeServiceSlo("api", 0.95, 0.9, 1.0)
svc_db = nb.CompositeServiceSlo("db", 0.98, 0.9, 1.0)
edge = nb.CompositeDependencyEdge("db", "api", 0.2)
svc_api.to_dict(); svc_api.to_json(); svc_api.to_yaml()
edge.to_dict(); edge.to_json(); edge.to_yaml()
nb.coerce_composite_service_slo(svc_api)
nb.coerce_composite_dependency_edge(edge)

graph = nb.CompositeSloGraph([svc_api, svc_db], [edge], 0.9)
graph.to_dict(); graph.to_json(); graph.to_yaml()
nb.coerce_composite_slo_graph(graph)
comp_eval = nb.evaluate_composite_slo_graph(graph)
comp_eval.to_dict(); comp_eval.to_json(); comp_eval.to_yaml()
for item in comp_eval.services:
    item.to_dict(); item.to_json(); item.to_yaml()

nb.calculate_availability(99, 100)
nb.calculate_error_budget(99.9, 86400)
nb.calculate_burn_rate([mp], 300)
nb.semantic_similarity_placeholder("hello", "hello", None)
nb.is_timestamp_in_window(1, 2, tw)
"#,
            None,
            Some(&locals),
        )
        .unwrap();
    });
}

#[test]
fn native_prometheus_exporter_renders_http_metrics() {
    let mut exporter = PrometheusExporter::with_namespace("neuralbudget");
    exporter.set_static_label("env", "test");
    exporter.observe_http_slo(
        "api-gateway",
        &HttpSloEvaluation {
            timestamp: 1_700_000_000,
            availability: 0.999,
            evaluated_percentile: 0.99,
            percentile_latency_ms: 180.0,
            latency_ok: true,
            availability_ok: true,
            pass: true,
        },
    );

    let rendered = exporter.render();
    assert!(rendered.contains("# TYPE neuralbudget_http_pass gauge"));
    assert!(rendered.contains("neuralbudget_http_pass{"));
    assert!(rendered.contains("service=\"api-gateway\""));
    assert!(rendered.contains("env=\"test\""));
    assert!(rendered.contains("neuralbudget_http_availability{"));
    assert!(rendered.contains("neuralbudget_http_percentile_latency_ms{"));
}

#[test]
fn native_prometheus_exporter_renders_composite_metrics() {
    let mut exporter = PrometheusExporter::with_namespace("nb");

    let evaluation = CompositeSloEvaluation {
        topological_order: vec!["db".to_string(), "api".to_string()],
        services: vec![
            CompositeServiceSloEvaluation {
                service: "db".to_string(),
                local_score: 0.99,
                effective_score: 0.99,
                min_pass_score: 0.9,
                dependency_adjusted: false,
                failed_dependencies: vec![],
                pass: true,
            },
            CompositeServiceSloEvaluation {
                service: "api".to_string(),
                local_score: 0.95,
                effective_score: 0.82,
                min_pass_score: 0.9,
                dependency_adjusted: true,
                failed_dependencies: vec!["db".to_string()],
                pass: false,
            },
        ],
        global_slo: 0.905,
        global_pass: true,
    };

    exporter.observe_composite_slo("checkout-graph", &evaluation);
    let rendered = exporter.render();

    assert!(rendered.contains("nb_composite_global_slo"));
    assert!(rendered.contains("graph=\"checkout-graph\""));
    assert!(rendered.contains("nb_composite_service_effective_score"));
    assert!(rendered.contains("service=\"api\""));
    assert!(rendered.contains("nb_composite_service_dependency_adjusted"));
}

#[test]
fn python_prometheus_export_helpers_and_class_work() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let module = PyModule::new_bound(py, "neuralbudget_prometheus_test").unwrap();
        neuralbudget(py, &module).unwrap();

        let locals = PyDict::new_bound(py);
        locals.set_item("nb", &module).unwrap();

        py.run_bound(
            r#"
sample = nb.HistogramSample(
    1,
    100,
    100,
    [nb.HistogramBucket(100.0, 100)],
    "prometheus_cumulative",
)
slo = nb.HttpSlo(200.0, 0.99, 0.999)
evaluation = slo.evaluate_histogram(sample)

text = nb.export_http_slo_prometheus("api", evaluation, "nb")
assert "nb_http_pass" in text
assert "service=\"api\"" in text

exporter = nb.PrometheusExporter("nb")
exporter.set_static_label("env", "test")
exporter.observe_http_slo("api", evaluation)
rendered = exporter.render()
assert "env=\"test\"" in rendered
assert "nb_http_availability" in rendered
"#,
            None,
            Some(&locals),
        )
        .unwrap();
    });
}

fn sample_otlp_payload_json() -> &'static str {
        r#"{
    "resourceMetrics": [
        {
            "scopeMetrics": [
                {
                    "metrics": [
                        {
                            "name": "http.server.duration",
                            "histogram": {
                                "dataPoints": [
                                    {
                                        "timeUnixNano": "1700000000000000000",
                                        "count": "100",
                                        "bucketCounts": ["70", "25", "5"],
                                        "explicitBounds": [100.0, 250.0]
                                    }
                                ]
                            }
                        },
                        {
                            "name": "service.error_budget.consumed",
                            "sum": {
                                "dataPoints": [
                                    {
                                        "timeUnixNano": "1700000000000000000",
                                        "asDouble": 0.25,
                                        "attributes": [
                                            {"key": "service", "value": {"stringValue": "api"}},
                                            {"key": "env", "value": {"stringValue": "prod"}}
                                        ]
                                    }
                                ]
                            }
                        }
                    ]
                }
            ]
        }
    ]
}"#
}

#[test]
fn otlp_histogram_ingestion_produces_histogram_samples() {
        let samples = ingest_otlp_histogram_json(sample_otlp_payload_json(), "http.server.duration")
                .unwrap();
        assert_eq!(samples.len(), 1);

        let sample = &samples[0];
        assert_eq!(sample.timestamp, 1_700_000_000);
        assert_eq!(sample.total, 100);
        assert_eq!(sample.success, 100);
        assert_eq!(sample.format, HistogramFormat::OpenTelemetryDelta);
        assert_eq!(sample.buckets.len(), 3);
        assert_eq!(sample.buckets[0].upper_bound_ms, 100.0);
        assert!(sample.buckets[2].upper_bound_ms.is_infinite());
}

#[test]
fn otlp_numeric_ingestion_produces_metric_points_with_labels() {
        let points = ingest_otlp_numeric_json(
                sample_otlp_payload_json(),
                "service.error_budget.consumed",
        )
        .unwrap();
        assert_eq!(points.len(), 1);

        let point = &points[0];
        assert_eq!(point.timestamp, 1_700_000_000);
        assert!((point.value - 0.25).abs() < 1e-12);
        assert_eq!(point.labels.get("service").unwrap(), "api");
        assert_eq!(point.labels.get("env").unwrap(), "prod");
}

#[test]
fn python_otlp_ingestion_and_http_evaluation_work() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
                let module = PyModule::new_bound(py, "neuralbudget_otlp_test").unwrap();
                neuralbudget(py, &module).unwrap();

                let locals = PyDict::new_bound(py);
                locals.set_item("nb", &module).unwrap();
                locals
                        .set_item("otlp_payload", sample_otlp_payload_json())
                        .unwrap();

                py.run_bound(
                        r#"
hist = nb.ingest_otlp_histogram(otlp_payload, "http.server.duration")
assert len(hist) == 1
assert hist[0].format == "open_telemetry_delta"

points = nb.ingest_otlp_numeric(otlp_payload, "service.error_budget.consumed")
assert len(points) == 1
assert points[0].labels["service"] == "api"

slo = nb.HttpSlo(200.0, 0.99, 0.95)
evaluations = nb.evaluate_http_slo_otlp(otlp_payload, "http.server.duration", slo)
assert len(evaluations) == 1
assert evaluations[0].availability == 1.0
"#,
                        None,
                        Some(&locals),
                )
                .unwrap();
        });
}

proptest! {
    #[test]
    fn prop_availability_is_bounded(success in 0u64..1_000_000, total in 1u64..1_000_000) {
        let bounded_success = success % (total + 1);
        let availability = calculate_availability(bounded_success, total);

        prop_assert!((0.0..=1.0).contains(&availability));
        prop_assert!((availability - (bounded_success as f64 / total as f64)).abs() < 1e-12);
    }

    #[test]
    fn prop_error_budget_stays_within_window(target in -2.0f64..3.0, window in 0u64..100_000) {
        let budget = calculate_error_budget(target, window);
        prop_assert!(budget >= 0.0);
        prop_assert!(budget <= window as f64);
    }

    #[test]
    fn prop_burn_rate_is_bounded(
        values in prop::collection::vec(-5.0f64..5.0, 1..500),
        start in 0i64..10_000,
        window in 1u64..500,
    ) {
        let stream: Vec<MetricPoint> = values
            .iter()
            .enumerate()
            .map(|(idx, value)| MetricPoint {
                timestamp: start + idx as i64,
                value: *value,
                labels: HashMap::new(),
            })
            .collect();

        let burn_rate = calculate_burn_rate(stream, window);
        prop_assert!((0.0..=1.0).contains(&burn_rate));
    }

    #[test]
    fn prop_slo_config_roundtrip_preserves_payload(
        target in 0.0f64..100.0,
        window in "[a-zA-Z0-9_]{1,12}",
    ) {
        let cfg = SloConfig { target, window };
        let json = cfg.to_json_string().unwrap();
        let yaml = cfg.to_yaml_string().unwrap();

        let from_json = SloConfig::from_json_str(&json).unwrap();
        let from_yaml = SloConfig::from_yaml_str(&yaml).unwrap();

        prop_assert_eq!(from_json.window.as_str(), cfg.window.as_str());
        prop_assert_eq!(from_yaml.window.as_str(), cfg.window.as_str());
        prop_assert!((from_json.target - cfg.target).abs() < 1e-9);
        prop_assert!((from_yaml.target - cfg.target).abs() < 1e-9);
    }

    #[test]
    fn prop_semantic_similarity_is_bounded(a in "[a-z ]{0,20}", b in "[a-z ]{0,20}") {
        let score = semantic_similarity_placeholder(&a, &b, None);
        prop_assert!((0.0..=1.0).contains(&score));
    }
}

#[test]
fn composite_slo_dag_propagates_dependency_failure_and_adjusts_dependents() {
    let graph = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "service_a".to_string(),
                local_score: 0.7,
                min_pass_score: 0.9,
                impact_weight: 3.0,
            },
            CompositeServiceSlo {
                service: "service_b".to_string(),
                local_score: 0.96,
                min_pass_score: 0.85,
                impact_weight: 2.0,
            },
            CompositeServiceSlo {
                service: "service_c".to_string(),
                local_score: 0.94,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![CompositeDependencyEdge {
            dependency: "service_a".to_string(),
            dependent: "service_b".to_string(),
            failure_penalty: 0.25,
        }],
        global_min_pass_score: 0.85,
    };

    let evaluation = evaluate_composite_slo(&graph).unwrap();
    assert_eq!(evaluation.topological_order.len(), 3);

    let pos_a = evaluation
        .topological_order
        .iter()
        .position(|name| name == "service_a")
        .unwrap();
    let pos_b = evaluation
        .topological_order
        .iter()
        .position(|name| name == "service_b")
        .unwrap();
    assert!(pos_a < pos_b);

    let by_name: HashMap<String, CompositeServiceSloEvaluation> = evaluation
        .services
        .iter()
        .map(|entry| (entry.service.clone(), entry.clone()))
        .collect();

    let service_a = by_name.get("service_a").unwrap();
    let service_b = by_name.get("service_b").unwrap();
    let service_c = by_name.get("service_c").unwrap();

    assert!(!service_a.pass);
    assert!((service_b.effective_score - 0.72).abs() < 1e-9);
    assert!(!service_b.pass);
    assert!(service_b.dependency_adjusted);
    assert_eq!(service_b.failed_dependencies, vec!["service_a".to_string()]);
    assert!(service_c.pass);
    assert!(!service_c.dependency_adjusted);

    let expected_global = (service_a.effective_score * 3.0
        + service_b.effective_score * 2.0
        + service_c.effective_score * 1.0)
        / 6.0;
    assert!((evaluation.global_slo - expected_global).abs() < 1e-9);
    assert!(!evaluation.global_pass);
}

#[test]
fn composite_slo_dag_rejects_cycles() {
    let graph = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "a".to_string(),
                local_score: 0.99,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
            CompositeServiceSlo {
                service: "b".to_string(),
                local_score: 0.99,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![
            CompositeDependencyEdge {
                dependency: "a".to_string(),
                dependent: "b".to_string(),
                failure_penalty: 0.2,
            },
            CompositeDependencyEdge {
                dependency: "b".to_string(),
                dependent: "a".to_string(),
                failure_penalty: 0.2,
            },
        ],
        global_min_pass_score: 0.9,
    };

    let error = evaluate_composite_slo(&graph).unwrap_err();
    assert_eq!(error, CompositeSloError::CycleDetected);
}

#[test]
fn composite_slo_dag_rejects_duplicate_edges() {
    let graph = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "a".to_string(),
                local_score: 0.95,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
            CompositeServiceSlo {
                service: "b".to_string(),
                local_score: 0.95,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![
            CompositeDependencyEdge {
                dependency: "a".to_string(),
                dependent: "b".to_string(),
                failure_penalty: 0.2,
            },
            CompositeDependencyEdge {
                dependency: "a".to_string(),
                dependent: "b".to_string(),
                failure_penalty: 0.4,
            },
        ],
        global_min_pass_score: 0.9,
    };

    let error = evaluate_composite_slo(&graph).unwrap_err();
    assert_eq!(
        error,
        CompositeSloError::DuplicateDependencyEdge {
            dependency: "a".to_string(),
            dependent: "b".to_string(),
        }
    );
}

#[test]
fn composite_slo_dag_has_deterministic_topological_order() {
    let graph = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "delta".to_string(),
                local_score: 0.98,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
            CompositeServiceSlo {
                service: "alpha".to_string(),
                local_score: 0.98,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
            CompositeServiceSlo {
                service: "beta".to_string(),
                local_score: 0.98,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
            CompositeServiceSlo {
                service: "gamma".to_string(),
                local_score: 0.98,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![
            CompositeDependencyEdge {
                dependency: "alpha".to_string(),
                dependent: "delta".to_string(),
                failure_penalty: 0.1,
            },
            CompositeDependencyEdge {
                dependency: "beta".to_string(),
                dependent: "delta".to_string(),
                failure_penalty: 0.1,
            },
        ],
        global_min_pass_score: 0.9,
    };

    let first = evaluate_composite_slo(&graph).unwrap();
    let second = evaluate_composite_slo(&graph).unwrap();

    assert_eq!(first.topological_order, second.topological_order);
    let pos_alpha = first
        .topological_order
        .iter()
        .position(|name| name == "alpha")
        .unwrap();
    let pos_beta = first
        .topological_order
        .iter()
        .position(|name| name == "beta")
        .unwrap();
    let pos_delta = first
        .topological_order
        .iter()
        .position(|name| name == "delta")
        .unwrap();

    assert!(pos_alpha < pos_delta);
    assert!(pos_beta < pos_delta);
}
