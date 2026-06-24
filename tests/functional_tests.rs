use neuralbudget::{
    calculate_availability, calculate_burn_rate, calculate_error_budget, calculate_web_api_slo,
    is_timestamp_in_window, HistogramBucket, HistogramFormat, HistogramSample, HttpSlo,
    HttpSloIterator, MetricPoint, OutlierFilterConfig, StatefulSample, StatefulSlo,
    StatefulSloIterator, TimeWindow, WebApiRequest, WebApiSloPolicy,
};

#[test]
fn functional_budget_pipeline_from_slo_to_burn_rate() {
    let slo_target = 0.999;
    let window_secs = 3_600;

    let budget_secs = calculate_error_budget(slo_target, window_secs);
    assert!((budget_secs - 3.6).abs() < 1e-9);

    let stream: Vec<MetricPoint> = (0..3_600)
        .map(|timestamp| MetricPoint {
            timestamp,
            value: if timestamp >= 3_300 { 1.0 } else { 0.0 },
            labels: Default::default(),
        })
        .collect();

    let burn_5m = calculate_burn_rate(stream.clone(), 300);
    let burn_1h = calculate_burn_rate(stream, 3_600);

    assert_eq!(burn_5m, 1.0);
    assert_eq!(burn_1h, 300.0 / 3_600.0);
    assert!(burn_5m > burn_1h);
}

#[test]
fn functional_window_and_stream_alignment() {
    let now = 90_000_i64;
    let window = TimeWindow::calendar_aligned(86_400, 18_000);

    assert!(window.contains(68_400, now));
    assert!(!window.contains(154_800, now));
    assert!(is_timestamp_in_window(68_400, now, window.clone()));

    let in_window_samples: Vec<MetricPoint> = vec![
        MetricPoint {
            timestamp: 68_400,
            value: 1.0,
            labels: Default::default(),
        },
        MetricPoint {
            timestamp: 90_000,
            value: 1.0,
            labels: Default::default(),
        },
    ];

    let burn = calculate_burn_rate(in_window_samples, 86_400);
    assert_eq!(burn, 2.0 / 86_400.0);
}

#[test]
fn functional_availability_and_budget_consistency() {
    let availability = calculate_availability(9_990, 10_000);
    let budget = calculate_error_budget(availability, 86_400);

    assert!((availability - 0.999).abs() < 1e-12);
    assert!((budget - 86.4).abs() < 1e-9);
}

#[test]
fn functional_web_api_slo_handles_ai_latency_anomaly() {
    let requests = vec![
        WebApiRequest {
            timestamp: 1,
            latency_ms: 120.0,
            status_code: 200,
            labels: Default::default(),
        },
        WebApiRequest {
            timestamp: 2,
            latency_ms: 130.0,
            status_code: 200,
            labels: Default::default(),
        },
        WebApiRequest {
            timestamp: 3,
            latency_ms: 4_000.0,
            status_code: 200,
            labels: Default::default(),
        },
        WebApiRequest {
            timestamp: 4,
            latency_ms: 110.0,
            status_code: 500,
            labels: Default::default(),
        },
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

    let report = calculate_web_api_slo(&requests, &policy, 5);

    assert_eq!(report.total_requests, 4);
    assert_eq!(report.successful_requests, 3);
    assert_eq!(report.filtered_outliers, 1);
    assert_eq!(report.latency_evaluated_requests, 3);
    assert_eq!(report.latency_compliant_requests, 3);
    assert!((report.latency_sli - 1.0).abs() < 1e-9);
}

#[test]
fn functional_http_slo_iterates_histogram_stream() {
    let slo = HttpSlo::default();
    let stream = vec![
        HistogramSample {
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
        },
        HistogramSample {
            timestamp: 2,
            success: 9_980,
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
            timestamp: 3,
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
        },
    ];

    let results: Vec<_> = HttpSloIterator::new(slo, stream.into_iter()).collect();

    assert_eq!(results.len(), 3);
    assert!(results[0].pass);
    assert!(!results[1].pass);
    assert!(results[2].pass);
}

#[test]
fn functional_stateful_slo_penalizes_connection_wait_regressions() {
    let slo = StatefulSlo::default();
    let samples = vec![
        StatefulSample {
            timestamp: 1,
            replication_lag_ms: 180.0,
            queue_depth: 700,
            connection_pool_saturation: 0.7,
            connection_wait_time_ms: 8.0,
        },
        StatefulSample {
            timestamp: 2,
            replication_lag_ms: 200.0,
            queue_depth: 800,
            connection_pool_saturation: 0.75,
            connection_wait_time_ms: 60.0,
        },
        StatefulSample {
            timestamp: 3,
            replication_lag_ms: 320.0,
            queue_depth: 1_300,
            connection_pool_saturation: 0.9,
            connection_wait_time_ms: 80.0,
        },
    ];

    let evaluations: Vec<_> = StatefulSloIterator::new(slo, samples.into_iter()).collect();

    assert_eq!(evaluations.len(), 3);
    assert!(evaluations[0].pass);
    assert!(evaluations[1].connection_wait_penalized);
    assert!(!evaluations[1].pass);
    assert!(!evaluations[2].replication_lag_ok);
    assert!(!evaluations[2].queue_depth_ok);
    assert!(!evaluations[2].connection_pool_ok);
}
