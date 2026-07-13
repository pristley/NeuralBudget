# Advanced ML/GenAI Features Guide

This document describes the advanced ML / GenAI features provided by NeuralBudget: model degradation detection, pluggable semantic similarity, LLM-specific metrics (token cost tracking and hallucination detection), and drift explanation with lightweight SHAP approximations.

## Overview

NeuralBudget provides four complementary feature pillars to help teams monitor, explain, and control GenAI systems in production:

- Model Performance Degradation Detection
- Custom Semantic Similarity Models
- LLM-Specific Metrics (Token Cost Tracking + Hallucination Detection)
- Drift Explanation (KS tests + SHAP-style approximations)

Use cases
- Continuous quality monitoring (detect gradual drift, spikes, oscillations)
- Automated alerts and retraining triggers
- Evaluating text generation quality via semantic similarity
- Cost-aware model selection and budgeting
- Root-cause analysis for model quality regressions using drift explanations

Quick start
1. Install the Python bindings (built via `maturin`) or use the Rust APIs directly.
2. Import the relevant helper classes from the `neuralbudget` module in Python.
3. Attach detectors and scorers to your SLO evaluation pipeline or run them as lightweight analytics services.

Example quick start (Python)
```python
from neuralbudget import DegradationDetector, SimilarityRegistry, DynamicCostCalculator

detector = DegradationDetector(window_size=100)
registry = SimilarityRegistry()
registry.register_scorer('bert', 'bert', {'endpoint': 'http://localhost:8000/embed', 'model_name': 'all-MiniLM-L6-v2'})
calc = DynamicCostCalculator()
```

---

## Feature 1: Model Performance Degradation Detection

### What it does
- Implements an ADWIN-like adaptive window detector to spot gradual distributional drift in streaming quality metrics.
- Provides z-score based spike detection for sudden outliers.
- Detects oscillating patterns that suggest instability (periodic up/down behavior).
- Returns a structured event with: `type` (one of `gradual`, `sudden`, `oscillating`, `stable`), `magnitude`, `confidence`, `duration_samples`, and `timestamp`.

### When to use
- Production monitoring of model quality SLIs (e.g., semantic similarity, accuracy, F1, hallucination scores).
- Trigger retraining when gradual degradation exceeds tolerance.
- Alert on sudden quality drops that require immediate intervention.

### Quick example
```python
from neuralbudget import DegradationDetector

detector = DegradationDetector(window_size=100)
for metric in metric_stream:
    detector.add_metric(metric.quality_score)
    event = detector.detect_change()
    if event:
        print(f"Degradation detected: {event['type']}, magnitude={event['magnitude']}")
```

### Configuration
- `window_size` (int): number of recent samples to track. Suggested range: 100–1000 depending on traffic and SLO sensitivity.
- `drift_threshold` (float): ADWIN sensitivity parameter controlling false-positive/false-negative tradeoff (recommended 0.1–0.3).
- `spike_threshold` (float): z-score threshold to treat a sample as a spike (recommended 2.0–3.0).

When choosing values:
- Use larger `window_size` for noisy metrics to improve stability.
- Increase `drift_threshold` for conservative detection in production.
- Tune `spike_threshold` to reduce alerts from transient noise.

### Implementation details
- ADWIN-like behaviour is implemented using an exponential histogram approximation for efficient, bounded-memory detection. This provides online estimates of the mean and variance over an adaptive window.
- Spike detection uses rolling mean and standard deviation to compute a z-score for each new sample; large z-scores produce `sudden` events.
- Oscillation detection inspects autocorrelation-like indicators across recent windows to identify repeated reversals.

### Python API (high-level)
- `neuralbudget.DegradationDetector(window_size=100, drift_threshold=0.15, spike_threshold=2.5)`
- Methods: `add_metric(value: float)`, `detect_change() -> Optional[dict]`, `reset()`, getters: `metric_count`, `window_size`.

Performance
- Designed to run inline in high-throughput evaluators; operations are O(1) amortized per sample and memory bounded by `window_size` and histogram bins.

---

## Feature 2: Custom Semantic Similarity Models

### What it does
- Provides a pluggable `SimilarityScorer` trait and a Python `SimilarityRegistry` to register and use scorers.
- Built-in scorer implementations:
  - `BertScorer` — HTTP-backed embedding/similarity service + LRU cache + fallback
  - `RobertaScorer` — variant configured for RoBERTa-style embeddings
  - `CosineSimScorer` — lightweight, token-overlap cosine approximation used as a fallback
