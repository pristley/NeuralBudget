# CLI Development Guide

## Architecture

The neuralbudget CLI is organized into modular subcommands:

```
src/bin/
├── main.rs           # CLI entry point with Clap parser
└── commands/
    ├── mod.rs        # Module organization
    ├── eval.rs       # Evaluation subcommand
    ├── gen_rules.rs  # Rule generation
    ├── check.rs      # Configuration validation
    └── serve.rs      # HTTP server (future)
```

## Building

### Development Build
```bash
cargo build --bin neuralbudget
```

### Release Build (Optimized)
```bash
cargo build --release --bin neuralbudget
```

### Run Tests
```bash
cargo test --bin neuralbudget
cargo test --test cli_integration_tests
```

## Adding a New Subcommand

### 1. Define Command in main.rs

Add variant to `Commands` enum:
```rust
enum Commands {
    // ... existing commands
    #[command(about = "Description of new command")]
    NewCommand {
        #[arg(help = "First argument")]
        arg1: String,
        
        #[arg(long, help = "Optional flag")]
        flag1: bool,
    },
}
```

### 2. Create Command Module

Create `src/bin/commands/new_command.rs`:
```rust
use anyhow::Result;

pub fn run(arg1: &str, flag1: bool) -> Result<()> {
    // Implementation
    Ok(())
}
```

### 3. Update Module Declarations

In `src/bin/commands/mod.rs`:
```rust
pub mod new_command;
```

### 4. Wire Up in main.rs

In main function:
```rust
Commands::NewCommand { arg1, flag1 } => {
    commands::new_command::run(&arg1, flag1)?;
}
```

### 5. Add Tests

Create or update `tests/cli_integration_tests.rs` with test cases.

## Error Handling

Use `anyhow::Result` with context:

```rust
use anyhow::{Context, Result};
use std::fs;

pub fn run(path: &str) -> Result<()> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read file: {}", path))?;
    
    // Process content
    Ok(())
}
```

Error output should be user-friendly:
```
Error: Failed to read configuration file: slo.yaml
  Caused by: No such file or directory (os error 2)
```

## Output Formatting

### Human-Readable Output
Use clear, structured format:
```
✓ SLO PASS

Availability:        99.95%
P99 Latency:         187ms
Requests Passed:     9995/10000
```

### JSON Output
When `--json` flag is used:
```json
{
  "status": "PASS",
  "availability_percent": 99.95,
  "p99_latency_ms": 187,
  "requests_total": 10000,
  "requests_passed": 9995
}
```

### Verbose Output
For `--verbose` flag:
```
[DEBUG] Loading config from: slo.yaml
[DEBUG] Parsing YAML...
[DEBUG] Config loaded: service=payment-api, target=99.95
[DEBUG] Loading sample from: sample.json
[DEBUG] Parsing JSON...
[DEBUG] Sample loaded: requests_total=10000, requests_successful=9995
[DEBUG] Evaluating SLO...
✓ SLO PASS
```

## Testing Strategy

### Unit Tests
Test individual command functions in isolation:
```rust
#[test]
fn test_eval_with_valid_files() {
    // Create temp files
    // Run command
    // Assert output
}
```

### Integration Tests
Test full command execution via subprocess:
```rust
let output = Command::new("cargo")
    .args(&["run", "--bin", "neuralbudget", "--", "eval", "slo.yaml", "sample.json"])
    .output()?;
```

### Manual Testing
```bash
# Create test config
cat > test_slo.yaml << 'EOF'
service: "test-api"
target: 99.9
EOF

# Create test sample
cat > test_sample.json << 'EOF'
{"requests_total": 100, "requests_successful": 99}
EOF

# Test each subcommand
cargo build --bin neuralbudget
./target/debug/neuralbudget eval test_slo.yaml test_sample.json
./target/debug/neuralbudget gen-rules test_slo.yaml
./target/debug/neuralbudget check test_slo.yaml
```

## Cross-Compilation

### Linux x86_64 (Default)
```bash
cargo build --release --bin neuralbudget --target x86_64-unknown-linux-gnu
```

### Linux ARM64
```bash
# Install cross
cargo install cross

# Build
cross build --release --bin neuralbudget --target aarch64-unknown-linux-gnu
```

### macOS
```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --release --bin neuralbudget --target x86_64-apple-darwin
cargo build --release --bin neuralbudget --target aarch64-apple-darwin
```

### Windows
```bash
rustup target add x86_64-pc-windows-msvc
cargo build --release --bin neuralbudget --target x86_64-pc-windows-msvc
```

## CI/CD Integration

### GitHub Actions Build
```yaml
- name: Build neuralbudget CLI
  run: cargo build --release --bin neuralbudget
  
- name: Run CLI tests
  run: cargo test --test cli_integration_tests
```

### Docker Build
```bash
docker build -t neuralbudget:latest .
docker run neuralbudget:latest eval slo.yaml sample.json
```

## Performance Targets

- **eval**: < 100ms for typical config/sample pair
- **gen-rules**: < 50ms for rule generation
- **check**: < 30ms for configuration validation

Benchmark:
```bash
cargo build --release --bin neuralbudget
time ./target/release/neuralbudget eval examples/slo_http.yaml examples/sample_http.json
```

## Dependencies

Core CLI dependencies:
- `clap 4.5` - Command-line argument parsing
- `anyhow 1.0` - Error handling
- `serde_yaml 0.9` - YAML parsing
- `tokio 1.40` - Async runtime (for future serve command)

These are carefully selected for:
- Minimal binary size (stripped)
- Apache 2.0/MIT licensing only
- Stable, well-maintained projects

## Future Enhancements

1. **HTTP Server Mode** (serve subcommand)
   - POST /eval for on-demand evaluation
   - GET /rules for rule generation
   - WebSocket support for real-time updates

2. **Configuration Management**
   - Remote config loading (HTTP/S3)
   - Config templates and inheritance
   - Built-in config validation schemas

3. **Streaming Mode**
   - Accept metrics stream via stdin
   - Real-time SLO monitoring
   - Webhook notifications on status changes

4. **Interactive Mode**
   - REPL for testing SLO configs
   - Real-time metric simulation
   - Visual dashboard (terminal UI)

## Troubleshooting

### Build Fails with "PyO3" Error
The CLI binary doesn't depend on PyO3. This error is from the library. Rebuild:
```bash
cargo clean
cargo build --release --bin neuralbudget
```

### Binary Size Too Large
Apply strip and compression:
```bash
cargo build --release --bin neuralbudget
strip target/release/neuralbudget
upx --best --lzma target/release/neuralbudget -o neuralbudget-compressed
```

### Cross-Compilation Fails
Use the `cross` tool for consistent builds:
```bash
cargo install cross
cross build --release --bin neuralbudget --target aarch64-unknown-linux-gnu
```
