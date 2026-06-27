# TTFT SLO: Time to First Token for Streaming GenAI

For streaming LLM outputs, **time until first token matters more than total response time**. Users perceive responsiveness by when the first token appears on screen, not when generation completes 30 seconds later.

**TTFT SLO** tracks two distinct metrics:
- **TTFT (Time to First Token)**: Latency until first token arrives
- **Inter-Token Latency**: Average time between successive tokens (streaming smoothness)

## Why TTFT Matters

### The User Experience Problem

```
Traditional SLO (P99 latency):
┌─ User submits query (t=0)
├─ Model thinking/processing (t=300ms)
├─ First token generated (t=300ms) ← User's response begins!
├─ Streaming tokens (t=300-30000ms)
└─ Response complete (t=30000ms) ← Measured here

P99 latency SLO: 30,000ms ✓ "Good!"
User experience: Blank screen for 300ms ✗ Frustrating!
```

With TTFT SLO:
```
TTFT requirement: P99 < 500ms
Actual P99 TTFT: 450ms ✓ Fast perceived response!
```

### Real Numbers

| Use Case | TTFT Impact | Inter-Token Impact |
|----------|------------|-------------------|
| **Chat/Q&A** | **CRITICAL** (user waits watching cursor) | Critical (choppy streaming looks broken) |
| **Code Generation** | Important (users watch code appear) | Important (steady typing effect) |
| **Summarization** | Moderate (lengthy output, some wait acceptable) | Important (preview while reading) |
| **Batch Processing** | Not relevant (no streaming) | Not applicable |

### Why Not Just Reduce Total Latency?

1. **Misaligned incentive**: Total latency = thinking_time + (tokens × token_latency)
   - Optimizing total latency penalizes verbose responses
   - User prefers quick start + steady stream over silence + speed burst

2. **Different bottlenecks**:
   - TTFT: Model loading, prompt processing, KV cache preparation
   - Inter-token: Token generation speed, decoding efficiency

3. **SLO achievability**: Reducing P99 total latency from 30s to 25s is hard; reducing P99 TTFT from 600ms to 400ms is achievable

## Quick Start (5 minutes)

### 1. Define Your TTFT SLO

```yaml
mode: genai

ttft_slo:
  enabled: true
  
  # First token deadline
  ttft_threshold_ms: 500      # Users see response within 500ms
  ttft_percentile: 0.99       # P99 requirement
  
  # Streaming smoothness
  inter_token_latency_threshold_ms: 50   # 50ms between tokens
  inter_token_percentile: 0.95            # P95 requirement
```

### 2. Collect Streaming Metrics

Instrument your LLM integration to track:

```rust
// At request start
let start = Instant::now();

// When first token received
let ttft = start.elapsed().as_millis() as f64;

// Track inter-token latencies
let mut inter_token_latencies = Vec::new();
let mut prev_token_time = start;

for token in stream {
    let token_time = Instant::now();
    let inter_token_ms = token_time.duration_since(prev_token_time).as_millis() as f64;
    inter_token_latencies.push(inter_token_ms);
    prev_token_time = token_time;
}

// Calculate inter-token latency (average or P95)
let avg_inter_token = inter_token_latencies.iter().sum::<f64>() / inter_token_latencies.len() as f64;

// Create sample
let sample = GenaiStreamSample {
    request_id,
    timestamp,
    time_to_first_token_ms: ttft,
    inter_token_latency_ms: avg_inter_token,
    total_tokens: token_count,
    total_response_time_ms: total_time,
    model: "gpt-4".to_string(),
    inter_token_latencies: Some(inter_token_latencies),
};
```

### 3. Evaluate Against SLO

```rust
use neuralbudget::genai_slo::{evaluate_ttft_slo, TtftSloParams};

let params = TtftSloParams {
    ttft_threshold_ms: 500.0,
    ttft_percentile: 0.99,
    inter_token_latency_threshold_ms: 50.0,
    inter_token_percentile: 0.95,
};

let eval = evaluate_ttft_slo(&sample, &params)?;

if eval.pass {
    println!("✓ TTFT SLO passed");
} else {
    if !eval.ttft_pass {
        println!("✗ TTFT {:.0}ms > {:.0}ms threshold", eval.ttft_ms, params.ttft_threshold_ms);
    }
    if !eval.inter_token_pass {
        println!("✗ Inter-token {:.0}ms > {:.0}ms threshold", eval.inter_token_latency_ms, params.inter_token_latency_threshold_ms);
    }
}
```

