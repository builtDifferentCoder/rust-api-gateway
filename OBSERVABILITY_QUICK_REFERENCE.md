# Observability Quick Reference

## Get Metrics

```bash
curl http://localhost:8080/metrics
```

## Environment Variables

```bash
# Set log level
export RUST_LOG=info                           # All logs at info level
export RUST_LOG=rust_api_gateway=debug         # Gateway only, debug level
export RUST_LOG=rust_api_gateway::observability=trace  # Specific module
```

## Prometheus Setup (5 minutes)

1. Add to `prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'api-gateway'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

2. Restart Prometheus

3. Visit `http://localhost:9090` and search for `api_gateway_`

## Key Metrics

| Metric | Query | Use Case |
|--------|-------|----------|
| Total requests | `rate(api_gateway_http_requests_total[5m])` | Overall load |
| P95 latency | `histogram_quantile(0.95, rate(api_gateway_http_request_duration_seconds_bucket[5m]))` | Performance |
| Error rate | `sum(rate(api_gateway_http_requests_total{status=~"5.."}[5m])) by (status)` | Health |
| Requests/route | `api_gateway_http_requests_per_route` | Traffic distribution |

## Common Queries

### Requests by method
```
sum(rate(api_gateway_http_requests_total[5m])) by (method)
```

### Latency by path
```
histogram_quantile(0.95, rate(api_gateway_http_request_duration_seconds_bucket[5m])) by (path)
```

### 4xx error rate
```
sum(rate(api_gateway_http_requests_total{status=~"4.."}[5m])) / sum(rate(api_gateway_http_requests_total[5m]))
```

### Rate limiter hits
```
increase(api_gateway_http_requests_total{status="429"}[5m])
```

## Log Output Example

```json
{
  "timestamp": "2026-05-24T12:34:56.789Z",
  "level": "INFO",
  "target": "rust_api_gateway::server",
  "message": "Request completed",
  "method": "GET",
  "path": "/users",
  "status": 200,
  "duration_ms": 145.23
}
```

## Monitoring Checklist

- [ ] `/metrics` endpoint accessible
- [ ] Prometheus scraping metrics
- [ ] Logs contain request data
- [ ] P95 latency < 500ms
- [ ] Error rate < 0.1%
- [ ] No memory leaks from metric cardinality

## Troubleshooting

**Issue**: Metrics endpoint returns 404  
**Fix**: Ensure `/metrics` route is not behind auth (it's public by default)

**Issue**: No requests appearing in metrics  
**Fix**: Restart the gateway after deploying observability code

**Issue**: High memory usage  
**Fix**: Check metric cardinality with `count(api_gateway_http_requests_total)` - if > 10k, reduce path specificity

**Issue**: No logs appearing  
**Fix**: Check `RUST_LOG` is set correctly, or logs are going to stderr

## Files Reference

- **Metrics Implementation**: `src/observability/metrics.rs`
- **Tracing Configuration**: `src/observability/tracing_config.rs`
- **Prometheus Config Example**: `prometheus-scrape-config.yml`
- **Full Documentation**: `OBSERVABILITY_GUIDE.md`
- **Implementation Details**: `OBSERVABILITY_IMPLEMENTATION.md`

## API Endpoints

- `GET /metrics` - Prometheus metrics (text/plain, 200 OK)

## Next Steps

1. Deploy the gateway
2. Configure Prometheus to scrape `/metrics`
3. Set up Grafana for visualization
4. Create alerts for:
   - High error rate (> 1%)
   - High latency (P95 > 1s)
   - Rate limiter activation

## Support

For issues with observability, check:
1. `OBSERVABILITY_GUIDE.md` - Full guide
2. `prometheus-scrape-config.yml` - Example config
3. Prometheus UI at `http://localhost:9090`
