# HTTP SLO Quick Start (5 Minutes)

Monitor availability and latency for HTTP/gRPC APIs in 5 minutes.

## ⚡ What You'll Do

1. Copy `slo.yaml` and `sample.json`
2. Run one command
3. See ✓ SLO PASS or ✗ SLO FAIL

## 📋 Step-by-Step

### Step 1: Copy the Files

```bash
# Copy slo.yaml
cp slo.yaml ./

# Copy sample.json  
cp sample.json ./
```

### Step 2: Install neuralbudget (if needed)

```bash
cargo install neuralbudget
# or
pip install neuralbudget
```

### Step 3: Evaluate

```bash
neuralbudget eval slo.yaml sample.json
```

### Expected Output

```
✓ SLO PASS
  Availability: 99.90% ✓ (target: 99.90%)
  Latency P99: 187.5ms ✓ (threshold: 200ms)
  Error Budget Used: 0.001% (30d window)
```

### Step 4: Experiment

**Make it FAIL** - Increase failures in `sample.json`:

```json
"successful": 49950,   // Change to 49800
```

Then re-run:

```bash
neuralbudget eval slo.yaml sample.json
```

Expected:
```
✗ SLO FAIL
  Availability: 99.60% ✗ (target: 99.90%)
  Alert: Fast burn rate triggered!
```

## 🎯 What's Happening

| Config | Meaning |
|--------|---------|
| `target: 99.9` | 99.9% uptime SLO |
| `latency_threshold_ms: 200` | P99 latency must be < 200ms |
| `alerts.1h.threshold: 0.10` | Alert if burning error budget at 10%/hour rate |

## 📚 Next Steps

- **Customize thresholds**: Edit `slo.yaml` to match your SLOs
- **Load real data**: Replace `sample.json` with Prometheus metrics
- **Set up alerts**: See [Advanced Alert Dispatch](../../guides/advanced_alert_dispatch.md)
- **Kubernetes**: See [Kubernetes Integration](../../guides/kubernetes-integration.md)
- **Production**: See [Production Deployment](../../guides/production-deployment.md)

## 🔗 Learn More

- [Full SLO Guide](../../guides/user-guide.md)
- [API Reference](../../reference/api.md)
- [Troubleshooting](../../guides/troubleshooting.md)
