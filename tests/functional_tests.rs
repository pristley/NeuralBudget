use neuralbudget::{
    calculate_availability, calculate_burn_rate, calculate_error_budget, calculate_web_api_slo,
    is_timestamp_in_window, MetricPoint, OutlierFilterConfig, TimeWindow, WebApiRequest,
    WebApiSloPolicy,
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
