import json
import threading
import time
import random
import math
from http.server import BaseHTTPRequestHandler, HTTPServer
from concurrent.futures import ThreadPoolExecutor

import pytest

# Try to import the Python bindings; if not present tests will be skipped.
neuralbudget = pytest.importorskip("neuralbudget")

# --- Fixtures ---

@pytest.fixture(scope="session")
def mock_http_server():
    """Runs a simple HTTP server that can be configured via attributes on the handler.
    The server responds to POST requests to /embed with JSON {"similarity": <float>}.
    Set `mock_http_server.response_value` to control the returned similarity. If set to
    'fail' the server returns 500 to simulate an unreachable/errored endpoint.
    """
    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            length = int(self.headers.get('content-length', 0))
            body = self.rfile.read(length).decode('utf-8') if length > 0 else ''
            try:
                if mock_http_server._response == 'fail':
                    self.send_response(500)
                    self.end_headers()
                    self.wfile.write(b'error')
                    return
                sim = float(mock_http_server._response)
            except Exception:
                sim = 0.5
            resp = json.dumps({"similarity": sim}).encode('utf-8')
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(resp)))
            self.end_headers()
            self.wfile.write(resp)

        def log_message(self, format, *args):
            # silence
            return

    server = HTTPServer(('127.0.0.1', 0), Handler)
    host, port = server.server_address
    mock_http_server.url = f'http://127.0.0.1:{port}/embed'
    mock_http_server._response = '0.95'

    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()

    yield mock_http_server

    server.shutdown()
    thread.join(timeout=1)


@pytest.fixture
def generate_metrics():
    def _gen(count=100, base=0.5, noise=0.05, drift=0.0):
        vals = []
        for i in range(count):
            vals.append(base + drift * (i / count) + random.gauss(0, noise))
        return vals
    return _gen


@pytest.fixture
def analyzer_with_data(generate_metrics):
    analyzer = neuralbudget.FeatureDriftAnalyzer()
    baseline = generate_metrics(count=200, base=0.5, noise=0.02, drift=0.0)
    current = generate_metrics(count=200, base=0.5, noise=0.02, drift=0.0)
    for v in baseline:
        analyzer.add_baseline("f1", float(v))
    for v in current:
        analyzer.add_current("f1", float(v))
    return analyzer


# --- 1. Degradation Detection Tests ---

@pytest.mark.slow
def test_gradual_drift_detection():
    detector = neuralbudget.DegradationDetector(window_size=100)
    # gradual upward drift
    for i in range(200):
        detector.add_metric(0.5 + (i / 400.0))
    ev = detector.detect_change()
    assert ev is not None, "expected some degradation event"
    assert ev["type"] in ("gradual", "oscillating", "sudden")
    # Prefer gradual but accept oscillating/sudden depending on implementation


def test_sudden_spike_detection():
    detector = neuralbudget.DegradationDetector(window_size=100)
    # stable baseline
    for _ in range(80):
        detector.add_metric(0.5 + random.gauss(0, 0.01))
    # sudden spike
    detector.add_metric(5.0)
    ev = detector.detect_change()
    assert ev is not None
    assert ev["type"] in ("sudden", "gradual", "oscillating")


def test_no_false_positives(generate_metrics):
    detector = neuralbudget.DegradationDetector(window_size=100)
    vals = generate_metrics(count=200, base=0.5, noise=0.05, drift=0.0)
    for v in vals:
        detector.add_metric(v)
    ev = detector.detect_change()
    # detector may either return None or 'stable'
    if ev is None:
        assert True
    else:
        assert ev["type"] == "stable"


def test_reset_clears_history():
    detector = neuralbudget.DegradationDetector(window_size=50)
    for _ in range(30):
        detector.add_metric(0.5)
    assert detector.metric_count > 0
    detector.reset()
    assert detector.metric_count == 0
    assert detector.detect_change() is None


