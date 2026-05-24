# API Documentation

- GET /metrics
  - Prometheus text exposition of metrics

- Reverse-proxied routes
  - Configured in `config/config.toml` under `routes`
  - Gateway will forward requests to upstreams defined per route

Error responses

- 429 Too Many Requests — JSON `{ "error": "rate_limited", "message": "..." }`
- 503 Service Unavailable — JSON `{ "error": "service_unavailable", "message": "All upstream services are unavailable" }`
- 502 Bad Gateway — returned when the gateway fails to contact an upstream

Observability

- Tracing: JSON structured logs via `tracing` / `tracing-subscriber`
- Metrics available at `/metrics` for Prometheus to scrape
