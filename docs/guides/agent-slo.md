# Agent SLO: Service Level Objectives for Autonomous Agents

LLM agents are powerful but unpredictable. They take multiple steps, call tools, and can loop forever. **Agent SLOs** track whether agents complete tasks reliably and efficiently.

## The Problem

When you deploy an LLM agent in production, you care about:

1. **Did it complete successfully?** (success rate)
2. **Did it get stuck in a loop?** (loop detection)
3. **How many steps did it take?** (efficiency)
4. **How often did tool calls fail?** (reliability)

Without observability, you don't know if your agent is:
- Struggling with API failures and retrying endlessly
- Getting confused and repeating the same search 5 times
- Taking 50 steps when it should take 5
- Timing out in production

## When to Use Agent SLOs

| Scenario | Agent SLO Value | Example |
|----------|-----------------|---------|
| **Research Agents** | Track search loops and efficiency | Agent should find answer in ≤ 5 searches |
| **Coding Assistants** | Ensure code generation and testing completes | Tool success rate ≥ 95% |
| **Customer Support Bots** | Verify resolution without escalation loops | Max 10 steps to resolve ticket |
| **Data Analysis** | Monitor query formation and execution | Detect repeated failed queries |
| **E-commerce** | Prevent infinite recommendation loops | Max repeated actions = 2 |
| **Healthcare** | Track multi-step diagnostic reasoning | 90%+ of diagnostic chains succeed |
| **Financial Analysis** | Monitor market data fetching and computation | Complete in ≤ 15 steps |
| **Content Generation** | Track research and writing steps | Tool success ≥ 95% |

## Quick Start (5 minutes)

### 1. Define Your Agent SLO

```yaml
mode: genai

agent_slo:
  enabled: true
  
  trajectory_metrics:
    max_steps: 10              # Agent should complete in ≤ 10 steps
    tool_success_threshold: 0.95   # 95% of tools calls should succeed
    max_repeated_actions: 2    # Flag if same action repeats > 2 times
    success_threshold: 0.90    # 90% of runs should succeed
  
  loop_detection_enabled: true
```

### 2. Collect Agent Trajectory

When your agent runs, capture each step:

```rust
let trajectory = AgentTrajectory {
    run_id: "agent_run_123".to_string(),
    timestamp: 1719446400,
    steps: vec![
        TrajectoryStep {
            step: 1,
            action: AgentAction::ToolCall,
            tool_name: Some("search".to_string()),
            success: true,
            output: Some("Found 5 results".to_string()),
            duration_ms: Some(800),
        },
        TrajectoryStep {
            step: 2,
            action: AgentAction::Response,
            tool_name: None,
            success: true,
            output: Some("Based on my search...".to_string()),
            duration_ms: Some(200),
        },
    ],
    final_status: FinalStatus::Success,
};
```

### 3. Evaluate Against SLO

```rust
use neuralbudget::agent_slo::{evaluate_agent_slo, AgentSloParams};

let params = AgentSloParams {
    max_steps: 10,
    tool_success_threshold: 0.95,
    max_repeated_actions: 2,
    success_threshold: 0.90,
    loop_detection_enabled: true,
};

let evaluation = evaluate_agent_slo(&trajectory, &params)?;

println!("SLO Pass: {}", evaluation.pass);
println!("Steps: {}/{}", evaluation.steps_taken, 10);
println!("Tool Success: {:.1}%", evaluation.tool_success_rate * 100.0);
println!("Loop Detected: {}", evaluation.loop_detected);
```

### 4. Monitor Batch Results

```rust
let batch_eval = evaluate_agent_batch(&trajectories, &params)?;

println!("Success Rate: {:.1}%", batch_eval.success_rate * 100.0);
println!("Avg Steps: {:.1}", batch_eval.avg_steps);
println!("Avg Tool Success: {:.1}%", batch_eval.avg_tool_success_rate * 100.0);
println!("Loops Detected: {:.1}%", batch_eval.loop_detection_rate * 100.0);
```

## Configuration Reference

### Trajectory Metrics

#### `max_steps` (u32)

Maximum number of steps agent should take before failing.

- **Purpose**: Catch agents stuck in loops or over-thinking
- **Typical Range**: 5-20 steps depending on task complexity
- **Default**: 10
- **Simple Task**: 5 (should answer immediately)
- **Complex Task**: 15 (multi-step reasoning)
- **Research Task**: 20 (may need multiple searches)

```yaml
trajectory_metrics:
  max_steps: 10  # Fail if agent takes > 10 steps
```

#### `tool_success_threshold` (0.0-1.0)

Minimum acceptable tool call success rate.