### 4. Monitor Batch Metrics

```rust
let batch_eval = evaluate_ttft_batch(&samples, &params)?;

println!("TTFT SLO Pass Rate: {:.1}%", batch_eval.ttft_pass_rate * 100.0);
println!("P99 TTFT: {:.0}ms", batch_eval.ttft_p99_ms);
println!("P95 Inter-token: {:.0}ms", batch_eval.inter_token_p95_ms);
println!("Throughput: {:.1} tokens/sec", batch_eval.avg_tokens_per_second);
```

## Configuration Reference

### `ttft_threshold_ms` (milliseconds)

Maximum acceptable TTFT (time to first token).

| Use Case | Recommended | Rationale |
|----------|------------|-----------|
| **Chat (conversational)** | 300-500ms | User watching cursor constantly |
| **Code Generation** | 500-800ms | Complex model thinking acceptable |
| **Search/Summarization** | 400-700ms | Depends on query complexity |
| **Mobile** | 200-400ms | Network latency adds up |
| **Enterprise** | 500-1000ms | Batch processing acceptable |

**Psychology**: 
- < 100ms: Feels instant
- 100-300ms: Responsive (user notices start delay slightly)
- 300-500ms: Acceptable (user notices delay but not frustrating)
- 500-1000ms: Tolerably slow (user perceives wait)
- \> 1000ms: Slow (user thinks something broke)

### `ttft_percentile` (0.0-1.0)

Percentile threshold for TTFT requirement.

- **0.99 (P99)**: 99% of requests must meet TTFT - strict, SaaS standard
- **0.95 (P95)**: 95% of requests must meet TTFT - balanced
- **0.90 (P90)**: 90% of requests must meet TTFT - tolerant, internal tools

```yaml
# Typical configurations
ttft_percentile: 0.99  # "We guarantee fast TTFT for 99% of users"
ttft_percentile: 0.95  # "Typical user gets fast TTFT"
ttft_percentile: 0.90  # "Most users get fast TTFT"
```

### `inter_token_latency_threshold_ms` (milliseconds)

Average time between consecutive tokens in the stream.

| Model | Typical Inter-Token | Threshold |
|-------|-------------------|-----------|
| **GPT-4 (small)** | 20-40ms | 50ms |
| **GPT-4 (large)** | 30-60ms | 80ms |
| **Claude (Haiku)** | 15-30ms | 40ms |
| **Claude (Opus)** | 40-80ms | 100ms |
| **Local (7B)** | 50-100ms | 150ms |

**Why it matters**:
- < 30ms: Feels like natural typing (imperceptible)
- 30-50ms: Smooth streaming (good user experience)
- 50-100ms: Noticeable but acceptable (brief pauses between tokens)
- \> 100ms: Choppy (feels broken or stuck)

### `inter_token_percentile` (0.0-1.0)

Percentile threshold for inter-token latency.

- **0.99 (P99)**: Catch tail latencies (rare slowdowns)
- **0.95 (P95)**: Standard for inter-token (5% occasional slowness acceptable)
- **0.90 (P90)**: Lenient (10% of tokens can be slow)

### Example Configurations

#### Chat Assistant (Responsive)
```yaml
ttft_slo:
  ttft_threshold_ms: 300          # User expects immediate response
  ttft_percentile: 0.99           # P99 strict
  inter_token_latency_threshold_ms: 40   # Smooth typing
  inter_token_percentile: 0.95
```

#### Code Generation (Tolerant)
```yaml
ttft_slo:
  ttft_threshold_ms: 700          # Complex generation takes time
  ttft_percentile: 0.95           # P95 acceptable
  inter_token_latency_threshold_ms: 60   # Acceptable pauses
  inter_token_percentile: 0.90
```

#### Mobile (Network-Limited)
```yaml
ttft_slo:
  ttft_threshold_ms: 600          # Network latency budget
  ttft_percentile: 0.99           # Still fast for 99%
  inter_token_latency_threshold_ms: 80   # Network jitter
  inter_token_percentile: 0.95
```

## Understanding the Metrics

### TTFT Utilization

```
TTFT Utilization = (Actual TTFT / Threshold) × 100%

90% utilization (450ms / 500ms):
- Good: Safely under threshold
- Warning: Little headroom for anomalies

105% utilization (525ms / 500ms):
- Bad: Exceeds threshold
- Alert: SLO breach
```

