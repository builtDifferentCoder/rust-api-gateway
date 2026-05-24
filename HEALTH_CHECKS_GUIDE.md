# Upstream Health Checks and Failover Guide

## Overview

The API Gateway includes automated health checking for upstream services. Unhealthy upstreams are automatically detected and excluded from traffic routing until they recover.

## Features

✅ Periodic HTTP health checks  
✅ Per-route configurable health endpoints  
✅ Automatic failover to healthy upstreams  
✅ 503 Service Unavailable when all upstreams are down  
✅ Automatic recovery detection  
✅ Background async tasks  
✅ Thread-safe state tracking  
✅ Structured logging

## Configuration

### Basic Setup

Update `config/config.toml`:

```toml
[health]
interval_seconds = 10

[[routes]]
path = "/users"
upstreams = [
  "http://localhost:3001",
  "http://localhost:3002",
]
health_path = "/health"

[[routes]]
path = "/orders"
upstreams = [
  "http://localhost:3003",
  "http://localhost:3004",
]
health_path = "/health"
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `interval_seconds` | u64 | 10 | Health check interval in seconds |
| `health_path` | string | "/health" | HTTP health endpoint per route |

## How It Works

### Health Check Process

1. **Periodic Polling**: Background tasks run every `interval_seconds`
2. **HTTP GET Request**: Sends GET request to `upstream + health_path`
3. **Status Check**: Expects 2xx response to mark healthy
4. **State Update**: Updates internal registry if status changes
5. **Logging**: Logs recovery/failure events

### Load Balancer Integration

When selecting an upstream for a request:

1. **Filter Healthy**: Only consider upstreams marked as healthy
2. **Round-Robin**: Select next healthy upstream in rotation
3. **Fallback**: If all unhealthy, return 503 Service Unavailable

### Example Flow

```
Request → GET /users
  ↓
Check health registry
  ↓
Filter to healthy upstreams: [localhost:3001, localhost:3002]
  ↓
Round-robin select: localhost:3001
  ↓
Proxy to localhost:3001/users
  ↓
Response
```

If all upstreams were unhealthy:

```
Request → GET /users
  ↓
Check health registry
  ↓
No healthy upstreams available
  ↓
Return 503 Service Unavailable
  ↓
{
  "error": "service_unavailable",
  "message": "All upstream services are unavailable"
}
```

## Health Check Details

### Successful Check

```
GET http://localhost:3001/health → 200 OK
→ Mark as Healthy
→ Log: "Upstream recovered to healthy"
```

### Failed Check

```
GET http://localhost:3001/health → 500 Internal Server Error
→ Mark as Unhealthy (reason: "HTTP 500")
→ Log: "Upstream marked as unhealthy"
```

### Connection Error

```
GET http://localhost:3001/health → Connection refused
→ Mark as Unhealthy (reason: "connection refused")
→ Log: "Upstream marked as unhealthy"
```

### Timeout

```
GET http://localhost:3001/health → No response (5 second timeout)
→ Mark as Unhealthy (reason: "timeout")
→ Log: "Upstream marked as unhealthy"
```

## Logging

Health check events are logged with structured tracing:

```json
{
  "target": "rust_api_gateway::health::checker",
  "level": "DEBUG",
  "message": "Upstream health check failed",
  "upstream": "http://localhost:3001",
  "reason": "timeout"
}
```

Recovery events are logged at WARNING level:

```json
{
  "target": "rust_api_gateway::health::registry",
  "level": "WARN",
  "message": "Upstream recovered to healthy",
  "upstream": "http://localhost:3001"
}
```

Enable debug logging:

```bash
export RUST_LOG=rust_api_gateway::health=debug
```

## Health Endpoint Requirements

Your upstream services should implement a `/health` endpoint (or custom path):

```rust
// Example in Rust with Axum
#[get("/health")]
async fn health() -> StatusCode {
    StatusCode::OK
}
```

```python
# Example in Python with Flask
@app.route('/health')
def health():
    return {'status': 'ok'}, 200
