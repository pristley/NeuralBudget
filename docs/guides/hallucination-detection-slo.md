# Hallucination Detection: Groundedness-Based Quality SLOs

## Problem Statement

An LLM model sounds confident but makes up facts. You ask: "What are the benefits of exercise?" and it responds with a detailed answer that includes claims like "increases bone density by 40%" — which you can't verify. You need to automatically detect when outputs aren't grounded in reality.

**The Gap**: Existing observability tools can't detect hallucinations. They measure latency and availability, but not whether claims are factually grounded.

**NeuralBudget's Solution**: Automatically extract claims from LLM responses and check if they're supported by retrieved documents (RAG) or web search results — without needing reference answers.

## When to Use Hallucination Detection

| Scenario | Use It? | Why |
|----------|---------|-----|
| Customer-facing Q&A assistant | ✅ Yes | Incorrect info damages trust |
| Code generation | ⚠️ Maybe | Syntax errors caught by linting, logic errors by tests |
| Medical/legal advice | ✅ Critical | False information has serious consequences |
| RAG-based retrieval | ✅ Yes | Easy to check against source documents |
| Real-time chat | ✅ Yes | Can detect hallucinations immediately |
| Offline batch analysis | ✅ Yes | Check historical outputs for accuracy |
| Knowledge graphs | ⚠️ Maybe | If you have ground truth, use it; otherwise LLM-judge |

## Quick Start (10 minutes)

### 1. Set Up Context Documents

For RAG scenario, you already have retrieved documents:

```yaml
mode: genai

context_docs:
  - text: "Regular exercise strengthens the heart and lungs."
    source: "WHO-health-guide.pdf"
  - text: "Physical activity releases endorphins that improve mood."
    source: "mental-health-study.pdf"
  - text: "Resistance training builds muscle mass and bone strength."
    source: "fitness-research.pdf"
```

### 2. Configure Hallucination Detection

Add to your SLO config:

```yaml
quality_evaluator:
  type: llm_judge
  model: gpt-4-mini
  
  # Existing LLM-as-Judge dimensions
  dimensions:
    - name: correctness
      prompt: "Is this response helpful? {response}\nScore 1-5."
      weight: 0.4
      threshold: 3
      cost_per_call_usd: 0.0001

hallucination_detection:
  enabled: true
  extraction_method: rule_based  # or llm_based
  scoring_method: token_overlap  # or embedding_similarity, tfidf, entailment
  groundedness_threshold: 0.5     # Each claim must be 50%+ similar to docs
  min_groundedness_score: 0.75    # At least 75% of claims must be grounded

# Sample data for testing
sample:
  timestamp: 1719518400
  query: "What are the benefits of exercise?"
  response: |
    Exercise provides multiple health benefits. It strengthens your heart and lungs,
    improving cardiovascular health. Physical activity releases endorphins that boost
    mood and mental health. Additionally, resistance training builds muscle strength.
  
  context_docs:
    - text: "Regular exercise strengthens the heart and lungs."
      source: "WHO-health-guide.pdf"
    - text: "Physical activity releases endorphins for mood improvement."
      source: "mental-health-study.pdf"
    - text: "Resistance training builds muscle strength."
      source: "fitness-research.pdf"
```

### 3. Run Evaluation

```python
from neuralbudget import GroundednessEvaluator, ClaimExtractionMethod, SimilarityMethod

evaluator = GroundednessEvaluator.new(
    extraction_method=ClaimExtractionMethod.RuleBased,
    similarity_method=SimilarityMethod.TokenOverlap,
    groundedness_threshold=0.75,
)

result = evaluator.evaluate(
    response="Exercise improves heart health and mood.",
    context_docs=[
        {"text": "Exercise strengthens the heart.", "source": "health.pdf"},
        {"text": "Activity releases endorphins.", "source": "mood.pdf"},
    ]
)

print(f"Groundedness: {result.groundedness_score:.2f}")
print(f"Hallucination rate: {result.hallucination_rate:.2%}")
print(f"Grounded claims: {result.grounded_count}/{len(result.claims)}")
```

### Expected Output

