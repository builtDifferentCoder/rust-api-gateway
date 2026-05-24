# Health Checks Quick Reference

## Configuration

Add to `config/config.toml`:

```toml
[health]
interval_seconds = 10

[[routes]]
path = "/users"
upstreams = ["http://localhost:3001", "http://localhost:3002"]
health_path = "/health"
```

## How It Works

1. Gateway checks health endpoint every 10 seconds
2. `GET http://localhost:3001/health` → 200 OK = Healthy
3. Unhealthy upstreams are skipped
4. If ALL unhealthy → returns 503 Service Unavailable

## Health Endpoint Example

```python
@app.route('/health')
def health():
    return {'status': 'ok'}, 200
```

## Logging

```bash
# See health check logs
export RUST_LOG=rust_api_gateway::health=debug
cargo run
```

Output shows:
- "Upstream marked as unhealthy" (when detected)
- "Upstream recovered to healthy" (when recovered)
- Reasons: "timeout", "connection refused", "HTTP 500", etc.

## Testing

### Test health endpoint manually
```bash
curl http://localhost:3001/health
```

### Simulate failure
Stop a service and check logs - gateway should mark it unhealthy within 10 seconds

### Verify failover
```bash
curl http://localhost:8080/users  # Should still work if backup is healthy
```

## Metrics

Check gateway metrics at `GET /metrics`:

```
# Requests returning 503
api_gateway_http_response_status{status="503"}

# Requests by route (healthy upstreams only)
api_gateway_http_requests_per_route{route="/users"}
```

## Configuration Reference

| Parameter | Default | Description |
|-----------|---------|-------------|
| `interval_seconds` | 10 | Check frequency |
| `health_path` | "/health" | Health endpoint per route |
| `timeout_seconds` | 5 | HTTP request timeout |

## Common Issues

**Problem**: "All upstream services are unavailable" (503 errors)
- **Fix**: Ensure upstream health endpoint returns 2xx status
- **Check**: `curl http://localhost:3001/health`

**Problem**: Requests still going to unhealthy upstreams
- **Check**: Logs show health checks are running?
- **Fix**: Ensure `health_path` matches your endpoint

**Problem**: Health checks not starting
- **Fix**: Ensure routes are configured in `config.toml`
- **Check**: Server startup logs should say "Health check background tasks started"

## Status Codes

| Status | Meaning |
|--------|---------|
| 2xx | Healthy ✓ |
| 5xx | Unhealthy ✗ (gateway marks down) |
| 4xx | Unhealthy ✗ (client error, likely misconfigured) |
| Timeout | Unhealthy ✗ (>5 seconds) |
| Connection refused | Unhealthy ✗ (service down) |

## Files Reference

- [Full Guide](HEALTH_CHECKS_GUIDE.md)
- Implementation: `src/health/`
  - `registry.rs` - Health state tracking
  - `checker.rs` - Periodic checks
  - `mod.rs` - Exports

## Example: Production Setup

```toml
# Production config with 3 redundant upstreams
[health]
interval_seconds = 10

[[routes]]
path = "/api"
upstreams = [
  "http://api-1.production.local:8080",
  "http://api-2.production.local:8080",
  "http://api-3.production.local:8080",
]
health_path = "/health"

[[routes]]
path = "/db"
upstreams = [
  "http://db-1.production.local:5432",
  "http://db-2.production.local:5432",
]
health_path = "/health"
```

Any upstream failure is automatically detected and traffic rerouted within 10 seconds.
