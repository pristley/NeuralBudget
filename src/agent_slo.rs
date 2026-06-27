/// Agent SLO (Service Level Objective) evaluation for LLM agent execution.
///
/// Tracks agent trajectories, tool call success, loop detection, and final outcomes
/// to provide reliability metrics for autonomous AI agents.

use crate::{NeuralBudgetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an action taken by the agent during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAction {
    /// Agent is calling an external tool (search, API, code execution, etc.)
    #[serde(rename = "tool_call")]
    ToolCall,
    /// Agent is generating a final response to the user
    #[serde(rename = "response")]
    Response,
    /// Agent is thinking/planning without external action
    #[serde(rename = "thought")]
    Thought,
}

impl std::fmt::Display for AgentAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentAction::ToolCall => write!(f, "tool_call"),
            AgentAction::Response => write!(f, "response"),
            AgentAction::Thought => write!(f, "thought"),
        }
    }
}

/// The outcome status of an agent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalStatus {
    /// Agent successfully completed the task
    #[serde(rename = "success")]
    Success,
    /// Agent failed to complete the task
    #[serde(rename = "failure")]
    Failure,
    /// Agent execution exceeded time limit
    #[serde(rename = "timeout")]
    Timeout,
    /// Agent got stuck in a loop
    #[serde(rename = "loop_detected")]
    LoopDetected,
    /// Agent exceeded maximum allowed steps
    #[serde(rename = "max_steps_exceeded")]
    MaxStepsExceeded,
}

impl std::fmt::Display for FinalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinalStatus::Success => write!(f, "success"),
            FinalStatus::Failure => write!(f, "failure"),
            FinalStatus::Timeout => write!(f, "timeout"),
            FinalStatus::LoopDetected => write!(f, "loop_detected"),
            FinalStatus::MaxStepsExceeded => write!(f, "max_steps_exceeded"),
        }
    }
}

/// A single step in an agent's execution trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    /// Step counter (1-indexed)
    pub step: u32,
    /// Type of action performed
    pub action: AgentAction,
    /// Name of the tool called (only for ToolCall actions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// Whether the step succeeded
    pub success: bool,
    /// Optional output or result from this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Duration of this step in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
}

/// Complete execution trajectory of an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrajectory {
    /// Unique identifier for this agent run
    pub run_id: String,
    /// Timestamp of agent execution (seconds since epoch)
    pub timestamp: u64,
    /// Sequence of steps taken by the agent
    pub steps: Vec<TrajectoryStep>,
    /// Final outcome of the agent execution
    pub final_status: FinalStatus,
}

/// Configuration parameters for agent SLO evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSloParams {
    /// Maximum number of steps agent should take (hard limit)
    pub max_steps: u32,
    /// Minimum acceptable tool call success rate (0.0-1.0)
    pub tool_success_threshold: f64,
    /// Maximum times the same action can repeat before flagging as loop
    pub max_repeated_actions: u32,
    /// Minimum acceptable overall success rate (0.0-1.0)
    pub success_threshold: f64,
    /// Enable loop detection
    pub loop_detection_enabled: bool,
}

impl Default for AgentSloParams {
    fn default() -> Self {
        Self {
            max_steps: 10,
            tool_success_threshold: 0.95,
            max_repeated_actions: 2,
            success_threshold: 0.90,
            loop_detection_enabled: true,
        }
    }
}

/// Result of evaluating an agent trajectory against SLO parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvaluation {
    /// Number of steps taken by the agent
    pub steps_taken: u32,
    /// Success rate of tool calls (0.0-1.0)
    pub tool_success_rate: f64,
    /// Number of tool calls made
    pub tool_calls_made: u32,
    /// Number of successful tool calls
    pub tool_calls_succeeded: u32,
    /// Whether a loop was detected in the trajectory
    pub loop_detected: bool,
    /// Repeated action counts
    pub action_repetitions: HashMap<String, u32>,
    /// Whether the final status was success
    pub success: bool,
    /// Overall SLO pass/fail
    pub pass: bool,
    /// Detailed pass/fail reasons
    pub details: EvaluationDetails,
}

/// Detailed breakdown of evaluation criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationDetails {
    /// Whether max_steps constraint was met
    pub max_steps_pass: bool,
    /// Whether tool_success_rate constraint was met
    pub tool_success_pass: bool,
    /// Whether no loop was detected
    pub loop_detection_pass: bool,
    /// Whether final_status was Success
    pub success_status_pass: bool,
}

