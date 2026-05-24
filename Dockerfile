# ===== Builder Stage =====
FROM rust:1.87 AS builder

WORKDIR /app

# Copy project files
COPY . .

# Build release binary
RUN cargo build --release

# ===== Runtime Stage =====
FROM debian:bookworm-slim

# Install SSL certificates
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary
COPY --from=builder /app/target/release/rust-api-gateway /usr/local/bin/rust-api-gateway

# Environment variables
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

CMD ["rust-api-gateway"]