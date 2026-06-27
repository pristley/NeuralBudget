// Comprehensive test suite for Agent SLO evaluation

use neuralbudget::agent_slo::*;

fn create_simple_trajectory(
    steps_data: Vec<(AgentAction, bool, Option<&str>)>,
    final_status: FinalStatus,
) -> AgentTrajectory {
    let steps = steps_data
        .into_iter()
        .enumerate()
        .map(|(i, (action, success, tool_name))| TrajectoryStep {
            step: (i + 1) as u32,
            action,
            tool_name: match action {
                AgentAction::ToolCall => Some(tool_name.unwrap_or("tool").to_string()),
                _ => None,
            },
            success,
            output: Some(format!("Output {}", i)),
            duration_ms: Some(100),
        })
        .collect();

    AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps,
        final_status,
    }
}

#[test]
fn test_simple_successful_agent_run() {
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("search")),
            (AgentAction::ToolCall, true, Some("summarize")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert_eq!(eval.steps_taken, 3);
    assert_eq!(eval.tool_success_rate, 1.0);
    assert!(!eval.loop_detected);
    assert!(eval.success);
    assert_eq!(eval.tool_calls_made, 2);
    assert_eq!(eval.tool_calls_succeeded, 2);
}

#[test]
fn test_agent_exceeds_max_steps() {
    let trajectory = AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps: (1..=12)
            .map(|i| TrajectoryStep {
                step: i as u32,
                action: AgentAction::ToolCall,
                tool_name: Some(format!("tool_{}", i)),
                success: true,
                output: Some("OK".to_string()),
                duration_ms: Some(100),
            })
            .collect(),
        final_status: FinalStatus::MaxStepsExceeded,
    };

    let params = AgentSloParams {
        max_steps: 10,
        ..Default::default()
    };
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(!eval.pass);
    assert!(!eval.details.max_steps_pass);
    assert_eq!(eval.steps_taken, 12);
}

#[test]
fn test_tool_success_rate_below_threshold() {
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("api1")),
            (AgentAction::ToolCall, false, Some("api2")),
            (AgentAction::ToolCall, false, Some("api3")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let params = AgentSloParams {
        tool_success_threshold: 0.95,
        ..Default::default()
    };
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(!eval.pass);
    assert!(!eval.details.tool_success_pass);
    assert_eq!(eval.tool_calls_made, 3);
    assert_eq!(eval.tool_calls_succeeded, 1);
    assert!((eval.tool_success_rate - (1.0 / 3.0)).abs() < 0.001);
}

#[test]
fn test_loop_detection_repeated_actions() {
    let trajectory = AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: Some("No results".to_string()),
                duration_ms: Some(100),
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: Some("Still no results".to_string()),
                duration_ms: Some(100),
            },
            TrajectoryStep {
                step: 3,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: Some("Still nothing".to_string()),
                duration_ms: Some(100),
            },
        ],
        final_status: FinalStatus::LoopDetected,
    };

    let params = AgentSloParams {
        max_repeated_actions: 2,
        ..Default::default()
    };
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(!eval.pass);
    assert!(eval.loop_detected);
    assert!(!eval.details.loop_detection_pass);
    assert_eq!(*eval.action_repetitions.get("tool_call").unwrap(), 3);
}

#[test]
fn test_final_status_failure() {
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("search")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Failure,
    );

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(!eval.pass);
    assert!(!eval.success);
    assert!(!eval.details.success_status_pass);
}

#[test]
fn test_batch_evaluation() {
    let trajectories = vec![
        create_simple_trajectory(
            vec![
                (AgentAction::ToolCall, true, Some("search")),
                (AgentAction::Response, true, None),
            ],
            FinalStatus::Success,
        ),
        create_simple_trajectory(
            vec![
                (AgentAction::ToolCall, true, Some("search")),
                (AgentAction::ToolCall, false, Some("api")),
                (AgentAction::Response, true, None),
            ],
            FinalStatus::Success,
        ),
        create_simple_trajectory(
            vec![
                (AgentAction::ToolCall, true, Some("search")),
                (AgentAction::ToolCall, true, Some("summarize")),
                (AgentAction::ToolCall, true, Some("rank")),
                (AgentAction::Response, true, None),
            ],
            FinalStatus::Success,
        ),
    ];

    let params = AgentSloParams::default();
    let batch_eval = evaluate_agent_batch(&trajectories, &params).unwrap();

    assert_eq!(batch_eval.total_evaluations, 3);
    assert!(batch_eval.success_rate > 0.0);
    assert!(batch_eval.avg_steps > 0.0);
    assert!(batch_eval.avg_tool_success_rate > 0.0);
}

#[test]
fn test_no_tool_calls_perfect_rate() {
    let trajectory = AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::Thought,
                tool_name: None,
                success: true,
                output: Some("Thinking...".to_string()),
                duration_ms: Some(50),
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::Response,
                tool_name: None,
                success: true,
                output: Some("Final answer".to_string()),
                duration_ms: Some(100),
            },
        ],
        final_status: FinalStatus::Success,
    };

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert_eq!(eval.tool_calls_made, 0);
    assert_eq!(eval.tool_success_rate, 1.0);
}

