use neuralbudget::{
    calculate_availability, calculate_burn_rate, calculate_error_budget, is_timestamp_in_window,
    MetricPoint, TimeWindow,
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
