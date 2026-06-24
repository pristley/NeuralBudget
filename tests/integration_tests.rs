use neuralbudget::{
    calculate_availability, calculate_burn_rate, calculate_error_budget, calculate_mad,
    calculate_web_api_slo, filter_statistical_outliers, ErrorBudget, JsonYamlExt, MetricPoint,
    OutlierFilterConfig, SloConfig, TimeWindow, WebApiRequest, WebApiSloPolicy,
};

#[test]
fn availability_matches_classic_sli_ratio() {
    let availability = calculate_availability(995, 1000);

    assert_eq!(availability, 0.995);
}

#[test]
fn error_budget_scales_from_slo_target() {
    let budget = calculate_error_budget(0.999, 3_600);

    assert!((budget - 3.6).abs() < 1e-9);
}

#[test]
fn burn_rate_compares_five_minutes_against_one_hour() {
    let stream: Vec<MetricPoint> = (0..3_600)
        .map(|timestamp| MetricPoint {
            timestamp,
            value: if timestamp >= 3_300 { 1.0 } else { 0.0 },
            labels: Default::default(),
        })
        .collect();

    assert_eq!(calculate_burn_rate(stream.clone(), 300), 1.0);
    assert_eq!(calculate_burn_rate(stream, 3_600), 300.0 / 3_600.0);
}

#[test]
fn mad_and_outlier_filter_handle_latency_spike() {
    let values = vec![120.0, 130.0, 110.0, 125.0, 4_000.0];
    let mad = calculate_mad(&values).unwrap();
    assert!(mad > 0.0);

    let stream: Vec<MetricPoint> = values
        .iter()
        .enumerate()
        .map(|(idx, value)| MetricPoint {
            timestamp: idx as i64,
            value: *value,
            labels: Default::default(),
        })
        .collect();

    let filtered = filter_statistical_outliers(&stream, 3.5, 3);
    assert_eq!(filtered.len(), 4);
    assert!(filtered.iter().all(|point| point.value < 1_000.0));
}

#[test]
fn web_api_slo_framework_reports_budget_and_outlier_filtered_latency() {
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
            latency_ms: 110.0,
            status_code: 200,
            labels: Default::default(),
        },
        WebApiRequest {
            timestamp: 4,
            latency_ms: 4_000.0,
            status_code: 200,
            labels: Default::default(),
        },
        WebApiRequest {
            timestamp: 5,
            latency_ms: 115.0,
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

    let report = calculate_web_api_slo(&requests, &policy, 6);

    assert_eq!(report.total_requests, 5);
    assert_eq!(report.filtered_outliers, 1);
    assert_eq!(report.latency_compliant_requests, 4);
    assert!((report.error_budget_seconds - 0.1).abs() < 1e-9);
}

#[test]
fn rolling_window_includes_recent_timestamps() {
    let window = TimeWindow::rolling(3_600);
    let now = 1_700_000_000_i64;

    assert!(window.contains(now - 1, now));
    assert!(window.contains(now - 3_600, now));
    assert!(!window.contains(now - 3_601, now));
    assert!(!window.contains(now + 1, now));
}

#[test]
fn calendar_aligned_window_respects_timezone_offset() {
    let window = TimeWindow::calendar_aligned(86_400, 18_000);
    let now = 90_000_i64;

    assert!(window.contains(68_400, now));
    assert!(window.contains(104_999, now));
    assert!(!window.contains(68_399, now));
    assert!(!window.contains(154_800, now));
}

#[test]
fn serialization_round_trips_across_models() {
    let config = SloConfig {
        target: 99.9,
        window: "7d".to_string(),
    };
    let budget = ErrorBudget {
        remaining: 0.42,
        velocity: 1.7,
    };
    let point = MetricPoint {
        timestamp: 1719220000,
        value: 0.998,
        labels: Default::default(),
    };
    let window = TimeWindow::calendar_aligned(86_400, 18_000);

    assert_eq!(
        SloConfig::from_json_str(&config.to_json_string().unwrap()).unwrap(),
        config
    );
    assert_eq!(
        ErrorBudget::from_yaml_str(&budget.to_yaml_string().unwrap()).unwrap(),
        budget
    );
    assert_eq!(
        MetricPoint::from_json_str(&point.to_json_string().unwrap()).unwrap(),
        point
    );
    assert_eq!(
        TimeWindow::from_yaml_str(&window.to_yaml_string().unwrap()).unwrap(),
        window
    );
}