#[test]
fn test_action_repetition_counts() {
    let trajectory = AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::Thought,
                tool_name: None,
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::Thought,
                tool_name: None,
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 3,
                action: AgentAction::ToolCall,
                tool_name: Some("api".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 4,
                action: AgentAction::ToolCall,
                tool_name: Some("api".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 5,
                action: AgentAction::Response,
                tool_name: None,
                success: true,
                output: None,
                duration_ms: None,
            },
        ],
        final_status: FinalStatus::Success,
    };

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert_eq!(*eval.action_repetitions.get("thought").unwrap(), 2);
    assert_eq!(*eval.action_repetitions.get("tool_call").unwrap(), 2);
    assert_eq!(*eval.action_repetitions.get("response").unwrap(), 1);
}

#[test]
fn test_timeout_status() {
    let trajectory = AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps: vec![TrajectoryStep {
            step: 1,
            action: AgentAction::ToolCall,
            tool_name: Some("slow_api".to_string()),
            success: false,
            output: Some("Timeout".to_string()),
            duration_ms: Some(30000),
        }],
        final_status: FinalStatus::Timeout,
    };

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(!eval.pass);
    assert!(!eval.success);
}

#[test]
fn test_large_batch_metrics() {
    let mut trajectories = Vec::new();
    for i in 0..100 {
        let steps = if i % 10 == 0 {
            vec![(AgentAction::ToolCall, false, Some("api"))]
        } else {
            vec![
                (AgentAction::ToolCall, true, Some("api")),
                (AgentAction::Response, true, None),
            ]
        };

        trajectories.push(create_simple_trajectory(steps, FinalStatus::Success));
    }

    let params = AgentSloParams::default();
    let batch_eval = evaluate_agent_batch(&trajectories, &params).unwrap();

    assert_eq!(batch_eval.total_evaluations, 100);
    assert!(batch_eval.success_rate >= 0.8);
}

#[test]
fn test_empty_trajectory_invalid() {
    let trajectories: Vec<AgentTrajectory> = vec![];
    let params = AgentSloParams::default();

    let result = evaluate_agent_batch(&trajectories, &params);
    assert!(result.is_err());
}

#[test]
fn test_serialization_trajectory() {
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("search")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let json = serde_json::to_string(&trajectory).unwrap();
    let deserialized: AgentTrajectory = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.run_id, trajectory.run_id);
    assert_eq!(deserialized.steps.len(), trajectory.steps.len());
}

#[test]
fn test_serialization_evaluation() {
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("search")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    let json = serde_json::to_string(&eval).unwrap();
    let deserialized: AgentEvaluation = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.pass, eval.pass);
    assert_eq!(deserialized.steps_taken, eval.steps_taken);
}

#[test]
fn test_all_slo_criteria_met() {
    let trajectory = AgentTrajectory {
        run_id: "perfect_run".to_string(),
        timestamp: 1000,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: Some("Found results".to_string()),
                duration_ms: Some(500),
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::ToolCall,
                tool_name: Some("summarize".to_string()),
                success: true,
                output: Some("Summary: ...".to_string()),
                duration_ms: Some(300),
            },
            TrajectoryStep {
                step: 3,
                action: AgentAction::Response,
                tool_name: None,
                success: true,
                output: Some("Final answer".to_string()),
                duration_ms: Some(100),
            },
        ],
        final_status: FinalStatus::Success,
    };

    let params = AgentSloParams {
        max_steps: 5,
        tool_success_threshold: 0.9,
        max_repeated_actions: 1,
        success_threshold: 0.8,
        loop_detection_enabled: true,
    };

    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert!(eval.details.max_steps_pass);
    assert!(eval.details.tool_success_pass);
    assert!(eval.details.loop_detection_pass);
    assert!(eval.details.success_status_pass);
}

#[test]
fn test_research_agent_scenario() {
    // Realistic research agent: search multiple times, synthesize answer
    let trajectory = AgentTrajectory {
        run_id: "research_123".to_string(),
        timestamp: 1719446400,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::Thought,
                tool_name: None,
                success: true,
                output: Some("Need to find latest AI trends".to_string()),
                duration_ms: Some(50),
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: Some("Found 5 articles".to_string()),
                duration_ms: Some(800),
            },
            TrajectoryStep {
                step: 3,
                action: AgentAction::ToolCall,
                tool_name: Some("fetch".to_string()),
                success: true,
                output: Some("Retrieved content".to_string()),
                duration_ms: Some(400),
            },
            TrajectoryStep {
                step: 4,
                action: AgentAction::ToolCall,
                tool_name: Some("summarize".to_string()),
                success: true,
                output: Some("Key points identified".to_string()),
                duration_ms: Some(600),
            },
            TrajectoryStep {
                step: 5,
                action: AgentAction::Response,
                tool_name: None,
                success: true,
                output: Some("Top 3 AI trends are...".to_string()),
                duration_ms: Some(200),
            },
        ],
        final_status: FinalStatus::Success,
    };

    let params = AgentSloParams {
        max_steps: 10,
        tool_success_threshold: 0.95,
        max_repeated_actions: 2,
        success_threshold: 0.90,
        loop_detection_enabled: true,
    };

    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert_eq!(eval.steps_taken, 5);
    assert_eq!(eval.tool_success_rate, 1.0);
    assert!(!eval.loop_detected);
}

