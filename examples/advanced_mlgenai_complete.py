#!/usr/bin/env python3
"""Advanced ML/GenAI Features - Production Example

Demonstrates how to wire Degradation detection, semantic similarity scoring,
cost tracking, hallucination detection, and drift explanation into a
stream-processing pipeline using the `neuralbudget` Python bindings.

This example is self-contained and uses a mock HTTP scoring endpoint and
synthetic data generator. Replace mocks with real services for production.
"""

import json
import math
import random
import threading
import time
import logging
from http.server import BaseHTTPRequestHandler, HTTPServer
from collections import defaultdict
from typing import Dict, List

# Attempt to import bindings and convenience wrapper
import pytest
try:
    import neuralbudget
except Exception as e:
    raise RuntimeError("neuralbudget bindings are required. Build with maturin: `maturin develop`")

# convenience Python wrapper created earlier
try:
    from neuralbudget.drift_explainer import DriftExplainer
except Exception:
    # fallback to rust-backed convenience if module not present
    try:
        from neuralbudget import DriftExplainer  # type: ignore
    except Exception:
        DriftExplainer = None

# Use HallucinationDetector name used in bindings
Hallucination = getattr(neuralbudget, 'HallucinationDetector', None)
PromExporter = getattr(neuralbudget, 'PrometheusExporter', None)
GenAiSlo = getattr(neuralbudget, 'GenAiSlo', None)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger('advanced_example')

# --- Mock HTTP embedding/similarity service ---
class SimpleEmbedHandler(BaseHTTPRequestHandler):
    # respond with a fixed similarity or simple heuristic based on body
    def do_POST(self):
        length = int(self.headers.get('content-length', 0))
        body = self.rfile.read(length).decode('utf-8') if length > 0 else ''
        # simple heuristic: if texts share a word 'blue' return high similarity
        sim = 0.5
        if 'blue' in body:
            sim = 0.95
        resp = json.dumps({'similarity': sim}).encode('utf-8')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(resp)))
        self.end_headers()
        self.wfile.write(resp)
    def log_message(self, format, *args):
        return

def start_mock_server(port=0):
    server = HTTPServer(('127.0.0.1', port), SimpleEmbedHandler)
    host, actual_port = server.server_address
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, f'http://127.0.0.1:{actual_port}/embed'

# --- Synthetic data generator ---
def synthetic_metric_stream(n: int):
    """Yields synthetic inference metrics for n samples.

    Each sample is a dict with:
    - features: dict of feature -> float
    - reference: str
    - generated: str
    - input_tokens, output_tokens
    """
    for i in range(n):
        # produce 10 numeric features with subtle drift after half of the stream
        features = {f'f{j}': 0.5 + (0.2 * ((i / n) if j in (1, 3) else 0.0)) + random.gauss(0, 0.02) for j in range(10)}
        # craft text so some samples contain the token 'blue' for high similarity
        if i % 10 == 0:
            generated = 'The sky is blue.'
            reference = 'Sky color is blue.'
        else:
            generated = 'A generic sentence about facts.'
            reference = 'A generic reference sentence.'
        input_tokens = random.randint(100, 800)
        output_tokens = random.randint(10, 300)
        yield {
            'features': features,
            'reference': reference,
            'generated': generated,
            'input_tokens': input_tokens,
            'output_tokens': output_tokens,
            'timestamp': int(time.time())
        }

# --- Main pipeline example ---