- LRU cache reduces latency for repeated comparisons.
- Fallback strategy: if the configured HTTP scorer fails or returns malformed responses, the registry/scorer falls back to the cosine scorer to avoid evaluation interruptions.

### When to use
- Compare generated text to references for semantic similarity-based SLIs.
- Evaluate paraphrase detection, summarization quality, or response relevance.
- Plug in custom HTTP-hosted embedding/similarity services (self-hosted embedder or an inference GPU service).

### Quick example (Python)
```python
from neuralbudget import SimilarityRegistry

registry = SimilarityRegistry()
registry.register_scorer(
    "bert", "bert", {
        "endpoint": "http://localhost:8000/embed",
        "model_name": "all-MiniLM-L6-v2"
    }
)
score = registry.score("bert", "Hello world", "Hi world")
print(score)  # e.g. 0.85
```

### Configuration
- `endpoint` (string): URL of the HTTP similarity/embedding endpoint.
- `model_name` (string): identifier passed to the HTTP service.
- `cache_size` (int): size of the LRU cache (default 1000 entries).
- `timeout_ms` (int): HTTP request timeout in milliseconds.

### Implementation details
- Registry is thread-safe: a `RwLock` protects scorer lookup while scorers themselves use `Arc<Mutex<..>>` patterns where needed.
- `BertScorer` uses a blocking `reqwest` client with a configurable timeout and an `lru::LruCache` protected by a `Mutex` for concurrent access.
- Cache keys are stable SHA-256 hashes of the reference/generated pair.
- On HTTP errors, the scorer logs a warning and computes a cosine-like fallback score.

Python API
- `SimilarityRegistry()` — create a registry
- `register_scorer(name: str, scorer_type: str, config: dict)` — register a scorer; `scorer_type` values: `bert`, `roberta`, `cosine`
- `get_scorer(name: str) -> SimilarityScorer` — returns a wrapper with `score()` and `batch_score()`
- `score(name, reference, generated) -> float`
- `batch_score(name, pairs) -> List[float]`

Performance
- HTTP-backed scorers are network-latency bound. Use local inference services or caching for production throughput.
- Cosine fallback is very fast and suitable for high throughput sanity checks.

---

## Feature 3: LLM-Specific Metrics

### Subsection A: Token Cost Tracking

#### What it does
- `DynamicCostCalculator` maintains per-model pricing configurations and computes cost breakdowns for input and output tokens.
- Supports wildcard patterns (e.g., `gpt-4*`) for matching model families.
- Produces `CostMetrics` with `cost_breakdown` (input, output, total), `model_used`, and `pricing_date`.

#### Quick example
```python
from neuralbudget import DynamicCostCalculator, TokenCostConfig

calc = DynamicCostCalculator()
cfg = TokenCostConfig("gpt-4-turbo", 10.0, 30.0, 1688000000, "USD")
# Note: TokenCostConfig values are per-1000 tokens, e.g. 0.01 means $0.01 per 1000 tokens
calc.update_pricing("gpt-4-turbo", cfg)
cost = calc.calculate_cost("gpt-4-turbo", 500, 250)
print(cost)
```

> Implementation note: the Rust API expects `input_cost_per_1k` and `output_cost_per_1k` as floats representing dollars per 1000 tokens.

#### Configuration
- `input_cost_per_1k` (float) — price per 1000 input tokens.
- `output_cost_per_1k` (float) — price per 1000 output tokens.
- `pricing_date` (int) — timestamp for the pricing (useful for audits).
- `currency` (string) — currency code (e.g., `USD`).

#### Wildcard model matching
- Register patterns like `gpt-4*` to cover model families; the calculator performs simple `*` wildcard matches and chooses the best (longest match) pattern.

#### Implementation details
- Uses `RwLock` + `HashMap` for thread-safe pricing updates and reads.
- `calculate_cost(model, input_tokens, output_tokens)` returns the sum of the prorated input and output costs.

#### Best practices
- Keep a short update window for price changes and record `pricing_date` for historical billing reconciliation.
- Prefer exact model matches when available; use wildcard patterns for fallback.

---

### Subsection B: Hallucination Detection

#### What it does
- Lightweight, rule-based approximations for:
  - Entailment detection — approximated via entity overlap and token overlap
  - Contradiction detection — looks for negation patterns near matched entities
  - Ungroundedness — entities in generated not present in reference or RAG context
  - Factual consistency score (0..1)
- Produces `HallucinationMetrics` with `factual_consistency`, `hallucination_types`, `confidence`, `explanation`, and `conflicting_entities`.

