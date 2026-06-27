# OpenSLO Compatibility & Conversion Guide

NeuralBudget provides bidirectional conversion support for OpenSLO, a CNCF-aligned vendor-neutral SLO format. This enables seamless migration to/from other tools and removes vendor lock-in.

## What is OpenSLO?

OpenSLO is a standardized specification for defining Service Level Objectives in a vendor-neutral way. It's maintained by the OpenSLO project and aligned with CNCF standards.

**Benefits:**
- ✅ **Vendor independence** - Use with any monitoring platform
- ✅ **Tool portability** - Migrate between observability platforms
- ✅ **Standardization** - Consistent SLO format across organizations
- ✅ **Integration** - Works with Nobl9, Lightstep, and other tools

## Getting Started with OpenSLO

### Basic Example

```yaml
apiVersion: openslo/v1
kind: SLO
metadata:
  name: api-gateway-slo
  namespace: platform
spec:
  service: api-gateway
  objectives:
    - ratioMetrics:
        total:
          metricSource:
            prometheus:
              query: rate(http_requests_total[5m])
        good:
          metricSource:
            prometheus:
              query: rate(http_requests_total{status=~"2.."}[5m])
      target: 0.999        # 99.9% availability
      window: rolling_1d
```

## Converting with NeuralBudget

### OpenSLO → NeuralBudget

Convert from OpenSLO format to NeuralBudget's simplified YAML:

```bash
neuralbudget convert \
  --input openslo-slo.yaml \
  --from openslo \
  --to neuralbudget \
  > neuralbudget-slo.yaml
```

**Input (OpenSLO):**
```yaml
apiVersion: openslo/v1
kind: SLO
metadata:
  name: payment-api-slo
spec:
  service: payment-api
  objectives:
    - ratioMetrics:
        total:
          prometheus:
            query: rate(payment_requests_total[5m])
        good:
          prometheus:
            query: rate(payment_requests_success[5m])
      target: 0.9995
      window: rolling_30d
```

**Output (NeuralBudget):**
```yaml
# Converted from OpenSLO format by NeuralBudget
# Original format: OpenSLO
# Converted to: NeuralBudget

availability_threshold: 0.9995
latency_threshold_ms: 200
latency_percentile: 0.99
```

### NeuralBudget → OpenSLO

Convert NeuralBudget SLO to OpenSLO for portability:

```bash
neuralbudget convert \
  --input neuralbudget-slo.yaml \
  --from neuralbudget \
  --to openslo \
  --service payment-api \
  --name payment-api-slo \
  > openslo-slo.yaml
```

**Input (NeuralBudget):**
```yaml
availability_threshold: 0.9995
latency_threshold_ms: 500
latency_percentile: 0.99
```

**Output (OpenSLO):**
```yaml
apiVersion: openslo/v1
kind: SLO
metadata:
  name: payment-api-slo
  namespace: default
  labels:
    service: payment-api
    generated-by: neuralbudget
spec:
  service: payment-api
  description: NeuralBudget SLO: payment-api
  objectives:
    - ratio_metrics:
        good:
          prometheus:
            query: rate(http_requests_total{status=~"2..",service="payment-api"}[5m])
        total:
          prometheus:
            query: rate(http_requests_total{service="payment-api"}[5m])
      target: 0.9995
      window: rolling_1d
      description: Availability SLO for payment-api
    - threshold_metrics:
        threshold: 500
        prometheus:
          query: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket{service="payment-api"}[5m]))
      target: 0.99
      window: rolling_1d
      description: P99 Latency SLO for payment-api (500ms)
```

## Supported OpenSLO Features

### Metric Sources

NeuralBudget currently supports these metric source types:

| Source | Status | Notes |
|--------|--------|-------|
| **Prometheus** | ✅ Full | PromQL queries |
| **Datadog** | 🟡 Parsing | Full conversion in roadmap |
| **CloudWatch** | 🟡 Parsing | Full conversion in roadmap |

### Objectives

Supported objective types:

| Type | Status | Notes |
|------|--------|-------|
| **Ratio Metrics** | ✅ Full | Good/total ratio (availability) |
| **Threshold Metrics** | ✅ Full | Latency, error rate thresholds |
| **Window Types** | ✅ Full | rolling_1h, rolling_1d, rolling_7d, etc. |

## Migration Scenarios

### From Nobl9

1. **Export SLOs from Nobl9** in OpenSLO format:
   ```bash
   nobl9 slo export --format openslo > nobl9-slos.yaml
   ```

2. **Convert to NeuralBudget:**
   ```bash
   neuralbudget convert \
     --input nobl9-slos.yaml \
     --from openslo \
     --to neuralbudget > neuralbudget-slos.yaml
   ```

3. **Evaluate with NeuralBudget:**
   ```bash
   neuralbudget eval neuralbudget-slos.yaml metrics.json
   ```

### From Lightstep

1. **Export from Lightstep** (if available in OpenSLO format):
   ```bash
   lightstep slo export --format openslo > lightstep-slos.yaml
   ```

2. **Use same conversion process** as Nobl9

### Multi-SLO Migration

Convert entire directory of SLOs:

