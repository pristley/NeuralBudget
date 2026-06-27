# Unified Composite GenAI SLO

**v0.3 Feature**: Combine all GenAI quality dimensions into a single, weighted evaluation score.

For GenAI systems, no single metric tells the complete story. This guide shows how to combine throughput, responsiveness, quality, grounding, cost, retrieval, and success into one balanced SLO.

## The Problem: Single Metrics Are Incomplete

### Before (Separate SLOs)

```yaml
# Separate evaluations, no correlation
ttft_slo: PASS ✓      # Fast first token
quality_slo: FAIL ✗   # But poor output
cost_slo: FAIL ✗      # And expensive
```

How do you know if your system is **actually working well**? You don't. You have 3 separate verdicts with no unified assessment.

### After (Composite GenAI SLO)

```yaml
# Single unified score, all dimensions considered
composite_score: 0.87 / 1.0 ✓
- Throughput:      0.90 (good)
- TTFT:            0.92 (fast)
- Quality:         0.88 (good)
- Groundedness:    0.96 (trustworthy)
- Cost:            0.85 (acceptable)
- Retrieval:       0.89 (strong)
- Success Rate:    0.99 (reliable)

Final verdict: PASS (all dimensions met thresholds AND composite ≥ 0.85)
```

Now you have one clear answer.

## Core Concept: Weighted Scoring

Each dimension contributes a score (0.0-1.0) multiplied by its weight:

```
Composite Score = 
    (0.90 × throughput_weight=0.15) +  # 0.135
    (0.92 × ttft_weight=0.15) +         # 0.138
    (0.88 × quality_weight=0.30) +      # 0.264
    (0.96 × groundedness_weight=0.15) + # 0.144
    (0.85 × cost_weight=0.10) +         # 0.085
    (0.89 × retrieval_weight=0.10) +    # 0.089
    (0.99 × success_rate_weight=0.05)   # 0.050
    ─────────────────────────────────
    = 0.905 (90.5% health)
```

## The Seven Dimensions

### 1. **Throughput** (TPS - Tokens Per Second)

**What it measures**: How fast are we generating tokens?

**Score calculation**:
```
throughput_score = actual_tps / target_tps

Example: 25 actual TPS / 30 target TPS = 0.83 score
```

**When to weight high**: 
- High-volume services (>1000 req/min)
- Batch processing
- Cost-sensitive scenarios

**When to weight low**:
- Low-volume, latency-critical (e.g., single API user)

### 2. **TTFT** (Time to First Token)

**What it measures**: How long until user sees first token?

**Score calculation**:
```
ttft_score = 1.0 if ttft_ms ≤ threshold_ms, else proportional

Example: 450ms actual / 500ms threshold = 0.9 score
```

**When to weight high**:
- Chat/conversational interfaces (user watching cursor)
- Interactive systems
- Mobile apps

**When to weight low**:
- Batch processing
- Background jobs

### 3. **Quality** (0.0-1.0 semantic score)

**What it measures**: How good are the outputs?

**Score calculation**:
```
quality_score = semantic_similarity_score
  OR = llm_judge_rating / 10.0
  OR = your_metric (0.0-1.0)

Example: LLM judge rates 8.5/10 → 0.85 score
```

**When to weight high**:
- Customer-facing applications
- High-stakes decisions (finance, medical)
- Brand reputation critical

**When to weight low**:
- Exploratory/draft generation
- Internal tools

### 4. **Groundedness** (1.0 - Hallucination Rate)

**What it measures**: How factual are the claims? Are they grounded in source material?

**Score calculation**:
```
groundedness_score = 1.0 - hallucination_rate

Example: 5% hallucination rate → 0.95 score
```

**When to weight high**:
- RAG systems (grounding critical)
- Information-sensitive domains (news, finance)
- Legal/compliance-sensitive
- Medical information

**When to weight low**:
- Creative generation (stories, brainstorms)
- Exploration/ideation

### 5. **Cost** (Budget Remaining)

**What it measures**: How efficiently are we using our token budget?

**Score calculation**:
```
cost_score = tokens_budget_remaining / total_budget

Example: $8 remaining / $10 budget = 0.8 score
```

**When to weight high**:
- Cost-sensitive SaaS (tight margins)
- Enterprise budgets
- Resource-constrained environments
- Price-sensitive customers

**When to weight low**:
- Premium offerings
- Performance-first architecture

### 6. **Retrieval** (Recall@k, MRR for RAG)

**What it measures**: For RAG, did we find the right context documents?