#### Quick example
```python
from neuralbudget import HallucinationDetector

analyzer = HallucinationDetector("Reference text here")
metrics = analyzer.analyze("Generated candidate text")
print(metrics.factual_consistency)
print(metrics.hallucination_types)
print(metrics.explanation)
```

#### When to use
- Automatic content validation pipelines for critical domains (medical, legal, financial)
- RAG pipelines where grounding to external context must be validated
- Post-generation filtering and routing to human review

#### Implementation details
- Default entity extractor: naive heuristic that picks capitalized tokens and alphanumeric tokens; can be overridden via `set_entity_extractor`.
- Contradiction detection uses a list of negation tokens ("not", "never", "doesn't") and looks for negations in the generated text around entities that also appear in the reference.
- Ungroundedness compares generated entities to both the reference and optional RAG context.
- The final `factual_consistency` is a weighted score combining contradiction penalties, ungroundedness penalties, and hedging detection.

#### Limitations
- Heuristic-based; not a replacement for full NLI or external fact-checkers.
- Best used as an early warning signal and routing mechanism rather than an absolute arbiter.

---

## Feature 4: Drift Explanation

### What it does
- Performs statistical two-sample comparisons using the Kolmogorov-Smirnov (KS) test to detect distribution shifts per feature.
- Ranks features by an importance metric derived from KS statistic and effect size.
- Provides an approximate SHAP-style explainer (`ShapExplainer`) that produces per-feature contribution estimates to the distributional change.
- Returns a `DriftExplanation` containing `top_features`, `ks_statistic`, `p_value`, `summary`, and optional `shap_values`.

### When to use
- After detecting degradation to understand which features changed.
- Prioritizing data collection and retraining for features with highest importance.

### Quick example (Python convenience wrapper)
```python
from neuralbudget.drift_explainer import DriftExplainer

baseline = {"f1": [1.0,1.1,0.9], "f2": [2.0,2.1,2.0]}
current = {"f1": [2.0,1.9,2.1], "f2": [2.0,2.1,2.2]}

explanation = DriftExplainer.explain(baseline, current)
print(explanation['summary'])
print(explanation['top_features'])
```

### Configuration
- `ks_threshold` (float): p-value or statistic cutoff used to decide whether a shift is significant.
- `top_k` (int): number of top features to report (default 5).

### Implementation details
- The KS implementation is a standard two-sample KS statistic with an asymptotic p-value approximation.
- Feature ranking combines the KS statistic with a stability/coverage heuristic so that tiny sample-size features do not dominate.
- `ShapExplainer` implements a lightweight additive approximation: given a vector of feature importances or baseline predictions, it computes per-feature contributions that approximately sum to the predicted change.

### SHAP approximation
- Not a full SHAP kernel implementation — a lightweight, fast approximation intended for high-level explanations inside monitoring pipelines.
- Produces `ShapValue` records with `feature`, `value` (contribution), and `confidence`.
- Use the values to prioritize features for deeper offline analysis with proper SHAP libraries if required.

---

## Configuration Guide

YAML examples you can drop into SLO pipelines or config stores.

Degradation detector config
```yaml
degradation_detector:
  window_size: 200
  drift_threshold: 0.15
  spike_threshold: 2.5
```

Similarity registry snippet
```yaml
similarity_registry:
  scorers:
    bert_primary:
      type: bert
      endpoint: http://localhost:8000/embed
      model_name: all-MiniLM-L6-v2
      cache_size: 1000
      timeout_ms: 5000
    fallback_cosine:
      type: cosine
```

Dynamic cost pricing example
```yaml
pricing:
  - model: gpt-4*
    input_cost_per_1k: 0.003
    output_cost_per_1k: 0.006
    pricing_date: 1688000000
    currency: USD
```

Drift analyzer example
```yaml
drift:
  ks_threshold: 0.05
  top_k: 5
```

Production recommendations
- Persist pricing updates with `pricing_date` and use a single writer pattern to avoid transient inconsistencies.
- Run similarity scorer services close to evaluation code (same VPC/zone) to reduce latency.
- Aggregate degradation events and apply debouncing rules for alerting to reduce noisy paging.
- Use a combination of SHAP approximations for quick triage and full SHAP / counterfactual analysis offline.

---

## Troubleshooting

Common errors and solutions

- "HTTP scoring endpoint returns invalid json"
  - Symptoms: `BertScorer` log shows `invalid json` errors and fallback to cosine.
  - Solution: Ensure the endpoint responds with JSON `{"similarity": <float>}`. Check `Content-Type` header and payload schema.