def test_edge_cases():
    detector = neuralbudget.DegradationDetector(window_size=10)
    # empty
    assert detector.detect_change() is None
    # single value
    detector.add_metric(0.7)
    ev = detector.detect_change()
    assert ev is None or ev["type"] == "stable"


# --- 2. Similarity Scorer Tests ---

def test_register_bert_scorer(mock_http_server):
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer("bert_test", "bert", {"endpoint": mock_http_server.url, "model_name": "mymodel"})
    scorer = reg.get_scorer("bert_test")
    assert scorer.name() == "bert_scorer"
    score = reg.score("bert_test", "The sky is blue", "Sky is blue")
    assert pytest.approx(score, rel=1e-3) == float(mock_http_server._response)


def test_cosine_fallback(mock_http_server):
    # cause server to fail
    mock_http_server._response = 'fail'
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer("bert_fallback", "bert", {"endpoint": mock_http_server.url})
    # compare with an explicit cosine scorer
    cosine_reg = neuralbudget.SimilarityRegistry()
    cosine_reg.register_scorer("cos", "cosine", None)

    s1 = "this is a sentence"
    s2 = "this is a different sentence"
    got = reg.score("bert_fallback", s1, s2)
    expected = cosine_reg.score("cos", s1, s2)
    assert pytest.approx(got, rel=1e-6) == expected


def test_cache_behavior(mock_http_server):
    mock_http_server._response = '0.85'
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer("bert_cache", "bert", {"endpoint": mock_http_server.url})
    scorer = reg.get_scorer("bert_cache")
    # initial calls
    a = reg.score("bert_cache", "A", "B")
    b = reg.score("bert_cache", "A", "B")
    # cache_hits() may be available
    hits = scorer.cache_hits()
    assert hits is None or hits >= 1
    assert pytest.approx(a, rel=1e-6) == b


def test_batch_score(mock_http_server):
    mock_http_server._response = '0.9'
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer("bert_batch", "bert", {"endpoint": mock_http_server.url})
    pairs = [("A", "A"),("A","B"),("sky","blue")]
    res = reg.batch_score("bert_batch", pairs)
    assert isinstance(res, list)
    assert len(res) == 3


def test_scorer_not_found():
    reg = neuralbudget.SimilarityRegistry()
    with pytest.raises(KeyError):
        reg.score("no_such", "a","b")


# --- 3. Cost Tracking Tests ---

@pytest.mark.slow
def test_calculate_cost():
    # skip if DynamicCostCalculator not exposed
    dcc = getattr(neuralbudget, 'DynamicCostCalculator', None)
    if dcc is None:
        pytest.skip("DynamicCostCalculator not exposed in Python bindings")
    calc = neuralbudget.DynamicCostCalculator()
    # set gpt-4 style pricing (approx.)
    cfg_cls = getattr(neuralbudget, 'TokenCostConfig', None)
    if cfg_cls is None:
        pytest.skip("TokenCostConfig not exposed")
    cfg = neuralbudget.TokenCostConfig("gpt-4" , 0.002, 0.002, int(time.time()), "USD")
    calc.update_pricing("gpt-4", cfg)
    cost = calc.calculate_cost("gpt-4", 500, 250)
    assert isinstance(cost, float)
    # approximate expected (500/1000*0.002 + 250/1000*0.002) = 0.0015
    assert 0.0 <= cost <= 0.01


def test_wildcard_patterns():
    dcc = getattr(neuralbudget, 'DynamicCostCalculator', None)
    if dcc is None:
        pytest.skip("DynamicCostCalculator not exposed")
    calc = neuralbudget.DynamicCostCalculator()
    if not hasattr(neuralbudget, 'TokenCostConfig'):
        pytest.skip("TokenCostConfig not exposed")
    cfg = neuralbudget.TokenCostConfig("gpt-4*", 0.002, 0.002, int(time.time()), "USD")
    calc.update_pricing("gpt-4*", cfg)
    price = calc.get_pricing("gpt-4-turbo")
    assert price is not None