```
Groundedness: 1.00
Hallucination rate: 0.00%
Grounded claims: 2/2

Claims:
  1. "Exercise improves heart health" → 0.95 (grounded, doc: health.pdf)
  2. "Activity releases endorphins" → 0.90 (grounded, doc: mood.pdf)
```

## Configuration Reference

### ClaimExtractionMethod

**Rule-Based** (Recommended for getting started)
```yaml
extraction_method: rule_based
# Splits response by sentence delimiters (. ! ?)
# Filters out short sentences (min 10 chars)
# Fast and no API calls
```

**LLM-Based** (Higher quality for complex claims)
```yaml
extraction_method: llm_based
# Uses LLM to intelligently extract atomic claims
# Better for multi-clause sentences
# Costs extra API calls
```

### SimilarityMethod

| Method | Speed | Quality | Cost | Best For |
|--------|-------|---------|------|----------|
| **token_overlap** | Fast | Good | Free | General purpose, first pass |
| **embedding_similarity** | Medium | Better | Free (local) | Semantic understanding |
| **tfidf** | Fast | Good | Free | Document similarity |
| **entailment** | Slow | Best | High | Logical entailment (future) |

**Recommended**: Start with `token_overlap`, upgrade to `embedding_similarity` if missing valid groundings.

### Thresholds

```yaml
# groundedness_threshold: Individual claim must be X% similar to best doc
# Range: 0.0-1.0
# Default: 0.5
# Too low: accept weak groundings (allow hallucinations)
# Too high: reject valid claims (false positives)
groundedness_threshold: 0.5

# min_groundedness_score: Overall pass/fail threshold
# Range: 0.0-1.0
# Default: 0.75
# Means: at least 75% of claims must be grounded to pass
min_groundedness_score: 0.75
```

## How It Works

### Step 1: Claim Extraction

**Rule-Based Example**:
```
Response: "Exercise improves cardiovascular health and increases muscle strength."

Claims extracted:
1. "Exercise improves cardiovascular health"
2. "increases muscle strength"
```

**LLM-Based Example** (more sophisticated):
```
Response: "Apples contain vitamin C which boosts immunity, but can cause tooth decay."

Claims extracted:
1. "Apples contain vitamin C"
2. "Vitamin C boosts immunity"
3. "Apples can cause tooth decay"
```

### Step 2: Similarity Scoring

For each claim, find the best-matching document:

```
Claim: "Exercise improves cardiovascular health"
Document 1: "Regular exercise strengthens the heart and lungs"
  Similarity: 0.85 ✓ (0.85 > 0.5 threshold) → GROUNDED

Claim: "Exercise increases bone density by 40%"
Document 2: "Resistance training builds muscle"
  Similarity: 0.42 ✗ (0.42 < 0.5 threshold) → HALLUCINATED
```

### Step 3: Aggregation

```
Groundedness score = grounded_claims / total_claims
                   = 1 / 2
                   = 0.50 (50%)

Pass? 0.50 < 0.75 (min_groundedness_score) → FAIL
```

## Similarity Methods Explained

### Token Overlap (Fastest)

```python
# Simple word-based matching
claim = "Exercise improves heart health"
doc = "Exercise strengthens the heart and lungs"

Matching tokens: ["exercise", "heart"] = 2/4 = 0.5 similarity
```

**Pros**: Fast, no ML models, offline  
**Cons**: Misses synonyms (fitness/exercise), word order matters

### Embedding Similarity (Recommended)

```python
# Semantic similarity via embeddings
claim = "Working out is good for fitness"
doc = "Exercise improves cardiovascular health"

Embedding distance: 0.87 (good match semantically)
```

**Pros**: Understands synonyms, semantic meaning  
**Cons**: Requires embedding model, slower

### TF-IDF

```python
# Statistical document similarity
# Rewards rare terms (less common words = more informative)
claim = "Exercise improves cardiovascular health"
doc1 = "Regular exercise strengthens heart"
doc2 = "Many people exercise daily"

TF-IDF(claim, doc1) = 0.8 > TF-IDF(claim, doc2) = 0.3
```

**Pros**: Balanced frequency and uniqueness  
**Cons**: Slower than token overlap

## Integration with LLM-as-Judge

