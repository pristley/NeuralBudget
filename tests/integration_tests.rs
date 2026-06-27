use neuralbudget::{
    calculate_availability, calculate_burn_rate, calculate_error_budget, calculate_mad,
    calculate_web_api_slo, evaluate_composite_slo, filter_statistical_outliers,
    semantic_similarity_placeholder, CompositeDependencyEdge, CompositeServiceSlo,
    CompositeSloError, CompositeSloGraph, ErrorBudget, GenAiSample, GenAiSlo, GenAiSloEvaluation,
    GenAiSloIterator, HistogramBucket, HistogramFormat, HistogramSample, HttpSlo,
    HttpSloEvaluation, HttpSloIterator, JsonExt, MetricPoint, MlSample, MlSlo, MlSloEvaluation,
    MlSloIterator, OutlierFilterConfig, SloConfig, StatefulPolicyProfileSet, StatefulSample,
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
    let mad = calculate_mad(&values).expect("MAD calculation should succeed for valid input");
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
        SloConfig::from_json_str(&config.to_json_string().expect("SloConfig serialization should succeed")).expect("SloConfig deserialization should succeed"),
        config
    );
    assert_eq!(
        ErrorBudget::from_json_str(&budget.to_json_string().expect("ErrorBudget serialization should succeed")).expect("ErrorBudget deserialization should succeed"),
        budget
    );
    assert_eq!(
        MetricPoint::from_json_str(&point.to_json_string().expect("MetricPoint serialization should succeed")).expect("MetricPoint deserialization should succeed"),
        point
    );
    assert_eq!(
        TimeWindow::from_json_str(&window.to_json_string().expect("TimeWindow serialization should succeed")).expect("TimeWindow deserialization should succeed"),
        window
    );
    assert_eq!(
        HistogramSample::from_json_str(&histogram.to_json_string().expect("HistogramSample serialization should succeed")).expect("HistogramSample deserialization should succeed"),
        histogram
    );
    assert_eq!(
        HttpSlo::from_json_str(&http_slo.to_json_string().expect("HttpSlo serialization should succeed")).expect("HttpSlo deserialization should succeed"),
        http_slo
    );
    assert_eq!(
        StatefulSlo::from_json_str(&stateful_slo.to_json_string().expect("StatefulSlo serialization should succeed")).expect("StatefulSlo deserialization should succeed"),
        stateful_slo
    );
    assert_eq!(
        StatefulSample::from_json_str(&stateful_sample.to_json_string().unwrap()).unwrap(),
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
        StatefulPolicyProfileSet::from_json_str(&profiles.to_json_string().unwrap()).unwrap();

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

#[test]
fn ml_slo_hybrid_score_uses_default_latency_and_drift_weights() {
    let slo = MlSlo::default();
    let sample = MlSample {
        timestamp: 11,
        inference_latency_ms: 220.0,
        gpu_utilization: 0.9,
        feature_drift: 0.1,
        prediction_confidence: 0.9,
    };

    let result = slo.evaluate_sample(&sample);

    assert!((result.inference_latency_score - (200.0 / 220.0)).abs() < 1e-9);
    assert!((result.gpu_utilization_score - (0.85 / 0.9)).abs() < 1e-9);
    assert!(
        (result.system_score
            - ((result.inference_latency_score + result.gpu_utilization_score) / 2.0))
            .abs()
            < 1e-9
    );
    assert!((result.feature_drift_score - 0.5).abs() < 1e-9);
    assert!((result.prediction_confidence_score - 1.0).abs() < 1e-9);
    assert!((result.drift_score - 0.75).abs() < 1e-9);

    let expected = result.latency_score * 0.6 + result.drift_score * 0.4;
    assert!((result.hybrid_score - expected).abs() < 1e-9);
    assert!(!result.pass);
}

#[test]
fn ml_slo_weight_normalization_handles_non_unit_weight_sums() {
    let slo = MlSlo {
        latency_weight: 3.0,
        drift_weight: 2.0,
        min_pass_score: 0.75,
        ..MlSlo::default()
    };
    let sample = MlSample {
        timestamp: 12,
        inference_latency_ms: 180.0,
        gpu_utilization: 0.7,
        feature_drift: 0.09,
        prediction_confidence: 0.92,
    };

    let result = slo.evaluate_sample(&sample);
    assert!((result.latency_weight - 0.6).abs() < 1e-9);
    assert!((result.drift_weight - 0.4).abs() < 1e-9);
    assert!(result.pass);
}

#[test]
fn ml_slo_iterator_and_serialization_round_trip() {
    let slo = MlSlo::default();
    let samples = vec![
        MlSample {
            timestamp: 20,
            inference_latency_ms: 160.0,
            gpu_utilization: 0.75,
            feature_drift: 0.08,
            prediction_confidence: 0.94,
        },
        MlSample {
            timestamp: 21,
            inference_latency_ms: 260.0,
            gpu_utilization: 0.95,
            feature_drift: 0.22,
            prediction_confidence: 0.72,
        },
    ];

    let results: Vec<MlSloEvaluation> =
        MlSloIterator::new(slo.clone(), samples.into_iter()).collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].pass);
    assert!(!results[1].pass);

    let round_trip = MlSlo::from_json_str(&slo.to_json_string().unwrap()).unwrap();
    assert_eq!(round_trip, slo);

    let sample_round_trip = MlSample::from_json_str(
        &MlSample {
            timestamp: 99,
            inference_latency_ms: 120.0,
            gpu_utilization: 0.5,
            feature_drift: 0.03,
            prediction_confidence: 0.98,
        }
        .to_json_string()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(sample_round_trip.timestamp, 99);
}

