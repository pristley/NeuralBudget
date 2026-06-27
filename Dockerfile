# Multi-stage build for neuralbudget CLI
# Optimized for size and performance

FROM rust:1.82 as builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.toml
COPY src src

# Build the CLI binary in release mode
RUN cargo build --bin neuralbudget --release

# Runtime stage: lightweight image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/neuralbudget /usr/local/bin/

ENTRYPOINT ["neuralbudget"]
CMD ["--help"]