Combine hallucination detection with quality dimensions:

```yaml
quality_evaluator:
  type: llm_judge
  
  dimensions:
    - name: correctness
      prompt: "Is this correct? {response}\nScore 1-5."
      weight: 0.3
      threshold: 3
      cost_per_call_usd: 0.0001
    
    - name: helpfulness
      prompt: "Is this helpful? {response}\nScore 1-5."
      weight: 0.3
      threshold: 3
      cost_per_call_usd: 0.0001

hallucination_detection:
  enabled: true
  scoring_method: token_overlap
  min_groundedness_score: 0.75  # Weight: 0.4 implicitly

# Final score = 0.3*correctness + 0.3*helpfulness + 0.4*groundedness
```

## Cost Estimation

### Per-Query Costs

| Component | Cost |
|-----------|------|
| Claim extraction (rule-based) | $0 |
| Claim extraction (LLM-based) | +$0.0001 |
| Similarity scoring (token overlap) | $0 |
| Similarity scoring (embedding) | $0 (local model) or $0.0005 (API) |
| **Total (rule-based + overlap)** | **$0** |
| **Total (LLM-based + embedding)** | **$0.0006** |

### Monthly Budget (10K queries/day)

| Scenario | Method | Cost |
|----------|--------|------|
| Baseline hallucination detection | Rule-based + token overlap | $0 |
| Enhanced detection | LLM-based + embedding | $180/month |
| High accuracy | All LLM-based + entailment (future) | $500+/month |

**Recommendation**: Start with rule-based + token overlap (free), upgrade if accuracy insufficient.

## Troubleshooting

### Issue: High False Positive Rate (rejecting valid claims)

**Symptom**: Claims that are clearly grounded are marked as hallucinated

**Causes**:
- `groundedness_threshold` too high
- Similarity method too strict
- Claims use different terminology than documents

**Solutions**:
1. Lower `groundedness_threshold`: 0.5 → 0.4
2. Switch to `embedding_similarity` (understands synonyms)
3. Ensure context documents use similar language
4. Use `extraction_method: llm_based` for smarter claim parsing

### Issue: High False Negative Rate (accepting hallucinations)

**Symptom**: Obviously false claims are marked as grounded

**Causes**:
- `groundedness_threshold` too low
- Similarity method too lenient
- Missing relevant context documents

**Solutions**:
1. Raise `groundedness_threshold`: 0.5 → 0.7
2. Add more comprehensive context documents
3. Use `min_groundedness_score: 0.95` (stricter overall)
4. Try `scoring_method: entailment` (checks logical entailment, not just similarity)

### Issue: "Claims extraction produces too few/many claims"

**Symptom**: Only 2 claims extracted from complex 5-sentence response

**Configuration tweaks**:
```yaml
# Adjust in extraction config (if exposed):
min_length: 5        # Shorter minimum sentence length
max_claims: 50       # Increase max claims per response
```

**If using LLM-based extraction**:
```
Add to prompt: "Extract all factual claims, including minor details."
```

## Examples

### Example 1: Healthcare Q&A

**Scenario**: Chatbot answering medical questions

**Response**:
```
"Vitamin C helps prevent colds and improves immune function. 
Taking 1000mg daily is safe and won't cause side effects."
```

**Context docs** (from medical resources):
```
Doc 1: "Vitamin C plays a role in immune function but evidence for cold 
prevention is mixed."
Doc 2: "High doses of vitamin C may cause kidney stones in susceptible people."
```

**Evaluation**:
```
Claim 1: "Vitamin C helps prevent colds"
  Similarity: 0.6 → Grounded (but doc says "mixed")
  Flag: TRUE_POSITIVE (claim is grounded, but qualified in source)

Claim 2: "improves immune function"
  Similarity: 0.85 → Grounded ✓

Claim 3: "Taking 1000mg daily is safe"
  Similarity: 0.2 → Hallucinated ✗ (contradicts doc about side effects)

Claim 4: "won't cause side effects"
  Similarity: 0.1 → Hallucinated ✗

Result: 2/4 grounded = 0.5 groundedness score → FAIL (threshold 0.75)
```