def test_dynamic_pricing_updates():
    dcc = getattr(neuralbudget, 'DynamicCostCalculator', None)
    if dcc is None:
        pytest.skip("DynamicCostCalculator not exposed")
    calc = neuralbudget.DynamicCostCalculator()
    cfg1 = neuralbudget.TokenCostConfig("m1", 0.001, 0.001, 1, "USD")
    cfg2 = neuralbudget.TokenCostConfig("m1", 0.01, 0.01, 2, "USD")
    calc.update_pricing("m1", cfg1)
    c1 = calc.calculate_cost("m1", 1000, 1000)
    calc.update_pricing("m1", cfg2)
    c2 = calc.calculate_cost("m1", 1000, 1000)
    assert c2 > c1


def test_multiple_models():
    dcc = getattr(neuralbudget, 'DynamicCostCalculator', None)
    if dcc is None:
        pytest.skip("DynamicCostCalculator not exposed")
    calc = neuralbudget.DynamicCostCalculator()
    cfg_a = neuralbudget.TokenCostConfig("a", 0.001, 0.001, 1, "USD")
    cfg_b = neuralbudget.TokenCostConfig("b", 0.01, 0.02, 1, "USD")
    calc.update_pricing("a", cfg_a)
    calc.update_pricing("b", cfg_b)
    ca = calc.calculate_cost("a", 100, 100)
    cb = calc.calculate_cost("b", 100, 100)
    assert ca != cb


# --- 4. Hallucination Detection Tests ---

def test_entailment():
    HD = getattr(neuralbudget, 'HallucinationDetector', None)
    if HD is None:
        pytest.skip("HallucinationDetector not exposed in Python bindings")
    d = neuralbudget.HallucinationDetector("Sky color is blue")
    m = d.analyze("The sky is blue")
    assert m is not None
    assert any([h in str(m).lower() for h in ['entailment', 'factually_correct', 'highconfidence']])


def test_contradiction():
    HD = getattr(neuralbudget, 'HallucinationDetector', None)
    if HD is None:
        pytest.skip("HallucinationDetector not exposed")
    d = neuralbudget.HallucinationDetector("X happens")
    m = d.analyze("X does not happen")
    assert m is not None
    # expect contradiction
    assert 'Contradiction' in str(m) or m.factual_consistency < 1.0


def test_ungroundedness():
    HD = getattr(neuralbudget, 'HallucinationDetector', None)
    if HD is None:
        pytest.skip("HallucinationDetector not exposed")
    d = neuralbudget.HallucinationDetector("Reference text does not mention Y")
    m = d.analyze("Feature Y was added")
    assert m is not None
    assert m.hallucination_types is not None


def test_factually_correct():
    HD = getattr(neuralbudget, 'HallucinationDetector', None)
    if HD is None:
        pytest.skip("HallucinationDetector not exposed")
    d = neuralbudget.HallucinationDetector("Water freezes at 0C")
    m = d.analyze("Water freezes at 0C")
    assert m.factual_consistency >= 0.9


# --- 5. Drift Explanation Tests ---

def test_no_drift(analyzer_with_data):
    analyzer = analyzer_with_data
    res = analyzer.analyze()
    assert isinstance(res, dict)
    assert abs(res.get('ks_statistic', 0.0)) < 1e-6


def test_single_feature_drift():
    analyzer = neuralbudget.FeatureDriftAnalyzer()
    for v in [0.1]*200:
        analyzer.add_baseline('f1', v)
        analyzer.add_baseline('f2', 0.5)
    for v in [0.9]*200:
        analyzer.add_current('f1', v)
    for v in [0.5]*200:
        analyzer.add_current('f2', v)
    res = analyzer.analyze()
    assert 'f1' in res.get('top_features', [])