**Score calculation**:
```
retrieval_score = recall@5 (how many relevant docs in top 5)
  OR = MRR (mean reciprocal rank)
  OR = your_metric (0.0-1.0)

Example: 4 out of 5 relevant docs → 0.8 score
```

**When to weight high**:
- RAG systems (retrieval drives quality)
- Knowledge-base QA
- Document-centric workflows

**When to weight low**:
- Non-RAG systems
- Standalone generation

### 7. **Success Rate** (Request Completion)

**What it measures**: What fraction of requests complete successfully?

**Score calculation**:
```
success_rate = successful_requests / total_requests

Example: 495 successful / 500 total → 0.99 score
```

**When to weight high**:
- Critical infrastructure
- SLA-sensitive services
- Availability-first systems

**When to weight low**:
- Highly exploratory systems (failures acceptable)

## Default Balanced Configuration

```yaml
mode: genai

composite_weights:
  throughput_weight: 0.15        # 15%
  ttft_weight: 0.15              # 15%
  quality_weight: 0.30           # 30% (highest)
  groundedness_weight: 0.15      # 15%
  cost_weight: 0.10              # 10%
  retrieval_weight: 0.10         # 10%
  success_rate_weight: 0.05      # 5%
  min_target_score: 0.85         # Must hit 85%

composite_thresholds:
  ttft_min: 0.90                 # TTFT dimension must be ≥90%
  quality_min: 0.85              # Quality dimension must be ≥85%
  groundedness_min: 0.95         # Groundedness must be ≥95%
  success_rate_min: 0.99         # Success must be ≥99%
```

### Pass Criteria

SLO passes when **BOTH**:
1. **All critical dimensions pass**: TTFT ≥ 0.90, Quality ≥ 0.85, Groundedness ≥ 0.95, Success ≥ 0.99
2. **Composite score meets target**: Weighted sum ≥ 0.85

If either fails, the SLO is violated.

## Weight Profiles for Different Use Cases

### Profile 1: Quality-First (Mission-Critical)

For healthcare, finance, legal where correctness > everything:

```yaml
composite_weights:
  throughput_weight: 0.10        # Speed not primary
  ttft_weight: 0.10              # Wait time acceptable
  quality_weight: 0.40           # Quality paramount
  groundedness_weight: 0.25      # Factuality critical
  cost_weight: 0.05              # Cost not primary
  retrieval_weight: 0.05         # Or not applicable
  success_rate_weight: 0.05      # Standard
  min_target_score: 0.90         # Strict 90% target

composite_thresholds:
  ttft_min: 0.80                 # More lenient TTFT
  quality_min: 0.95              # Very high quality
  groundedness_min: 0.98         # Very high factuality
  success_rate_min: 0.99         # Standard
```

**When to use**: Healthcare diagnosis, financial advice, legal documents, claims investigation

**Expected scores**: Quality 0.96+, Groundedness 0.97+

### Profile 2: Cost-Optimized (Price-Sensitive)

For price-sensitive SaaS where efficiency matters:

```yaml
composite_weights:
  throughput_weight: 0.20        # Higher: maximize serving
  ttft_weight: 0.10              # Lower: wait acceptable
  quality_weight: 0.25           # Moderate: acceptable quality
  groundedness_weight: 0.10      # Lower: some hallucinations OK
  cost_weight: 0.25              # Very high: strict cost control
  retrieval_weight: 0.05         # Lower: loose matching OK
  success_rate_weight: 0.05      # Standard
  min_target_score: 0.80         # More lenient 80% target

composite_thresholds:
  ttft_min: 0.75                 # Lenient TTFT
  quality_min: 0.80              # Moderate quality
  groundedness_min: 0.85         # Tolerant of hallucinations
  success_rate_min: 0.95         # Slightly lower success
```

**When to use**: Freemium SaaS, budget-constrained deployments, mass-market chat

**Expected scores**: Cost 0.95+, Throughput 0.92+

### Profile 3: Responsiveness-First (Interactive)

For chat, real-time interfaces where user perception matters:

```yaml
composite_weights:
  throughput_weight: 0.15        # Good streaming
  ttft_weight: 0.30              # High: perceived responsiveness
  quality_weight: 0.25           # Good but not paramount
  groundedness_weight: 0.10      # Lower: some hallucinations OK
  cost_weight: 0.10              # Moderate: reasonable efficiency
  retrieval_weight: 0.05         # Lower: or not applicable
  success_rate_weight: 0.05      # Standard
  min_target_score: 0.85         # Standard target

composite_thresholds:
  ttft_min: 0.95                 # Very fast first token
  quality_min: 0.82              # Lighter quality requirement
  groundedness_min: 0.90         # Moderate grounding
  success_rate_min: 0.99         # Standard
```