### Inter-Token Utilization

```
Inter-Token Utilization = (Actual / Threshold) × 100%

80% utilization (40ms / 50ms):
- Good: Smooth streaming
- Healthy: Tokens flowing steadily

110% utilization (55ms / 50ms):
- Bad: Choppy streaming
- Alert: Degraded throughput
```

### TTFT Fraction of Total

```
TTFT Fraction = TTFT / Total Response Time

5% (500ms TTFT / 10s total):
- Typical: User sees first token immediately
- Good: Rest of response streams naturally

50% (500ms TTFT / 1s total):
- Problem: Half the time spent waiting
- Issue: Something wrong with generation

Healthy range: 1-20% (TTFT much smaller than total)
```

### Throughput (Tokens/Second)

```
TPS = (Total Tokens / Total Time in ms) × 1000

100 TPS (1000 tokens in 10 seconds):
- Good: 10ms per token average
- ~P95 inter-token: 40ms acceptable

20 TPS (500 tokens in 25 seconds):
- Moderate: 50ms per token average
- Acceptable for complex reasoning

1 TPS (100 tokens in 100 seconds):
- Bad: Model completely stalled
- Alert: Severe performance issue
```

## Real-World Examples

### Example 1: Chat Assistant Response

```
Configuration:
  TTFT threshold: 300ms (P99)
  Inter-token threshold: 40ms (P95)

Actual metrics:
  TTFT: 280ms ✓
  Inter-token: 38ms ✓
  Tokens: 200
  Total time: 10s

Evaluation:
  TTFT pass: true (280 < 300)
  Inter-token pass: true (38 < 40)
  Utilization: TTFT 93%, Inter-token 95%
  Throughput: 20 tokens/sec
  Result: PASS ✓
```

### Example 2: Code Generation Batch

```
100 requests evaluated:

Pass rates:
  TTFT pass: 95 (95% pass rate)
  Inter-token pass: 98 (98% pass rate)
  Overall: 93 (93% both pass)

Percentiles:
  P50 TTFT: 450ms
  P95 TTFT: 620ms
  P99 TTFT: 750ms (just under 800ms threshold) ✓
  
  P50 Inter-token: 35ms
  P95 Inter-token: 55ms (just under 60ms threshold) ✓

Alert status:
  Overall pass rate 93% > 90% threshold ✓
  P99 TTFT 750ms < 800ms threshold ✓
  Result: SLO met
```

### Example 3: Mobile App (Network-Constrained)

```
Configuration:
  TTFT threshold: 600ms (network variability)
  Inter-token threshold: 80ms (cell data)

Actual:
  TTFT: 520ms ✓ (under 600ms)
  Inter-token: 72ms ✓ (under 80ms)
  Throughput: 12 tokens/sec (acceptable for mobile)

Result: PASS
Note: TTFT higher due to network, but acceptable
```

## Typical Thresholds by Industry

### SaaS Products (Paid Users = Expectations High)
```yaml
chat_slo:
  ttft_threshold_ms: 300
  ttft_percentile: 0.99
  inter_token_latency_threshold_ms: 40
  inter_token_percentile: 0.95

# Must ensure 99% of responses start within 300ms
```

### Internal Tools (Cost-Focused)
```yaml
internal_slo:
  ttft_threshold_ms: 800
  ttft_percentile: 0.90
  inter_token_latency_threshold_ms: 100
  inter_token_percentile: 0.80

# Lenient: focus on availability > latency
```

### Mobile/Low-Bandwidth
```yaml
mobile_slo:
  ttft_threshold_ms: 1000      # Network overhead
  ttft_percentile: 0.95
  inter_token_latency_threshold_ms: 150   # Network jitter
  inter_token_percentile: 0.90
```

### Research/Batch
```yaml
batch_slo:
  ttft_threshold_ms: 5000      # Thinking time acceptable
  ttft_percentile: 0.90
  inter_token_latency_threshold_ms: 200   # Throughput over latency
  inter_token_percentile: 0.80
```

## Monitoring & Alerting

### Key Metrics to Track

```
Per-Request:
- ttft_ms: First token latency
- inter_token_latency_ms: Average token interval
- tokens_per_second: Throughput
- pass: SLO pass/fail

Per-Batch (hourly/daily):
- ttft_p99_ms: Worst-case first token
- inter_token_p95_ms: Worst-case streaming smoothness
- pass_rate: Fraction of requests passing SLO
- avg_tokens_per_second: Overall throughput
```

