# neuralbudget CLI

The command-line interface for the neuralbudget library - a Rust-first tool for SLO data modeling and error budget analysis.

## Features

✅ **Evaluate SLOs** - Check if your service meets SLO targets  
✅ **Generate Rules** - Create Prometheus alerting rules from SLO configs  
✅ **Validate Configs** - Check configurations for common mistakes  
🔜 **HTTP Server** - RESTful API for SLO evaluation (coming soon)  

## Quick Start

### Install

**macOS (Homebrew)**
```bash
brew install neuralbudget
```

**Linux (AUR)**
```bash
yay -S neuralbudget
```

**Docker**
```bash
docker run ghcr.io/neuralbudget/neuralbudget:latest eval --help
```

**From source**
```bash
cargo install --path . --bin neuralbudget
```

### Usage

```bash
# Evaluate an SLO
neuralbudget eval slo.yaml sample.json

# Generate Prometheus rules
neuralbudget gen-rules slo.yaml

# Validate configuration
neuralbudget check slo.yaml

# Get help
neuralbudget --help
neuralbudget eval --help
```

## Configuration

Example `slo.yaml`:
```yaml
service: "payment-api"
target: 99.95
latency_threshold_ms: 200
window: "30d"
```

Example `sample.json`:
```json
{
  "requests_total": 10000,
  "requests_successful": 9995,
  "latency_p99_ms": 187.5
}
```

## Commands

### eval - Evaluate SLO

Check if metrics meet SLO targets.

```bash
neuralbudget eval <CONFIG> <SAMPLE> [--json] [--verbose]
```

Options:
- `--json` - Output as JSON
- `--verbose` - Show debug information

**Example:**
```bash
neuralbudget eval slo.yaml sample.json --json | jq .
```

### gen-rules - Generate Prometheus Rules

Create alerting and recording rules from SLO config.

```bash
neuralbudget gen-rules <CONFIG> [--kubernetes] [--namespace NAMESPACE]
```

Options:
- `--kubernetes` - Output as PrometheusRule CRD
- `--namespace` - Kubernetes namespace (default: monitoring)

**Example:**
```bash
# Plain YAML rules
neuralbudget gen-rules slo.yaml > rules.yaml

# Kubernetes CRD
neuralbudget gen-rules slo.yaml --kubernetes | kubectl apply -f -
```

### check - Validate Configuration

Validate SLO configuration and warn on issues.

```bash
neuralbudget check <CONFIG> [--strict]
```

Options:
- `--strict` - Treat warnings as errors

**Example:**
```bash
neuralbudget check slo.yaml --strict || exit 1
```

### serve - HTTP Server (Coming Soon)

Run an HTTP server for SLO evaluation.

```bash
neuralbudget serve [--port PORT] [--bind ADDRESS]
```

## Examples

### CI/CD Integration

```yaml
# .github/workflows/validate-slos.yml
name: Validate SLOs
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: |
          curl -sSL https://github.com/pristley/neuralbudget/releases/download/v0.1.3/neuralbudget-linux-x86_64 -o neuralbudget
          chmod +x neuralbudget
          ./neuralbudget check slo.yaml --strict
```

### Docker Compose

```yaml
version: '3.8'
services:
  validate-slo:
    image: ghcr.io/neuralbudget/neuralbudget:latest
    volumes:
      - ./slo.yaml:/slo.yaml
      - ./sample.json:/sample.json
    command: eval /slo.yaml /sample.json --json
```

### Kubernetes CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: slo-evaluator
spec:
  schedule: "*/5 * * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: evaluator
            image: ghcr.io/neuralbudget/neuralbudget:latest
            command: ["neuralbudget", "eval", "/config/slo.yaml", "/metrics/sample.json"]
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
              name: slo-metrics
          restartPolicy: OnFailure
```

## Performance

- **eval**: < 100ms
- **gen-rules**: < 50ms
- **check**: < 30ms

## Output Formats

### Human Readable

```
✓ SLO PASS

Availability:        99.95%
P99 Latency:         187ms
Requests Passed:     9995/10000

Error Budget: 6.25 hours remaining (out of 10500 hours/month)
```

### JSON

```bash
neuralbudget eval slo.yaml sample.json --json
```

```json
{
  "status": "PASS",
  "service": "payment-api",
  "slo_target": 99.95,
  "actual_availability": 99.95,
  "latency_p99_ms": 187.5,
  "requests_total": 10000,
  "requests_passed": 9995,
  "error_budget_hours_remaining": 6.25
}
```

## Exit Codes

- `0` - Success (SLO passed or operation completed)
- `1` - Failure (SLO failed, validation error, or file not found)
- `2` - Invalid arguments or format error

## Error Handling

Clear error messages with context:

```
$ neuralbudget eval missing.yaml sample.json

Error: Failed to read configuration file
  Cause: No such file or directory (os error 2)
  File: missing.yaml
```

## Platform Support

| Platform | Status | Binary |
|----------|--------|--------|
| Linux x86_64 | ✅ | [neuralbudget-linux-x86_64](https://github.com/pristley/neuralbudget/releases/latest) |
| Linux ARM64 | ✅ | [neuralbudget-linux-arm64](https://github.com/pristley/neuralbudget/releases/latest) |
| macOS x86_64 | ✅ | [neuralbudget-macos-x86_64](https://github.com/pristley/neuralbudget/releases/latest) |
| macOS ARM64 | ✅ | [neuralbudget-macos-arm64](https://github.com/pristley/neuralbudget/releases/latest) |
| Windows | ✅ | [neuralbudget-windows-x86_64.exe](https://github.com/pristley/neuralbudget/releases/latest) |

## Build from Source

```bash
# Clone repository
git clone https://github.com/pristley/neuralbudget
cd neuralbudget

# Build CLI
cargo build --release --bin neuralbudget

# Run
./target/release/neuralbudget --help
```

## Docker

```bash
# Pull image
docker pull ghcr.io/neuralbudget/neuralbudget:latest

# Evaluate SLO
docker run -v $(pwd)/slo.yaml:/slo.yaml \
           -v $(pwd)/sample.json:/sample.json \
           ghcr.io/neuralbudget/neuralbudget:latest \
           eval /slo.yaml /sample.json

# Generate rules
docker run -v $(pwd)/slo.yaml:/slo.yaml \
           ghcr.io/neuralbudget/neuralbudget:latest \
           gen-rules /slo.yaml > prometheus-rules.yaml
```

## Development

See [CLI_DEVELOPMENT.md](../CLI_DEVELOPMENT.md) for development guidelines.

## Documentation

- [User Guide](../CLI_USER_GUIDE.md) - Comprehensive usage guide with examples
- [Development Guide](../CLI_DEVELOPMENT.md) - Building and extending the CLI
- [Main README](../README.md) - Full neuralbudget project documentation

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines.

## Support

- **Issues**: [GitHub Issues](https://github.com/pristley/neuralbudget/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pristley/neuralbudget/discussions)
- **Documentation**: [CLI_USER_GUIDE.md](../CLI_USER_GUIDE.md)

## Changelog

See [CHANGELOG.md](../CHANGELOG.md) for version history.
