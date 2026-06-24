use neuralbudget::{
    calculate_availability, ErrorBudget, JsonYamlExt, MetricPoint, SloConfig, TimeWindow,
};

#[test]
fn availability_matches_classic_sli_ratio() {
    let availability = calculate_availability(995, 1000);

    assert_eq!(availability, 0.995);
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