#[test]
fn genai_slo_combines_tps_ttft_and_semantic_quality() {
    let slo = GenAiSlo::default();
    let sample = GenAiSample {
        timestamp: 100,
        tokens_generated: 240,
        generation_duration_ms: 6_000.0,
        time_to_first_token_ms: 800.0,
        reference_text: "the cat sat on the mat".to_string(),
        generated_text: "the cat sat on the mat".to_string(),
    };

    let result = slo.evaluate_sample(&sample);
    assert!((result.tokens_per_second - 40.0).abs() < 1e-9);
    assert!(result.tokens_per_second_ok);
    assert!(result.time_to_first_token_ok);
    assert!(result.semantic_similarity >= 0.95);
    assert!(result.semantic_similarity_ok);
    assert!(result.pass);
}

#[test]
fn genai_slo_iterator_and_roundtrip_cover_qualitative_model() {
    let slo = GenAiSlo::default();
    let samples = vec![
        GenAiSample {
            timestamp: 101,
            tokens_generated: 300,
            generation_duration_ms: 10_000.0,
            time_to_first_token_ms: 700.0,
            reference_text: "high quality answer".to_string(),
            generated_text: "high quality answer".to_string(),
        },
        GenAiSample {
            timestamp: 102,
            tokens_generated: 100,
            generation_duration_ms: 10_000.0,
            time_to_first_token_ms: 1_800.0,
            reference_text: "high quality answer".to_string(),
            generated_text: "unrelated output tokens".to_string(),
        },
    ];

    let results: Vec<GenAiSloEvaluation> =
        GenAiSloIterator::new(slo.clone(), samples.into_iter()).collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].pass);
    assert!(!results[1].pass);

    let round_trip = GenAiSlo::from_json_str(&slo.to_json_string().unwrap()).unwrap();
    assert_eq!(round_trip, slo);
}

#[test]
fn semantic_similarity_placeholder_is_bounded() {
    let same = semantic_similarity_placeholder("hello world", "hello world", None);
    let different = semantic_similarity_placeholder("hello world", "quantum banana", None);

    assert!((0.0..=1.0).contains(&same));
    assert!((0.0..=1.0).contains(&different));
    assert!(same >= different);
}

#[test]
fn composite_slo_dag_computes_global_slo_with_dependency_impact() {
    let graph = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "edge".to_string(),
                local_score: 0.72,
                min_pass_score: 0.9,
                impact_weight: 2.0,
            },
            CompositeServiceSlo {
                service: "api".to_string(),
                local_score: 0.97,
                min_pass_score: 0.9,
                impact_weight: 3.0,
            },
            CompositeServiceSlo {
                service: "worker".to_string(),
                local_score: 0.95,
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![CompositeDependencyEdge {
            dependency: "edge".to_string(),
            dependent: "api".to_string(),
            failure_penalty: 0.2,
        }],
        global_min_pass_score: 0.85,
    };

    let result = evaluate_composite_slo(&graph).unwrap();
    assert_eq!(result.topological_order.len(), 3);

    let edge_eval = result
        .services
        .iter()
        .find(|entry| entry.service == "edge")
        .unwrap();
    let api_eval = result
        .services
        .iter()
        .find(|entry| entry.service == "api")
        .unwrap();
    let worker_eval = result
        .services
        .iter()
        .find(|entry| entry.service == "worker")
        .unwrap();

    assert!(!edge_eval.pass);
    assert!(api_eval.dependency_adjusted);
    assert_eq!(api_eval.failed_dependencies, vec!["edge".to_string()]);
    assert!((api_eval.effective_score - 0.776).abs() < 1e-9);
    assert!(!api_eval.pass);
    assert!(worker_eval.pass);

    let expected_global = (edge_eval.effective_score * 2.0
        + api_eval.effective_score * 3.0
        + worker_eval.effective_score)
        / 6.0;
    assert!((result.global_slo - expected_global).abs() < 1e-9);
    assert!(!result.global_pass);
}

#[test]
fn composite_slo_dag_detects_invalid_dependency_graphs() {
    let unknown_service_graph = CompositeSloGraph {
        services: vec![CompositeServiceSlo {
            service: "a".to_string(),
            local_score: 0.95,
            min_pass_score: 0.9,
            impact_weight: 1.0,
        }],
        dependencies: vec![CompositeDependencyEdge {
            dependency: "a".to_string(),
            dependent: "missing".to_string(),
            failure_penalty: 0.2,
        }],
        global_min_pass_score: 0.9,
    };

    let unknown_err = evaluate_composite_slo(&unknown_service_graph).unwrap_err();
    assert_eq!(
        unknown_err,
        CompositeSloError::UnknownService("missing".to_string())
    );

    let cycle_graph = CompositeSloGraph {
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
                dependency: "b".to_string(),
                dependent: "a".to_string(),
                failure_penalty: 0.2,
            },
        ],
        global_min_pass_score: 0.9,
    };

    let cycle_err = evaluate_composite_slo(&cycle_graph).unwrap_err();
    assert_eq!(cycle_err, CompositeSloError::CycleDetected);
}