### Example 2: RAG-Based Customer Support

**Query**: "What payment methods do you accept?"

**LLM Response**:
```
"We accept all major credit cards including Visa, Mastercard, and American Express.
We also support PayPal, Apple Pay, and bank transfers. Bitcoin is coming soon."
```

**Retrieved Context** (from knowledge base):
```
Doc: "Accepted payment methods: Visa, Mastercard, American Express, PayPal, Apple Pay"
```

**Evaluation**:
```
Claim 1: "We accept Visa, Mastercard, American Express"
  Similarity: 1.0 → Grounded ✓

Claim 2: "We support PayPal, Apple Pay"
  Similarity: 0.95 → Grounded ✓

Claim 3: "bank transfers"
  Similarity: 0.1 → Hallucinated ✗ (not in knowledge base)

Claim 4: "Bitcoin is coming soon"
  Similarity: 0.0 → Hallucinated ✗ (not in knowledge base)

Result: 2/4 grounded = 0.5 → FAIL
Action: Return "We accept Visa, Mastercard, AmEx, PayPal, and Apple Pay"
```

## Monitoring & Alerts

### Key Metrics to Track

```yaml
alerts:
  - name: hallucination_spike
    condition: "hallucination_rate > 0.25"
    severity: warning
    action: investigate
  
  - name: claim_extraction_failure
    condition: "claims_extracted < 1"
    severity: info
    action: log
  
  - name: groundedness_degradation
    condition: "7d_avg(groundedness) < threshold - 0.1"
    severity: critical
    action: notify_team
```

### Dashboards

Track over time:
- Average groundedness score per query
- Hallucination rate trend (daily/weekly)
- Most common hallucinated claims
- Context document quality (coverage)

## Advanced: Custom Grounding Methods

### Future: Entailment-Based Scoring

```python
# Check if claim is logically entailed by document
model = SentenceTransformer('cross-encoder/nli-deberta-v3-large')
similarity = model.predict(
    [["Exercise improves health.", "Regular exercise strengthens the heart."]]
)[0]
# Returns: entailment=0.95, contradiction=0.01, neutral=0.04
```

### Future: Web Search Grounding

```yaml
hallucination_detection:
  enabled: true
  source: web_search  # Instead of provided_documents
  web_search_provider: google  # or bing, duckduckgo
  top_k: 3
```

## Comparison: Hallucination Detection Methods

| Method | Setup Time | Accuracy | Cost | Recommended |
|--------|-----------|----------|------|-------------|
| **Token overlap + rule-based** | 5 min | Good | Free | ✅ Start here |
| **Embedding similarity** | 10 min | Better | Free (local) | ✅ For synonyms |
| **Entailment model** (future) | 20 min | Best | High | For critical systems |
| **Web search** (future) | 30 min | Mixed | Medium | For internet queries |
| **Manual review** | Slow | Perfect | Expensive | Baseline only |

## FAQ

**Q: Does this catch all hallucinations?**
A: No. It catches claims unsupported by provided documents. It won't catch logical inconsistencies or subtle errors within grounded claims.

**Q: How do I improve accuracy?**
A: 1) Ensure documents are relevant and comprehensive
   2) Use embedding similarity if documents use different terminology
   3) Combine with LLM-as-Judge for subjective quality checks

**Q: What about long-form responses?**
A: Works best for factual claims with clear grounding. For creative writing or analysis, use LLM-as-Judge instead.

**Q: Can I use this without context documents?**
A: Not with the current implementation. Future versions will support web search.

**Q: Cost-benefit tradeoff?**
A: Rule-based + token overlap is free. Use it as first pass. Upgrade to embedding similarity only if needed.

## References

- [Hallucination in LLMs](https://arxiv.org/abs/2309.01219) — Ji et al., 2023
- [Detecting Hallucinations in LLMs](https://arxiv.org/abs/2311.07397)
- [RAG Evaluation](https://arxiv.org/abs/2309.15217)

## See Also

- [LLM-as-Judge Evaluator](llm-judge-eval.md) — Quality scoring with cached LLM
- [GenAI SLO Integration](user-guide.md#genai-mode) — Using hallucination detection in SLOs
