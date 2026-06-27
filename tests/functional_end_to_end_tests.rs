/// Comprehensive functional tests for end-to-end workflows
///
/// Tests cover:
/// - Complete SLO evaluation pipelines
/// - Multi-SLO scenarios with different types
/// - Error handling in real workflows
/// - Performance characteristics
/// - Complex composite DAG scenarios
use neuralbudget::{
    calculate_burn_rate, calculate_error_budget, evaluate_composite_slo, CompositeDependencyEdge,
    CompositeServiceSlo, CompositeSloError, CompositeSloGraph, GenAiSample, GenAiSlo,
    GenAiSloIterator, HistogramBucket, HistogramFormat, HistogramSample, HttpSlo, HttpSloIterator,
    MetricPoint, MlSample, MlSlo, MlSloIterator, SloConfig, StatefulSample, StatefulSlo,
    StatefulSloIterator, TimeWindow,
};

// ============================================================================
// SCENARIO 1: Complete HTTP SLO Evaluation Pipeline
// ============================================================================

#[test]
fn end_to_end_http_slo_evaluation_pipeline() {
    let slo = HttpSlo {
        latency_threshold_ms: 200.0,
        latency_percentile: 0.99,
        availability_threshold: 0.999,
    };

    // Simulate histogram data collection over 5 samples
    let histogram_stream = vec![
        HistogramSample {
            timestamp: 1000,
            success: 9_991,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 8_000,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 9_950,
                },
                HistogramBucket {
                    upper_bound_ms: 300.0,
                    count: 10_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        },
        HistogramSample {
            timestamp: 2000,
            success: 9_995,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 7_900,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 9_950,
                },
                HistogramBucket {
                    upper_bound_ms: 300.0,
                    count: 10_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        },
        HistogramSample {
            timestamp: 3000,
            success: 9_999,
            total: 10_000,
            buckets: vec![
                HistogramBucket {
                    upper_bound_ms: 100.0,
                    count: 8_200,
                },
                HistogramBucket {
                    upper_bound_ms: 200.0,
                    count: 9_950,
                },
                HistogramBucket {
                    upper_bound_ms: 300.0,
                    count: 10_000,
                },
            ],
            format: HistogramFormat::PrometheusCumulative,
        },
    ];

    let results: Vec<_> = HttpSloIterator::new(slo, histogram_stream.into_iter()).collect();

    // Verify all samples were evaluated
    assert_eq!(results.len(), 3);

    // All samples should pass (high availability and latency compliance)
    assert!(
        results.iter().all(|r| r.pass),
        "All HTTP SLO samples should pass"
    );
    assert!(
        results.iter().all(|r| r.availability_ok),
        "All samples should have availability OK"
    );
    assert!(
        results.iter().all(|r| r.latency_ok),
        "All samples should have latency OK"
    );
}

// ============================================================================
// SCENARIO 2: Multi-SLO Evaluation (HTTP + Stateful + ML)
// ============================================================================

#[test]
fn multi_slo_evaluation_across_service_types() {
    // HTTP SLO evaluation
    let http_slo = HttpSlo::default();
    let http_histogram = HistogramSample {
        timestamp: 1000,
        success: 9_991,
        total: 10_000,
        buckets: vec![
            HistogramBucket {
                upper_bound_ms: 150.0,
                count: 9_700,
            },
            HistogramBucket {
                upper_bound_ms: 200.0,
                count: 9_950,
            },
            HistogramBucket {
                upper_bound_ms: 300.0,
                count: 10_000,
            },
        ],
        format: HistogramFormat::PrometheusCumulative,
    };

    let http_result = HttpSloIterator::new(http_slo, vec![http_histogram].into_iter())
        .next()
        .unwrap();

    // Stateful SLO evaluation
    let stateful_slo = StatefulSlo::default();
    let stateful_sample = StatefulSample {
        timestamp: 1000,
        replication_lag_ms: 150.0,
        queue_depth: 500,
        connection_pool_saturation: 0.65,
        connection_wait_time_ms: 15.0,
    };

    let stateful_result = StatefulSloIterator::new(stateful_slo, vec![stateful_sample].into_iter())
        .next()
        .unwrap();

    // ML SLO evaluation
    let ml_slo = MlSlo::default();
    let ml_sample = MlSample {
        timestamp: 1000,
        inference_latency_ms: 150.0,
        gpu_utilization: 0.75,
        feature_drift: 0.05,
        prediction_confidence: 0.96,
    };

    let ml_result = MlSloIterator::new(ml_slo, vec![ml_sample].into_iter())
        .next()
        .unwrap();

    // Verify all three types evaluated successfully
    assert!(http_result.pass, "HTTP SLO should pass");
    assert!(stateful_result.pass, "Stateful SLO should pass");
    assert!(ml_result.pass, "ML SLO should pass");
}

