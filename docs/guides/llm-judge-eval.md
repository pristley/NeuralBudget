# LLM-as-Judge: Reference-Free Quality SLOs

## Overview

LLM-as-Judge enables you to evaluate GenAI outputs without reference text. This is critical for production systems where ground truth is expensive, unavailable, or subjective.

**The Problem**: Most LLM evaluation frameworks compare generated outputs against a reference ("ground truth"). But in production:
- You rarely have reference text
- Creating references is expensive and introduces bias
- Quality is subjective (multiple valid answers exist)

**The Solution**: Use a cached LLM evaluator to score outputs on dimensions you define, with deterministic scoring and transparent cost tracking.

## When to Use LLM-as-Judge

| Use Case | Good Fit? | Why |
|----------|-----------|-----|
| Customer support AI | ✅ Yes | Safety, tone, accuracy are subjective and depend on context |
| Code generation | ✅ Yes | Multiple valid implementations exist; testing catches bugs |
| Content generation | ✅ Yes | Tone, style, brand alignment are subjective |
| Summarization | ⚠️ Partial | If you have reference summaries, use reference-based; otherwise LLM-judge |
| Translation | ✅ Yes | Quality depends on context and nuance |
| Classification | ❌ No | Use reference-based eval; outputs are discrete |

## Quick Start

### 1. Set Up Redis Cache (Optional but Recommended)

```bash
# Docker
docker run -d -p 6379:6379 redis:7-alpine

# Or install locally
brew install redis  # macOS
apt-get install redis-server  # Ubuntu
```

### 2. Configure Your SLO

Create `slo.yaml`:

```yaml
mode: genai

quality_evaluator:
  type: llm_judge
  model: gpt-4-mini
  provider: openai
  
  cache_config:
    redis_url: "redis://localhost:6379"
    ttl_seconds: 86400
  
  dimensions:
    - name: "correctness"
      prompt: |
        Is this response correct?
        User query: {query}
        Response: {response}
        Score 1-5.
      weight: 0.5
      threshold: 3
      cost_per_call_usd: 0.0001
    
    - name: "safety"
      prompt: |
        Is this response safe?
        Response: {response}
        Score 1-5 (5=safe).
      weight: 0.5
      threshold: 4
      cost_per_call_usd: 0.0001
```

### 3. Run Evaluation

```python
from neuralbudget import LlmJudgeEvaluator, LlmProvider, LlmJudgeDimension

evaluator = LlmJudgeEvaluator.new(
    LlmProvider.OpenAI {
        api_key: "sk-...",
        model: "gpt-4-mini"
    },
    vec![
        LlmJudgeDimension {
            name: "correctness".into(),
            prompt: "Is this correct? Score 1-5.".into(),
            weight: 0.5,
            threshold: 3.0,
            cost_per_call_usd: 0.0001,
        }
    ]
).with_redis_cache("redis://localhost:6379", 86400)?;

let result = evaluator.evaluate(
    "What is the capital of France?",
    "Paris"
).await?;

println!("Score: {}", result.weighted_score);
println!("Cost: ${}", result.total_cost_usd);
println!("From cache: {}", result.from_cache);
```

## Configuration Guide

### Dimension Specification

Each dimension has:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique identifier (e.g., "correctness", "tone") |
| `prompt` | string | Template with `{query}` and `{response}` placeholders |
| `weight` | float | Relative importance (0.0-1.0); normalized during aggregation |
| `threshold` | float | Minimum score to pass (1.0-5.0) |
| `cost_per_call_usd` | float | Cost per API call |

### Prompt Best Practices

**Good Prompt**:
```
Is this customer support response professional and helpful?

Query: {query}
Response: {response}

Score 1-5 where:
1 = Rude or unhelpful
2 = Cold but functional
3 = Acceptable
4 = Professional and helpful
5 = Exceptional
```

**Bad Prompt** (ambiguous):
```
Rate the response.

Response: {response}

Score 1-5.
```

**Extraction**:
- We extract the first number 1-5 from the response
- Normalize to 0.0-1.0 as: `(score - 1.0) / 4.0`
- Compare against `threshold`

### Weight Aggregation

The final score is calculated as:

$$\text{weighted\_score} = \frac{\sum_{i=1}^{n} \text{normalized\_score}_i \times \text{weight}_i}{\sum_{i=1}^{n} \text{weight}_i}$$