```bash
for file in *.yaml; do
  neuralbudget convert \
    --input "$file" \
    --from openslo \
    --to neuralbudget \
    --output "converted/$file"
done
```

## Known Limitations

### Limited to HTTP SLOs

Currently, OpenSLO conversion is optimized for **HTTP/gRPC SLOs** with availability and latency targets. Other SLO modes (Stateful, ML, GenAI) are on the roadmap.

### Latency Percentile Inference

OpenSLO doesn't explicitly specify latency percentile (P50, P99, etc.). NeuralBudget defaults to:
- **P99** for HTTP SLOs
- Can be overridden in converted YAML

**Example workaround:**

```yaml
# Edit after conversion to specify percentile
availability_threshold: 0.999
latency_threshold_ms: 200
latency_percentile: 0.95  # Override from default 0.99
```

### PromQL Query Preservation

When converting OpenSLO → NeuralBudget → OpenSLO, the original PromQL queries are replaced with NeuralBudget's canonical queries. If you need to preserve custom PromQL:

1. Keep original OpenSLO file as source of truth
2. Use NeuralBudget for evaluation only
3. Re-export to OpenSLO as needed

**Example canonical query (generated):**
```promql
rate(http_requests_total{status=~"2..",service="api-gateway"}[5m])
```

### Unsupported OpenSLO Features

| Feature | Status | Workaround |
|---------|--------|-----------|
| Alert rules | 🚫 | Use NeuralBudget's gen-rules command |
| Custom metric sources | 🟡 | Only Prometheus fully supported |
| SLI definitions | 🟡 | Inferred from objectives |
| Multiple windows per objective | 🟡 | Only first window used |

## Round-Trip Guarantees

OpenSLO → NeuralBudget → OpenSLO conversions are designed to be **lossy with documentation**:

- ✅ **Availability target** - Preserved exactly
- ✅ **Latency threshold** - Preserved exactly  
- ✅ **Service name** - Preserved exactly
- ⚠️ **PromQL queries** - Replaced with canonical form
- ⚠️ **Custom labels** - Simplified to standard labels
- ⚠️ **Descriptions** - May be truncated

## Python API

Convert programmatically in Python:

```python
from neuralbudget import openslo

# Parse OpenSLO
openslo_yaml = open("slo.yaml").read()
slo = openslo.parse_openslo_yaml(openslo_yaml)

# Extract service name
service = openslo.parse_openslo_service(openslo_yaml)

# Convert back to OpenSLO
converted = openslo.to_openslo_yaml(slo, service, "my-slo")
print(converted)
```

## Troubleshooting

### Error: "Unknown format"

```
Error: Invalid source format: yaml
Valid formats are: 'openslo', 'slo', 'neuralbudget', 'nb'
```

**Solution:** Use correct format names:
```bash
neuralbudget convert --from openslo --to neuralbudget ...
```

### Error: "Failed to parse OpenSLO"

```
Error: Failed to parse OpenSLO: invalid YAML at line 5
```

**Solution:** Validate your OpenSLO with:
```bash
yamllint openslo-slo.yaml
```

### Warning: "Multiple objectives found"

When converting OpenSLO with multiple objectives, NeuralBudget uses:
1. First objective as availability target
2. Second objective as latency threshold (if present)

**Solution:** If you need different behavior, manually edit the converted file.

### Missing service name

```
Error: Service name required when converting to OpenSLO
```

**Solution:** Provide service name:
```bash
neuralbudget convert --service my-service ...
```

## Best Practices

1. **Keep OpenSLO as source** if you need portability across tools
2. **Use NeuralBudget for evaluation** of converted SLOs
3. **Validate after conversion** with `neuralbudget check`
4. **Test round-trips** for critical SLOs to catch data loss
5. **Document custom changes** made post-conversion

## See Also

- [OpenSLO Specification](https://github.com/openslo/spec)
- [CNCF OpenSLO Project](https://www.cncf.io/)
- [Prometheus Rule Generation](prometheus-rule-generation.md)
- [NeuralBudget User Guide](user-guide.md)

## Examples

### Complete Migration Example

**Step 1: Export from source tool**
```bash
# Nobl9
nobl9 slo export --format openslo > nobl9-export.yaml
```

**Step 2: Convert to NeuralBudget**
```bash
neuralbudget convert \
  --input nobl9-export.yaml \
  --from openslo \
  --to neuralbudget \
  > neuralbudget-slos.yaml
```

**Step 3: Validate**
```bash
neuralbudget check neuralbudget-slos.yaml
```

**Step 4: Evaluate**
```bash
neuralbudget eval neuralbudget-slos.yaml metrics.json
```

**Step 5: Generate rules (optional)**
```bash
neuralbudget gen-rules neuralbudget-slos.yaml --kubernetes \
  | kubectl apply -f -
```

## Future Roadmap

- ✅ OpenSLO → NeuralBudget (basic)
- ✅ NeuralBudget → OpenSLO (basic)
- 🟡 Full support for all metric sources
- 🟡 Stateful SLO mapping
- 🟡 ML SLO mapping
- 🟡 Schema validation
- 🟡 Batch conversion with progress reporting
