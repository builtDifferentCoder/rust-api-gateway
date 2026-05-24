# Multi-stage Dockerfile for rust-api-gateway

# Builder
FROM rust:1.70 as builder
WORKDIR /app

# Copy manifests first for caching
COPY Cargo.toml Cargo.lock ./
COPY config ./config
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime image
FROM debian:buster-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/rust-api-gateway /usr/local/bin/rust-api-gateway

# Runtime defaults
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/rust-api-gateway"]