- "Degradation detector triggers too often"
  - Symptoms: Frequent `gradual` events with small magnitude.
  - Solution: Increase `drift_threshold`, increase `window_size`, or smooth incoming SLI values before feeding the detector.

- "Cost calculation model not found"
  - Symptoms: `DynamicCostCalculator::get_pricing` returns model not found.
  - Solution: Register a pricing config for the exact model name or a wildcard pattern like `gpt-4*`.

Performance tips
- For high throughput, run a local similarity scoring service (e.g., a GPU-backed embedder) and tune `cache_size`.
- Keep `DegradationDetector` instances lightweight and colocated with evaluation threads; they are safe to wrap in shared Mutex/Arc if needed.

---

## Examples

Notebook-style walkthrough (high level)
1. Build and install the Python extension:
```bash
pip install maturin
maturin develop --release
```
2. Create a small evaluation harness that:
   - Streams predictions and references
   - Uses `SimilarityRegistry` to score each sample
   - Adds the similarity to `DegradationDetector`
   - Calls `FeatureDriftAnalyzer` periodically for diagnosis
3. Use `PrometheusExporter` to expose selected metrics and alerts to your monitoring stack.

Real-world scenario: paraphrase quality monitoring
- Use `BertScorer` to compute semantic similarity between model outputs and high-quality references.
- Feed rolling average similarity to `DegradationDetector`.
- On `gradual` degradation, run `DriftExplainer` over recent features to identify input distribution changes.
- If cost budget is a concern, use `DynamicCostCalculator` to compare model variants and pick the most cost-effective option that meets SLOs.

Integration patterns
- Sidecar evaluator: run similarity scoring and degradation detection in a sidecar that receives request/response events via a lightweight queue.
- Inline SDK: import `neuralbudget` in your inference process and call the scorers/detectors directly for synchronous evaluation.

---

## Performance Characteristics

Typical latencies (approximate & subject to deployment):
- `CosineSimScorer.score`: < 0.3 ms (pure Python binding overhead included)
- `BertScorer.score` (first call): network latency + embedder time (10–200 ms depending on service)
- `BertScorer.score` (cached): similar to `CosineSimScorer` (< 1 ms)
- `DegradationDetector.add_metric` + `detect_change`: O(1) amortized, sub-ms per sample in Rust
- `FeatureDriftAnalyzer.analyze`: depends on sample sizes; for a few thousand samples per feature, expect tens to low hundreds of ms in Rust implementation

Memory usage
- `BertScorer` cache: default 1000 entries; tuned for average embedding hash size.
- `DegradationDetector`: bounded by `window_size` and histogram bins.

Scalability
- Run scoring services with autoscaling for inference-backed scorers.
- Use caching and local cosine fallback to minimize dependency on remote services.

---

## API Reference (selected)

- `DegradationDetector(window_size=int, drift_threshold=float, spike_threshold=float)`
  - `add_metric(value: float) -> None`
  - `detect_change() -> Optional[dict]`
  - `reset() -> None`
  - `metric_count` (property)

- `SimilarityRegistry()`
  - `register_scorer(name: str, scorer_type: str, config: dict)`
  - `get_scorer(name: str) -> SimilarityScorer`
  - `score(name, reference, generated) -> float`
  - `batch_score(name, pairs: List[Tuple[str, str]]) -> List[float]`

- `DynamicCostCalculator()`
  - `update_pricing(model: str, config: TokenCostConfig)`
  - `calculate_cost(model: str, input_tokens: int, output_tokens: int) -> float`
  - `get_pricing(model: str) -> TokenCostConfig`

- `HallucinationDetector(reference: str)`
  - `analyze(generated: str) -> HallucinationMetrics`
  - `set_rag_context(ctx: str) -> None`
  - `set_entity_extractor(func) -> None`

- `FeatureDriftAnalyzer(ks_threshold: float)` (Python wrapper)
  - `add_baseline(feature: str, value: float)`
  - `add_current(feature: str, value: float)`
  - `analyze() -> dict`

- `ShapExplainer(importances: Dict[str, float], baseline: Optional[float])`
  - `explain() -> Dict[str, float]`
  - `explain_with_context(prediction: float) -> Dict[str, float]`

---

If you'd like, I can:
- Add a runnable notebook example under `quickstart/` that wires all components together.
- Add more configuration YAML examples per deployment (Kubernetes, Docker Compose).
- Add a short troubleshooting playbook for common alert patterns.

File: [docs/advanced-mlgenai-features.md](docs/advanced-mlgenai-features.md)