/// Evaluate an agent trajectory against SLO parameters.
///
/// # Arguments
/// * `trajectory` - The agent's execution trajectory
/// * `params` - SLO evaluation parameters
///
/// # Returns
/// * `Result<AgentEvaluation>` - Evaluation results with pass/fail determination
pub fn evaluate_agent_slo(
    trajectory: &AgentTrajectory,
    params: &AgentSloParams,
) -> Result<AgentEvaluation> {
    // Check max steps constraint
    let max_steps_pass = (trajectory.steps.len() as u32) <= params.max_steps;

    // Calculate tool success rate
    let (tool_calls_made, tool_calls_succeeded, tool_success_rate) =
        calculate_tool_success_rate(&trajectory.steps);
    let tool_success_pass = tool_success_rate >= params.tool_success_threshold;

    // Detect loops and repetitions
    let (loop_detected, action_repetitions) =
        detect_loops(&trajectory.steps, params.max_repeated_actions);
    let loop_detection_pass = !loop_detected || !params.loop_detection_enabled;

    // Check final success status
    let success_status_pass = trajectory.final_status == FinalStatus::Success;

    // Overall pass/fail
    let pass = max_steps_pass && tool_success_pass && loop_detection_pass && success_status_pass;

    Ok(AgentEvaluation {
        steps_taken: trajectory.steps.len() as u32,
        tool_success_rate,
        tool_calls_made,
        tool_calls_succeeded,
        loop_detected,
        action_repetitions,
        success: success_status_pass,
        pass,
        details: EvaluationDetails {
            max_steps_pass,
            tool_success_pass,
            loop_detection_pass,
            success_status_pass,
        },
    })
}

/// Calculate tool call success rate from trajectory steps.
///
/// Returns (total_calls, successful_calls, success_rate)
fn calculate_tool_success_rate(steps: &[TrajectoryStep]) -> (u32, u32, f64) {
    let tool_calls: Vec<&TrajectoryStep> = steps
        .iter()
        .filter(|s| s.action == AgentAction::ToolCall)
        .collect();

    if tool_calls.is_empty() {
        return (0, 0, 1.0); // No tool calls = perfect rate
    }

    let succeeded = tool_calls.iter().filter(|s| s.success).count() as u32;
    let total = tool_calls.len() as u32;
    let rate = succeeded as f64 / total as f64;

    (total, succeeded, rate)
}

/// Detect loops and action repetitions in trajectory.
///
/// Returns (loop_detected, action_repetition_counts)
fn detect_loops(
    steps: &[TrajectoryStep],
    max_allowed_repeats: u32,
) -> (bool, HashMap<String, u32>) {
    let mut action_counts: HashMap<String, u32> = HashMap::new();

    for step in steps {
        let action_str = step.action.to_string();
        *action_counts.entry(action_str).or_insert(0) += 1;
    }

    let loop_detected = action_counts
        .values()
        .any(|&count| count > max_allowed_repeats);

    (loop_detected, action_counts)
}

/// Batch evaluation of multiple agent trajectories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBatchEvaluation {
    /// Total trajectories evaluated
    pub total_evaluations: u32,
    /// Number of successful evaluations (pass=true)
    pub successful_evaluations: u32,
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
    /// Average steps per evaluation
    pub avg_steps: f64,
    /// Average tool success rate across all evaluations
    pub avg_tool_success_rate: f64,
    /// Percentage of evaluations with loops detected
    pub loop_detection_rate: f64,
}