**Example** with 3 dimensions:
- Correctness: 4/5 → 0.75, weight=0.5 → 0.375
- Safety: 5/5 → 1.0, weight=0.3 → 0.3
- Tone: 3/5 → 0.5, weight=0.2 → 0.1
- **Total**: (0.375 + 0.3 + 0.1) / 1.0 = 0.775

## Provider Setup

### OpenAI

```rust
let evaluator = LlmJudgeEvaluator::new(
    LlmProvider::OpenAI {
        api_key: std::env::var("OPENAI_API_KEY")?,
        model: "gpt-4-mini".to_string(),
    },
    dimensions,
);
```

**Costs** (as of 2024):
- `gpt-4-mini`: $0.00015 / 1K input tokens
- `gpt-4-turbo`: $0.01 / 1K input tokens
- `gpt-3.5-turbo`: $0.0005 / 1K input tokens

### Anthropic

```rust
let evaluator = LlmJudgeEvaluator::new(
    LlmProvider::Anthropic {
        api_key: std::env::var("ANTHROPIC_API_KEY")?,
        model: "claude-3-haiku".to_string(),
    },
    dimensions,
);
```

**Costs** (as of 2024):
- `claude-3-haiku`: $0.00025 / 1K input tokens
- `claude-3-sonnet`: $0.003 / 1K input tokens

### Local Models (via LM Studio, Ollama)

```rust
let evaluator = LlmJudgeEvaluator::new(
    LlmProvider::Local {
        base_url: "http://localhost:11434".to_string(),
        model: "llama2".to_string(),
    },
    dimensions,
);
```

**Setup**:
```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Download model
ollama pull llama2

# Start server (default: http://localhost:11434)
ollama serve
```

## Cost Estimation

### Per-Query Costs

| Dimension | Est. Tokens | OpenAI (mini) | Anthropic (haiku) |
|-----------|-------------|---------------|-------------------|
| Single 50-char eval | 80 | $0.000012 | $0.00002 |
| 4 dimensions | 320 | $0.000048 | $0.00008 |
| 4 dims + cache hit | 0 | $0 | $0 |

### Monthly Budget (10,000 queries/month)

With **4 dimensions** and **95% cache hit rate**:

| Provider | Uncached Cost | Monthly Cost (5% uncached) |
|----------|---------------|---------------------------|
| OpenAI GPT-4 Mini | $0.00048 | $2.40 |
| Anthropic Haiku | $0.00008 | $0.40 |
| Local (Ollama) | $0 | $0 |

**With cache**: 95% of queries reuse cached scores from yesterday
- 10,000 queries/day
- 500 fresh queries (5%) × 4 calls/day = 2,000 fresh evaluations/day
- 2,000 × 30 days = 60,000 new evaluations/month
- 60,000 × $0.0001 (OpenAI) = **$6/month**

## Cache Configuration

### Redis Setup

```yaml
cache_config:
  redis_url: "redis://localhost:6379"
  ttl_seconds: 86400  # 24 hours
```

### TTL Recommendations

| TTL | Use Case | Rationale |
|-----|----------|-----------|
| 1 hour | Real-time updates required | Short cache for frequently-asked questions |
| 24 hours | Daily batch evaluations | Standard cache; most production systems |
| 7 days | Offline analysis | Long cache for stable content |
| 30 days | Archived data | Cache historical evaluations |

### Cache Key Format

Keys are SHA256 hashes of `query|response`:

```
llm_judge:a1b2c3d4e5f6...
```

### Sizing Recommendations

**Formula**: Entries = (queries/day × TTL_days × 1.5)

| Queries/Day | TTL | Est. Entries | Size @1KB ea |
|------------|-----|--------------|-------------|
| 1,000 | 1d | 1,500 | 1.5 MB |
| 10,000 | 1d | 15,000 | 15 MB |
| 10,000 | 7d | 105,000 | 105 MB |
| 100,000 | 7d | 1,050,000 | 1 GB |

**Redis Configuration**:
```redis
# ~/.redis/redis.conf
maxmemory 256mb
maxmemory-policy allkeys-lru  # Evict oldest on overflow
```

## Comparison: LLM-Judge vs Reference-Based

| Aspect | LLM-Judge | Reference-Based |
|--------|-----------|-----------------|
| Setup | ~5 min (1 config + API key) | ~1 hour (data collection + embedding model) |
| Cost | ~$0.0001-0.001 per eval | ~$0 (local model) or $0.00001 (SaaS) |
| When ready | Immediately | After collecting reference dataset |
| Ground truth | None needed | Requires reference text |
| Scalability | Unlimited (cloud LLM) | Model hardware-limited |
| Latency | ~1-2 seconds | ~100-500 ms (with caching) |
| For subjective tasks | ✅ Better (LLM as judge) | ⚠️ Harder (what makes a good reference?) |
| For objective tasks | ⚠️ Overkill | ✅ Better |

