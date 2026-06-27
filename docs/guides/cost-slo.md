# Cost-Based SLOs: Token Usage and Budget Control

## Problem Statement

You have a reliability budget (99.9% uptime). But what about cost?

**The Issue**: Without cost SLOs, you can optimize for quality without considering spend:
- Every request costs money (input tokens + output tokens)
- Quality improves with larger models (more expensive)
- Budget overruns happen silently until the bill arrives

**Real Example**:
- You switch from GPT-4 Mini ($0.00015/1K input) to GPT-4 Turbo ($0.01/1K input)
- Quality improves by 5%
- Cost increases by 65× ($0.015 → $1.00 per request)
- 10M requests/month: $1.5M → $100M 💰

**NeuralBudget's Solution**: Treat cost as a first-class SLI, alongside quality and availability.

## When to Use Cost SLOs

| Scenario | Use It? | Why |
|----------|---------|-----|
| Cost-sensitive workloads (startups) | ✅ Critical | Control margin impact |
| High-volume inference (millions/day) | ✅ Critical | Small per-unit savings add up |
| Per-request billing model | ✅ Yes | Direct budget visibility |
| Cost vs quality tradeoff | ✅ Yes | Make informed model choices |
| Enterprise cost allocation | ✅ Yes | Charge-back to departments |
| Quality-first (budget unlimited) | ⚠️ Maybe | Skip cost SLO if unconstrained |
| Batch processing | ✅ Yes | Forecast monthly spend |

## Quick Start (5 minutes)

### 1. Choose a Model Pricing

Common models (as of June 2026):

| Model | Input $/1K | Output $/1K | Per-Request (100→200) |
|-------|-----------|------------|----------------------|
| GPT-4 Mini | $0.00015 | $0.0006 | ~$0.00016 |
| GPT-4 Turbo | $0.01 | $0.03 | ~$0.01 |
| Claude 3 Haiku | $0.00025 | $0.00125 | ~$0.00026 |
| Claude 3 Sonnet | $0.003 | $0.015 | ~$0.003 |

### 2. Configure Budget

```yaml
cost_slo:
  enabled: true
  budget:
    input_cost_per_1k: 0.00015    # GPT-4 Mini
    output_cost_per_1k: 0.0006
    max_per_request: 0.015         # 1.5 cents max
  
  cost_threshold: 0.95             # Accept 95% budget usage
  monthly_limit: 10000             # $10k/month cap
```

### 3. Run Evaluation

```python
from neuralbudget import CostSloEvaluator, CostBudget, GenaiCostSample

# Create evaluator with GPT-4 Mini pricing
evaluator = CostSloEvaluator::new(CostBudget::gpt4_mini())

# Evaluate a request
sample = GenaiCostSample::new(1000, 50, 120)  # timestamp, input_tokens, output_tokens
result = evaluator.evaluate_request(&sample).await?

print(f"Cost: ${result.total_cost:.6f}")
print(f"Budget remaining: {(1.0 - result.cost_score) * 100:.1f}%")
print(f"Pass: {result.pass}")
```

### Expected Output

```
Cost: $0.000080
Budget remaining: 99.5%
Pass: True

Per-request breakdown:
  - Input cost: $0.0000075 (50 tokens × $0.00015/1K)
  - Output cost: $0.000072 (120 tokens × $0.0006/1K)
  - Total: $0.0000795
```

## Configuration Reference

### Budget Configuration

```yaml
cost_slo:
  enabled: true
  budget:
    # Token pricing from your provider
    input_cost_per_1k: 0.00015
    output_cost_per_1k: 0.0006
    
    # Maximum allowed per request
    max_per_request: 0.015
  
  # Cost acceptability threshold
  # 0.95 = up to 95% of budget is "OK"
  # 0.99 = very strict, only 99% utilization acceptable
  cost_threshold: 0.95
  
  # Optional: monthly cap
  monthly_limit: 10000  # $10k/month
  
  # Optional: weight in hybrid scoring
  cost_weight: 0.1  # 10% cost, 90% quality
```

### Threshold Guidance

| `cost_threshold` | Behavior | Recommended For |
|---|---|---|
| 0.99 | Accept 99% budget use | Strict cost control |
| 0.95 | Accept 95% budget use | Balanced (default) |
| 0.80 | Accept 80% budget use | Lenient, rarely fail |
| 0.50 | Accept 50% budget use | Extreme cost sensitivity |

**Recommendation**: Start with 0.95, adjust based on cost alerts.

## How Cost SLOs Work

### Step 1: Token Counting

LLM APIs return token counts in response metadata:
```json
{
  "content": "...",
  "usage": {
    "prompt_tokens": 50,        // Input tokens
    "completion_tokens": 120    // Output tokens
  }
}
```

### Step 2: Cost Calculation

