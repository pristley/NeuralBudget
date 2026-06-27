# neuralbudget CLI User Guide

## Installation

### From Homebrew (macOS/Linux)
```bash
brew install neuralbudget
```

### From AUR (Arch Linux)
```bash
yay -S neuralbudget
```

### From Docker
```bash
docker pull ghcr.io/neuralbudget/neuralbudget:latest
docker run neuralbudget:latest eval slo.yaml sample.json
```

### From Source
```bash
git clone https://github.com/neuralbudget/neuralbudget
cd neuralbudget
cargo build --release --bin neuralbudget
./target/release/neuralbudget --help
```

## Quick Start

### 1. Evaluate SLO Against Sample Data
```bash
neuralbudget eval slo.yaml sample.json
```

Output:
```
✓ SLO PASS

Availability:        99.95%
P99 Latency:         187ms
Requests Passed:     9995/10000

Error Budget: 6.25 hours remaining (out of 10500 hours/month)
```

### 2. Generate Prometheus Rules
```bash
neuralbudget gen-rules slo.yaml > rules.yaml
```

Or for Kubernetes:
```bash
neuralbudget gen-rules slo.yaml --kubernetes --namespace monitoring
```

### 3. Validate Configuration
```bash
neuralbudget check slo.yaml
```

Output:
```
✓ Service field present
✓ Target percentage is realistic (99.95%)
✓ Latency threshold is realistic (200ms)
✓ Configuration is valid
```

With strict mode:
```bash
neuralbudget check slo.yaml --strict
```

Warnings become errors in strict mode.

### 4. HTTP Server Mode (Coming Soon)
```bash
neuralbudget serve --port 8080 --bind 0.0.0.0
```

Then POST to `/eval`:
```bash
curl -X POST http://localhost:8080/eval \
  -H "Content-Type: application/json" \
  -d @sample.json \
  --data-binary @slo.yaml
```

## Command Reference

### eval - Evaluate SLO

Evaluate an SLO configuration against sample metrics data.

**Usage:**
```bash
neuralbudget eval <CONFIG> <SAMPLE> [OPTIONS]
```

**Arguments:**
- `CONFIG`: Path to YAML SLO configuration file
- `SAMPLE`: Path to JSON sample metrics file

**Options:**
- `--json`: Output result as JSON instead of human-readable format
- `-v, --verbose`: Enable verbose output with debug information

**Examples:**
```bash
# Basic evaluation
neuralbudget eval slo.yaml sample.json

# Output JSON for integration with other tools
neuralbudget eval slo.yaml sample.json --json | jq '.status'

# Verbose output for debugging
neuralbudget eval slo.yaml sample.json --verbose
```

### gen-rules - Generate Prometheus Rules

Generate production-ready Prometheus alerting and recording rules from an SLO configuration.

**Usage:**
```bash
neuralbudget gen-rules <CONFIG> [OPTIONS]
```

**Arguments:**
- `CONFIG`: Path to YAML SLO configuration file

**Options:**
- `--kubernetes`: Output as Kubernetes PrometheusRule CRD
- `--namespace <NAMESPACE>`: Kubernetes namespace (default: monitoring)

**What it generates:**

1. **Recording Rules** (updated every 30 seconds):
   - `neuralbudget:slo:availability` - Availability SLI (%)
   - `neuralbudget:slo:latency_p99_ms` - P99 latency (milliseconds)
   - `neuralbudget:slo:error_rate` - Error rate (percentage)
   - `neuralbudget:slo:error_budget_remaining` - Error budget available (%)
   - `neuralbudget:slo:burn_rate_*` - Multi-window burn rates

2. **Alerting Rules** (evaluated every 1 minute):
   - `SloErrorBudgetBurnRate{1h,6h,24h,3d}` - Multi-burn-rate alerts
   - `SloLatencyExceeded` - Latency SLI violation
   - `SloErrorBudgetExhausted` - Budget fully consumed

**Multi-Burn-Rate Alerting:**

The generated rules implement Google SRE's multi-burn-rate strategy for fast anomaly detection:

| Window | Burn Rate | Alert Threshold | Evaluation | Use Case |
|--------|-----------|-----------------|-----------|----------|
| 1h | 10% | Aggressive (0.05% error for 99.95% SLO) | Every 1m | Fast burn detection |
| 6h | 5% | Medium (0.025% error) | Every 5m | Sustained degradation |
| 24h | 2% | Slow (0.01% error) | Every 15m | Trends & patterns |
| 3d | 1% | Critical (0.005% error) | Every 1h | Budget exhaustion prediction |

**Examples:**

```bash
# Generate plain YAML rules
neuralbudget gen-rules slo.yaml > prometheus-rules.yaml

# Generate Kubernetes CRD
neuralbudget gen-rules slo.yaml --kubernetes --namespace monitoring

# Save to file
neuralbudget gen-rules slo.yaml > rules.yaml

# Apply to Kubernetes
neuralbudget gen-rules slo.yaml --kubernetes | kubectl apply -f -

# Validate generated rules
neuralbudget gen-rules slo.yaml | yamllint -

# Preview without writing
neuralbudget gen-rules slo.yaml
```