- **Purpose**: Track API/tool reliability issues
- **Typical Range**: 0.90-0.99
- **Default**: 0.95
- **Strict**: 0.99 (require near-perfect API reliability)
- **Tolerant**: 0.85 (accept occasional failures)
- **Formula**: `successful_calls / total_calls >= threshold`

```yaml
trajectory_metrics:
  tool_success_threshold: 0.95  # Require 95% of tools to succeed
```

#### `max_repeated_actions` (u32)

Maximum times the same action can repeat before flagging as loop.

- **Purpose**: Detect agents retrying same action endlessly
- **Typical Range**: 1-3
- **Default**: 2
- **Strict**: 1 (no repetition allowed)
- **Tolerant**: 3 (allow retry patterns)

```yaml
trajectory_metrics:
  max_repeated_actions: 2  # Flag if "search" action repeated 3+ times
```

#### `success_threshold` (0.0-1.0)

Minimum acceptable overall success rate across batches.

- **Purpose**: Track task completion rates
- **Typical Range**: 0.80-0.95
- **Default**: 0.90
- **Strict**: 0.95 (require 95% of runs to succeed)
- **Tolerant**: 0.80 (accept 20% failure rate)

```yaml
trajectory_metrics:
  success_threshold: 0.90  # Require 90% of agent runs to succeed
```

### Loop Detection

#### `loop_detection_enabled` (boolean)

Enable automatic loop detection.

- **Purpose**: Flag agents stuck in repeating patterns
- **Default**: true
- **Detection Logic**: Count action occurrences, flag if any action exceeds `max_repeated_actions`
- **Examples of Loops**:
  - Search → Search → Search (no progress)
  - Parse → Parse → Parse (stuck on same data)
  - Think → Think → Think → Think (overthinking)

```yaml
agent_slo:
  loop_detection_enabled: true
```

## Evaluation Criteria

An agent run **passes** SLO when **ALL** criteria are met:

```
PASS = 
  (steps_taken <= max_steps) AND
  (tool_success_rate >= tool_success_threshold) AND
  (!loop_detected OR !loop_detection_enabled) AND
  (final_status == Success)
```

### Example 1: Simple Search Agent ✓

```
Trajectory:
  Step 1: Search for "AI trends 2026" (tool_call, success)
  Step 2: Return answer (response, success)

Evaluation:
  Max steps: 2 <= 10 ✓
  Tool success: 1/1 = 100% >= 95% ✓
  Loop detected: false ✓
  Final status: success ✓
  
Result: PASS ✓
```

### Example 2: Agent in Loop ✗

```
Trajectory:
  Step 1: Search (tool_call, success)
  Step 2: Search (tool_call, success)  <- same action
  Step 3: Search (tool_call, success)  <- same action again (loop!)
  Step 4: Search (tool_call, success)

Evaluation:
  Max steps: 4 <= 10 ✓
  Tool success: 4/4 = 100% >= 95% ✓
  Loop detected: true (search repeated 4x > max 2) ✗
  Final status: loop_detected ✗
  
Result: FAIL ✗
```

### Example 3: Unreliable Tools ✗

```
Trajectory:
  Step 1: API call (tool_call, success)
  Step 2: API call (tool_call, FAILURE)
  Step 3: API call (tool_call, FAILURE)
  Step 4: API call (tool_call, FAILURE)
  Step 5: Response (response, success)

Evaluation:
  Max steps: 5 <= 10 ✓
  Tool success: 1/4 = 25% >= 95% ✗
  Loop detected: false ✓
  Final status: success ✓
  
Result: FAIL ✗
```

## Real-World Scenarios

### Scenario 1: Research Agent

**Goal**: Find answer to user question in ≤ 5 steps

```yaml
agent_slo:
  enabled: true
  trajectory_metrics:
    max_steps: 5          # Keep research focused
    tool_success_threshold: 0.95  # Most searches should work
    max_repeated_actions: 1       # No repeated searches
    success_threshold: 0.90       # Answer 90% of questions
```

**Success Pattern**:
- Step 1: Search for topic
- Step 2: Read summary
- Step 3: Answer question

### Scenario 2: Coding Assistant

**Goal**: Generate working code with ≤ 3 test iterations

```yaml
agent_slo:
  enabled: true
  trajectory_metrics:
    max_steps: 10
    tool_success_threshold: 0.95  # Tests should pass
    max_repeated_actions: 3       # Allow test-fix cycles
    success_threshold: 0.85       # 85% success (hard problem)
```

**Success Pattern**:
- Step 1-3: Write code
- Step 4: Run tests (succeeds)
- Step 5: Return code

### Scenario 3: Customer Support Bot

**Goal**: Resolve ticket in ≤ 10 steps, no escalation loops