```
input_cost = input_tokens × (input_cost_per_1k / 1000)
output_cost = output_tokens × (output_cost_per_1k / 1000)
total_cost = input_cost + output_cost

Example (GPT-4 Mini):
  input_cost = 50 × (0.00015 / 1000) = $0.0000075
  output_cost = 120 × (0.0006 / 1000) = $0.000072
  total_cost = $0.0000795
```

### Step 3: Budget Check

```
within_budget = total_cost <= max_per_request
cost_score = (max_per_request - total_cost) / max_per_request

Example:
  within_budget = $0.000080 <= $0.015 → TRUE
  cost_score = ($0.015 - $0.000080) / $0.015 = 0.9947
```

### Step 4: Pass/Fail

```
pass = within_budget && cost_score >= cost_threshold

Example:
  pass = TRUE && 0.9947 >= 0.95 → TRUE ✓
```

## Integration with Quality SLOs

### Hybrid Scoring

Combine cost and quality:

```yaml
quality_evaluator:
  type: llm_judge
  dimensions:
    - name: helpfulness
      weight: 0.5
    - name: accuracy
      weight: 0.4

cost_slo:
  cost_weight: 0.1  # 10% of final score

# Final score = 0.5*helpfulness + 0.4*accuracy + 0.1*cost
```

### Combined Pass/Fail

Both must pass:

```rust
let quality_pass = quality_score >= quality_threshold;
let cost_pass = cost_score >= cost_threshold;
let overall_pass = quality_pass && cost_pass;
```

**Example**:
- Quality: 0.92 (passes 0.9 threshold) ✓
- Cost: 0.9947 (passes 0.95 threshold) ✓
- Overall: PASS

## Cost Estimation

### Per-Request Costs

For typical GenAI interactions:

| Scenario | Input | Output | Cost (Mini) | Cost (Turbo) |
|----------|-------|--------|-------------|------------|
| Q&A | 100 | 200 | $0.00016 | $0.01 |
| Summarization | 2000 | 500 | $0.00060 | $0.035 |
| Code generation | 3000 | 1500 | $0.00180 | $0.075 |
| Long document analysis | 10000 | 2000 | $0.00315 | $0.130 |

### Monthly Budget Scenarios

**Scenario 1: Startup (1M requests/month)**
```
Average: 100 input, 200 output tokens
GPT-4 Mini: 1M × $0.00016 = $160/month
Budget: $0.015 per request = $15k/month max
Status: ✅ Well within budget
```

**Scenario 2: Scale-up (10M requests/month)**
```
Average: 200 input, 500 output tokens
GPT-4 Turbo: 10M × $0.011 = $110k/month
Budget: $0.05 per request = $500k/month max
Status: ✅ Affordable but visible
```

**Scenario 3: Enterprise (100M requests/month)**
```
Average: 500 input, 1000 output tokens
Claude Sonnet: 100M × $0.0065 = $650k/month
Budget: $0.10 per request = $10M/month max
Status: ⚠️ Starting to impact margin
```

### ROI Analysis: Cost vs Quality

Question: Should we upgrade models?

| Model Pair | Cost Increase | Quality Increase | ROI |
|-----------|---|---|---|
| Mini → Turbo | 65× | 5-10% | ❌ Bad (unless premium feature) |
| Haiku → Sonnet | 12× | 20-30% | ✅ Good (if quality-sensitive) |
| Mini → Haiku | 1.7× | 10-15% | ✅ Fair (balanced) |

**Decision Framework**:
1. Can you afford the cost increase?
2. Does the quality improvement justify it?
3. Can you absorb it in pricing or margin?

## Monitoring & Alerts

### Key Metrics

```yaml
metrics:
  genai_request_cost_usd        # Per-request cost
  genai_input_tokens_total      # Total input tokens
  genai_output_tokens_total     # Total output tokens
  genai_monthly_cost_usd        # Running monthly total
  genai_cost_budget_remaining   # Budget headroom
  genai_cost_utilization_ratio  # % of budget used
```

### Alert Examples

```yaml
alerts:
  - name: cost_spike
    condition: "hourly_cost > 50"
    severity: warning
    action: notify
  
  - name: monthly_budget_exceeded
    condition: "monthly_cost > 10000"
    severity: critical
    action: alert_cfo
  
  - name: per_request_overrun
    condition: "cost_score < 0.5"
    severity: info
    action: log
```

## Troubleshooting

### Issue: Cost per Request Higher Than Expected

**Symptom**: `total_cost > max_per_request`

**Causes**:
1. Model generates more tokens than expected
2. Input context larger than designed (prompt injection, large documents)
3. Pricing changed (provider rate increase)

**Solutions**:
1. Increase `max_per_request` budget
2. Optimize prompt to reduce input tokens
3. Switch to cheaper model
4. Implement prompt compression (summarize context first)

### Issue: Monthly Budget Overrun

**Symptom**: `monthly_cost > monthly_limit`

