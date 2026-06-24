use neuralbudget::{
    calculate_availability, calculate_burn_rate, calculate_error_budget, calculate_mad,
    calculate_web_api_slo, filter_statistical_outliers, ErrorBudget, HistogramBucket,
    HistogramFormat, HistogramSample, HttpSlo, HttpSloEvaluation, HttpSloIterator, JsonYamlExt,
    MetricPoint, OutlierFilterConfig, SloConfig, StatefulPolicyProfileSet, StatefulSample,
    StatefulSlo, StatefulSloEvaluation, StatefulSloIterator, StatefulTier, TimeWindow,
    WebApiRequest, WebApiSloPolicy,
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
    let histogram = HistogramSample {
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
                upper_bound_ms: 220.0,
                count: 100,
            },
        ],
        format: HistogramFormat::OpenTelemetryDelta,
    };
    let http_slo = HttpSlo::default();
    let stateful_slo = StatefulSlo::default();
    let stateful_sample = StatefulSample {
        timestamp: 7,
        replication_lag_ms: 150.0,
        queue_depth: 400,
        connection_pool_saturation: 0.7,
        connection_wait_time_ms: 12.0,
    };

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
    assert_eq!(
        HistogramSample::from_json_str(&histogram.to_json_string().unwrap()).unwrap(),
        histogram
    );
    assert_eq!(
        HttpSlo::from_yaml_str(&http_slo.to_yaml_string().unwrap()).unwrap(),
        http_slo
    );
    assert_eq!(
        StatefulSlo::from_json_str(&stateful_slo.to_json_string().unwrap()).unwrap(),
        stateful_slo
    );
    assert_eq!(
        StatefulSample::from_yaml_str(&stateful_sample.to_yaml_string().unwrap()).unwrap(),
        stateful_sample
    );
}

#[test]
fn http_slo_iterator_produces_pass_fail_per_histogram_sample() {
    let slo = HttpSlo::default();
    let samples = vec![
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
            success: 9_995,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 9_000,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 9_500,
                },
                HistogramBucket {
                    upper_bound_ms: 500.0,
                    count: 10_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        },
    ];

    let results: Vec<HttpSloEvaluation> = HttpSloIterator::new(slo, samples.into_iter()).collect();

    assert_eq!(results.len(), 2);
    assert!(results[0].pass);
    assert!(!results[1].pass);
    assert!(!results[1].latency_ok);
    assert!(results[1].availability_ok);
}

#[test]
fn http_slo_iterator_accepts_opentelemetry_delta_histograms() {
    let slo = HttpSlo::default();
    let samples = vec![HistogramSample {
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
    }];

    let result = HttpSloIterator::new(slo, samples.into_iter())
        .next()
        .unwrap();

    assert!(result.pass);
    assert_eq!(result.evaluated_percentile, 0.99);
    assert!(result.percentile_latency_ms < 200.0);
    assert!(result.availability > 0.999);
}

#[test]
fn http_slo_custom_percentile_policy_changes_latency_gate() {
    let slo = HttpSlo {
        latency_threshold_ms: 150.0,
        latency_percentile: 0.95,
        availability_threshold: 0.999,
    };
    let samples = vec![HistogramSample {
        timestamp: 5,
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
    }];

    let result = HttpSloIterator::new(slo, samples.into_iter())
        .next()
        .unwrap();

    assert_eq!(result.evaluated_percentile, 0.95);
    assert!(result.percentile_latency_ms <= 150.0);
    assert!(result.pass);
}

#[test]
fn stateful_slo_penalty_impacts_pass_fail() {
    let slo = StatefulSlo {
        connection_wait_penalty_weight: 0.25,
        min_pass_score: 0.85,
        ..StatefulSlo::default()
    };
    let samples = vec![
        StatefulSample {
            timestamp: 1,
            replication_lag_ms: 100.0,
            queue_depth: 200,
            connection_pool_saturation: 0.6,
            connection_wait_time_ms: 10.0,
        },
        StatefulSample {
            timestamp: 2,
            replication_lag_ms: 100.0,
            queue_depth: 200,
            connection_pool_saturation: 0.6,
            connection_wait_time_ms: 80.0,
        },
    ];

    let results: Vec<StatefulSloEvaluation> =
        StatefulSloIterator::new(slo, samples.into_iter()).collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].pass);
    assert!(results[1].connection_wait_penalized);
    assert!((results[1].score - 0.75).abs() < 1e-9);
    assert!(!results[1].pass);
}

#[test]
fn weighted_stateful_policy_profiles_round_trip_and_diverge_by_tier() {
    let profiles = StatefulPolicyProfileSet::default();
    let round_trip =
        StatefulPolicyProfileSet::from_yaml_str(&profiles.to_yaml_string().unwrap()).unwrap();

    assert_eq!(round_trip, profiles);
    assert_eq!(
        profiles.profile_for_tier(StatefulTier::Database).name,
        "database_primary"
    );
    assert_eq!(
        profiles.profile_for_tier(StatefulTier::Queue).name,
        "queue_hot_path"
    );

    let sample = StatefulSample {
        timestamp: 9,
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
}