**When to use**: Chat interfaces, real-time dashboards, autocomplete, interactive tutorials

**Expected scores**: TTFT 0.98+, Quality 0.85+

### Profile 4: RAG-Optimized (Knowledge-Heavy)

For RAG systems where retrieval drives quality:

```yaml
composite_weights:
  throughput_weight: 0.10        # Moderate: not primary
  ttft_weight: 0.10              # Moderate: acceptable wait
  quality_weight: 0.25           # High: depends on retrieval
  groundedness_weight: 0.20      # High: grounding from retrieval
  cost_weight: 0.10              # Moderate: retrieval expensive
  retrieval_weight: 0.20         # Very high: critical for quality
  success_rate_weight: 0.05      # Standard
  min_target_score: 0.85         # Standard target

composite_thresholds:
  ttft_min: 0.85                 # Moderate: retrieval adds latency
  quality_min: 0.88              # Moderate: depends on retrieval
  groundedness_min: 0.96         # High: grounding essential
  success_rate_min: 0.99         # Standard
```

**When to use**: Document QA, knowledge-base search, context-augmented chat

**Expected scores**: Retrieval 0.95+, Groundedness 0.97+

## Real-World Examples

### Example 1: Customer Chat (Balanced Profile)

```yaml
Settings:
  - Weight: Quality 30%, TTFT 15%, Throughput 15%, Groundedness 15%, Cost 10%, Retrieval 10%, Success 5%
  - Min target: 0.85
  - Thresholds: TTFT 0.90, Quality 0.85, Groundedness 0.95, Success 0.99

Sample Metrics (Hourly):
  - Throughput: 2,500 TPS (vs 2,000 target) = 1.0 score
  - TTFT: P99 = 320ms (vs 300ms target) = 0.94 score
  - Quality: LLM judge = 8.7/10 = 0.87 score
  - Groundedness: 3% hallucination rate = 0.97 score
  - Cost: $50 spent / $75 budget = 0.67 score
  - Retrieval: N/A (non-RAG) = 0.85 default
  - Success: 9,995 successful / 10,000 total = 0.9995 score

Composite: (1.0×0.15 + 0.94×0.15 + 0.87×0.30 + 0.97×0.15 + 0.67×0.10 + 0.85×0.10 + 0.9995×0.05)
         = 0.15 + 0.141 + 0.261 + 0.1455 + 0.067 + 0.085 + 0.050
         = 0.897 ✓

Verdict: PASS
- All dimensions pass ✓ (TTFT 0.94>0.90, Quality 0.87>0.85, Groundedness 0.97>0.95, Success 0.9995>0.99)
- Composite 0.897 > 0.85 ✓
```

### Example 2: Healthcare GenAI (Quality-First Profile)

```yaml
Settings:
  - Weight: Quality 40%, Groundedness 25%, Success 5%, Throughput 10%, TTFT 10%, Cost 5%, Retrieval 5%
  - Min target: 0.90
  - Thresholds: TTFT 0.80, Quality 0.95, Groundedness 0.98, Success 0.99

Sample Metrics (Hourly):
  - Throughput: 1,200 TPS = 0.85 score
  - TTFT: 650ms (acceptable wait) = 0.87 score
  - Quality: Medical expert review = 9.5/10 = 0.95 score
  - Groundedness: 1% hallucination rate = 0.99 score
  - Cost: $200 / $250 = 0.8 score
  - Retrieval: MRR = 0.94 = 0.94 score
  - Success: 2,495 / 2,500 = 0.998 score

Composite: (0.85×0.10 + 0.87×0.10 + 0.95×0.40 + 0.99×0.25 + 0.8×0.05 + 0.94×0.05 + 0.998×0.05)
         = 0.085 + 0.087 + 0.38 + 0.2475 + 0.04 + 0.047 + 0.0499
         = 0.9364 ✓

Verdict: PASS
- All dimensions pass ✓ (TTFT 0.87>0.80, Quality 0.95≥0.95, Groundedness 0.99>0.98, Success 0.998>0.99)
- Composite 0.9364 > 0.90 ✓
```

### Example 3: Cost-Optimized Chat (Price-Sensitive SaaS)