// ============================================================================
// SCENARIO 3: Complex Composite SLO with Multiple Dependency Levels
// ============================================================================

#[test]
fn composite_slo_with_cascading_dependencies() {
    let graph = CompositeSloGraph {
        services: vec![
            // Tier 1: Core services
            CompositeServiceSlo {
                service: "db".to_string(),
                local_score: 0.98,
                min_pass_score: 0.95,
                impact_weight: 2.0,
            },
            CompositeServiceSlo {
                service: "cache".to_string(),
                local_score: 0.99,
                min_pass_score: 0.95,
                impact_weight: 1.5,
            },
            // Tier 2: Dependent services
            CompositeServiceSlo {
                service: "api".to_string(),
                local_score: 0.97,
                min_pass_score: 0.95,
                impact_weight: 2.5,
            },
            // Tier 3: Consumer
            CompositeServiceSlo {
                service: "gateway".to_string(),
                local_score: 0.96,
                min_pass_score: 0.95,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![
            CompositeDependencyEdge {
                dependency: "db".to_string(),
                dependent: "api".to_string(),
                failure_penalty: 0.15,
            },
            CompositeDependencyEdge {
                dependency: "cache".to_string(),
                dependent: "api".to_string(),
                failure_penalty: 0.1,
            },
            CompositeDependencyEdge {
                dependency: "api".to_string(),
                dependent: "gateway".to_string(),
                failure_penalty: 0.2,
            },
        ],
        global_min_pass_score: 0.90,
    };

    let result = evaluate_composite_slo(&graph).expect("Composite SLO evaluation should succeed");

    // Verify topological sort
    assert_eq!(
        result.topological_order.len(),
        4,
        "All 4 services should be ordered"
    );

    // Verify dependency propagation
    let api_entry = result
        .services
        .iter()
        .find(|s| s.service == "api")
        .expect("API service should be evaluated");

    assert!(
        !api_entry.dependency_adjusted,
        "API should not have dependency adjustment when all deps pass"
    );
    assert_eq!(
        api_entry.failed_dependencies.len(),
        0,
        "No failed dependencies (all scores > threshold)"
    );

    // Verify global SLO calculation
    let expected_weights = 2.0 + 1.5 + 2.5 + 1.0;
    let expected_global = (0.98 * 2.0 + 0.99 * 1.5 + 0.97 * 2.5 + 0.96 * 1.0) / expected_weights;
    assert!(
        (result.global_slo - expected_global).abs() < 1e-9,
        "Global SLO should match weighted average"
    );
    assert!(
        result.global_pass,
        "Global SLO should pass (all > threshold)"
    );
}

// ============================================================================
// SCENARIO 4: Composite SLO with Multiple Failures
// ============================================================================

#[test]
fn composite_slo_handles_multiple_failures_and_cascading_impact() {
    let graph = CompositeSloGraph {
        services: vec![
            CompositeServiceSlo {
                service: "db".to_string(),
                local_score: 0.80, // Below 0.9 threshold - FAILS
                min_pass_score: 0.9,
                impact_weight: 3.0,
            },
            CompositeServiceSlo {
                service: "api".to_string(),
                local_score: 0.85, // Would pass but depends on db
                min_pass_score: 0.9,
                impact_weight: 2.0,
            },
            CompositeServiceSlo {
                service: "web".to_string(),
                local_score: 0.92, // Passes independently
                min_pass_score: 0.9,
                impact_weight: 1.0,
            },
        ],
        dependencies: vec![CompositeDependencyEdge {
            dependency: "db".to_string(),
            dependent: "api".to_string(),
            failure_penalty: 0.25,
        }],
        global_min_pass_score: 0.85,
    };

    let result =
        evaluate_composite_slo(&graph).expect("Should evaluate composite SLO with failures");

    let db_entry = result.services.iter().find(|s| s.service == "db").unwrap();
    let api_entry = result.services.iter().find(|s| s.service == "api").unwrap();
    let web_entry = result.services.iter().find(|s| s.service == "web").unwrap();

    // DB fails due to score < threshold
    assert!(!db_entry.pass, "DB should fail (0.80 < 0.9 threshold)");

    // API fails due to dependency on DB failure
    assert!(
        api_entry.dependency_adjusted,
        "API should have dependency adjustment"
    );
    assert_eq!(
        api_entry.failed_dependencies,
        vec!["db".to_string()],
        "DB should be in failed dependencies"
    );
    let expected_api_score = 0.85 * (1.0 - 0.25); // Original * (1 - penalty) = multiplicative
    assert!(
        (api_entry.effective_score - expected_api_score).abs() < 1e-9,
        "API score should have penalty applied"
    );
    assert!(!api_entry.pass, "API should fail after penalty");

    // Web passes independently
    assert!(web_entry.pass, "Web should pass (0.92 > 0.9)");
    assert!(
        !web_entry.dependency_adjusted,
        "Web should have no dependency adjustment"
    );

    // Global calculation
    let expected_global = (0.80 * 3.0 + expected_api_score * 2.0 + 0.92 * 1.0) / 6.0;
    assert!(
        (result.global_slo - expected_global).abs() < 1e-9,
        "Global SLO should reflect failures"
    );
    assert!(
        !result.global_pass,
        "Global SLO should fail when weighted average < threshold"
    );
}

// ============================================================================
// SCENARIO 5: Error Budget Burn Rate Over Multiple Windows
// ============================================================================

#[test]
fn error_budget_burn_analysis_with_window_comparison() {
    // SLO: 99.9% availability over 30-day month
    let target = 0.999;
    let month_seconds = 30 * 24 * 60 * 60;
    let _budget_seconds = calculate_error_budget(target, month_seconds);

    // Create metric stream using binary error flags (1.0 = error, 0.0 = ok).
    // First 55 minutes: no errors; last 5 minutes: all errors (spike at end of window).
    // calculate_burn_rate looks at the most recent window, so spike must be at the end.
    let stream: Vec<MetricPoint> = (0..3600)
        .map(|ts| MetricPoint {
            timestamp: ts as i64,
            value: if ts >= 3300 { 1.0 } else { 0.0 },
            labels: Default::default(),
        })
        .collect();

    // Calculate burn rates over different windows
    let burn_5m = calculate_burn_rate(stream.clone(), 300);
    let burn_1h = calculate_burn_rate(stream.clone(), 3600);

    // Last 5 minutes: 100% errors → burn = 1.0; shows spike vs full-hour average
    assert!(burn_5m > burn_1h, "5-minute burn should show spike");

    // Full hour: averaged down due to quiet recovery period (300/3600)
    assert!(
        burn_1h < burn_5m,
        "1-hour burn should be lower than 5-minute spike"
    );

    // Monthly rate estimate: spike is tiny fraction of a month
    let monthly_burn = calculate_burn_rate(stream, month_seconds);
    assert!(
        monthly_burn < burn_1h,
        "Monthly average should be much lower than 1-hour average"
    );
}

// ============================================================================
// SCENARIO 6: Serialization Round-Trip for Configuration
// ============================================================================

#[test]
fn slo_configuration_serialization_round_trip() {
    let original_config = SloConfig {
        target: 99.95,
        window: "7d".to_string(),
    };

    // JSON serialization round trip
    let json_string = original_config
        .to_json_string()
        .expect("Config should serialize to JSON");
    let deserialized =
        SloConfig::from_json_str(&json_string).expect("Config should deserialize from JSON");

    assert_eq!(
        deserialized, original_config,
        "Round-trip should preserve configuration"
    );
    assert_eq!(deserialized.target, 99.95);
    assert_eq!(deserialized.window, "7d");
}

// ============================================================================
// SCENARIO 7: Calendar vs Rolling Window Behavior
// ============================================================================

#[test]
fn window_selection_impacts_slo_evaluation() {
    let now = 1_700_000_000_i64;

    // Calendar window: daily boundary (86400 seconds), offset by 5 hours
    let calendar_window = TimeWindow::calendar_aligned(86_400, 5 * 3600);

    // Rolling window: 24 hours always
    let rolling_window = TimeWindow::rolling(86_400);

    // Test boundaries
    let just_after_calendar_start = 1_700_000_005; // Just after calendar window starts
    let just_before_calendar_end = 1_700_074_799; // Just before calendar window ends

    assert!(
        calendar_window.contains(just_after_calendar_start, now),
        "Just after start should be in calendar window"
    );
    assert!(
        calendar_window.contains(just_before_calendar_end, now),
        "Just before end should be in calendar window"
    );

    // Rolling window looks back 86400 seconds from now
    let older_than_rolling = now - 86_401;
    assert!(
        !rolling_window.contains(older_than_rolling, now),
        "Too old for rolling window"
    );
    assert!(
        rolling_window.contains(now - 86_400, now),
        "Exactly 24h back should be in rolling window"
    );
}

// ============================================================================
// SCENARIO 8: GenAI SLO with Quality and Performance Trade-offs
// ============================================================================

#[test]
fn genai_slo_balances_speed_and_quality() {
    let slo = GenAiSlo::default();

    // Sample 1: Fast but perfect quality
    let fast_perfect = GenAiSample {
        timestamp: 1000,
        tokens_generated: 500,
        generation_duration_ms: 2000.0, // 250 tokens/sec - fast
        time_to_first_token_ms: 200.0,
        reference_text: "what is AI".to_string(),
        generated_text: "what is AI".to_string(), // Perfect match
    };

    // Sample 2: Slow with degraded quality
    let slow_degraded = GenAiSample {
        timestamp: 2000,
        tokens_generated: 500,
        generation_duration_ms: 30000.0, // 16 tokens/sec - below threshold
        time_to_first_token_ms: 3000.0,  // Slow to first token
        reference_text: "what is AI".to_string(),
        generated_text: "AI is technology".to_string(), // Different but related
    };

    let results: Vec<_> =
        GenAiSloIterator::new(slo, vec![fast_perfect, slow_degraded].into_iter()).collect();

    assert_eq!(results.len(), 2);
    assert!(results[0].pass, "Fast perfect should pass");
    assert!(results[0].tokens_per_second_ok, "Fast should be OK on TPS");
    assert!(results[0].time_to_first_token_ok, "Low TTFT should be OK");

    assert!(!results[1].pass, "Slow degraded should fail");
    assert!(!results[1].tokens_per_second_ok, "Slow should fail TPS");
    assert!(!results[1].time_to_first_token_ok, "High TTFT should fail");
}

// ============================================================================
// SCENARIO 9: Error Detection in Malformed Data
// ============================================================================

#[test]
fn composite_slo_error_detection() {
    // Test: Unknown service reference
    let graph_unknown_service = CompositeSloGraph {
        services: vec![CompositeServiceSlo {
            service: "api".to_string(),
            local_score: 0.95,
            min_pass_score: 0.9,
            impact_weight: 1.0,
        }],
        dependencies: vec![CompositeDependencyEdge {
            dependency: "cache".to_string(), // Doesn't exist
            dependent: "api".to_string(),
            failure_penalty: 0.1,
        }],
        global_min_pass_score: 0.9,
    };

    let result = evaluate_composite_slo(&graph_unknown_service);
    assert!(result.is_err(), "Should error on unknown service");
    assert_eq!(
        result.unwrap_err(),
        CompositeSloError::UnknownService("cache".to_string())
    );

    // Test: Cyclic dependency detection
    let graph_cycle = CompositeSloGraph {
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
                failure_penalty: 0.1,
            },
            CompositeDependencyEdge {
                dependency: "b".to_string(),
                dependent: "a".to_string(),
                failure_penalty: 0.1,
            },
        ],
        global_min_pass_score: 0.9,
    };

    let result = evaluate_composite_slo(&graph_cycle);
    assert!(result.is_err(), "Should error on cycle detection");
    assert_eq!(result.unwrap_err(), CompositeSloError::CycleDetected);
}