#[test]
fn test_coding_agent_scenario() {
    // Realistic coding agent: write code, test, fix
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::Thought, true, None),
            (AgentAction::ToolCall, true, Some("write_code")),
            (AgentAction::ToolCall, true, Some("run_tests")),
            (AgentAction::ToolCall, false, Some("run_tests")), // Test failure
            (AgentAction::ToolCall, true, Some("fix_code")),
            (AgentAction::ToolCall, true, Some("run_tests")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let params = AgentSloParams {
        max_steps: 10,
        tool_success_threshold: 0.85, // Tolerate some test failures
        max_repeated_actions: 3,       // Allow test-fix cycles
        success_threshold: 0.90,
        loop_detection_enabled: true,
    };

    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert_eq!(eval.tool_calls_made, 5);
    assert!(eval.tool_success_rate >= 0.85);
}

#[test]
fn test_support_agent_scenario() {
    // Support bot: lookup, analyze, respond
    let trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("lookup_customer")),
            (AgentAction::ToolCall, true, Some("search_kb")),
            (AgentAction::ToolCall, true, Some("fetch_docs")),
            (AgentAction::Thought, true, None),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let params = AgentSloParams {
        max_steps: 10,
        tool_success_threshold: 0.95,
        max_repeated_actions: 1,
        success_threshold: 0.95,
        loop_detection_enabled: true,
    };

    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert!(!eval.loop_detected);
}

#[test]
fn test_tool_success_rate_exactly_at_threshold() {
    // Test boundary condition
    let trajectory = AgentTrajectory {
        run_id: "boundary_test".to_string(),
        timestamp: 1000,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::ToolCall,
                tool_name: Some("tool1".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::ToolCall,
                tool_name: Some("tool2".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 3,
                action: AgentAction::ToolCall,
                tool_name: Some("tool3".to_string()),
                success: false,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 4,
                action: AgentAction::ToolCall,
                tool_name: Some("tool4".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 5,
                action: AgentAction::Response,
                tool_name: None,
                success: true,
                output: None,
                duration_ms: None,
            },
        ],
        final_status: FinalStatus::Success,
    };

    // 3/4 = 0.75 tool success
    let params = AgentSloParams {
        tool_success_threshold: 0.75,
        ..Default::default()
    };

    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass); // Exactly at threshold
    assert!(eval.details.tool_success_pass);
}

#[test]
fn test_batch_with_mixed_results() {
    let passing_trajectory = create_simple_trajectory(
        vec![
            (AgentAction::ToolCall, true, Some("api")),
            (AgentAction::Response, true, None),
        ],
        FinalStatus::Success,
    );

    let failing_trajectory = AgentTrajectory {
        run_id: "fail_run".to_string(),
        timestamp: 1000,
        steps: vec![TrajectoryStep {
            step: 1,
            action: AgentAction::ToolCall,
            tool_name: Some("search".to_string()),
            success: true,
            output: None,
            duration_ms: None,
        }],
        final_status: FinalStatus::Timeout,
    };

    let trajectories = vec![passing_trajectory, failing_trajectory];
    let params = AgentSloParams::default();
    let batch_eval = evaluate_agent_batch(&trajectories, &params).unwrap();

    assert_eq!(batch_eval.total_evaluations, 2);
    assert_eq!(batch_eval.successful_evaluations, 1);
    assert!((batch_eval.success_rate - 0.5).abs() < 0.001);
}

#[test]
fn test_loop_detection_disabled() {
    let trajectory = AgentTrajectory {
        run_id: "test_run".to_string(),
        timestamp: 1000,
        steps: vec![
            TrajectoryStep {
                step: 1,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 2,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
            TrajectoryStep {
                step: 3,
                action: AgentAction::ToolCall,
                tool_name: Some("search".to_string()),
                success: true,
                output: None,
                duration_ms: None,
            },
        ],
        final_status: FinalStatus::Success,
    };

    // Same trajectory as loop test, but with detection disabled
    let params = AgentSloParams {
        max_repeated_actions: 2,
        loop_detection_enabled: false, // Disabled
        ..Default::default()
    };

    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass); // Passes because loop detection is disabled
    assert!(eval.loop_detected); // But loop is still detected in data
    assert!(eval.details.loop_detection_pass); // Pass criteria ignores it
}

#[test]
fn test_very_simple_agent() {
    let trajectory = AgentTrajectory {
        run_id: "simple".to_string(),
        timestamp: 1000,
        steps: vec![TrajectoryStep {
            step: 1,
            action: AgentAction::Response,
            tool_name: None,
            success: true,
            output: Some("Direct answer".to_string()),
            duration_ms: Some(100),
        }],
        final_status: FinalStatus::Success,
    };

    let params = AgentSloParams::default();
    let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

    assert!(eval.pass);
    assert_eq!(eval.steps_taken, 1);
    assert_eq!(eval.tool_calls_made, 0);
}