```

```javascript
// Example in Node.js with Express
app.get('/health', (req, res) => {
  res.status(200).json({ status: 'ok' });
});
```

**Requirements:**
- Returns HTTP 2xx status code for healthy
- Returns HTTP 5xx for unhealthy
- Responds within 5 seconds (configurable)
- Should be fast (<100ms)

## Architecture

### Components

**HealthRegistry**
- Thread-safe tracking of upstream status
- Uses `Arc<RwLock<HashMap>>` for concurrent access
- Supports recovery detection

**HealthChecker**
- Performs periodic HTTP GET requests
- Configurable timeout (default 5s)
- Logs failures and recovery

**RoundRobin Load Balancer**
- Enhanced with health-aware selection
- Filters unhealthy upstreams
- Falls back to sync selection if no registry

### Thread Safety

- `Arc<HealthRegistry>` shared across threads
- `RwLock<HashMap>` for concurrent reads/writes
- Async `tokio::spawn` tasks for background checks

## Monitoring

### Check Health Status

Access registry state in logs:

```bash
# Filter for health-related logs
export RUST_LOG=rust_api_gateway::health=debug

# Check output for "marked as unhealthy" or "recovered"
```

### Prometheus Metrics

The gateway records metrics including:
- Total requests per route
- Response status distribution
- Request latency histograms

These can be used to detect when upstreams are down:

```
# 503 responses indicate all upstreams unhealthy
rate(api_gateway_http_response_status{status="503"}[5m])
```

## Troubleshooting

### Health Checks Not Running

**Check:**
1. Routes configured in `config/config.toml`
2. `health_path` specified or defaults to `/health`
3. Logs show "Health check background tasks started"

```bash
export RUST_LOG=rust_api_gateway::server=info
cargo run
```

### Upstreams Always Marked Unhealthy

**Check:**
1. Upstream service is running
2. Health endpoint returns 2xx for healthy
3. Health endpoint responds within 5 seconds
4. Network connectivity (firewalls, routing)

**Test manually:**
```bash
curl -v http://localhost:3001/health
```

### No Failover to Healthy Upstreams

**Check:**
1. Multiple upstreams configured
2. At least one marked as healthy
3. Load balancer uses `next_async()` method
4. Health registry passed to router

## Performance

- Health checks run in background tasks (non-blocking)
- Check timeout: 5 seconds (configurable)
- Check interval: configurable (default 10 seconds)
- Per-request overhead: <1μs (hash lookup)
- Memory overhead: ~100 bytes per upstream

## Production Recommendations

1. **Health Endpoint**: Implement comprehensive checks (DB, disk, etc.)
2. **Interval**: Set to 10-30 seconds depending on SLA
3. **Timeout**: 5 seconds is reasonable for most services
4. **Alerting**: Alert when upstreams are down for >5 minutes
5. **Logging**: Enable debug logs to track state changes
6. **Metrics**: Monitor 503 responses and error rates

## Example: Multi-Region Failover

```toml
[[routes]]
path = "/api"
upstreams = [
  "http://us-east.service.local:8080",    # Primary
  "http://us-west.service.local:8080",    # Failover
  "http://eu-central.service.local:8080", # Fallback
]
health_path = "/api/health"

[health]
interval_seconds = 5
```

If `us-east` goes down:
- Detected within 5 seconds
- Traffic redirected to `us-west`
- When `us-east` recovers, traffic gradually redistributed

## Example: Canary Deployments

```toml
[[routes]]
path = "/v2"
upstreams = [
  "http://canary.service.local:8080",  # New version (monitoring)
  "http://stable.service.local:8080",  # Current version (primary)
  "http://stable2.service.local:8080", # Backup
]
health_path = "/health"

[health]
interval_seconds = 5
```

If canary becomes unhealthy, all traffic goes to stable versions.

## Future Enhancements

Not yet implemented:
- [ ] Circuit breaker pattern
- [ ] Active TCP connection checking
- [ ] Kubernetes discovery
- [ ] Retry logic with health-aware selection
- [ ] Custom health check predicates
- [ ] Health check metrics