**Recommendation**: Use LLM-Judge for:
- Subjective quality (tone, professionalism, safety)
- Initial rollouts (no reference dataset yet)
- Cost-sensitive systems (with high cache hit rates)

Use reference-based for:
- Objective tasks (classification, parsing)
- Streaming/real-time (lower latency needed)
- Offline batch analysis

## Monitoring & Alerts

### Set Up Budget Alerts

```yaml
alerts:
  - name: "cost_spike"
    condition: "daily_cost > $10"
    action: "slack"
  
  - name: "quality_drop"
    condition: "weighted_score < 0.80"
    action: "pagerduty"
  
  - name: "cache_miss_rate"
    condition: "cache_miss_ratio > 0.20"
    action: "log"
```

### Key Metrics to Monitor

1. **Cache Hit Rate**: Percentage of evaluations from cache
   - Target: >90% for stable systems
   - Alert: <70% cache hit rate

2. **Cost Per Evaluation**: Rolling average of costs
   - Track weekly trends
   - Alert: >50% cost increase

3. **Quality Score Distribution**: P50, P90, P99 of weighted_score
   - Alert: P50 < 0.75

4. **Dimension Failures**: Which dimensions are most common failures?
   - Action: Tighten threshold if dimension always passes
   - Action: Relax threshold if dimension always fails

## Troubleshooting

### Issue: Scores vary wildly despite identical input

**Cause**: LLM stochasticity (different output each call)

**Solution**: Cache results and/or lower temperature:
```rust
// Implemented in genai_evaluator.rs with temperature=0.7
// For more deterministic: request temperature=0 in prompt
```

### Issue: Cache hit rate is <50%

**Cause**: Queries are too diverse (unique every time)

**Solution**:
1. Increase TTL to capture day-over-day patterns
2. Hash query+response to exact match similar requests
3. Monitor which queries cache miss most

### Issue: Cost is higher than expected

**Cause**: 
- Too many low-cache-hit queries
- Prompt is very long (more tokens)
- Using expensive model (GPT-4 vs GPT-4-mini)

**Solution**:
1. Review most common uncached queries
2. Shorten prompts (remove explanations)
3. Switch to cheaper model for testing

### Issue: LLM refuses to score (safety violations)

**Cause**: LLM thinks query or response violates policy

**Solution**:
1. Rephrase prompt to be more neutral
2. Use local model with less restrictive policies
3. Add "For testing purposes" prefix

## Advanced: Custom Extractors

By default, we extract the first digit 1-5 from LLM response. For custom scoring:

```rust
// TODO: Implement custom score extractors
pub trait ScoreExtractor: Send + Sync {
    fn extract(&self, response: &str) -> Result<f64>;
}

impl ScoreExtractor for RangeExtractor {
    fn extract(&self, response: &str) -> Result<f64> {
        // Custom extraction logic
    }
}
```

## Examples

See full examples in `examples/`:
- `slo_genai_llm_judge.yaml` - Complete YAML configuration
- `slo_genai_quality_eval.json` - JSON equivalent
- `python_genai_quality_tests.py` - Python integration tests

## FAQ

**Q: Can I use LLM-Judge with local models?**
A: Yes! Ollama, LM Studio, vLLM all support the local endpoint pattern.

**Q: What happens if Redis is down?**
A: Cache misses degrade gracefully; all evaluations still work, just slower and costlier.

**Q: Can I combine LLM-Judge with other SLO modes (HTTP, ML)?**
A: Yes, via composite SLOs. See docs/guides/composite-dag-slo.md.

**Q: How do I migrate from manual review to LLM-Judge?**
A: 1. Shadow the LLM for 1 week
   2. Compare LLM scores vs manual scores
   3. Adjust dimensions/thresholds
   4. Full rollout

**Q: Is the LLM-Judge reproducible?**
A: Yes, with temperature=0 (deterministic). With temperature>0, results vary.

## References

- [OpenAI API Docs](https://platform.openai.com/docs)
- [Anthropic API Docs](https://docs.anthropic.com)
- [Ollama](https://ollama.ai)
- [Redis Cache Patterns](https://redis.io/resources/patterns)