/// Evaluate a batch of agent trajectories.
///
/// # Arguments
/// * `trajectories` - Slice of agent trajectories
/// * `params` - SLO parameters
///
/// # Returns
/// * `Result<AgentBatchEvaluation>` - Batch evaluation results
pub fn evaluate_agent_batch(
    trajectories: &[AgentTrajectory],
    params: &AgentSloParams,
) -> Result<AgentBatchEvaluation> {
    if trajectories.is_empty() {
        return Err(NeuralBudgetError::ConfigError(
            "Cannot evaluate empty trajectory batch".to_string(),
        ));
    }

    let mut total_steps = 0u32;
    let mut total_tool_success_rate = 0.0f64;
    let mut loops_detected = 0u32;
    let mut evaluations_passed = 0u32;

    for trajectory in trajectories {
        let eval = evaluate_agent_slo(trajectory, params)?;
        total_steps += eval.steps_taken;
        total_tool_success_rate += eval.tool_success_rate;
        if eval.loop_detected {
            loops_detected += 1;
        }
        if eval.pass {
            evaluations_passed += 1;
        }
    }

    let count = trajectories.len() as f64;
    Ok(AgentBatchEvaluation {
        total_evaluations: trajectories.len() as u32,
        successful_evaluations: evaluations_passed,
        success_rate: evaluations_passed as f64 / count,
        avg_steps: total_steps as f64 / count,
        avg_tool_success_rate: total_tool_success_rate / count,
        loop_detection_rate: loops_detected as f64 / count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_trajectory(steps_data: Vec<(AgentAction, bool)>) -> AgentTrajectory {
        let steps = steps_data
            .into_iter()
            .enumerate()
            .map(|(i, (action, success))| TrajectoryStep {
                step: (i + 1) as u32,
                action,
                tool_name: match action {
                    AgentAction::ToolCall => Some(format!("tool_{}", i)),
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
            final_status: FinalStatus::Success,
        }
    }

    #[test]
    fn test_simple_successful_agent_run() {
        let trajectory = create_sample_trajectory(vec![
            (AgentAction::ToolCall, true),
            (AgentAction::ToolCall, true),
            (AgentAction::Response, true),
        ]);

        let params = AgentSloParams::default();
        let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

        assert!(eval.pass);
        assert_eq!(eval.steps_taken, 3);
        assert_eq!(eval.tool_success_rate, 1.0);
        assert!(!eval.loop_detected);
        assert!(eval.success);
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
        let trajectory = create_sample_trajectory(vec![
            (AgentAction::ToolCall, true),
            (AgentAction::ToolCall, false),
            (AgentAction::ToolCall, false),
            (AgentAction::Response, true),
        ]);

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
        let trajectory = create_sample_trajectory(vec![
            (AgentAction::ToolCall, true),
            (AgentAction::Response, true),
        ]);

        let mut trajectory = trajectory;
        trajectory.final_status = FinalStatus::Failure;

        let params = AgentSloParams::default();
        let eval = evaluate_agent_slo(&trajectory, &params).unwrap();

        assert!(!eval.pass);
        assert!(!eval.success);
        assert!(!eval.details.success_status_pass);
    }

    #[test]
    fn test_batch_evaluation() {
        let trajectories = vec![
            create_sample_trajectory(vec![
                (AgentAction::ToolCall, true),
                (AgentAction::Response, true),
            ]),
            create_sample_trajectory(vec![
                (AgentAction::ToolCall, true),
                (AgentAction::ToolCall, false),
                (AgentAction::Response, true),
            ]),
            create_sample_trajectory(vec![
                (AgentAction::ToolCall, true),
                (AgentAction::ToolCall, true),
                (AgentAction::ToolCall, true),
                (AgentAction::Response, true),
            ]),
        ];

        let params = AgentSloParams::default();
        let batch_eval = evaluate_agent_batch(&trajectories, &params).unwrap();

        assert_eq!(batch_eval.total_evaluations, 3);
        assert!(batch_eval.success_rate > 0.0);
        assert!(batch_eval.avg_steps > 0.0);
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
        assert_eq!(eval.tool_success_rate, 1.0); // Perfect rate with no calls
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
                vec![(AgentAction::ToolCall, false), (AgentAction::Response, true)]
            } else {
                vec![(AgentAction::ToolCall, true), (AgentAction::Response, true)]
            };

            trajectories.push(create_sample_trajectory(steps));
        }

        let params = AgentSloParams::default();
        let batch_eval = evaluate_agent_batch(&trajectories, &params).unwrap();

        assert_eq!(batch_eval.total_evaluations, 100);
        assert!(batch_eval.success_rate >= 0.8); // 80% have all tools succeed
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
        let trajectory = create_sample_trajectory(vec![
            (AgentAction::ToolCall, true),
            (AgentAction::Response, true),
        ]);

        let json = serde_json::to_string(&trajectory).unwrap();
        let deserialized: AgentTrajectory = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.run_id, trajectory.run_id);
        assert_eq!(deserialized.steps.len(), trajectory.steps.len());
    }

    #[test]
    fn test_serialization_evaluation() {
        let trajectory = create_sample_trajectory(vec![
            (AgentAction::ToolCall, true),
            (AgentAction::Response, true),
        ]);

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
            max_repeated_actions: 2,  // Allow up to 2 repeated ToolCall actions
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
}