**Configuration Requirements:**

Your SLO YAML must include:

```yaml
service: "payment-api"
availability_threshold: 0.9995    # e.g., 99.95%
latency_threshold_ms: 200         # P99 threshold
job_label: "payment-api"          # Prometheus job label (optional)

# Optional: customize burn rate windows
alerts:
  - window: "1h"
    threshold: 0.10
  - window: "6h"
    threshold: 0.05
  - window: "24h"
    threshold: 0.02
  - window: "3d"
    threshold: 0.01
```

**Metrics Requirements:**

Your application must export standard HTTP metrics:

```
http_requests_total{job="payment-api", status="200"} counter
http_requests_total{job="payment-api", status="500"} counter
http_request_duration_seconds_bucket{job="payment-api", le="0.2"} histogram
```

**Learn More:**

See [Prometheus Rule Generation Guide](../guides/prometheus-rule-generation.md) for:
- Detailed burn rate math and thresholds
- Alertmanager integration
- Troubleshooting and customization
- Generated rules examples

**See Also:** [prometheus-rule-generation.md](../guides/prometheus-rule-generation.md)

### check - Validate Configuration

Check an SLO configuration for validity and common mistakes.

**Usage:**
```bash
neuralbudget check <CONFIG> [OPTIONS]
```

**Arguments:**
- `CONFIG`: Path to YAML SLO configuration file

**Options:**
- `--strict`: Treat warnings as errors (exit code 1)

**Validates:**
- Required fields (service, target)
- SLO target is between 0 and 100%
- Latency thresholds are realistic (10ms - 30s)
- Recommended best practices

**Examples:**
```bash
# Check configuration
neuralbudget check slo.yaml

# Strict mode (warnings = errors)
neuralbudget check slo.yaml --strict

# In CI/CD pipeline
neuralbudget check slo.yaml --strict || exit 1
```

### serve - HTTP Server Mode (Coming Soon)

Run an HTTP server for on-demand SLO evaluation.

**Usage:**
```bash
neuralbudget serve [OPTIONS]
```

**Options:**
- `-p, --port <PORT>`: Server port (default: 8080)
- `--bind <ADDRESS>`: Bind address (default: 127.0.0.1)

**Note:** This feature is planned for a future release.

## Example Workflows

### GitHub Actions CI Integration
```yaml
name: Validate SLOs
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release --bin neuralbudget
      - run: ./target/release/neuralbudget check slo.yaml --strict
      - run: ./target/release/neuralbudget eval slo.yaml sample.json --json
```

### Kubernetes CronJob for SLO Evaluation
```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: slo-evaluator
spec:
  schedule: "*/5 * * * *"  # Every 5 minutes
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: evaluator
            image: ghcr.io/neuralbudget/neuralbudget:latest
            command: 
            - neuralbudget
            - eval
            - /config/slo.yaml
            - /metrics/sample.json
            - --json
            volumeMounts:
            - name: config
              mountPath: /config
            - name: metrics
              mountPath: /metrics
          volumes:
          - name: config
            configMap:
              name: slo-config
          - name: metrics
            configMap:
              name: slo-sample
          restartPolicy: OnFailure
```

### Local Development
```bash
# Create config and sample
cat > slo.yaml << 'EOF'
service: "my-api"
target: 99.9
latency_threshold_ms: 200
EOF

cat > sample.json << 'EOF'
{"requests_total": 10000, "requests_successful": 9990}
EOF

# Validate
neuralbudget check slo.yaml

# Evaluate
neuralbudget eval slo.yaml sample.json

# Generate rules for monitoring
neuralbudget gen-rules slo.yaml > prometheus-rules.yaml
```

## Performance

- **eval subcommand**: < 100ms per evaluation (target)
- **gen-rules subcommand**: < 50ms per config (target)
- **check subcommand**: < 30ms per config (target)

## Exit Codes

- `0`: Success (SLO passed or operation completed without errors)
- `1`: Failure (SLO failed or validation error)
- `2`: Invalid arguments or configuration format error

## Troubleshooting

### "Error: No such file or directory"
Check that file paths are correct:
```bash
neuralbudget eval ./path/to/slo.yaml ./path/to/sample.json
```

### "YAML parsing error"
Ensure YAML is valid and properly formatted:
```bash
neuralbudget check slo.yaml --verbose
```

### "Invalid SLO target"
SLO target must be between 0 and 100%:
```yaml
target: 99.95  # Valid
target: 101    # Invalid
```

## Configuration Examples

See [examples/](../../examples/) directory for:
- `slo_http.yaml` - HTTP service SLO
- `slo_ml.yaml` - Machine learning service SLO
- `sample_http.json` - HTTP metrics sample