**Causes**:
1. Request volume higher than forecast
2. Token counts increased (quality feature added)
3. Model selection changed (to more expensive version)

**Solutions**:
1. Raise `monthly_limit`
2. Reduce request rate (throttle, queue)
3. Switch models (Mini vs Turbo)
4. Batch requests (amortize tokens)
5. Implement caching to avoid redundant calls

### Issue: Cost SLO Too Strict

**Symptom**: Many requests fail cost check despite being "reasonable"

**Solutions**:
1. Lower `cost_threshold`: 0.95 → 0.80
2. Increase `max_per_request`
3. Re-evaluate pricing (are you using current rates?)

## Advanced Patterns

### Cost-Optimized Model Selection

Use cheapest model that meets quality threshold:

```rust
let models = vec![
    ("gpt-4-mini", CostBudget::gpt4_mini()),
    ("claude-haiku", CostBudget::claude3_haiku()),
];

for (name, budget) in models {
    let evaluator = CostSloEvaluator::new(budget);
    let result = evaluator.evaluate_request(&sample)?;
    
    if result.pass {
        println!("Use {} (cost: ${:.6})", name, result.total_cost);
        break;
    }
}
```

### Dynamic Budget Adjustment

Adjust budget based on request value:

```yaml
# High-value requests: willing to pay more
high_value_config:
  max_per_request: 0.10  # 10 cents
  monthly_limit: 50000   # $50k

# Low-value requests: minimize cost
low_value_config:
  max_per_request: 0.001  # 0.1 cents
  monthly_limit: 1000     # $1k
```

### Batch Processing for Cost

Process multiple queries in single request:

```
Input: "Answer all of these: Q1, Q2, Q3, Q4, Q5"
Output: "A1\nA2\nA3\nA4\nA5"

Cost savings: 5 queries in 1 request = 4× cheaper
Trade-off: Slightly lower quality per question
```

## Pricing Tables

### OpenAI (June 2026)

| Model | Input | Output | Notes |
|-------|-------|--------|-------|
| GPT-4 Mini | $0.00015 | $0.0006 | Recommended baseline |
| GPT-4 Turbo | $0.01 | $0.03 | High quality, 65× cost |
| GPT-4 Omni | $0.025 | $0.1 | Multimodal, expensive |

### Anthropic (June 2026)

| Model | Input | Output | Notes |
|-------|-------|--------|-------|
| Claude 3 Haiku | $0.00025 | $0.00125 | Fastest, cheapest |
| Claude 3 Sonnet | $0.003 | $0.015 | Balanced |
| Claude 3 Opus | $0.015 | $0.075 | Best quality |

### Open Models (Free)

| Model | Hosting | Input/Output | Setup |
|-------|---------|-------------|-------|
| Llama 2 | Ollama | Free | Local GPU |
| Mistral | vLLM | Free | Cloud GPU rental |
| Phi | Ollama | Free | CPU capable |

## Comparison: Cost SLO Methods

| Method | Cost Tracking | Budget Control | Integration | Recommended |
|--------|---|---|---|---|
| **Cost SLO (NeuralBudget)** | ✅ Native | ✅ Per-request + monthly | ✅ Direct | ✅ Yes |
| **Manual token counting** | ⚠️ Ad-hoc | ❌ None | ❌ External | For small scale |
| **Provider usage dashboard** | ✅ Native | ⚠️ Alerts only | ❌ Separate tool | Supplementary |
| **Cloud cost allocation** | ⚠️ Aggregated | ❌ Coarse-grained | ❌ Separate | For budget tracking |

## FAQ

**Q: How often should I review cost budgets?**
A: Weekly for the first month (validate estimates), then monthly. Review when:
   - Changing models
   - Adding features (longer prompts)
   - Scaling request volume

**Q: What if my model costs change?**
A: Update `input_cost_per_1k` and `output_cost_per_1k` immediately. Use Prometheus to replay historical costs.

**Q: Can I set different budgets for different users/teams?**
A: Yes, create separate SLO configs for each service/team with their budgets.

**Q: How do I forecast monthly cost?**
A: `monthly_forecast = avg_cost_per_request × forecast_requests_per_month`

**Q: What's the difference between `cost_threshold` and `max_per_request`?**
A: `max_per_request` is absolute limit (hard cap).
   `cost_threshold` is relative pass/fail (0.95 = accept up to 95% utilization).

**Q: Can cost and quality be at odds?**
A: Yes. Better models cost more. Use hybrid scoring with weights to balance.

## See Also

- [LLM-as-Judge Evaluator](llm-judge-eval.md) — Quality scoring with cached LLM
- [Hallucination Detection](hallucination-detection-slo.md) — Groundedness checking
- [GenAI Integration Guide](user-guide.md#genai-mode) — Full GenAI SLO setup
- [Prometheus Integration](applying-prometheus-rules.md) — Cost metrics export
