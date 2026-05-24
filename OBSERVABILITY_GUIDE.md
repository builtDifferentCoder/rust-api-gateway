# API Gateway Observability and Metrics Guide

## Overview

The API Gateway includes production-ready observability features using Prometheus metrics and structured logging with tracing.

## Features

### Prometheus Metrics

The gateway exposes Prometheus metrics at the `/metrics` endpoint (public, no authentication required).

#### Available Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `api_gateway_http_requests_total` | Counter | Total HTTP requests received | `method`, `path`, `status` |
| `api_gateway_http_request_duration_seconds` | Histogram | HTTP request duration | `method`, `path` |
| `api_gateway_http_response_status` | Counter | Response status code distribution | `status` |
| `api_gateway_http_requests_per_route` | Counter | Requests per route | `route` |

#### Metric Buckets

Request duration histogram uses the following buckets (in seconds):
- 0.001 (1ms)
- 0.01 (10ms)
- 0.05 (50ms)
- 0.1 (100ms)
- 0.25 (250ms)
- 0.5 (500ms)
- 1.0 (1s)
- 2.5 (2.5s)
- 5.0 (5s)

### Structured Tracing

The gateway uses structured JSON logging with the `tracing` crate.

#### Log Format

Logs include:
- Timestamp
- Log level
- Target module
- Thread ID
- File and line number
- Message
- Structured fields (method, path, status, duration_ms, etc.)

#### Tracing Configuration

Tracing can be controlled via the `RUST_LOG` environment variable:

```bash
# All logs
export RUST_LOG=info

# Only gateway logs
export RUST_LOG=rust_api_gateway=debug

# Specific module
export RUST_LOG=rust_api_gateway::observability=trace
```

## Accessing Metrics

### Local Access

```bash
curl http://localhost:8080/metrics
```

### Prometheus Integration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'api-gateway'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

See `prometheus-scrape-config.yml` for a complete example.

## Example Queries

### Request Rate

Total requests per minute:
```
rate(api_gateway_http_requests_total[1m])
```

### Request Duration

95th percentile latency:
```
histogram_quantile(0.95, rate(api_gateway_http_request_duration_seconds_bucket[5m]))
```

99th percentile latency:
```
histogram_quantile(0.99, rate(api_gateway_http_request_duration_seconds_bucket[5m]))
```

Average request duration:
```
rate(api_gateway_http_request_duration_seconds_sum[5m]) / rate(api_gateway_http_request_duration_seconds_count[5m])
```

### Request Distribution

Requests per HTTP method:
```
sum(rate(api_gateway_http_requests_total[5m])) by (method)
```

Requests per route:
```
api_gateway_http_requests_per_route
```

### Error Rates

5xx error rate:
```
sum(rate(api_gateway_http_requests_total{status=~"5.."}[5m])) / sum(rate(api_gateway_http_requests_total[5m]))
```

4xx error rate:
```
sum(rate(api_gateway_http_requests_total{status=~"4.."}[5m])) / sum(rate(api_gateway_http_requests_total[5m]))
```

### Status Code Distribution

Response status counts:
```
api_gateway_http_response_status
```

## Logging Examples

### Request Completion Log

```json
{
  "timestamp": "2026-05-24T12:34:56.789Z",
  "level": "INFO",
  "target": "rust_api_gateway::server",
  "thread_id": "1",
  "file": "src/server/mod.rs",
  "line": 42,
  "message": "Request completed",
  "method": "GET",
  "path": "/users",
  "status": 200,
  "duration_ms": 145.23
}
```

## Performance Considerations

### Metrics Overhead

- Metrics are recorded in microseconds
- Minimal overhead: <1ms per request
- Memory usage: O(n) where n = unique (method, path, status) combinations

### Tracing Overhead

- JSON formatting adds <1ms per request
- Can be disabled by setting `RUST_LOG=off`
- Only enabled if `tracing-subscriber` is initialized

## Best Practices

1. **Prometheus Scrape Interval**: Set to 10-15 seconds
2. **Data Retention**: Prometheus default is 15 days (configurable)
3. **Alerting**: Set up alerts for:
   - High error rates (>1%)
   - High latency (p95 > 1s)
   - Rate limiter activation
4. **Dashboard**: Use Grafana or Prometheus UI for visualization

## Monitoring Checklist

- [ ] Prometheus scraping metrics endpoint successfully
- [ ] No "rate limiter exceeded" spikes during normal traffic
- [ ] Request latency p95 < 500ms
- [ ] Error rate < 0.1%
- [ ] Structured logs appear in log aggregator

## Troubleshooting

### Metrics endpoint returns empty

Ensure `/metrics` route is accessible and not behind authentication.

```bash
curl -v http://localhost:8080/metrics
```

### High latency spikes

Check individual route latencies:

```
histogram_quantile(0.95, rate(api_gateway_http_request_duration_seconds_bucket{path="/users"}[5m]))
```

### Memory growth

Monitor metric cardinality (unique combinations of labels). High cardinality can be addressed by:
- Reducing path specificity
- Aggregating similar routes
- Implementing metric pruning

## Future Enhancements

Planned observability features (not yet implemented):
- Distributed tracing with OpenTelemetry
- Jaeger integration
- Grafana dashboard templates
- Custom alerting rules
- User-based quotas with metrics
- Per-upstream metrics