```yaml
agent_slo:
  enabled: true
  trajectory_metrics:
    max_steps: 10
    tool_success_threshold: 0.90  # Some lookups may fail
    max_repeated_actions: 1       # No circular reasoning
    success_threshold: 0.95       # Resolve 95% of tickets
```

**Success Pattern**:
- Step 1: Analyze issue
- Step 2: Fetch customer data
- Step 3: Lookup KB article
- Step 4-5: Craft response
- Step 6: Send response

### Scenario 4: Data Analysis Agent

**Goal**: Complete analysis in ≤ 15 steps with high tool reliability

```yaml
agent_slo:
  enabled: true
  trajectory_metrics:
    max_steps: 15
    tool_success_threshold: 0.98  # Database/API must be reliable
    max_repeated_actions: 2       # Allow query refinement
    success_threshold: 0.92       # Complete 92% of analyses
```

## Monitoring and Alerting

### Key Metrics

Track these metrics in your monitoring system:

```python
# Per-run metrics
steps_taken: int                  # Number of steps
tool_success_rate: float (0-1)   # Fraction of successful tool calls
loop_detected: boolean            # Was a loop detected?
success: boolean                  # Did final status = success?
pass: boolean                     # Does it pass SLO?

# Batch metrics (over last 1h/1d/1w)
success_rate: float (0-1)         # Fraction of runs that passed SLO
avg_steps: float                  # Average steps per run
avg_tool_success_rate: float      # Average tool success rate
loop_detection_rate: float        # Fraction of runs with loops
```

### Alert Rules

```yaml
alerts:
  - name: agent_success_rate_low
    condition: success_rate < 0.90
    duration: 5m
    severity: warning

  - name: high_loop_detection_rate
    condition: loop_detection_rate > 0.10
    duration: 5m
    severity: warning

  - name: tool_success_degradation
    condition: avg_tool_success_rate < 0.95
    duration: 5m
    severity: critical

  - name: agent_step_explosion
    condition: avg_steps > max_steps * 0.9
    duration: 5m
    severity: warning
```

## Integration Examples

### LangChain Integration

```python
from langchain.agents import initialize_agent, load_tools
from neuralbudget import AgentTrajectory, TrajectoryStep, FinalStatus, evaluate_agent_slo

# Create custom callback to capture trajectory
class SLOCallback:
    def __init__(self):
        self.steps = []
        self.run_id = "run_123"
    
    def on_tool_start(self, serialized, input_str, **kwargs):
        self.steps.append({
            'step': len(self.steps) + 1,
            'action': 'tool_call',
            'tool_name': serialized['name'],
            'success': True,  # Assume success for now
        })
    
    def on_tool_end(self, output, **kwargs):
        self.steps[-1]['output'] = output
    
    def on_agent_finish(self, finish, **kwargs):
        trajectory = AgentTrajectory(
            run_id=self.run_id,
            timestamp=int(time.time()),
            steps=[TrajectoryStep(**s) for s in self.steps],
            final_status=FinalStatus.Success,
        )
        
        eval = evaluate_agent_slo(trajectory, params)
        print(f"Agent SLO: {'PASS' if eval.pass else 'FAIL'}")
        return eval

# Use with agent
callback = SLOCallback()
agent.run(query, callbacks=[callback])
```

### OpenAI Assistants Integration

```python
import json
import time
from neuralbudget import (
    AgentTrajectory, TrajectoryStep, AgentAction, FinalStatus,
    evaluate_agent_slo, AgentSloParams
)

def monitor_assistant_run(client, thread_id, assistant_id, params: AgentSloParams):
    """Monitor OpenAI Assistant run against SLO."""
    
    run = client.beta.threads.runs.create(
        thread_id=thread_id,
        assistant_id=assistant_id,
    )
    
    steps = []
    step_num = 1
    
    # Poll for completion
    while run.status == "in_progress":
        time.sleep(1)
        run = client.beta.threads.runs.retrieve(thread_id, run.id)
        
        # Capture step data
        if hasattr(run, 'steps'):
            for step in run.steps:
                steps.append(TrajectoryStep(
                    step=step_num,
                    action=AgentAction.ToolCall if step.type == 'tool_calls' else AgentAction.Thought,
                    tool_name=step.tool_calls[0].function.name if step.tool_calls else None,
                    success=True,  # Assume success unless marked as error
                    output=str(step.step_details),
                    duration_ms=None,
                ))
                step_num += 1
        
        if len(steps) > params.max_steps:
            return {'pass': False, 'reason': 'max_steps_exceeded'}
    
    # Evaluate trajectory
    trajectory = AgentTrajectory(
        run_id=run.id,
        timestamp=int(time.time()),
        steps=steps,
        final_status=FinalStatus.Success if run.status == 'completed' else FinalStatus.Failure,
    )
    
    return evaluate_agent_slo(trajectory, params).dict()
```