### Alert Rules

```yaml
alerts:
  - name: ttft_p99_high
    condition: ttft_p99_ms > ttft_threshold_ms * 1.1
    duration: 5m
    severity: warning
    message: "P99 TTFT {{ ttft_p99_ms }}ms approaching threshold"

  - name: ttft_slo_breach
    condition: ttft_pass_rate < 0.99
    duration: 5m
    severity: critical
    message: "TTFT SLO breach: {{ ttft_pass_rate }}% pass rate"

  - name: throughput_degradation
    condition: avg_tokens_per_second < baseline * 0.8
    duration: 5m
    severity: warning
    message: "Throughput degraded: {{ avg_tokens_per_second }} TPS vs baseline {{ baseline }}"

  - name: inter_token_spike
    condition: inter_token_p95_ms > inter_token_threshold_ms
    duration: 5m
    severity: warning
    message: "Inter-token latency spiking: P95 {{ inter_token_p95_ms }}ms"
```

### Prometheus Export

```
# TTFT metrics
genai_ttft_ms{model="gpt-4",percentile="p99"} 480
genai_ttft_ms{model="gpt-4",percentile="p95"} 420
genai_ttft_ms{model="gpt-4",percentile="p50"} 350

# Inter-token metrics
genai_inter_token_ms{model="gpt-4",percentile="p95"} 45
genai_inter_token_ms{model="gpt-4",percentile="p50"} 38

# Throughput
genai_throughput_tps{model="gpt-4"} 25.5

# SLO compliance
genai_slo_pass_rate{slo="ttft"} 0.98
genai_slo_pass_rate{slo="inter_token"} 0.96
```

## Integration Examples

### LangChain Streaming Callback

```python
from langchain_core.callbacks import BaseCallbackHandler
import time

class TTFTMetricsCallback(BaseCallbackHandler):
    def on_llm_start(self, serialized, prompts, **kwargs):
        self.start_time = time.time()
        self.first_token_time = None
        self.token_times = []
    
    def on_llm_new_token(self, token, **kwargs):
        current_time = time.time()
        
        if self.first_token_time is None:
            self.first_token_time = current_time
            ttft = (current_time - self.start_time) * 1000
            print(f"TTFT: {ttft:.0f}ms")
        
        if self.token_times:
            inter_token = (current_time - self.token_times[-1]) * 1000
            print(f"Inter-token: {inter_token:.0f}ms")
        
        self.token_times.append(current_time)
    
    def on_llm_end(self, response, **kwargs):
        total_time = (time.time() - self.start_time) * 1000
        tokens = len(self.token_times)
        ttft = (self.first_token_time - self.start_time) * 1000
        
        inter_tokens = []
        for i in range(1, len(self.token_times)):
            inter = (self.token_times[i] - self.token_times[i-1]) * 1000
            inter_tokens.append(inter)
        
        avg_inter = sum(inter_tokens) / len(inter_tokens) if inter_tokens else 0
        
        sample = {
            "ttft": ttft,
            "inter_token": avg_inter,
            "tokens": tokens,
            "total_time": total_time,
        }
        
        # Evaluate against SLO
        evaluate_ttft_slo(sample, params)
```

### OpenAI Streaming

```python
import openai
import time

start = time.time()
ttft = None
inter_tokens = []
prev_time = start

for chunk in openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "..."}],
    stream=True,
):
    current_time = time.time()
    
    if ttft is None:
        ttft = (current_time - start) * 1000
        print(f"TTFT: {ttft:.0f}ms")
    else:
        inter = (current_time - prev_time) * 1000
        inter_tokens.append(inter)
    
    prev_time = current_time
    yield chunk["choices"][0]["delta"].get("content", "")

# Create metrics sample
total_time = (time.time() - start) * 1000
avg_inter = sum(inter_tokens) / len(inter_tokens) if inter_tokens else 0

sample = GenaiStreamSample(
    time_to_first_token_ms=ttft,
    inter_token_latency_ms=avg_inter,
    total_tokens=token_count,
    total_response_time_ms=total_time,
)
```

## Troubleshooting

### Problem: High TTFT but Good Throughput

**Symptom**: P99 TTFT = 800ms, but tokens/sec = 50