def run_pipeline(total_samples: int = 1000, cost_budget: float = 5.0):
    # Part 1: Setup & Configuration
    server, endpoint = start_mock_server()
    logger.info('Started mock similarity endpoint at %s', endpoint)

    # Degradation detector: monitors a scalar quality SLI
    detector = neuralbudget.DegradationDetector(window_size=100, drift_threshold=0.15, spike_threshold=2.5)

    # Similarity registry and scorer configuration
    registry = neuralbudget.SimilarityRegistry()
    registry.register_scorer('bert_local', 'bert', {'endpoint': endpoint, 'model_name': 'mock-model'})

    # Cost calculator
    calc = neuralbudget.DynamicCostCalculator()
    # Register example prices (per 1000 tokens)
    cfg_type = getattr(neuralbudget, 'TokenCostConfig', None)
    now_ts = int(time.time())
    if cfg_type is not None:
        cfg_gpt4 = neuralbudget.TokenCostConfig('gpt-4-turbo', 0.01, 0.03, now_ts, 'USD')
        cfg_claude = neuralbudget.TokenCostConfig('claude-3', 0.005, 0.015, now_ts, 'USD')
        calc.update_pricing('gpt-4-turbo', cfg_gpt4)
        calc.update_pricing('claude-3', cfg_claude)

    # Hallucination detector (requires reference text)
    hallucination_cls = Hallucination
    hall_detector = None
    if hallucination_cls is not None:
        hall_detector = hallucination_cls('')  # will be set per-sample with real reference

    # Prometheus exporter for reporting
    exporter = PromExporter.with_namespace('advanced_demo') if PromExporter is not None else None

    # Collector structures
    degradation_timeline = []
    total_cost = 0.0
    model_costs = defaultdict(float)
    hallucination_count = 0
    drift_baseline = defaultdict(list)
    drift_current = defaultdict(list)

    # Part 2: Stream Processing
    sample_iter = synthetic_metric_stream(total_samples)
    for idx, sample in enumerate(sample_iter):
        try:
            # 1) Semantic similarity quality scoring
            try:
                score = registry.score('bert_local', sample['reference'], sample['generated'])
            except Exception as e:
                logger.warning('Similarity scoring failed: %s; using fallback cosine scorer', e)
                # try fallback scorer
                try:
                    registry.register_scorer('cos_fallback', 'cosine', None)
                    score = registry.score('cos_fallback', sample['reference'], sample['generated'])
                except Exception:
                    score = 0.0

            # feed the quality SLI into the degradation detector
            detector.add_metric(float(score))
            ev = detector.detect_change()
            if ev:
                logger.info('Degradation event at %d: %s', idx, ev)
                degradation_timeline.append({'idx': idx, 'event': ev})

            # 3) Cost calculation (choose model heuristically)
            model = 'gpt-4-turbo' if idx % 2 == 0 else 'claude-3'
            c = 0.0
            try:
                c = calc.calculate_cost(model, int(sample['input_tokens']), int(sample['output_tokens']))
            except Exception as e:
                logger.warning('Cost calc error: %s', e)
            total_cost += c
            model_costs[model] += c

            # 4) Hallucination detection
            if hall_detector is not None:
                # create per-sample detector bound to the reference
                per = hallucination_cls(sample['reference'])
                try:
                    hm = per.analyze(sample['generated'])
                    # treat low factual_consistency as hallucination
                    if hm.factual_consistency < 0.9:
                        hallucination_count += 1
                except Exception as e:
                    # hallucination analysis requires non-empty reference
                    logger.debug('Hallucination analysis skipped: %s', e)

            # collect features for drift analysis
            for k, v in sample['features'].items():
                # use first half as baseline, second half as current
                if idx < (total_samples // 2):
                    drift_baseline[k].append(v)
                else:
                    drift_current[k].append(v)

            # register Prometheus metrics if available
            if exporter is not None:
                try:
                    exporter.observe_counter('processed_samples_total', 1.0, labels={'model': model})
                    # lightweight degradation metric
                    exporter.observe_degradation_event('slo_service', ev if ev else None)
                except Exception:
                    pass

            # simple alerting checks
            if ev:
                print('ALERT: Degradation event detected:', ev)
            if total_cost > cost_budget:
                print('ALERT: cost budget exceeded: ${:.4f}'.format(total_cost))
                # in production, integrate with alerting/incident system and throttle
                break

        except KeyboardInterrupt:
            logger.info('Interrupted')
            break
        except Exception as e:
            logger.exception('Error processing sample %d: %s', idx, e)

    # Part 3: Analysis & Reporting
    # Run DriftExplainer on collected baseline/current distributions
    drift_report = None
    if DriftExplainer is not None:
        try:
            drift_report = DriftExplainer.explain(drift_baseline, drift_current)
        except Exception as e:
            logger.warning('Drift explainer failed: %s', e)
    else:
        # try rust-backed py_explain_drift if available
        py_explain = getattr(neuralbudget, 'py_explain_drift', None)
        if py_explain is not None:
            try:
                drift_report = py_explain(drift_baseline, drift_current)
            except Exception as e:
                logger.warning('py_explain_drift failed: %s', e)

    # Collate final report
    report = {
        'samples_processed': idx + 1,
        'degradation_events': degradation_timeline,
        'total_cost': total_cost,
        'cost_by_model': dict(model_costs),
        'hallucination_count': hallucination_count,
        'hallucination_rate': hallucination_count / max(1, idx + 1),
        'drift_report': drift_report,
    }

    # Export report JSON
    out_path = 'examples/advanced_mlgenai_report.json'
    with open(out_path, 'w') as f:
        json.dump(report, f, indent=2)
    logger.info('Wrote report to %s', out_path)

    # Export Prometheus exposition if exporter configured
    if exporter is not None:
        try:
            text = exporter.render()
            with open('examples/advanced_mlgenai_metrics.prom', 'w') as f:
                f.write(text)
            logger.info('Wrote Prometheus metrics to examples/advanced_mlgenai_metrics.prom')
        except Exception as e:
            logger.warning('Failed to render Prometheus metrics: %s', e)

    # Part 4: Alerting summary
    if report['hallucination_rate'] > 0.05:
        logger.warning('High hallucination rate: %.2f%%', report['hallucination_rate'] * 100.0)

    logger.info('Pipeline completed: processed %d samples; total cost $%.4f', idx + 1, total_cost)

    # Shutdown mock server
    try:
        server.shutdown()
    except Exception:
        pass


if __name__ == '__main__':
    # default run for demo
    run_pipeline(total_samples=1000, cost_budget=20.0)

# Notes:
# - Replace the mock scoring endpoint with a real embedder (e.g., a local GPU service) by changing
#   `start_mock_server` to the actual endpoint and passing it to `register_scorer`.
# - For production, run exporters in a background thread that periodically scrapes state rather than
#   emitting one-shot files.
# - Ensure the Python bindings are built with `maturin develop` before running this example.