// ============================================================================
// SCENARIO 10: Performance Under Load
// ============================================================================

#[test]
fn http_slo_handles_large_histogram_stream() {
    let slo = HttpSlo::default();

    // Generate 1000 histogram samples
    let stream: Vec<HistogramSample> = (0..1000)
        .map(|i| {
            let success = 9_000 + (i % 1_000);
            HistogramSample {
                timestamp: i as i64,
                success: success as u64,
                total: 10_000,
                buckets: vec![
                    HistogramBucket {
                        upper_bound_ms: 100.0,
                        count: 7_000 + (i % 3000) as u64,
                    },
                    HistogramBucket {
                        upper_bound_ms: 200.0,
                        count: 9_500 + (i % 500) as u64,
                    },
                    HistogramBucket {
                        upper_bound_ms: 500.0,
                        count: 10_000,
                    },
                ],
                format: HistogramFormat::PrometheusCumulative,
            }
        })
        .collect();

    let results: Vec<_> = HttpSloIterator::new(slo, stream.into_iter()).collect();

    assert_eq!(results.len(), 1000, "All 1000 samples should be evaluated");

    // Verify some pass and some fail
    let pass_count = results.iter().filter(|r| r.pass).count();
    assert!(pass_count > 0, "Some should pass");
    assert!(
        pass_count < 1000,
        "Some should fail due to low availability"
    );
}
