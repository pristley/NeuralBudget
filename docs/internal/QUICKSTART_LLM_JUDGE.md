# Quick Start: LLM-as-Judge Evaluator

**Goal**: Evaluate GenAI output quality in 5 minutes  
**Complexity**: Low (just config + 1 API key)  
**Cost**: ~$0.0001 per evaluation (with caching: ~$0.00005)

## Step 1: Set Up Cache (2 minutes)

```bash
# Docker (recommended)
docker run -d -p 6379:6379 redis:7-alpine

# Or: native install
brew install redis  # macOS
apt install redis-server  # Ubuntu
```

Verify:
```bash
redis-cli ping
# Response: PONG
```

## Step 2: Create Configuration (2 minutes)

Create `my_slo.yaml`:

```yaml
mode: genai
service: "my-genai-app"

quality_evaluator:
  type: llm_judge
  model: gpt-4-mini
  provider: openai
  
  cache_config:
    redis_url: "redis://localhost:6379"
    ttl_seconds: 86400

  dimensions:
    - name: correctness
      prompt: "Is this response correct? {response}\nScore 1-5."
      weight: 0.6
      threshold: 3
      cost_per_call_usd: 0.0001
    
    - name: safety
      prompt: "Is this response safe? {response}\nScore 1-5."
      weight: 0.4
      threshold: 4
      cost_per_call_usd: 0.0001
```

## Step 3: Get API Key (1 minute)

### OpenAI
1. Go to https://platform.openai.com/api/keys
2. Click "Create new secret key"
3. Copy the key

### Anthropic
1. Go to https://console.anthropic.com/keys
2. Click "Create Key"
3. Copy the key

## Step 4: Run Evaluation (1 minute)

### Rust
```rust
use neuralbudget::{LlmJudgeEvaluator, LlmProvider, LlmJudgeDimension};

#[tokio::main]
async fn main() {
    let evaluator = LlmJudgeEvaluator::new(
        LlmProvider::OpenAI {
            api_key: "sk-...".to_string(),
            model: "gpt-4-mini".to_string(),
        },
        vec![
            LlmJudgeDimension {
                name: "correctness".to_string(),
                prompt: "Is this correct? {response}\nScore 1-5.".to_string(),
                weight: 0.6,
                threshold: 3.0,
                cost_per_call_usd: 0.0001,
            },
        ],
    )
    .with_redis_cache("redis://localhost:6379", 86400)
    .await
    .expect("Failed to setup cache");

    let result = evaluator.evaluate(
        "What is 2+2?",
        "The answer is 4"
    )
    .await
    .expect("Evaluation failed");

    println!("Score: {:.2}", result.weighted_score);
    println!("Pass: {}", result.pass);
    println!("Cost: ${}", result.total_cost_usd);
    println!("From cache: {}", result.from_cache);
}
```

### Python
```python
from neuralbudget import LlmJudgeEvaluator, LlmProvider

evaluator = LlmJudgeEvaluator.new(
    LlmProvider.OpenAI(
        api_key="sk-...",
        model="gpt-4-mini"
    ),
    [
        {
            "name": "correctness",
            "prompt": "Is this correct? {response}\nScore 1-5.",
            "weight": 0.6,
            "threshold": 3.0,
            "cost_per_call_usd": 0.0001,
        }
    ]
).with_redis_cache("redis://localhost:6379", 86400)

result = await evaluator.evaluate(
    query="What is 2+2?",
    response="The answer is 4"
)

print(f"Score: {result['weighted_score']:.2f}")
print(f"Pass: {result['pass']}")
print(f"Cost: ${result['total_cost_usd']}")
```

## Expected Output

```
Score: 0.88
Pass: True
Cost: $0.0002
From cache: False
```

Second run (cached):
```
Score: 0.88
Pass: True
Cost: $0.0000
From cache: True
```

## Common Issues

### "Redis connection failed"
- Is Redis running? `redis-cli ping`
- Change URL: `redis://localhost:6379` (verify host:port)
- Can remove cache: remove `with_redis_cache()` call

### "Invalid API key"
- Check key format: OpenAI `sk-...`, Anthropic `sk-ant-...`
- Set env var: `export OPENAI_API_KEY=sk-...`
- Check permissions on API key

### "Cannot extract score"
- LLM didn't return 1-5 score
- Check prompt: must ask for 1-5 scale
- Try simpler prompt: "Rate this: {response}. Score 1-5. Answer with just the number."

### "Zero cache hits after first run"
- Queries must be identical (cache key = SHA256(query|response))
- Different queries = different cache key
- If caching identical query/response, should hit on 2nd call

## Next Steps

1. **Integrate into your pipeline**:
   ```python
   # Evaluate after each GenAI response
   for query, response in batch:
       result = await evaluator.evaluate(query, response)
       if not result['pass']:
           log_alert(f"Quality issue: {result['weighted_score']}")
   ```

2. **Monitor costs**:
   ```python
   total_cost = sum(r['total_cost_usd'] for r in results)
   cache_hit_rate = sum(r['from_cache'] for r in results) / len(results)
   print(f"Cost: ${total_cost}, Cache rate: {cache_hit_rate:.0%}")
   ```

3. **Set up alerts**:
   ```python
   if result['weighted_score'] < 0.75:
       send_alert("Quality SLO breached")
   ```

4. **Adjust dimensions**: Fine-tune weights and thresholds based on results

## Documentation

- Full guide: [docs/guides/llm-judge-eval.md](../../docs/guides/llm-judge-eval.md)
- Examples: [examples/slo_genai_llm_judge.yaml](../../examples/slo_genai_llm_judge.yaml)
- Tests: [tests/genai_quality_evaluation_tests.rs](../../tests/genai_quality_evaluation_tests.rs)

## Support

- Configuration issues → Check [docs/guides/llm-judge-eval.md#troubleshooting](../../docs/guides/llm-judge-eval.md#troubleshooting)
- Cost questions → See [docs/guides/llm-judge-eval.md#cost-estimation](../../docs/guides/llm-judge-eval.md#cost-estimation)
- Provider setup → [docs/guides/llm-judge-eval.md#provider-setup](../../docs/guides/llm-judge-eval.md#provider-setup)
