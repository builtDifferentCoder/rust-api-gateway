# Rust API Gateway

Production-ready notes and quickstart.

- Build locally: `cargo build --release`
- Run locally: `HOST=0.0.0.0 PORT=8080 ./target/release/rust-api-gateway`

Docker

- Build: `docker build -t rust-api-gateway .`
- Run: `docker run -e PORT=8080 -p 8080:8080 rust-api-gateway`

Environment variables

- `HOST` (default `0.0.0.0`)
- `PORT` (default `8080`)
- `RATE_LIMIT_REQUESTS_PER_MINUTE` (overrides config)
- `HEALTH_INTERVAL_SECONDS` (overrides config)
- `RUST_LOG` (controls tracing level)

Deploying to Render

- Use the provided `Dockerfile` and `render.yaml` or configure a Docker-based service and set `PORT` to `8080`.

Graceful shutdown

- The server listens for `SIGINT`/`SIGTERM` and will perform a graceful shutdown.

Metrics

- Prometheus metrics are exposed at `/metrics`.

For more details see ARCHITECTURE.md and API_DOCS.md.