**Causes**:
- Model thinking time (planning before generation)
- Input processing overhead (embedding, attention on long context)
- Queue delays (model busy with other requests)

**Solutions**:
1. Profile model latency breakdown (what takes time before first token?)
2. Increase model parallelism/batch size
3. Optimize prompt/system message length
4. Use model distillation for faster thinking

### Problem: Low TTFT but Chunky Streaming

**Symptom**: TTFT = 200ms ✓, but inter-token = 100ms ✗

**Causes**:
- Token generation bottleneck (slow decoding)
- Network buffering (chunks arriving in bursts)
- Client-side processing (parsing delays)

**Solutions**:
1. Profile token generation latency
2. Reduce batch size (prioritize latency over throughput)
3. Check network jitter
4. Optimize client parsing/rendering

### Problem: SLO Pass Rate Degrading Over Time

**Symptom**: Pass rate was 98% last week, now 90%

**Causes**:
- Model overload (more concurrent requests)
- Context window growing (longer prompts)
- Resource contention (other workloads interfering)
- Model drift (fine-tuning made model slower)

**Solutions**:
1. Check request volume trends
2. Monitor model instance CPU/memory
3. Review recent model changes/deployments
4. Consider scaling up

### Problem: Different Models Have Very Different TTFT

**Symptom**: GPT-4 P99 TTFT = 300ms, Claude P99 TTFT = 600ms

**Cause**: Expected. Different architectures, inference speeds.

**Solutions**:
1. Set model-specific thresholds in SLO
2. Accept higher TTFT for more capable models
3. Use smaller models for latency-critical paths
4. Implement fallback logic (GPT-4 if available, else Claude)

## Best Practices

1. **Start Loose, Tighten Gradually**
   - Week 1: Collect baselines (ttft_threshold_ms: 2000)
   - Week 2: Set 95th percentile as threshold
   - Week 3: Tighten to 90th percentile
   - Week 4: Target 99th percentile

2. **Separate TTFT from Total Latency**
   - Different thresholds: TTFT 300ms, Total 30s
   - Different purposes: TTFT (perceived responsiveness), Total (resource budget)

3. **Account for Network**
   - Mobile clients: +200-500ms network latency
   - Enterprise networks: +50-200ms latency
   - Set thresholds with network in mind

4. **Monitor by Model**
   - Track separate SLOs per model
   - Compare model performance objectively
   - Make informed model selection decisions

5. **Combine with Quality SLOs**
   - TTFT alone doesn't measure quality
   - Combine with hallucination detection, cost budget
   - Ensure fast responses are also correct

## FAQ

**Q: What's the difference between TTFT and response time SLO?**

A: 
- TTFT: When does user see FIRST token? (perceived responsiveness)
- Response time: When is FULL response done? (resource efficiency)
  
Example: Chat response takes 30 seconds total, but first token arrives in 300ms. User perceives fast start even if total is slow.

**Q: Should I use P99 or P95 TTFT?**

A: 
- **P99** (more strict): SaaS, paid products, mission-critical
- **P95** (balanced): Internal tools, non-critical features
- **P90** (lenient): Research, low-priority services

**Q: How do I handle TTFT for batch generation?**

A: 
- Batch generation isn't streaming (no TTFT)
- You care about: total batch time, throughput
- Use different SLO: batch_slo instead of ttft_slo

**Q: Can I use TTFT with non-streaming APIs?**

A:
- If API returns complete response at once: No TTFT concept
- If API is OpenAI-like with streaming: Yes, measure TTFT

**Q: What if my TTFT is <100ms?**

A: Excellent! You have headroom. Consider:
- Increasing concurrency (more requests)
- Adding more processing (longer prompts, more context)
- Using larger models for quality improvements

**Q: Inter-token latency is variable (spikes then smooths). What to use?**

A:
- **Average**: Simple, hides spikes
- **P95**: Catches tail latencies (recommended)
- **Max**: Strictest, may be unrealistic

Use P95 for SLO (avoids penalizing rare spikes).

## See Also

- [Cost-Based SLOs](cost-slo.md) - Token budget tracking
- [LLM-as-Judge](llm-judge-eval.md) - Response quality evaluation
- [Agent SLOs](agent-slo.md) - Agent execution reliability
- [Prometheus Integration](prometheus-scraping-examples.md) - Monitoring
- [Production Deployment](production-deployment.md) - Scale to production