```yaml
Settings:
  - Weight: Cost 25%, Throughput 20%, Quality 25%, TTFT 10%, Groundedness 10%, Retrieval 5%, Success 5%
  - Min target: 0.80
  - Thresholds: TTFT 0.75, Quality 0.80, Groundedness 0.85, Success 0.95

Sample Metrics (Hourly):
  - Throughput: 5,000 TPS / 4,000 target = 1.0 score
  - TTFT: 280ms / 400ms = 0.70 score (but 0.75 threshold = pass)
  - Quality: 8.0/10 = 0.80 score (meets threshold)
  - Groundedness: 8% hallucination rate = 0.92 score
  - Cost: $95 / $100 budget = 0.95 score ✓ (excellent)
  - Retrieval: N/A = 0.85
  - Success: 49,950 / 50,000 = 0.999 score

Composite: (1.0×0.20 + 0.70×0.10 + 0.80×0.25 + 0.92×0.10 + 0.95×0.25 + 0.85×0.05 + 0.999×0.05)
         = 0.20 + 0.07 + 0.20 + 0.092 + 0.2375 + 0.0425 + 0.04995
         = 0.86245 ✓

Verdict: PASS
- All dimensions pass ✓ (even with lower TTFT 0.70<0.75, Cost excellent at 0.95)
- Composite 0.862 > 0.80 ✓
- Cost efficiency demonstrates return on investment
```

## Tuning Your Weights

### Step 1: Understand Your Priorities

Ask your team:
- "What matters most to our users?"
- "What would make them most unhappy?"
- "What are our business constraints?"

### Step 2: Map to Dimensions

| Priority | Primary Dimension | Weight |
|----------|------------------|--------|
| "No hallucinations" | Groundedness | 0.20+ |
| "Fast responses" | TTFT | 0.20+ |
| "Perfect answers" | Quality | 0.40+ |
| "Keep costs down" | Cost | 0.20+ |
| "Always available" | Success Rate | 0.10+ |
| "Find right docs" | Retrieval | 0.20+ |
| "Serve many users" | Throughput | 0.20+ |

### Step 3: Weight Allocation

Total available: 1.0

```python
# Example: Quality-first for healthcare
quality_weight = 0.40         # Primary (40%)
groundedness_weight = 0.25    # Secondary (25%)
success_rate_weight = 0.10    # Tertiary (10%)
retrieval_weight = 0.10       # Tertiary (10%)
ttft_weight = 0.10            # Minor (10%)
throughput_weight = 0.05      # Minor (5%)
cost_weight = 0.05            # Minor (5%)
# Sum = 1.00 ✓
```

### Step 4: Set Dimension Thresholds

For each critical dimension, what's the **minimum acceptable score**?

```yaml
composite_thresholds:
  quality_min: 0.95              # We demand high quality
  groundedness_min: 0.98         # We demand factuality
  success_rate_min: 0.99         # We demand reliability
  ttft_min: 0.80                 # TTFT less critical
```

### Step 5: Validate with Historical Data

Backtest your weights against recent metrics:

```python
# Calculate what your SLO would have been last week
historical_metrics = [
    {"throughput": 0.92, "ttft": 0.91, "quality": 0.88, ...},
    {"throughput": 0.89, "ttft": 0.93, "quality": 0.85, ...},
    {"throughput": 0.95, "ttft": 0.88, "quality": 0.90, ...},
]

weights = {
    "throughput": 0.15,
    "ttft": 0.15,
    "quality": 0.30,
    ...
}

for sample in historical_metrics:
    score = calculate_composite(sample, weights)
    print(f"Would have been: {'PASS' if score >= 0.85 else 'FAIL'}")

# Adjust weights until backtest accuracy is 90%+
```

## Monitoring and Alerting

### Alert Rules

```yaml
alerts:
  - name: CompositeGenAiSLOBreach
    condition: composite_score < min_target_score
    duration: 5m
    severity: critical
    message: "GenAI SLO breach: composite score {{ composite_score }} < {{ min_target }}"

  - name: QualityDimensionLow
    condition: quality_score < 0.85
    duration: 10m
    severity: warning
    message: "Quality dimension degraded: {{ quality_score }}"

  - name: GroundednessDropped
    condition: groundedness_score < 0.95
    duration: 5m
    severity: warning
    message: "Hallucination rate increased: {{ hallucination_rate }}%"

  - name: CostBudgetExceeded
    condition: cost_score < 0.50
    duration: 1m
    severity: critical
    message: "Cost budget critical: {{ tokens_spent }} / {{ tokens_budget }}"
```

### Prometheus Export