## Best Practices

### 1. Start Conservative, Relax Over Time

```yaml
# Week 1: Relaxed thresholds to establish baseline
agent_slo:
  trajectory_metrics:
    max_steps: 20
    tool_success_threshold: 0.90
    success_threshold: 0.80

# Week 4: Tighten based on baseline
agent_slo:
  trajectory_metrics:
    max_steps: 10       # Agents average 6 steps
    tool_success_threshold: 0.95  # 98% tool success measured
    success_threshold: 0.92       # 95% success rate measured
```

### 2. Distinguish by Agent Type

```yaml
# Simple agent (single lookup)
simple_agent_slo:
  max_steps: 3
  tool_success_threshold: 0.99

# Complex agent (multi-hop reasoning)
complex_agent_slo:
  max_steps: 15
  tool_success_threshold: 0.90

# Research agent (iterative search)
research_agent_slo:
  max_steps: 20
  tool_success_threshold: 0.95
```

### 3. Monitor Tool Performance Separately

Track not just agent SLO, but individual tool metrics:

```python
tool_metrics = {
    'search_api': {'success_rate': 0.98, 'avg_latency_ms': 450},
    'database': {'success_rate': 0.999, 'avg_latency_ms': 50},
    'summarizer': {'success_rate': 0.95, 'avg_latency_ms': 2000},
}
```

### 4. Use Hybrid Scoring with Quality

Combine agent efficiency with output quality:

```rust
let quality_score = evaluate_llm_quality(&agent_response);  // 0-1
let efficiency_score = if eval.pass { 1.0 } else { 0.0 };
let hybrid = 0.7 * quality_score + 0.3 * efficiency_score;
```

## Troubleshooting

### Problem: High Loop Detection Rate

**Symptom**: `loop_detection_rate > 0.1`

**Causes**:
- Agent confusion on edge cases
- Tool returning same error repeatedly
- Ambiguous instructions

**Solutions**:
1. Increase `max_repeated_actions` if retry behavior is expected
2. Improve tool error messages for agent clarity
3. Add example queries to system prompt
4. Implement exponential backoff in tools instead of immediate retry

### Problem: Tool Success Rate Too Low

**Symptom**: `avg_tool_success_rate < 0.95`

**Causes**:
- API outages or rate limiting
- Malformed requests from agent
- Tool misconfiguration

**Solutions**:
1. Check tool/API status dashboard
2. Add request validation before tool call
3. Implement circuit breaker pattern
4. Reduce concurrent agent requests

### Problem: Agents Exceeding Max Steps

**Symptom**: `steps_taken > max_steps` on 10%+ of runs

**Causes**:
- Task complexity underestimated
- Agent over-thinking
- Insufficient search results

**Solutions**:
1. Increase `max_steps` based on actual distribution
2. Add summary instruction: "Complete in ≤ 5 steps"
3. Implement dynamic step budgets by task type
4. Improve search quality to reduce iterations

## FAQ

**Q: How do I capture trajectory data from my agent?**

A: Implement callback/hook in your agent framework. See LangChain and OpenAI examples above. Key is capturing:
- Each step (action type, tool name, success/failure)
- Final status (success, failure, timeout, loop_detected)

**Q: What if my agent legitimately needs to call the same tool twice?**

A: That's fine! `max_repeated_actions: 2` means flag if the same action repeats more than 2 times. Two search calls pass; three fails.

**Q: Can I have different SLO thresholds by task type?**

A: Yes. Maintain separate `AgentSloParams` for different agent types:
```rust
let research_params = AgentSloParams { max_steps: 20, ... };
let simple_params = AgentSloParams { max_steps: 5, ... };
```

**Q: How does this compare to cost-based SLOs?**

A: Complementary:
- **Cost SLO**: "What is the cost efficiency?" (tokens, cost/task)
- **Agent SLO**: "Did the agent complete reliably?" (steps, tool success, loops)

**Q: Can I use Agent SLO with non-LLM agents?**

A: Yes! Agent SLO measures execution trajectory. Works with:
- LLM agents (Claude, GPT-4, etc.)
- Symbolic agents (rule-based reasoning)
- Hybrid agents (LLM + logic)
- Robotic process automation (RPA)

## See Also

- [Cost-Based SLOs](./cost-slo.md) - Token costs and budget control
- [LLM-as-Judge](./llm-judge-eval.md) - Quality evaluation via reference-free scoring
- [Hallucination Detection](./hallucination-detection-slo.md) - Groundedness checking
- [SLO Best Practices](./DEPLOYMENT_GUIDE.md) - Production deployment guidance