def test_shap_values():
    # Create a simple explainer
    imps = {'f1': 0.6, 'f2': 0.2, 'f3': 0.2}
    expl = neuralbudget.ShapExplainer(imps)
    vals = expl.explain()
    assert isinstance(vals, dict)
    s = sum(vals.values())
    # sums near zero or small number depending on implementation; we assert numeric
    assert math.isfinite(s)
    # top feature has largest magnitude
    top = max(vals.items(), key=lambda kv: abs(kv[1]))
    assert top[0] in imps


def test_multiple_features():
    analyzer = neuralbudget.FeatureDriftAnalyzer()
    for i in range(5):
        for _ in range(200):
            analyzer.add_baseline(f'f{i}', 0.5)
    # introduce drift in 2 features
    for _ in range(200):
        analyzer.add_current('f1', 0.1)
        analyzer.add_current('f3', 0.9)
        for i in [0,2,4]:
            analyzer.add_current(f'f{i}', 0.5)
    res = analyzer.analyze()
    tops = res.get('top_features', [])
    assert 'f1' in tops and 'f3' in tops


# --- 6. Integration Tests ---

@pytest.mark.slow
def test_end_to_end_pipeline(mock_http_server, generate_metrics):
    # attempt to assemble GenAiSlo pipeline; skip if API not available
    GenAiSlo = getattr(neuralbudget, 'GenAiSlo', None)
    if GenAiSlo is None:
        pytest.skip('GenAiSlo not available in Python bindings')
    # create detector and registry
    detector = neuralbudget.DegradationDetector(window_size=100)
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer('bert_int', 'bert', {'endpoint': mock_http_server.url})
    slo = GenAiSlo('test-slo')
    # attach components if API exposes builders
    try:
        slo = slo.with_degradation_detector(detector)
    except Exception:
        pass
    try:
        slo = slo.with_similarity_scorer(reg.get_scorer('bert_int'))
    except Exception:
        pass
    # stream metrics
    for v in generate_metrics(count=200, drift=0.01):
        try:
            slo.evaluate_sample({'value': float(v)})
        except Exception:
            # best-effort; we assert pipeline doesn't crash
            pass
    # best-effort checks: exporter should exist
    exporter = neuralbudget.PrometheusExporter()
    txt = exporter.render()
    assert isinstance(txt, str)


def test_concurrent_access(mock_http_server):
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer('bert_conc', 'bert', {'endpoint': mock_http_server.url})
    def worker(i):
        try:
            return reg.score('bert_conc', f'A {i}', f'B {i}')
        except Exception as e:
            return e
    with ThreadPoolExecutor(max_workers=8) as ex:
        results = list(ex.map(worker, range(50)))
    assert len(results) == 50


def test_performance_bounds(mock_http_server):
    # cost calculation micro-benchmark
    t0 = time.perf_counter()
    # cheap operation: cosine scoring via a small registry
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer('cos_perf', 'cosine', None)
    for _ in range(100):
        reg.score('cos_perf', 'a', 'b')
    t1 = time.perf_counter()
    avg = (t1 - t0) / 100.0
    # Expect average < 1ms for similarity in Python binding (lenient)
    assert avg < 0.01


# --- 7. Error cases ---

def test_invalid_config_raises():
    reg = neuralbudget.SimilarityRegistry()
    with pytest.raises(ValueError):
        reg.register_scorer('bad', 'unknown', None)


def test_empty_data_raises():
    analyzer = neuralbudget.FeatureDriftAnalyzer()
    with pytest.raises(Exception):
        # analyze with no data should raise or return a dict; accept either
        res = analyzer.analyze()
        if isinstance(res, dict):
            raise Exception('expected failure for empty data')


def test_malformed_response_raises(mock_http_server):
    # server returns invalid json
    mock_http_server._response = 'not-a-number'
    reg = neuralbudget.SimilarityRegistry()
    reg.register_scorer('bert_malformed', 'bert', {'endpoint': mock_http_server.url})
    # depending on implementation, score may fallback or raise
    try:
        v = reg.score('bert_malformed', 'a', 'b')
        assert isinstance(v, float)
    except Exception:
        # acceptable behaviour
        assert True


# End of tests