```
genai_composite_score{service="chat-api"} 0.897
genai_quality_score{service="chat-api"} 0.87
genai_groundedness_score{service="chat-api"} 0.96
genai_ttft_score{service="chat-api"} 0.94
genai_throughput_score{service="chat-api"} 1.0
genai_cost_score{service="chat-api"} 0.67
genai_retrieval_score{service="chat-api"} 0.85
genai_success_rate_score{service="chat-api"} 0.9995

genai_slo_pass{service="chat-api"} 1
```

## FAQ

**Q: Why have both dimension thresholds AND a composite score requirement?**
> A: Composite score alone could hide problems. A high cost efficiency (weight 0.25) could mask terrible quality (weight 0.30). We require all critical dimensions meet minimums PLUS the composite score hits the target.

**Q: Can I adjust weights dynamically?**
> A: Yes! Adjust per time-of-day, per customer tier, per feature. Use environment variables or config files.

**Q: What if I don't use RAG?**
> A: Set retrieval_weight to 0 (add it to another weight), or set retrieval_score to 0.85 (neutral default).

**Q: How do I get these scores?**
> A: Combine NeuralBudget's existing evaluators:
> - Throughput: TPS = tokens/time
> - TTFT: From `evaluate_ttft_slo()`
> - Quality: From `evaluate_quality()` or LLM judge
> - Groundedness: From `evaluate_hallucination_detection()`
> - Cost: tokens * price / budget
> - Retrieval: Your RAG retrieval metrics
> - Success Rate: success_requests / total

**Q: My composite score is 0.82 (below 0.85) but all dimensions pass. Is that good?**
> A: Not quite. You need BOTH conditions:
> - All critical dimensions pass ✓
> - Composite ≥ min_target (0.82 < 0.85) ✗
> 
> Increase your weakest dimensions or adjust weights toward your strengths.

**Q: Should I use 0.85 or something else for min_target?**
> A: Depends on your risk tolerance:
> - 0.95: "We want excellent" (premium SaaS)
> - 0.90: "We want good" (standard production)
> - 0.85: "We want acceptable" (cost-sensitive)
> - 0.80: "We want viable" (experimental)

## Integration Example (Pseudocode)

```python
from neuralbudget import (
    evaluate_ttft_slo,
    evaluate_quality,
    evaluate_hallucination_detection,
    evaluate_composite_genai_slo,
    CompositeGenAiDimensions,
    CompositeGenAiWeights,
    CompositeGenAiThresholds,
)

# 1. Collect individual dimension scores
ttft_eval = evaluate_ttft_slo(ttft_sample, ttft_params)
ttft_score = 1.0 if ttft_eval.pass else 0.0  # Or normalize to [0,1]

quality_eval = evaluate_quality(output, reference)
quality_score = quality_eval.semantic_similarity  # 0.0-1.0

hallucination_eval = evaluate_hallucination_detection(output, source_docs)
groundedness_score = 1.0 - hallucination_eval.hallucination_rate

cost_score = budget_remaining / total_budget

retrieval_eval = evaluate_retrieval(query, docs, ranked_docs)
retrieval_score = retrieval_eval.recall_at_k

success_rate = successful_requests / total_requests

throughput = tokens_generated / time_sec

# 2. Create dimension scores struct
dimensions = CompositeGenAiDimensions(
    throughput_score=throughput,
    ttft_score=ttft_score,
    quality_score=quality_score,
    groundedness_score=groundedness_score,
    cost_score=cost_score,
    retrieval_score=retrieval_score,
    success_rate=success_rate,
)

# 3. Use your configured weights
weights = CompositeGenAiWeights.load_from_yaml("config.yaml")
thresholds = CompositeGenAiThresholds.load_from_yaml("config.yaml")

# 4. Evaluate
eval = evaluate_composite_genai_slo(dimensions, weights, thresholds)

if eval.pass:
    print("✓ GenAI SLO PASS")
    print(f"  Composite score: {eval.composite_score:.2%}")
else:
    print("✗ GenAI SLO FAIL")
    if not eval.all_dimensions_pass:
        print(f"  Failed dimensions: {eval.dimension_pass_status}")
    if not eval.composite_pass:
        print(f"  Composite {eval.composite_score:.2%} < {weights.min_target_score:.2%}")
```

## See Also

- [TTFT SLO](ttft-slo.md) - First-token latency tracking
- [Quality SLO](llm-judge-eval.md) - LLM-as-Judge evaluation
- [Hallucination Detection](hallucination-detection-slo.md) - Groundedness tracking
- [Cost SLO](cost-slo.md) - Token budget management
- [Agent SLO](agent-slo.md) - Agent reliability
- [Prometheus Integration](prometheus-scraping-examples.md) - Monitoring setup
